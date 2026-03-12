// FIXME: This module duplicates game-initialization logic from civsim/src/main.rs.
//        In a client-server split the GameState should live server-side only.
//        Replace the functions here with HTTP calls:
//          POST /api/games            -> returns { game_id, view }
//          GET  /api/games/{id}/state -> returns GameView (serialized snapshot)
//          POST /api/games/{id}/action -> mutates state, returns updated GameView

use std::collections::HashSet;

use libciv::{
    CityId, CivId, GameState, UnitCategory, UnitDomain, UnitId, UnitTypeId,
};
use libciv::civ::{Agenda, BasicUnit, City, Civilization, Leader};
use libciv::game::state::UnitTypeDef;
use libciv::world::terrain::{
    BuiltinTerrain, Desert, Grassland, Mountain, Ocean, Plains, Tundra,
};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::Rng;

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// In-WASM game session.  Mirrors the TUI `Session` in civsim/src/main.rs.
///
/// FIXME: Once a server exists, `state` should not be held client-side.
///        The frontend should hold only a lightweight `GameView` snapshot
///        returned by the server after each action.
pub struct Session {
    pub state:         GameState,
    pub civ_id:        CivId,
    #[allow(dead_code)]  // used by future city-action commands
    pub city_id:       CityId,
    pub selected_unit: Option<UnitId>,
    /// Parallel to `state.unit_type_defs` — same insertion order.
    pub unit_type_ids: Vec<UnitTypeId>,
}

// ---------------------------------------------------------------------------
// Agenda stub
// ---------------------------------------------------------------------------

struct NoOpAgenda;

impl std::fmt::Debug for NoOpAgenda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoOpAgenda")
    }
}

impl Agenda for NoOpAgenda {
    fn name(&self) -> &'static str { "Expansionist" }
    fn description(&self) -> &'static str { "Likes open land." }
    fn attitude(&self, _: CivId) -> i32 { 0 }
}

// ---------------------------------------------------------------------------
// Terrain randomization
// ---------------------------------------------------------------------------

// FIXME: Map generation should run server-side and be sent to the client as
//        part of the initial game state snapshot.
fn randomize_terrain(state: &mut GameState, seed: u64, safe_coord: HexCoord) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let safe: HashSet<HexCoord> = {
        let mut s = HashSet::new();
        s.insert(safe_coord);
        for n in state.board.neighbors(safe_coord) { s.insert(n); }
        s
    };
    let coords: Vec<HexCoord> = state.board.all_coords();
    for coord in coords {
        let terrain = if safe.contains(&coord) {
            match rng.random_range(0u8..4) {
                0 | 1 => BuiltinTerrain::Grassland(Grassland),
                _     => BuiltinTerrain::Plains(Plains),
            }
        } else {
            match rng.random_range(0u8..100) {
                0..35  => BuiltinTerrain::Grassland(Grassland),
                35..60 => BuiltinTerrain::Plains(Plains),
                60..75 => BuiltinTerrain::Desert(Desert),
                75..85 => BuiltinTerrain::Tundra(Tundra),
                85..93 => BuiltinTerrain::Mountain(Mountain),
                _      => BuiltinTerrain::Ocean(Ocean),
            }
        };
        if let Some(tile) = state.board.tile_mut(coord) {
            tile.terrain = terrain;
        }
    }
}

// ---------------------------------------------------------------------------
// Session constructor
// ---------------------------------------------------------------------------

// FIXME: This function should become a call to POST /api/games.  The server
//        runs all of this logic and returns a serialised GameView; the client
//        never touches GameState directly.
pub fn build_session() -> Session {
    let seed = 42u64;
    let mut state = GameState::new(seed, 14, 8);

    // Civilization: Rome / Caesar.
    let civ_id = state.id_gen.next_civ_id();
    let leader = Leader {
        name: "Caesar",
        civ_id,
        abilities: Vec::new(),
        agenda: Box::new(NoOpAgenda),
    };
    state.civilizations.push(Civilization::new(civ_id, "Rome", "Roman", leader));

    // Capital city at (3, 3).
    let city_coord = HexCoord::from_qr(3, 3);
    let city_id    = state.id_gen.next_city_id();
    let mut city   = City::new(city_id, "Roma".to_string(), civ_id, city_coord);
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations[0].cities.push(city_id);

    randomize_terrain(&mut state, seed, city_coord);

    // Unit-type registry.
    state.unit_type_defs.extend([
        UnitTypeDef { name: "warrior", production_cost: 40,  max_movement: 200,
                      combat_strength: Some(20), domain: UnitDomain::Land, category: UnitCategory::Combat   },
        UnitTypeDef { name: "settler", production_cost: 80,  max_movement: 200,
                      combat_strength: None,     domain: UnitDomain::Land, category: UnitCategory::Civilian },
        UnitTypeDef { name: "builder", production_cost: 50,  max_movement: 200,
                      combat_strength: None,     domain: UnitDomain::Land, category: UnitCategory::Civilian },
        UnitTypeDef { name: "slinger", production_cost: 35,  max_movement: 200,
                      combat_strength: Some(10), domain: UnitDomain::Land, category: UnitCategory::Combat   },
    ]);

    let unit_type_ids: Vec<UnitTypeId> = state.unit_type_defs.iter()
        .map(|_| UnitTypeId::from_ulid(state.id_gen.next_ulid()))
        .collect();

    // Starting Warrior.
    let unit_id      = state.id_gen.next_unit_id();
    let warrior_type = unit_type_ids[0];
    state.units.push(BasicUnit {
        id:              unit_id,
        unit_type:       warrior_type,
        owner:           civ_id,
        coord:           HexCoord::from_qr(7, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        health:          100,
    });

    Session {
        state,
        civ_id,
        city_id,
        selected_unit: Some(unit_id),
        unit_type_ids,
    }
}
