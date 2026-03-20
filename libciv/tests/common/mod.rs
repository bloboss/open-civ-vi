/// Shared helpers for libciv integration tests.
///
/// `build_scenario()` creates a deterministic two-civilisation game:
///   * Rome  — capital "Roma"   at (3, 3), Warrior at (5, 3)
///   * Babylon — capital "Babylon" at (10, 5), Warrior at (8, 5)
///
/// Both civs share the same unit-type registry (warrior + settler + builder).
/// Visibility is calculated for both civs at the end of setup.
///
/// `advance_turn()` processes one full game turn: TurnEngine, movement
/// reset, and recalculate_visibility for every civilisation in the state.
#[allow(dead_code)]

use libciv::{
    CivId, CityId, GameState, DefaultRulesEngine, TurnEngine,
    UnitCategory, UnitDomain, UnitId, UnitTypeId,
};
use libciv::civ::{Agenda, BasicUnit, City, Civilization, Leader};
use libciv::game::recalculate_visibility;
use libciv::game::state::UnitTypeDef;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Minimal Agenda stub
// ---------------------------------------------------------------------------

struct NoOpAgenda;

impl std::fmt::Debug for NoOpAgenda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoOpAgenda")
    }
}

impl Agenda for NoOpAgenda {
    fn name(&self) -> &'static str { "Neutral" }
    fn description(&self) -> &'static str { "No preferences." }
    fn attitude(&self, _: CivId) -> i32 { 0 }
}

fn stub_leader(name: &'static str, civ_id: CivId) -> Leader {
    Leader {
        name,
        civ_id,
        abilities:  Vec::new(),
        agenda:     Box::new(NoOpAgenda),
    }
}

// ---------------------------------------------------------------------------
// Scenario
// ---------------------------------------------------------------------------

/// All stable IDs produced by `build_scenario`.
pub struct Scenario {
    pub state:            GameState,
    /// Rome — the "player" civilisation.
    pub rome_id:          CivId,
    pub rome_city:        CityId,
    pub rome_warrior:     UnitId,
    /// Babylon — the "opponent" civilisation.
    pub babylon_id:       CivId,
    pub babylon_city:     CityId,
    pub babylon_warrior:  UnitId,
    /// Shared unit-type IDs (same registry for both civs).
    pub warrior_type:     UnitTypeId,
    pub settler_type:     UnitTypeId,
    pub builder_type:     UnitTypeId,
}

/// Build a deterministic two-civ scenario on a 14×8 board.
pub fn build_scenario() -> Scenario {
    let mut state = GameState::new(42, 14, 8);

    // ── Shared unit-type registry ─────────────────────────────────────────
    let warrior_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let builder_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef {
            id: warrior_type, name: "warrior", production_cost: 40,
            max_movement: 200, combat_strength: Some(20),
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
        },
        UnitTypeDef {
            id: settler_type, name: "settler", production_cost: 80,
            max_movement: 200, combat_strength: None,
            domain: UnitDomain::Land, category: UnitCategory::Civilian,
            range: 0, vision_range: 2, can_found_city: true, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
        },
        UnitTypeDef {
            id: builder_type, name: "builder", production_cost: 50,
            max_movement: 200, combat_strength: None,
            domain: UnitDomain::Land, category: UnitCategory::Civilian,
            range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 3, exclusive_to: None, replaces: None,
        },
    ]);

    // ── Rome ─────────────────────────────────────────────────────────────
    let rome_id = state.id_gen.next_civ_id();
    state.civilizations.push(
        Civilization::new(rome_id, "Rome", "Roman", stub_leader("Caesar", rome_id))
    );

    let rome_city = state.id_gen.next_city_id();
    let mut city = City::new(rome_city, "Roma".into(), rome_id, HexCoord::from_qr(3, 3));
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations.iter_mut()
        .find(|c| c.id == rome_id).unwrap()
        .cities.push(rome_city);

    let rome_warrior = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: rome_warrior, unit_type: warrior_type, owner: rome_id,
        coord: HexCoord::from_qr(5, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        health: 100, range: 0, vision_range: 2, charges: None,
    });

    // ── Babylon ───────────────────────────────────────────────────────────
    let babylon_id = state.id_gen.next_civ_id();
    state.civilizations.push(
        Civilization::new(babylon_id, "Babylon", "Babylonian", stub_leader("Hammurabi", babylon_id))
    );

    let babylon_city = state.id_gen.next_city_id();
    let mut city = City::new(babylon_city, "Babylon".into(), babylon_id, HexCoord::from_qr(10, 5));
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations.iter_mut()
        .find(|c| c.id == babylon_id).unwrap()
        .cities.push(babylon_city);

    let babylon_warrior = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: babylon_warrior, unit_type: warrior_type, owner: babylon_id,
        coord: HexCoord::from_qr(8, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        health: 100, range: 0, vision_range: 2, charges: None,
    });

    // ── Initial visibility for both civs ──────────────────────────────────
    recalculate_visibility(&mut state, rome_id);
    recalculate_visibility(&mut state, babylon_id);

    Scenario {
        state,
        rome_id, rome_city, rome_warrior,
        babylon_id, babylon_city, babylon_warrior,
        warrior_type, settler_type, builder_type,
    }
}

// ---------------------------------------------------------------------------
// Turn helpers
// ---------------------------------------------------------------------------

/// Advance one full turn: process rules, reset movement, refresh visibility.
pub fn advance_turn(s: &mut Scenario) {
    let engine = TurnEngine::new();
    let rules  = DefaultRulesEngine;
    engine.process_turn(&mut s.state, &rules);
    for unit in &mut s.state.units {
        unit.movement_left = unit.max_movement;
    }
    // Refresh visibility for every civilisation.
    let civ_ids: Vec<CivId> = s.state.civilizations.iter().map(|c| c.id).collect();
    for cid in civ_ids {
        recalculate_visibility(&mut s.state, cid);
    }
}

/// Apply the `UnitMoved` deltas returned by `move_unit` to the state.
pub fn apply_move(state: &mut GameState, diff: &libciv::GameStateDiff) {
    use libciv::game::StateDelta;
    for delta in &diff.deltas {
        if let StateDelta::UnitMoved { unit, to, cost, .. } = delta {
            if let Some(u) = state.unit_mut(*unit) {
                u.coord        = *to;
                u.movement_left = u.movement_left.saturating_sub(*cost);
            }
        }
    }
}
