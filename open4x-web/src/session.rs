// FIXME: This module duplicates game-initialization logic from civsim/src/main.rs.
//        In a client-server split the GameState should live server-side only.
//        Replace the functions here with HTTP calls:
//          POST /api/games            -> returns { game_id, view }
//          GET  /api/games/{id}/state -> returns GameView (serialized snapshot)
//          POST /api/games/{id}/action -> mutates state, returns updated GameView

use libciv::{
    CityId, CivId, GameState, UnitCategory, UnitDomain, UnitId, UnitTypeId,
};
use libciv::civ::{Agenda, BasicUnit, City, Civilization, Leader};
use libciv::game::state::UnitTypeDef;
use libciv::game::recalculate_visibility;
use libciv::world::mapgen::{MapGenConfig, generate as mapgen_generate};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// GameConfig
// ---------------------------------------------------------------------------

/// Pre-game configuration set on the map-config screen.
#[derive(Clone, Debug, PartialEq)]
pub struct GameConfig {
    pub width:  u32,
    pub height: u32,
    pub seed:   u64,
    /// Number of AI opponents.  0 = solo play.
    pub num_ai: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self { width: 40, height: 24, seed: 42, num_ai: 1 }
    }
}

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
    /// CivId of the AI adversary (Babylon), or None in solo play.
    pub ai_civ_id:     Option<CivId>,
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
// Session constructor
// ---------------------------------------------------------------------------

// FIXME: This function should become a call to POST /api/games.  The server
//        runs all of this logic and returns a serialised GameView; the client
//        never touches GameState directly.
pub fn build_session(config: &GameConfig) -> Session {
    let seed = config.seed;
    let w    = config.width;
    let h    = config.height;
    let mut state = GameState::new(seed, w, h);

    // ── Terrain via mapgen pipeline ──────────────────────────────────────
    let num_starts = 1 + config.num_ai;
    let mapgen_result = mapgen_generate(
        &MapGenConfig {
            width: w, height: h, seed,
            land_fraction:  None,
            num_continents: None,
            num_zone_seeds: None,
            num_starts,
        },
        &mut state.board,
    );
    let starts = &mapgen_result.starting_positions;
    let city_coord         = starts.first().copied()
        .unwrap_or(HexCoord::from_qr(w as i32 / 4, h as i32 / 2));
    let babylon_city_coord = starts.get(1).copied()
        .unwrap_or(HexCoord::from_qr(w as i32 * 3 / 4, h as i32 / 2));

    // ── Civilization: Rome / Caesar ──────────────────────────────────────
    let civ_id = state.id_gen.next_civ_id();
    state.civilizations.push(Civilization::new(
        civ_id, "Rome", "Roman",
        Leader { name: "Caesar", civ_id, abilities: Vec::new(),
                 agenda: Box::new(NoOpAgenda) },
    ));

    // Capital city.
    let city_id  = state.id_gen.next_city_id();
    let mut city = City::new(city_id, "Roma".to_string(), civ_id, city_coord);
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations[0].cities.push(city_id);

    // Claim initial territory for Rome.
    {
        let initial: Vec<HexCoord> = std::iter::once(city_coord)
            .chain(state.board.neighbors(city_coord))
            .collect();
        for &coord in &initial {
            if let Some(tile) = state.board.tile_mut(coord) {
                tile.owner = Some(civ_id);
            }
        }
        if let Some(c) = state.cities.iter_mut().find(|c| c.id == city_id) {
            c.territory = initial.into_iter().collect();
        }
    }

    // ── Unit-type registry ────────────────────────────────────────────────
    let warrior_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let builder_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let slinger_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let trader_type_id  = UnitTypeId::from_ulid(state.id_gen.next_ulid());
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
        UnitTypeDef { id: trader_type_id, name: "trader", production_cost: 40,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Trader,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None },
    ]);

    // ── Starting units ────────────────────────────────────────────────────
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: unit_id, unit_type: warrior_type_id, owner: civ_id,
        coord: city_coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(20),
        promotions: Vec::new(), health: 100, range: 0, vision_range: 2,
    });

    let builder_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: builder_id, unit_type: builder_type_id, owner: civ_id,
        coord: city_coord, domain: UnitDomain::Land, category: UnitCategory::Civilian,
        movement_left: 200, max_movement: 200, combat_strength: None,
        promotions: Vec::new(), health: 100, range: 0, vision_range: 2,
    });

    let trader_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: trader_id, unit_type: trader_type_id, owner: civ_id,
        coord: city_coord, domain: UnitDomain::Land, category: UnitCategory::Trader,
        movement_left: 200, max_movement: 200, combat_strength: None,
        promotions: Vec::new(), health: 100, range: 0, vision_range: 2,
    });

    recalculate_visibility(&mut state, civ_id);

    // ── Optional AI adversary: Babylon ────────────────────────────────────
    let ai_civ_id = if config.num_ai > 0 {
        let babylon_id = state.id_gen.next_civ_id();
        state.civilizations.push(Civilization::new(
            babylon_id, "Babylon", "Babylonian",
            Leader { name: "Hammurabi", civ_id: babylon_id,
                     abilities: Vec::new(), agenda: Box::new(NoOpAgenda) },
        ));

        let babylon_city_id = state.id_gen.next_city_id();
        let mut bc = City::new(babylon_city_id, "Babylon".to_string(),
                               babylon_id, babylon_city_coord);
        bc.is_capital = true;
        state.cities.push(bc);
        state.civilizations.iter_mut()
            .find(|c| c.id == babylon_id).unwrap()
            .cities.push(babylon_city_id);

        // Claim initial territory for Babylon.
        {
            let initial: Vec<HexCoord> = std::iter::once(babylon_city_coord)
                .chain(state.board.neighbors(babylon_city_coord))
                .collect();
            for &coord in &initial {
                if let Some(tile) = state.board.tile_mut(coord) {
                    tile.owner = Some(babylon_id);
                }
            }
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == babylon_city_id) {
                c.territory = initial.into_iter().collect();
            }
        }

        // Babylon's starting warrior.
        let bab_warrior = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id: bab_warrior, unit_type: warrior_type_id, owner: babylon_id,
            coord: babylon_city_coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200, combat_strength: Some(20),
            promotions: Vec::new(), health: 100, range: 0, vision_range: 2,
        });

        recalculate_visibility(&mut state, babylon_id);
        Some(babylon_id)
    } else {
        None
    };

    Session {
        state,
        civ_id,
        city_ids: vec![city_id],
        current_city: 0,
        selected_unit: Some(unit_id),
        ai_civ_id,
    }
}
