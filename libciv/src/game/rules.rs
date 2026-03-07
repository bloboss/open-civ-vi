use crate::{CivId, TechId, UnitId, YieldBundle};
use crate::civ::unit::Unit;
use crate::rules::modifier::{EffectType, resolve_modifiers};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::{HexCoord, HexDir};
use libhexgrid::types::MovementCost;
use libhexgrid::{HexEdge, HexTile};

use super::diff::{GameStateDiff, StateDelta};
use super::state::GameState;

/// Core rules evaluation interface.
pub trait RulesEngine: std::fmt::Debug {
    /// Validate and apply a unit move. Returns the resulting diff, or
    /// `Err(InsufficientMovement(partial_diff))` when the unit cannot reach
    /// the destination within its remaining movement budget.
    fn move_unit(
        &self,
        state: &GameState,
        unit: UnitId,
        to: HexCoord,
    ) -> Result<GameStateDiff, RulesError>;

    /// Compute all yields for a civilization this turn (tile yields + building
    /// yields + resolved modifier effects).
    fn compute_yields(&self, state: &GameState, civ: CivId) -> YieldBundle;

    /// Advance the game state by one turn. Returns diff.
    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff;
}

/// Errors returned by rules engine operations.
#[derive(Debug, Clone)]
pub enum RulesError {
    UnitNotFound,
    /// No path exists to the destination (impassable terrain or out of bounds).
    DestinationImpassable,
    /// A path exists but the unit's movement budget was exhausted before reaching
    /// the destination. The inner diff records the partial move that did occur
    /// (if any movement was possible).
    InsufficientMovement(GameStateDiff),
    InvalidCoord,
    NotYourTurn,
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulesError::UnitNotFound             => write!(f, "unit not found"),
            RulesError::DestinationImpassable    => write!(f, "destination is impassable"),
            RulesError::InsufficientMovement(_)  => write!(f, "insufficient movement points"),
            RulesError::InvalidCoord             => write!(f, "invalid coordinate"),
            RulesError::NotYourTurn              => write!(f, "not your turn"),
        }
    }
}

impl std::error::Error for RulesError {}

// ── DefaultRulesEngine ────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DefaultRulesEngine;

impl RulesEngine for DefaultRulesEngine {
    fn move_unit(
        &self,
        state: &GameState,
        unit_id: UnitId,
        to: HexCoord,
    ) -> Result<GameStateDiff, RulesError> {
        let unit = state.unit(unit_id).ok_or(RulesError::UnitNotFound)?;
        let from   = unit.coord();
        let budget = unit.movement_left();

        let to_norm = state.board.normalize(to).ok_or(RulesError::InvalidCoord)?;

        // Determine whether a path to the destination exists at all.
        let full_path = state.board
            .find_path(from, to_norm, u32::MAX)
            .ok_or(RulesError::DestinationImpassable)?;

        // Walk the path, consuming movement budget step by step.
        let mut spent  = 0u32;
        let mut reached = from;

        for i in 1..full_path.len() {
            let prev = full_path[i - 1];
            let next = full_path[i];

            let tile_cost = match state.board.tile(next) {
                Some(t) => {
                    let base = match t.road.as_ref() {
                        Some(r) => r.as_def().movement_cost(),
                        None    => t.movement_cost(),
                    };
                    match base {
                        MovementCost::Impassable => break,
                        MovementCost::Cost(c)    => c,
                    }
                }
                None => break,
            };

            let edge_cost = neighbor_dir(&state.board, prev, next)
                .and_then(|dir| state.board.edge(prev, dir))
                .map(|e| match e.crossing_cost() {
                    MovementCost::Impassable => u32::MAX,
                    MovementCost::Cost(c)    => c,
                })
                .unwrap_or(0);

            if edge_cost == u32::MAX {
                break;
            }

            let step = tile_cost + edge_cost;
            if spent + step > budget {
                break;
            }
            spent   += step;
            reached  = next;
        }

        let mut diff = GameStateDiff::new();

        if reached == from {
            // Zero movement occurred (budget was 0 or first step too costly).
            return Err(RulesError::InsufficientMovement(diff));
        }

        diff.push(StateDelta::UnitMoved { unit: unit_id, from, to: reached });

        if reached == to_norm {
            Ok(diff)
        } else {
            // Partial move: unit moved but did not reach the destination.
            Err(RulesError::InsufficientMovement(diff))
        }
    }

    fn compute_yields(&self, state: &GameState, civ_id: CivId) -> YieldBundle {
        let mut total = YieldBundle::default();

        // Sum tile yields for all cities owned by this civ.
        // Worked tiles = city center + its 6 neighbors (Phase 2 approximation).
        for city in state.cities.iter().filter(|c| c.owner == civ_id) {
            let board = &state.board;

            if let Some(tile) = board.tile(city.coord) {
                total += tile.total_yields();
            }
            for neighbor in board.neighbors(city.coord) {
                if let Some(tile) = board.tile(neighbor) {
                    total += tile.total_yields();
                }
            }
        }

        // Collect modifiers from active policies, current government, and leader abilities.
        let mut modifiers = Vec::new();
        if let Some(civ) = state.civ(civ_id) {
            for pid in &civ.active_policies {
                if let Some(policy) = state.policies.iter().find(|p| p.id == *pid) {
                    modifiers.extend(policy.modifiers.iter().cloned());
                }
            }
            if let Some(gov_id) = civ.current_government {
                if let Some(gov) = state.governments.iter().find(|g| g.id == gov_id) {
                    modifiers.extend(gov.inherent_modifiers.iter().cloned());
                }
            }
            for ability in &civ.leader.abilities {
                modifiers.extend(ability.modifiers());
            }
        }

        let effects = resolve_modifiers(&modifiers);
        apply_effects(&effects, total)
    }

    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff {
        let mut diff = GameStateDiff::new();

        // ── Per-city food accumulation and population growth ──────────────────
        // Collect food yields first (immutable board borrow), then mutate cities.
        let city_food: Vec<(usize, i32)> = {
            let board = &state.board;
            state.cities.iter().enumerate().map(|(i, city)| {
                let mut food = board.tile(city.coord)
                    .map(|t| t.total_yields().food)
                    .unwrap_or(0);
                for n in board.neighbors(city.coord) {
                    food += board.tile(n).map(|t| t.total_yields().food).unwrap_or(0);
                }
                (i, food)
            }).collect()
        };

        for (i, food) in city_food {
            let city = &mut state.cities[i];
            if food > 0 {
                city.food_stored += food as u32;
            }
            if city.food_stored >= city.food_to_grow {
                city.food_stored  = 0;
                city.population  += 1;
                city.food_to_grow = 15 + 6 * (city.population - 1);
                diff.push(StateDelta::PopulationGrew {
                    city: city.id,
                    new_population: city.population,
                });
            }
        }

        // ── Per-civ yields: gold, science, culture ────────────────────────────
        let civ_ids: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();

        // Collect yields while state is immutably borrowed.
        let civ_yields: Vec<(CivId, YieldBundle)> = civ_ids.iter()
            .map(|&id| (id, self.compute_yields(state, id)))
            .collect();

        // Apply gold.
        for (civ_id, yields) in &civ_yields {
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id) {
                if yields.gold != 0 {
                    civ.gold += yields.gold;
                    diff.push(StateDelta::GoldChanged { civ: *civ_id, delta: yields.gold });
                }
            }
        }

        // Apply science → tech progress and check completion.
        // Two-pass: first update progress (mutates civilizations), then check
        // tech_tree (different field, disjoint borrow).
        struct TechCheck { civ_idx: usize, civ_id: CivId, tech_id: TechId, progress: u32 }
        let mut tech_checks: Vec<TechCheck> = Vec::new();

        for (civ_id, yields) in &civ_yields {
            if yields.science <= 0 { continue; }
            if let Some((idx, civ)) = state.civilizations.iter_mut()
                .enumerate().find(|(_, c)| c.id == *civ_id)
            {
                if let Some(tp) = civ.tech_in_progress.as_mut() {
                    tp.progress += yields.science as u32;
                    tech_checks.push(TechCheck {
                        civ_idx: idx,
                        civ_id:  *civ_id,
                        tech_id: tp.tech_id,
                        progress: tp.progress,
                    });
                }
            }
        }

        for tc in tech_checks {
            let node_info = state.tech_tree.get(tc.tech_id)
                .map(|n| (n.cost, n.name));
            if let Some((cost, name)) = node_info {
                if tc.progress >= cost {
                    let civ = &mut state.civilizations[tc.civ_idx];
                    civ.researched_techs.push(tc.tech_id);
                    civ.tech_in_progress = None;
                    diff.push(StateDelta::TechResearched { civ: tc.civ_id, tech: name });
                }
            }
        }

        // Apply culture → civic progress (same pattern as science).
        struct CivicCheck { civ_idx: usize, civ_id: CivId, civic_id: crate::CivicId, progress: u32 }
        let mut civic_checks: Vec<CivicCheck> = Vec::new();

        for (civ_id, yields) in &civ_yields {
            if yields.culture <= 0 { continue; }
            if let Some((idx, civ)) = state.civilizations.iter_mut()
                .enumerate().find(|(_, c)| c.id == *civ_id)
            {
                if let Some(cp) = civ.civic_in_progress.as_mut() {
                    cp.progress += yields.culture as u32;
                    civic_checks.push(CivicCheck {
                        civ_idx: idx,
                        civ_id:  *civ_id,
                        civic_id: cp.civic_id,
                        progress: cp.progress,
                    });
                }
            }
        }

        for cc in civic_checks {
            let node_info = state.civic_tree.get(cc.civic_id)
                .map(|n| (n.cost, n.name));
            if let Some((cost, name)) = node_info {
                if cc.progress >= cost {
                    let civ = &mut state.civilizations[cc.civ_idx];
                    civ.completed_civics.push(cc.civic_id);
                    civ.civic_in_progress = None;
                    diff.push(StateDelta::CivicCompleted { civ: cc.civ_id, civic: name });
                }
            }
        }

        // ── Advance turn counter ──────────────────────────────────────────────
        let prev = state.turn;
        state.turn += 1;
        diff.push(StateDelta::TurnAdvanced { from: prev, to: state.turn });

        diff
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the `HexDir` from `from` to an adjacent `to`, handling board wrapping.
fn neighbor_dir(board: &super::board::WorldBoard, from: HexCoord, to: HexCoord) -> Option<HexDir> {
    HexDir::ALL.iter().find(|&&dir| {
        board.normalize(from + dir.unit_vec()) == Some(to)
    }).copied()
}

/// Apply a resolved set of `EffectType`s to a `YieldBundle`.
/// Flat bonuses are applied first, then percentage bonuses.
fn apply_effects(effects: &[EffectType], mut base: YieldBundle) -> YieldBundle {
    for &effect in effects {
        if let EffectType::YieldFlat(yt, amount) = effect {
            base.add_yield(yt, amount);
        }
    }
    for &effect in effects {
        if let EffectType::YieldPercent(yt, pct) = effect {
            let current = base.get(yt);
            let bonus   = (current * pct) / 100;
            base.add_yield(yt, bonus);
        }
    }
    base
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::{BasicUnit, Civilization, City, Leader};
    use crate::civ::civilization::Agenda;
    use libhexgrid::coord::HexCoord;
    use crate::{CivId, UnitCategory, UnitDomain};

    // ── Shared test helpers ───────────────────────────────────────────────────

    struct NoOpAgenda;
    impl std::fmt::Debug for NoOpAgenda {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "NoOpAgenda")
        }
    }
    impl Agenda for NoOpAgenda {
        fn name(&self) -> &'static str { "No-op" }
        fn description(&self) -> &'static str { "" }
        fn attitude(&self, _: CivId) -> i32 { 0 }
    }

    fn test_leader(civ_id: CivId) -> Leader {
        Leader {
            name: "TestLeader",
            civ_id,
            abilities: Vec::new(),
            agenda: Box::new(NoOpAgenda),
        }
    }

    fn make_state() -> (GameState, CivId) {
        let mut state = GameState::new(42, 10, 10);
        let civ_id = state.id_gen.next_civ_id();
        let civ = Civilization::new(civ_id, "TestCiv", "Test", test_leader(civ_id));
        state.civilizations.push(civ);
        (state, civ_id)
    }

    fn spawn_unit(state: &mut GameState, civ_id: CivId, coord: HexCoord, movement: u32) -> crate::UnitId {
        let unit_id   = state.id_gen.next_unit_id();
        let unit_type = state.id_gen.next_ulid();
        let unit_type_id = crate::UnitTypeId::from_ulid(unit_type);
        state.units.push(BasicUnit {
            id:             unit_id,
            unit_type:      unit_type_id,
            owner:          civ_id,
            coord,
            domain:         UnitDomain::Land,
            category:       UnitCategory::Combat,
            movement_left:  movement,
            max_movement:   movement,
            combat_strength: Some(20),
            promotions:     Vec::new(),
            health:         100,
        });
        unit_id
    }

    // ── move_unit tests ───────────────────────────────────────────────────────

    #[test]
    fn test_move_unit_full_move() {
        let (mut state, civ_id) = make_state();
        let start = HexCoord::from_qr(2, 2);
        let dest  = HexCoord::from_qr(4, 2);
        // Hex distance = 2. Each Grassland tile costs 100. Budget = 300 (ample).
        let uid = spawn_unit(&mut state, civ_id, start, 300);

        let engine = DefaultRulesEngine;
        let result = engine.move_unit(&state, uid, dest);

        assert!(result.is_ok(), "full move should succeed: {:?}", result.err());
        let diff = result.unwrap();
        assert_eq!(diff.len(), 1);
        match &diff.deltas[0] {
            StateDelta::UnitMoved { unit, from, to } => {
                assert_eq!(*unit, uid);
                assert_eq!(*from, start);
                assert_eq!(*to, dest);
            }
            other => panic!("unexpected delta: {:?}", other),
        }
    }

    #[test]
    fn test_move_unit_impassable_destination() {
        use crate::world::terrain::{BuiltinTerrain, Mountain};

        let (mut state, civ_id) = make_state();
        let start = HexCoord::from_qr(2, 2);
        let mountain = HexCoord::from_qr(3, 2);

        // Block the only direct neighbor in the E direction.
        if let Some(t) = state.board.tile_mut(mountain) {
            t.terrain = BuiltinTerrain::Mountain(Mountain);
        }
        // Also block all other neighbours so there's truly no path.
        for dir in libhexgrid::coord::HexDir::ALL {
            let nb = state.board.normalize(start + dir.unit_vec());
            if let Some(coord) = nb {
                if coord != mountain {
                    if let Some(t) = state.board.tile_mut(coord) {
                        t.terrain = BuiltinTerrain::Mountain(Mountain);
                    }
                }
            }
        }

        let uid = spawn_unit(&mut state, civ_id, start, 500);
        let engine = DefaultRulesEngine;
        // Mountain itself is impassable, and all other neighbours blocked → no path.
        let result = engine.move_unit(&state, uid, mountain);
        assert!(
            matches!(result, Err(RulesError::DestinationImpassable)),
            "move to impassable tile should fail: {:?}", result
        );
        // State must be unaffected.
        assert_eq!(state.unit(uid).unwrap().coord, start);
    }

    #[test]
    fn test_move_unit_partial_move() {
        let (mut state, civ_id) = make_state();
        let start = HexCoord::from_qr(0, 5);
        let far   = HexCoord::from_qr(4, 5);

        // Budget = 150. Each Grassland tile costs 100.
        // Direct path (4 steps): total cost = 400. Unit can only do 1 step (100 ≤ 150).
        let uid = spawn_unit(&mut state, civ_id, start, 150);
        let engine = DefaultRulesEngine;

        let result = engine.move_unit(&state, uid, far);

        match result {
            Err(RulesError::InsufficientMovement(diff)) => {
                assert!(!diff.is_empty(), "partial diff must record the move that occurred");
                match &diff.deltas[0] {
                    StateDelta::UnitMoved { unit, from, to } => {
                        assert_eq!(*unit, uid);
                        assert_eq!(*from, start);
                        // Moved one step (100 ≤ 150) but not all four.
                        assert_ne!(*to, start, "unit must have moved at least one tile");
                        assert_ne!(*to, far,   "unit must not have reached the destination");
                    }
                    other => panic!("unexpected delta: {:?}", other),
                }
            }
            other => panic!("expected InsufficientMovement, got {:?}", other),
        }
    }

    // ── compute_yields tests ──────────────────────────────────────────────────

    #[test]
    fn test_compute_yields_grassland_city() {
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let city    = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        state.cities.push(city);

        let engine = DefaultRulesEngine;
        let yields = engine.compute_yields(&state, civ_id);

        // City center (1) + 6 neighbours = 7 Grassland tiles × 2 Food = 14.
        assert_eq!(yields.food, 14, "7 Grassland tiles should yield 14 food");
    }

    // ── advance_turn tests ────────────────────────────────────────────────────

    #[test]
    fn test_advance_turn_population_grows() {
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let city    = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        state.cities.push(city);

        // Grassland gives 14 Food/turn. food_to_grow starts at 15.
        // Turn 1: food_stored = 14  < 15 → no growth.
        // Turn 2: food_stored = 28 >= 15 → growth; reset to 0, population = 2.
        let engine = DefaultRulesEngine;

        let diff1 = engine.advance_turn(&mut state);
        assert_eq!(state.cities[0].population, 1, "no growth after turn 1");
        assert!(!diff1.deltas.iter().any(|d| matches!(d, StateDelta::PopulationGrew { .. })));

        let diff2 = engine.advance_turn(&mut state);
        assert_eq!(state.cities[0].population, 2, "population should grow after turn 2");
        assert!(diff2.deltas.iter().any(|d| matches!(d, StateDelta::PopulationGrew { city, new_population: 2 } if *city == city_id)));
    }

    #[test]
    fn test_advance_turn_increments_turn_counter() {
        let (mut state, _) = make_state();
        let engine = DefaultRulesEngine;
        assert_eq!(state.turn, 0);
        engine.advance_turn(&mut state);
        assert_eq!(state.turn, 1);
        engine.advance_turn(&mut state);
        assert_eq!(state.turn, 2);
    }
}
