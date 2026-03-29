//! Deterministic heuristic AI agent.
//!
//! [`HeuristicAgent`] implements [`Agent`] for a single civilization.  Every
//! decision is purely deterministic: no random numbers are consumed and all
//! iteration is over sorted collections so the same `GameState` always
//! produces the same sequence of actions.
//!
//! ## Behaviour
//!
//! **Production (fixed priority)**
//! For each city with an empty production queue (cities iterated in ascending
//! [`CityId`] order):
//! 1. Prefer the combat [`UnitTypeDef`] with the highest `production_cost`.
//! 2. If no combat unit is registered, fall back to any unit type.
//! 3. Ties broken lexicographically by `name`.
//!
//! **Unit movement (gradient descent)**
//! For each unit owned by this civ (iterated in ascending [`UnitId`] order)
//! that still has movement points remaining:
//! * Score every adjacent hex with [`HeuristicAgent::score_tile`].
//! * Move to the neighbour with the highest score.
//! * Ties in score break on [`HexCoord`] natural order (smallest coord wins).
//! * If the best score is no higher than the unit's current tile the unit
//!   stays put.

use std::collections::BTreeMap;

use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::MovementCost;
use libhexgrid::HexTile;

use crate::civ::ProductionItem;
use crate::game::{GameStateDiff, RulesEngine, StateDelta};
use crate::game::state::GameState;
use crate::{CivId, UnitCategory, UnitId};

// ── Agent trait ───────────────────────────────────────────────────────────────

/// A game-playing agent that controls one civilization.
pub trait Agent: std::fmt::Debug {
    /// Execute all decisions for this agent's civilization for one game turn.
    ///
    /// Mutates `state` in-place (e.g. applies unit moves, queues production)
    /// and returns the aggregate [`GameStateDiff`] describing every change
    /// made.
    fn take_turn(&self, state: &mut GameState, rules: &dyn RulesEngine) -> GameStateDiff;
}

// ── HeuristicAgent ────────────────────────────────────────────────────────────

/// A simple, fully deterministic baseline AI agent.
///
/// Useful for verifying that movement, combat, and production systems work
/// correctly in end-to-end tests.
#[derive(Debug, Clone)]
pub struct HeuristicAgent {
    pub civ_id: CivId,
}

impl HeuristicAgent {
    pub fn new(civ_id: CivId) -> Self {
        Self { civ_id }
    }

    /// Score a tile for movement purposes.
    ///
    /// | Condition                   | Score      |
    /// |-----------------------------|------------|
    /// | Invalid / impassable        | `i32::MIN` |
    /// | Never explored              | `100`      |
    /// | Explored but not visible    | `50`       |
    /// | Currently visible           | `10`       |
    pub fn score_tile(coord: HexCoord, state: &GameState, civ_id: CivId) -> i32 {
        match state.board.tile(coord) {
            None => return i32::MIN,
            Some(t) => {
                if matches!(t.movement_cost(), MovementCost::Impassable) {
                    return i32::MIN;
                }
            }
        }

        let civ = match state.civ(civ_id) {
            Some(c) => c,
            None    => return i32::MIN,
        };

        if !civ.explored_tiles.contains(&coord) {
            100
        } else if !civ.visible_tiles.contains(&coord) {
            50
        } else {
            10
        }
    }

    // ── private helpers ───────────────────────────────────────────────────

    fn decide_production(&self, state: &mut GameState) -> GameStateDiff {
        let mut diff = GameStateDiff::new();

        // Collect and sort this civ's city IDs for determinism.
        let city_ids: Vec<_> = {
            match state.civ(self.civ_id) {
                Some(c) => {
                    let mut ids = c.cities.clone();
                    ids.sort();
                    ids
                }
                None => return diff,
            }
        };

        // Pick the best producible unit type: highest-cost combat unit, or any unit.
        let best_unit_type = {
            // Collect combat units first.
            let mut candidates: Vec<_> = state.unit_type_defs
                .iter()
                .filter(|d| d.category == UnitCategory::Combat)
                .collect();

            if candidates.is_empty() {
                // Fall back to any unit type.
                candidates = state.unit_type_defs.iter().collect();
            }

            // Deterministic sort: highest production_cost first; ties by name.
            candidates.sort_by(|a, b| {
                b.production_cost.cmp(&a.production_cost)
                    .then_with(|| a.name.cmp(b.name))
            });

            candidates.first().map(|d| (d.id, d.name))
        };

        if let Some((unit_type_id, unit_name)) = best_unit_type {
            for city_id in city_ids {
                if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id)
                    && city.owner == self.civ_id && city.production_queue.is_empty()
                {
                    city.production_queue.push_back(ProductionItem::Unit(unit_type_id));
                    diff.push(StateDelta::ProductionStarted {
                        city: city_id,
                        item: unit_name,
                    });
                }
            }
        }

        diff
    }

    fn decide_movement(&self, state: &mut GameState, rules: &dyn RulesEngine) -> GameStateDiff {
        let mut diff = GameStateDiff::new();

        // Collect units owned by this civ that still have movement, sorted for determinism.
        // Skip traders with assigned trade destinations — they move autonomously
        // during advance_turn.
        let unit_ids: Vec<UnitId> = {
            let mut ids: Vec<UnitId> = state.units.iter()
                .filter(|u| u.owner == self.civ_id && u.movement_left > 0
                    && u.trade_destination.is_none())
                .map(|u| u.id)
                .collect();
            ids.sort();
            ids
        };

        for unit_id in unit_ids {
            // Re-read position each iteration since state has been mutated.
            let (unit_coord, owner) = match state.unit(unit_id) {
                Some(u) if u.movement_left > 0 => (u.coord, u.owner),
                _ => continue,
            };

            // Build scored neighbour map.
            // Key: (-score, coord) — lowest key = highest score, smallest coord breaks ties.
            let mut scored: BTreeMap<(i32, HexCoord), HexCoord> = BTreeMap::new();
            for neighbor in unit_coord.neighbors() {
                let score = Self::score_tile(neighbor, state, owner);
                if score > i32::MIN {
                    scored.insert((-score, neighbor), neighbor);
                }
            }

            // Always move to the best-scored passable neighbour (gradient descent).
            // Ties are broken by the smallest HexCoord (natural BTree order on the key).
            if let Some((_, &dest)) = scored.iter().next() {
                match rules.move_unit(state, unit_id, dest) {
                    Ok(move_diff) => {
                        apply_unit_moved(state, &move_diff);
                        diff.deltas.extend(move_diff.deltas);
                    }
                    Err(_) => {
                        // Cannot move there (e.g. occupied by a friendly unit); skip.
                    }
                }
            }
        }

        diff
    }
}

impl Agent for HeuristicAgent {
    fn take_turn(&self, state: &mut GameState, rules: &dyn RulesEngine) -> GameStateDiff {
        let mut aggregate = GameStateDiff::new();

        // 1. Production decisions (no rules-engine call needed; direct queue mutation).
        aggregate.deltas.extend(self.decide_production(state).deltas);

        // 2. Movement decisions.
        aggregate.deltas.extend(self.decide_movement(state, rules).deltas);

        aggregate
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Apply `UnitMoved` deltas from `diff` to the game state.
///
/// This mirrors the `apply_move` helper used in integration tests and must be
/// called after every successful `move_unit` to keep `state` consistent for
/// subsequent decisions within the same turn.
fn apply_unit_moved(state: &mut GameState, diff: &GameStateDiff) {
    for delta in &diff.deltas {
        if let StateDelta::UnitMoved { unit, to, cost, .. } = delta
            && let Some(u) = state.unit_mut(*unit)
        {
            u.coord         = *to;
            u.movement_left = u.movement_left.saturating_sub(*cost);
        }
    }
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use libhexgrid::coord::HexCoord;
    use crate::{GameState, UnitDomain, UnitCategory, UnitTypeId};
    use crate::civ::{BasicUnit, City, Civilization, Leader, Agenda};
    use crate::game::state::UnitTypeDef;

    // Minimal Agenda stub
    #[derive(Debug)]
    struct NoOpAgenda;

    impl Agenda for NoOpAgenda {
        fn name(&self) -> &'static str { "Neutral" }
        fn description(&self) -> &'static str { "" }
        fn attitude(&self, _: CivId) -> i32 { 0 }
    }

    fn stub_leader(name: &'static str, civ_id: CivId) -> Leader {
        Leader { name, civ_id, abilities: Vec::new(), agenda: Box::new(NoOpAgenda) }
    }

    /// Build a minimal state with one civ and one unit.
    fn minimal_state() -> (GameState, CivId, UnitId, UnitTypeId) {
        let mut state = GameState::new(1, 10, 10);

        let warrior_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
        state.unit_type_defs.push(UnitTypeDef {
            id: warrior_type, name: "warrior", production_cost: 40,
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(20),
            range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
        });

        let civ_id = state.id_gen.next_civ_id();
        state.civilizations.push(
            Civilization::new(civ_id, "Rome", "Roman", stub_leader("Caesar", civ_id))
        );

        let unit_id = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id: unit_id, unit_type: warrior_type, owner: civ_id,
            coord: HexCoord::from_qr(5, 5),
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200,
            combat_strength: Some(20), promotions: Vec::new(),
            health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        (state, civ_id, unit_id, warrior_type)
    }

    // ── score_tile ────────────────────────────────────────────────────────────

    #[test]
    fn score_unexplored_tile_is_100() {
        let (state, civ_id, _, _) = minimal_state();
        let coord = HexCoord::from_qr(3, 3);
        // coord is not in explored_tiles (no visibility computed yet)
        assert_eq!(HeuristicAgent::score_tile(coord, &state, civ_id), 100);
    }

    #[test]
    fn score_explored_not_visible_is_50() {
        let (mut state, civ_id, _, _) = minimal_state();
        let coord = HexCoord::from_qr(3, 3);
        // Mark as explored but not visible.
        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        civ.explored_tiles.insert(coord);
        assert_eq!(HeuristicAgent::score_tile(coord, &state, civ_id), 50);
    }

    #[test]
    fn score_visible_tile_is_10() {
        let (mut state, civ_id, _, _) = minimal_state();
        let coord = HexCoord::from_qr(3, 3);
        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        civ.explored_tiles.insert(coord);
        civ.visible_tiles.insert(coord);
        assert_eq!(HeuristicAgent::score_tile(coord, &state, civ_id), 10);
    }

    #[test]
    fn score_out_of_bounds_is_min() {
        let (state, civ_id, _, _) = minimal_state();
        // r=-1 is out of bounds for a 10×10 board
        let coord = HexCoord::from_qr(5, -1);
        assert_eq!(HeuristicAgent::score_tile(coord, &state, civ_id), i32::MIN);
    }

    // ── production ────────────────────────────────────────────────────────────

    #[test]
    fn production_queues_unit_when_empty() {
        let (mut state, civ_id, _, warrior_type) = minimal_state();

        // Add a city with an empty queue.
        let city_id = state.id_gen.next_city_id();
        state.cities.push(City::new(city_id, "Roma".into(), civ_id, HexCoord::from_qr(3, 3)));
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id).unwrap()
            .cities.push(city_id);

        let agent = HeuristicAgent::new(civ_id);
        let rules = crate::DefaultRulesEngine;
        agent.take_turn(&mut state, &rules);

        let city = state.city(city_id).unwrap();
        assert_eq!(
            city.production_queue.front(),
            Some(&ProductionItem::Unit(warrior_type)),
            "agent should have queued the combat unit"
        );
    }

    #[test]
    fn production_does_not_replace_existing_queue() {
        let (mut state, civ_id, _, warrior_type) = minimal_state();

        let city_id = state.id_gen.next_city_id();
        let mut city = City::new(city_id, "Roma".into(), civ_id, HexCoord::from_qr(3, 3));
        // Pre-populate queue.
        city.production_queue.push_back(ProductionItem::Unit(warrior_type));
        state.cities.push(city);
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id).unwrap()
            .cities.push(city_id);

        let agent = HeuristicAgent::new(civ_id);
        let rules = crate::DefaultRulesEngine;
        agent.take_turn(&mut state, &rules);

        // Queue length must remain 1 (not 2).
        assert_eq!(
            state.city(city_id).unwrap().production_queue.len(), 1,
            "agent must not double-queue when queue is non-empty"
        );
    }

    #[test]
    fn production_prefers_higher_cost_combat_unit() {
        let (mut state, civ_id, _, warrior_type) = minimal_state();

        // Add a second, more expensive combat unit type.
        let legion_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
        state.unit_type_defs.push(UnitTypeDef {
            id: legion_type, name: "legionary", production_cost: 80,
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(35),
            range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
        });
        let _ = warrior_type; // lower cost; should not be chosen

        let city_id = state.id_gen.next_city_id();
        state.cities.push(City::new(city_id, "Roma".into(), civ_id, HexCoord::from_qr(3, 3)));
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id).unwrap()
            .cities.push(city_id);

        let agent = HeuristicAgent::new(civ_id);
        let rules = crate::DefaultRulesEngine;
        agent.take_turn(&mut state, &rules);

        assert_eq!(
            state.city(city_id).unwrap().production_queue.front(),
            Some(&ProductionItem::Unit(legion_type)),
            "agent should prefer higher-cost combat unit"
        );
    }

    // ── movement ─────────────────────────────────────────────────────────────

    #[test]
    fn unit_moves_toward_unexplored_tile() {
        let (mut state, civ_id, unit_id, _) = minimal_state();

        // Mark all tiles visible/explored except one neighbour.
        let unit_coord = HexCoord::from_qr(5, 5);
        let target = HexCoord::from_qr(6, 5); // one neighbour

        let all_neighbors = unit_coord.neighbors();
        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        for n in all_neighbors {
            if n != target {
                civ.explored_tiles.insert(n);
                civ.visible_tiles.insert(n);
            }
        }
        civ.explored_tiles.insert(unit_coord);
        civ.visible_tiles.insert(unit_coord);

        let agent = HeuristicAgent::new(civ_id);
        let rules = crate::DefaultRulesEngine;
        agent.take_turn(&mut state, &rules);

        let unit = state.unit(unit_id).unwrap();
        assert_eq!(
            unit.coord, target,
            "unit should have moved to the only unexplored neighbour"
        );
    }

    #[test]
    fn unit_moves_to_smallest_coord_when_all_neighbours_tied() {
        let (mut state, civ_id, unit_id, _) = minimal_state();

        // Mark the unit tile and all neighbours as visible (score=10 everywhere).
        let unit_coord = HexCoord::from_qr(5, 5);
        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        civ.explored_tiles.insert(unit_coord);
        civ.visible_tiles.insert(unit_coord);
        for n in unit_coord.neighbors() {
            civ.explored_tiles.insert(n);
            civ.visible_tiles.insert(n);
        }

        let agent = HeuristicAgent::new(civ_id);
        let rules = crate::DefaultRulesEngine;
        agent.take_turn(&mut state, &rules);

        // All neighbours score 10 (tied); tie-break picks the smallest HexCoord.
        let neighbors = unit_coord.neighbors();
        let expected = *neighbors.iter()
            .filter(|&&c| {
                // Must be passable (on-board).
                matches!(state.board.tile(c), Some(_))
                    && !matches!(
                        state.board.tile(c).map(|t| t.movement_cost()),
                        Some(libhexgrid::types::MovementCost::Impassable)
                    )
            })
            .min()
            .expect("at least one passable neighbour");

        let unit = state.unit(unit_id).unwrap();
        assert_eq!(
            unit.coord, expected,
            "unit should move to the smallest-coord neighbour when all scores are tied"
        );
    }

    // ── determinism ───────────────────────────────────────────────────────────

    #[test]
    fn agent_is_deterministic_across_two_runs() {
        // Build two identical minimal states from the same seed.
        fn build() -> (GameState, CivId) {
            let mut state = GameState::new(99, 10, 10);

            let warrior_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
            state.unit_type_defs.push(UnitTypeDef {
                id: warrior_type, name: "warrior", production_cost: 40,
                domain: UnitDomain::Land, category: UnitCategory::Combat,
                max_movement: 200, combat_strength: Some(20),
                range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
            });

            let civ_id = state.id_gen.next_civ_id();
            state.civilizations.push(
                Civilization::new(civ_id, "Rome", "Roman", {
                    #[derive(Debug)]
                    struct NoOp;
                    impl Agenda for NoOp {
                        fn name(&self) -> &'static str { "" }
                        fn description(&self) -> &'static str { "" }
                        fn attitude(&self, _: CivId) -> i32 { 0 }
                    }
                    Leader { name: "Caesar", civ_id, abilities: Vec::new(), agenda: Box::new(NoOp) }
                })
            );

            let unit_id = state.id_gen.next_unit_id();
            state.units.push(BasicUnit {
                id: unit_id, unit_type: warrior_type, owner: civ_id,
                coord: HexCoord::from_qr(5, 5),
                domain: UnitDomain::Land, category: UnitCategory::Combat,
                movement_left: 200, max_movement: 200,
                combat_strength: Some(20), promotions: Vec::new(),
                health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
            });

            (state, civ_id)
        }

        let (mut state_a, civ_a) = build();
        let (mut state_b, civ_b) = build();

        let rules = crate::DefaultRulesEngine;

        let diff_a = HeuristicAgent::new(civ_a).take_turn(&mut state_a, &rules);
        let diff_b = HeuristicAgent::new(civ_b).take_turn(&mut state_b, &rules);

        // Both runs must produce diffs of the same length.
        assert_eq!(
            diff_a.len(), diff_b.len(),
            "same input state must yield same number of diff events"
        );

        // Units must end on the same tile.
        let unit_a = state_a.units.first().unwrap();
        let unit_b = state_b.units.first().unwrap();
        assert_eq!(unit_a.coord, unit_b.coord, "unit positions must be identical");
    }
}
