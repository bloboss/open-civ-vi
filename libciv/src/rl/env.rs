//! Gym-like environment wrapper around the game engine.
//!
//! [`CivEnv`] provides `reset` / `step` / `available_actions` for RL training.

use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::MovementCost;

use crate::ai::deterministic::HeuristicAgent;
use crate::civ::city::ProductionItem;
use crate::civ::civilization::{Leader, TechProgress};
use crate::civ::{BasicUnit, BuiltinAgenda, City, CityKind, Civilization};
use crate::game::state::{GameState, UnitTypeDef};
use crate::game::visibility::recalculate_visibility;
use crate::game::{compute_score, BuiltinVictoryCondition, DefaultRulesEngine, RulesEngine};
use crate::world::mapgen::{self, MapGenConfig};
use crate::{CivId, UnitCategory, UnitDomain, UnitTypeId};

use super::action::Action;
use super::observation::{observe, Observation};
use super::reward::compute_reward;

// ── Public result types ─────────────────────────────────────────────────────

/// The result of a single `step()` call.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub observation: Observation,
    pub reward: f64,
    pub done: bool,
    pub info: StepInfo,
}

/// Auxiliary information returned alongside each step.
#[derive(Debug, Clone)]
pub struct StepInfo {
    pub turn: u32,
    /// `Ok(())` when the action was applied successfully; `Err(msg)` when it
    /// was rejected by the rules engine (the turn still advances on `EndTurn`).
    pub action_result: Result<(), String>,
}

// ── CivEnv ────────────��─────────────────────────────────────────────────────

/// A gym-like environment that wraps the full game engine.
///
/// One environment instance manages a single game between an RL agent (playing
/// `agent_civ`) and one or more heuristic opponents.
pub struct CivEnv {
    pub state: GameState,
    rules: DefaultRulesEngine,
    agent_civ: CivId,
    opponent_agents: Vec<(CivId, HeuristicAgent)>,
    prev_score: u32,
}

impl std::fmt::Debug for CivEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CivEnv")
            .field("turn", &self.state.turn)
            .field("agent_civ", &self.agent_civ)
            .finish_non_exhaustive()
    }
}

impl CivEnv {
    /// Create a new environment. Does **not** generate the map or place civs;
    /// call [`reset`](Self::reset) to initialise the game.
    pub fn new(seed: u64, map_width: u32, map_height: u32) -> Self {
        let mut env = Self {
            state: GameState::new(seed, map_width, map_height),
            rules: DefaultRulesEngine,
            agent_civ: CivId::from_ulid(ulid::Ulid::nil()),
            opponent_agents: Vec::new(),
            prev_score: 0,
        };
        env.setup_game(seed, map_width, map_height);
        env
    }

    /// Reset the environment to a fresh game with the given seed. Returns the
    /// initial observation for the agent's civilisation.
    pub fn reset(&mut self, seed: u64) -> Observation {
        let w = self.state.board.width();
        let h = self.state.board.height();
        self.state = GameState::new(seed, w, h);
        self.opponent_agents.clear();
        self.setup_game(seed, w, h);
        self.prev_score = compute_score(&self.state, self.agent_civ);
        observe(&self.state, self.agent_civ)
    }

    /// Apply an action and return the resulting observation, reward, and done flag.
    pub fn step(&mut self, action: Action) -> StepResult {
        let action_result = self.dispatch_action(&action);

        // If the action was EndTurn, run the turn pipeline.
        if matches!(action, Action::EndTurn) {
            // Opponent agents take their turns.
            for (civ_id, agent) in &self.opponent_agents {
                let _ = crate::ai::deterministic::Agent::take_turn(
                    agent,
                    &mut self.state,
                    &self.rules,
                );
                recalculate_visibility(&mut self.state, *civ_id);
            }

            // Advance the game turn (production, research, diplomacy, victories, etc.).
            let _ = self.rules.advance_turn(&mut self.state);

            // Recalculate agent visibility after the turn.
            recalculate_visibility(&mut self.state, self.agent_civ);
        }

        let curr_score = compute_score(&self.state, self.agent_civ);
        let obs = observe(&self.state, self.agent_civ);
        let done = obs.game_over;
        let won = obs.is_winner;
        let reward = compute_reward(self.prev_score, curr_score, done, won);
        self.prev_score = curr_score;

        StepResult {
            observation: obs,
            reward,
            done,
            info: StepInfo {
                turn: self.state.turn,
                action_result: action_result.map_err(|e| e.to_string()),
            },
        }
    }

    /// Enumerate all currently valid actions for the agent's civilisation.
    ///
    /// This is intentionally simplified: movement to every passable neighbor,
    /// attack against adjacent enemies, founding cities with settlers, and
    /// production queueing in idle cities.
    pub fn available_actions(&self) -> Vec<Action> {
        let mut actions = vec![Action::EndTurn];
        let civ_id = self.agent_civ;

        // Per-unit actions.
        for unit in self.state.units.iter().filter(|u| u.owner == civ_id) {
            if unit.movement_left == 0 {
                continue;
            }

            for neighbor in unit.coord.neighbors() {
                // Check that the tile exists and is passable.
                if let Some(tile) = self.state.board.tile(neighbor)
                    && tile.terrain.movement_cost() != MovementCost::Impassable
                {
                    actions.push(Action::MoveUnit {
                        unit: unit.id,
                        to: neighbor,
                    });
                }
            }

            // Attack: adjacent enemy units.
            if unit.combat_strength.is_some() {
                for enemy in self.state.units.iter().filter(|e| e.owner != civ_id) {
                    if unit.coord.distance(&enemy.coord) <= 1 {
                        actions.push(Action::Attack {
                            attacker: unit.id,
                            target: enemy.id,
                        });
                    }
                }
            }

            // Found city with settler.
            let is_settler = self
                .state
                .unit_type_defs
                .iter()
                .any(|d| d.id == unit.unit_type && d.can_found_city);
            if is_settler {
                actions.push(Action::FoundCity {
                    settler: unit.id,
                    name: format!("City_{}", self.state.cities.len() + 1),
                });
            }
        }

        // Per-city production actions: if the city has an empty production queue,
        // offer every unit type that could be produced.
        for city in self.state.cities.iter().filter(|c| {
            c.owner == civ_id && !matches!(c.kind, CityKind::CityState(_))
        }) {
            if city.production_queue.is_empty() {
                for def in &self.state.unit_type_defs {
                    actions.push(Action::QueueProduction {
                        city: city.id,
                        item: ProductionItem::Unit(def.id),
                    });
                }
            }
        }

        actions
    }

    // ── Private helpers ──────��──────────────────��───────────────────────────

    /// Set up a minimal two-civilisation game with mapgen, starting cities, and units.
    fn setup_game(&mut self, seed: u64, width: u32, height: u32) {
        // Generate terrain.
        let mapgen_result = mapgen::generate(
            &MapGenConfig {
                width,
                height,
                seed,
                land_fraction: None,
                num_continents: None,
                num_zone_seeds: None,
                num_starts: 2,
            },
            &mut self.state.board,
        );

        let starts = &mapgen_result.starting_positions;
        let agent_start = starts
            .first()
            .copied()
            .unwrap_or(HexCoord::from_qr(3, 3));
        let opponent_start = starts
            .get(1)
            .copied()
            .unwrap_or(HexCoord::from_qr(width as i32 - 4, height as i32 - 4));

        // Unit type registry: warrior + settler.
        let warrior_type = UnitTypeId::from_ulid(self.state.id_gen.next_ulid());
        let settler_type = UnitTypeId::from_ulid(self.state.id_gen.next_ulid());
        self.state.unit_type_defs.extend([
            UnitTypeDef {
                id: warrior_type,
                name: "warrior",
                production_cost: 40,
                max_movement: 200,
                combat_strength: Some(20),
                domain: UnitDomain::Land,
                category: UnitCategory::Combat,
                range: 0,
                vision_range: 2,
                can_found_city: false,
                resource_cost: None,
                siege_bonus: 0,
                max_charges: 0,
                exclusive_to: None,
                replaces: None,
                era: None,
                promotion_class: None,
            },
            UnitTypeDef {
                id: settler_type,
                name: "settler",
                production_cost: 80,
                max_movement: 200,
                combat_strength: None,
                domain: UnitDomain::Land,
                category: UnitCategory::Civilian,
                range: 0,
                vision_range: 2,
                can_found_city: true,
                resource_cost: None,
                siege_bonus: 0,
                max_charges: 0,
                exclusive_to: None,
                replaces: None,
                era: None,
                promotion_class: None,
            },
        ]);

        // ── Agent civilisation ──────────────────────────────────────────────
        let agent_id = self.state.id_gen.next_civ_id();
        self.agent_civ = agent_id;
        let leader = Leader {
            name: "Agent",
            civ_id: agent_id,
            agenda: BuiltinAgenda::Default,
        };
        self.state
            .civilizations
            .push(Civilization::new(agent_id, "Agent", "Agent", leader));

        let agent_city_id = self.state.id_gen.next_city_id();
        let mut agent_city =
            City::new(agent_city_id, "AgentCapital".to_string(), agent_id, agent_start);
        agent_city.is_capital = true;
        self.state.cities.push(agent_city);
        self.state
            .civilizations
            .iter_mut()
            .find(|c| c.id == agent_id)
            .unwrap()
            .cities
            .push(agent_city_id);

        // Starting warrior.
        let agent_warrior = self.state.id_gen.next_unit_id();
        self.state.units.push(BasicUnit {
            id: agent_warrior,
            unit_type: warrior_type,
            owner: agent_id,
            coord: agent_start,
            domain: UnitDomain::Land,
            category: UnitCategory::Combat,
            movement_left: 200,
            max_movement: 200,
            combat_strength: Some(20),
            promotions: Vec::new(),
            experience: 0,
            health: 100,
            range: 0,
            vision_range: 2,
            charges: None,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
            is_embarked: false,
        });

        // ── Opponent civilisation ─────���─────────────────────────────────────
        let opp_id = self.state.id_gen.next_civ_id();
        let opp_leader = Leader {
            name: "Opponent",
            civ_id: opp_id,
            agenda: BuiltinAgenda::Default,
        };
        self.state
            .civilizations
            .push(Civilization::new(opp_id, "Opponent", "Opposing", opp_leader));

        let opp_city_id = self.state.id_gen.next_city_id();
        let mut opp_city = City::new(
            opp_city_id,
            "OpponentCapital".to_string(),
            opp_id,
            opponent_start,
        );
        opp_city.is_capital = true;
        self.state.cities.push(opp_city);
        self.state
            .civilizations
            .iter_mut()
            .find(|c| c.id == opp_id)
            .unwrap()
            .cities
            .push(opp_city_id);

        // Opponent starting warrior.
        let opp_warrior = self.state.id_gen.next_unit_id();
        self.state.units.push(BasicUnit {
            id: opp_warrior,
            unit_type: warrior_type,
            owner: opp_id,
            coord: opponent_start,
            domain: UnitDomain::Land,
            category: UnitCategory::Combat,
            movement_left: 200,
            max_movement: 200,
            combat_strength: Some(20),
            promotions: Vec::new(),
            experience: 0,
            health: 100,
            range: 0,
            vision_range: 2,
            charges: None,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
            is_embarked: false,
        });

        // Register opponent heuristic agent.
        self.opponent_agents
            .push((opp_id, HeuristicAgent { civ_id: opp_id }));

        // Score victory at turn 100 (short games for training).
        let vc_id = self.state.id_gen.next_victory_id();
        self.state
            .victory_conditions
            .push(BuiltinVictoryCondition::Score {
                id: vc_id,
                turn_limit: 100,
            });

        // Initial visibility for both civs.
        recalculate_visibility(&mut self.state, agent_id);
        recalculate_visibility(&mut self.state, opp_id);

        self.prev_score = compute_score(&self.state, agent_id);
    }

    /// Dispatch an action to the appropriate rules engine method.
    fn dispatch_action(&mut self, action: &Action) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            Action::EndTurn => {
                // EndTurn is handled by the caller after dispatch; nothing to do here.
                Ok(())
            }
            Action::MoveUnit { unit, to } => {
                // move_unit takes &GameState (immutable) and returns a diff.
                // InsufficientMovement carries a partial diff we still apply.
                match self.rules.move_unit(&self.state, *unit, *to) {
                    Ok(diff) => {
                        crate::game::apply_diff(&mut self.state, &diff);
                        recalculate_visibility(&mut self.state, self.agent_civ);
                        Ok(())
                    }
                    Err(crate::game::RulesError::InsufficientMovement(diff)) => {
                        crate::game::apply_diff(&mut self.state, &diff);
                        recalculate_visibility(&mut self.state, self.agent_civ);
                        Ok(())
                    }
                    Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                }
            }
            Action::Attack { attacker, target } => {
                let _diff = self
                    .rules
                    .attack(&mut self.state, *attacker, *target)?;
                Ok(())
            }
            Action::FoundCity { settler, name } => {
                let _diff = self
                    .rules
                    .found_city(&mut self.state, *settler, name.clone())?;
                recalculate_visibility(&mut self.state, self.agent_civ);
                Ok(())
            }
            Action::QueueProduction { city, item } => {
                if let Some(c) = self.state.cities.iter_mut().find(|c| c.id == *city) {
                    c.production_queue.push_back(item.clone());
                }
                Ok(())
            }
            Action::ResearchTech { tech_name } => {
                // Find the tech ID in the tech tree and queue it for the agent's civ.
                let tech_id = self
                    .state
                    .tech_tree
                    .nodes
                    .values()
                    .find(|n| n.name == *tech_name)
                    .map(|n| n.id);
                if let Some(tid) = tech_id
                    && let Some(civ) = self
                        .state
                        .civilizations
                        .iter_mut()
                        .find(|c| c.id == self.agent_civ)
                {
                    civ.research_queue.push_back(TechProgress {
                        tech_id: tid,
                        progress: 0,
                        boosted: false,
                    });
                }
                Ok(())
            }
            Action::PlaceImprovement { coord, improvement } => {
                let _diff = self.rules.place_improvement(
                    &mut self.state,
                    self.agent_civ,
                    *coord,
                    *improvement,
                    None,
                )?;
                Ok(())
            }
        }
    }
}
