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
use libciv::game::recalculate_visibility;
use libciv::world::terrain::BuiltinTerrain;
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
    pub city_ids:      Vec<CityId>,
    #[allow(dead_code)]
    pub current_city:  usize,
    pub selected_unit: Option<UnitId>,
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
                0 | 1 => BuiltinTerrain::Grassland,
                _     => BuiltinTerrain::Plains,
            }
        } else {
            match rng.random_range(0u8..100) {
                0..35  => BuiltinTerrain::Grassland,
                35..60 => BuiltinTerrain::Plains,
                60..75 => BuiltinTerrain::Desert,
                75..85 => BuiltinTerrain::Tundra,
                85..93 => BuiltinTerrain::Mountain,
                _      => BuiltinTerrain::Ocean,
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

    // Claim the city center and ring-1 as initial territory (mirrors what
    // found_city() does internally via try_claim_tile).
    {
        let initial: Vec<HexCoord> = std::iter::once(city_coord)
            .chain(state.board.neighbors(city_coord))
            .collect();
        for &coord in &initial {
            if let Some(tile) = state.board.tile_mut(coord) {
                tile.owner = Some(civ_id);
            }
        }
        if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
            city.territory = initial.into_iter().collect();
        }
    }

    randomize_terrain(&mut state, seed, city_coord);

    // Unit-type registry.
    let warrior_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let builder_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let slinger_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef { id: warrior_type_id, name: "warrior", production_cost: 40,
                      max_movement: 200, combat_strength: Some(20),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None },
        UnitTypeDef { id: settler_type_id, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None },
        UnitTypeDef { id: builder_type_id, name: "builder", production_cost: 50,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None },
        UnitTypeDef { id: slinger_type_id, name: "slinger", production_cost: 35,
                      max_movement: 200, combat_strength: Some(10),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 2, vision_range: 2, can_found_city: false, resource_cost: None },
    ]);

    // Starting Warrior.
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id:              unit_id,
        unit_type:       warrior_type_id,
        owner:           civ_id,
        coord:           HexCoord::from_qr(7, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        health:          100,
        range:           0,
        vision_range:    2,
    });

    // Starting Builder at city coord for testing improvements.
    let builder_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id:              builder_id,
        unit_type:       builder_type_id,
        owner:           civ_id,
        coord:           city_coord,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Civilian,
        movement_left:   200,
        max_movement:    200,
        combat_strength: None,
        promotions:      Vec::new(),
        health:          100,
        range:           0,
        vision_range:    2,
    });

    recalculate_visibility(&mut state, civ_id);

    Session {
        state,
        civ_id,
        city_ids: vec![city_id],
        current_city: 0,
        selected_unit: Some(unit_id),
    }
}
