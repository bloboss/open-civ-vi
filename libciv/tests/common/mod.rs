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
use libciv::civ::{BasicUnit, BuiltinAgenda, City, Civilization, Leader};
use libciv::game::recalculate_visibility;
use libciv::game::state::UnitTypeDef;
use libhexgrid::coord::HexCoord;

fn stub_leader(name: &'static str, civ_id: CivId) -> Leader {
    Leader {
        name,
        civ_id,
        agenda: BuiltinAgenda::Default,
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
            range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None,
        },
        UnitTypeDef {
            id: settler_type, name: "settler", production_cost: 80,
            max_movement: 200, combat_strength: None,
            domain: UnitDomain::Land, category: UnitCategory::Civilian,
            range: 0, vision_range: 2, can_found_city: true, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None,
        },
        UnitTypeDef {
            id: builder_type, name: "builder", production_cost: 50,
            max_movement: 200, combat_strength: None,
            domain: UnitDomain::Land, category: UnitCategory::Civilian,
            range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 3, exclusive_to: None, replaces: None, era: None, promotion_class: None,
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

    let rome_warrior = SpawnUnit::combat(warrior_type, rome_id, HexCoord::from_qr(5, 3))
        .build(&mut state);

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

    let babylon_warrior = SpawnUnit::combat(warrior_type, babylon_id, HexCoord::from_qr(8, 5))
        .build(&mut state);

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

// ---------------------------------------------------------------------------
// Unit spawning helper
// ---------------------------------------------------------------------------

/// Builder for spawning test units with sensible defaults.
///
/// Only the fields that differ between tests need to be set explicitly.
/// Call `.build(state)` to generate the ID, push the unit, and return its ID.
pub struct SpawnUnit {
    pub unit_type: UnitTypeId,
    pub owner: CivId,
    pub coord: HexCoord,
    pub domain: UnitDomain,
    pub category: UnitCategory,
    pub movement: u32,
    pub combat_strength: Option<u32>,
    pub health: u32,
    pub charges: Option<u8>,
    pub vision_range: u8,
    pub range: u8,
}

impl SpawnUnit {
    /// Create a land combat unit with standard defaults.
    pub fn combat(unit_type: UnitTypeId, owner: CivId, coord: HexCoord) -> Self {
        Self {
            unit_type,
            owner,
            coord,
            domain: UnitDomain::Land,
            category: UnitCategory::Combat,
            movement: 200,
            combat_strength: Some(20),
            health: 100,
            charges: None,
            vision_range: 2,
            range: 0,
        }
    }

    /// Create a land civilian unit with standard defaults.
    pub fn civilian(unit_type: UnitTypeId, owner: CivId, coord: HexCoord) -> Self {
        Self {
            unit_type,
            owner,
            coord,
            domain: UnitDomain::Land,
            category: UnitCategory::Civilian,
            movement: 200,
            combat_strength: None,
            health: 100,
            charges: None,
            vision_range: 2,
            range: 0,
        }
    }

    pub fn domain(mut self, domain: UnitDomain) -> Self {
        self.domain = domain;
        self
    }

    pub fn category(mut self, category: UnitCategory) -> Self {
        self.category = category;
        self
    }

    pub fn movement(mut self, movement: u32) -> Self {
        self.movement = movement;
        self
    }

    pub fn combat_strength(mut self, cs: Option<u32>) -> Self {
        self.combat_strength = cs;
        self
    }

    pub fn health(mut self, hp: u32) -> Self {
        self.health = hp;
        self
    }

    pub fn charges(mut self, c: u8) -> Self {
        self.charges = Some(c);
        self
    }

    /// Push the unit into `state.units` and return the generated `UnitId`.
    pub fn build(self, state: &mut GameState) -> UnitId {
        let id = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id,
            unit_type: self.unit_type,
            owner: self.owner,
            coord: self.coord,
            domain: self.domain,
            category: self.category,
            movement_left: self.movement,
            max_movement: self.movement,
            combat_strength: self.combat_strength,
            health: self.health,
            range: self.range,
            vision_range: self.vision_range,
            charges: self.charges,
            promotions: Vec::new(),
            experience: 0,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
            is_embarked: false,
        });
        id
    }
}
