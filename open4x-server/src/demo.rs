//! Run a full AI-vs-AI demo game and return per-turn GameViews.

use libciv::ai::{Agent, HeuristicAgent};
use libciv::civ::{BasicUnit, City, Civilization, Leader};
use libciv::game::recalculate_visibility;
use libciv::game::state::{GameState, UnitTypeDef};
use libciv::world::mapgen::{MapGenConfig, generate as mapgen_generate};
use libciv::{CivId, DefaultRulesEngine, TurnEngine, UnitCategory, UnitDomain, UnitTypeId};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use open4x_api::view::GameView;
use crate::projection::project_game_view;

/// No-op agenda for AI civs.
struct NoOpAgenda;
impl std::fmt::Debug for NoOpAgenda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoOpAgenda")
    }
}
impl libciv::civ::Agenda for NoOpAgenda {
    fn name(&self) -> &'static str { "Expansionist" }
    fn description(&self) -> &'static str { "" }
    fn attitude(&self, _: CivId) -> i32 { 0 }
}

/// Result of running a demo game.
#[derive(serde::Serialize)]
pub struct DemoGameResult {
    /// Per-turn snapshots, each containing views for both civilizations.
    pub turns: Vec<DemoTurnSnapshot>,
    /// Civ names in order.
    pub civ_names: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct DemoTurnSnapshot {
    pub turn: u32,
    /// One GameView per civ (spectator view — all tiles visible).
    pub views: Vec<GameView>,
}

/// Run a full AI-vs-AI demo game and return the history.
pub fn run_demo_game(seed: u64, width: u32, height: u32, num_turns: u32) -> DemoGameResult {
    let mut state = GameState::new(seed, width, height);

    // ── Map generation ───────────────────────────────────────────────────
    let mapgen_result = mapgen_generate(
        &MapGenConfig {
            width, height, seed,
            land_fraction: None,
            num_continents: None,
            num_zone_seeds: None,
            num_starts: 2,
        },
        &mut state.board,
    );
    let starts = &mapgen_result.starting_positions;

    // ── Unit type registry ───────────────────────────────────────────────
    let warrior_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let builder_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let trader_type  = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef { id: warrior_type, name: "warrior", production_cost: 40,
                      max_movement: 200, combat_strength: Some(20),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None,
                      siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None },
        UnitTypeDef { id: settler_type, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None,
                      siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None },
        UnitTypeDef { id: builder_type, name: "builder", production_cost: 50,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None,
                      siege_bonus: 0, max_charges: 3, exclusive_to: None, replaces: None },
        UnitTypeDef { id: trader_type, name: "trader", production_cost: 40,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Trader,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None,
                      siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None },
    ]);

    // ── Civilization 1: Rome ─────────────────────────────────────────────
    let rome_id = state.id_gen.next_civ_id();
    let rome_coord = starts.first().copied()
        .unwrap_or(HexCoord::from_qr(width as i32 / 4, height as i32 / 2));
    state.civilizations.push(Civilization::new(
        rome_id, "Rome", "Roman",
        Leader { name: "Caesar", civ_id: rome_id, abilities: Vec::new(),
                 agenda: Box::new(NoOpAgenda) },
    ));
    let rome_city_id = state.id_gen.next_city_id();
    let mut rome_city = City::new(rome_city_id, "Roma".to_string(), rome_id, rome_coord);
    rome_city.is_capital = true;
    state.cities.push(rome_city);
    state.civilizations.iter_mut().find(|c| c.id == rome_id).unwrap().cities.push(rome_city_id);

    // Claim initial territory.
    {
        let initial: Vec<HexCoord> = std::iter::once(rome_coord)
            .chain(state.board.neighbors(rome_coord))
            .collect();
        for &coord in &initial {
            if let Some(tile) = state.board.tile_mut(coord) { tile.owner = Some(rome_id); }
        }
        if let Some(c) = state.cities.iter_mut().find(|c| c.id == rome_city_id) {
            c.territory = initial.into_iter().collect();
        }
    }

    state.units.push(BasicUnit {
        id: state.id_gen.next_unit_id(), unit_type: warrior_type, owner: rome_id,
        coord: rome_coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(20),
        promotions: Vec::new(), health: 100, range: 0, vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None,
    });

    // ── Civilization 2: Babylon ───────────────────────────────────────────
    let babylon_id = state.id_gen.next_civ_id();
    let babylon_coord = starts.get(1).copied()
        .unwrap_or(HexCoord::from_qr(width as i32 * 3 / 4, height as i32 / 2));
    state.civilizations.push(Civilization::new(
        babylon_id, "Babylon", "Babylonian",
        Leader { name: "Hammurabi", civ_id: babylon_id, abilities: Vec::new(),
                 agenda: Box::new(NoOpAgenda) },
    ));
    let babylon_city_id = state.id_gen.next_city_id();
    let mut babylon_city = City::new(babylon_city_id, "Babylon".to_string(), babylon_id, babylon_coord);
    babylon_city.is_capital = true;
    state.cities.push(babylon_city);
    state.civilizations.iter_mut().find(|c| c.id == babylon_id).unwrap().cities.push(babylon_city_id);

    {
        let initial: Vec<HexCoord> = std::iter::once(babylon_coord)
            .chain(state.board.neighbors(babylon_coord))
            .collect();
        for &coord in &initial {
            if let Some(tile) = state.board.tile_mut(coord) { tile.owner = Some(babylon_id); }
        }
        if let Some(c) = state.cities.iter_mut().find(|c| c.id == babylon_city_id) {
            c.territory = initial.into_iter().collect();
        }
    }

    state.units.push(BasicUnit {
        id: state.id_gen.next_unit_id(), unit_type: warrior_type, owner: babylon_id,
        coord: babylon_coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(20),
        promotions: Vec::new(), health: 100, range: 0, vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None,
    });

    // ── Initial visibility ───────────────────────────────────────────────
    recalculate_visibility(&mut state, rome_id);
    recalculate_visibility(&mut state, babylon_id);

    // Make all tiles explored for both civs so the spectator view shows everything.
    let all_coords: Vec<HexCoord> = state.board.all_coords().into_iter().collect();
    for civ in &mut state.civilizations {
        for &coord in &all_coords {
            civ.explored_tiles.insert(coord);
            civ.visible_tiles.insert(coord);
        }
    }

    // ── AI agents ────────────────────────────────────────────────────────
    let rome_agent = HeuristicAgent::new(rome_id);
    let babylon_agent = HeuristicAgent::new(babylon_id);
    let rules = DefaultRulesEngine;
    let engine = TurnEngine::new();
    let civ_ids = [rome_id, babylon_id];

    // ── Record initial state ─────────────────────────────────────────────
    let mut turns = Vec::with_capacity(num_turns as usize + 1);
    turns.push(DemoTurnSnapshot {
        turn: state.turn,
        views: civ_ids.iter().map(|&cid| project_game_view(&state, cid)).collect(),
    });

    // ── Game loop ────────────────────────────────────────────────────────
    for _ in 0..num_turns {
        // 1. Advance turn (production, yields, etc.)
        engine.process_turn(&mut state, &rules);

        // 2. Reset movement.
        for unit in &mut state.units {
            unit.movement_left = unit.max_movement;
        }

        // 3. AI decisions.
        rome_agent.take_turn(&mut state, &rules);
        babylon_agent.take_turn(&mut state, &rules);

        // 4. Recalculate visibility — make spectator view show everything.
        let all_coords: Vec<HexCoord> = state.board.all_coords().into_iter().collect();
        for civ in &mut state.civilizations {
            for &coord in &all_coords {
                civ.explored_tiles.insert(coord);
                civ.visible_tiles.insert(coord);
            }
        }

        // 5. Snapshot.
        turns.push(DemoTurnSnapshot {
            turn: state.turn,
            views: civ_ids.iter().map(|&cid| project_game_view(&state, cid)).collect(),
        });

        if state.game_over.is_some() {
            break;
        }
    }

    DemoGameResult {
        turns,
        civ_names: vec!["Rome".to_string(), "Babylon".to_string()],
    }
}
