//! Run a full AI-vs-AI demo game and return per-turn GameViews.

use libciv::ai::{Agent, HeuristicAgent};
use libciv::civ::{BasicUnit, City, Civilization, Leader};
use libciv::game::recalculate_visibility;
use libciv::game::state::{GameState, UnitTypeDef};
use libciv::world::mapgen::{MapGenConfig, generate as mapgen_generate};
use libciv::{CivId, DefaultRulesEngine, TurnEngine, UnitCategory, UnitDomain, UnitTypeId};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::types::view::GameView;
use crate::server::projection::project_game_view;

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

/// Pool of civilizations available for demo games.
const CIV_POOL: &[(&str, &str, &str, &str)] = &[
    // (civ_name, adjective, leader_name, color_hint)
    ("Rome",    "Roman",      "Trajan",     "#e05050"),
    ("Babylon", "Babylonian", "Hammurabi",  "#50a0e0"),
    ("Greece",  "Greek",      "Pericles",   "#5090d0"),
    ("Egypt",   "Egyptian",   "Cleopatra",  "#e0c050"),
    ("Germany", "German",     "Barbarossa", "#707070"),
    ("Japan",   "Japanese",   "Hojo",       "#e06080"),
    ("India",   "Indian",     "Gandhi",     "#60c060"),
    ("Arabia",  "Arabian",    "Saladin",    "#c08040"),
];

/// Result of running a demo game.
#[derive(serde::Serialize)]
pub struct DemoGameResult {
    /// Per-turn snapshots, each containing views for all civilizations
    /// plus a spectator view as the last entry.
    pub turns: Vec<DemoTurnSnapshot>,
    /// Civ names in order (spectator is appended last).
    pub civ_names: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct DemoTurnSnapshot {
    pub turn: u32,
    /// One GameView per civ, plus one spectator view at the end.
    pub views: Vec<GameView>,
}

/// Run a full AI-vs-AI demo game and return the history.
pub fn run_demo_game(
    seed: u64,
    width: u32,
    height: u32,
    num_turns: u32,
    num_players: u32,
) -> DemoGameResult {
    let num_players = (num_players as usize).clamp(2, CIV_POOL.len());
    let mut state = GameState::new(seed, width, height);

    // ── Map generation ───────────────────────────────────────────────────
    let mapgen_result = mapgen_generate(
        &MapGenConfig {
            width, height, seed,
            land_fraction: None,
            num_continents: None,
            num_zone_seeds: None,
            num_starts: num_players as u32,
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
                      siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: settler_type, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None,
                      siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: builder_type, name: "builder", production_cost: 50,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None,
                      siege_bonus: 0, max_charges: 3, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: trader_type, name: "trader", production_cost: 40,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Trader,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None,
                      siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
    ]);

    // ── Spawn civilizations from pool ────────────────────────────────────
    let mut civ_ids: Vec<CivId> = Vec::with_capacity(num_players);
    let mut agents: Vec<HeuristicAgent> = Vec::with_capacity(num_players);
    let mut civ_names: Vec<String> = Vec::with_capacity(num_players + 1);

    let default_coord = |i: usize| {
        let angle = (i as f64) * std::f64::consts::TAU / (num_players as f64);
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let r = (width.min(height) as f64) / 3.0;
        HexCoord::from_qr((cx + r * angle.cos()) as i32, (cy + r * angle.sin()) as i32)
    };

    for (i, &(name, adjective, leader_name, _color)) in CIV_POOL.iter().enumerate().take(num_players) {
        let civ_id = state.id_gen.next_civ_id();
        let coord = starts.get(i).copied().unwrap_or_else(|| default_coord(i));

        state.civilizations.push(Civilization::new(
            civ_id, name, adjective,
            Leader { name: leader_name, civ_id, abilities: Vec::new(),
                     agenda: Box::new(NoOpAgenda) },
        ));

        // Capital city.
        let city_id = state.id_gen.next_city_id();
        let city_name = format!("{name} Capital");
        let mut city = City::new(city_id, city_name, civ_id, coord);
        city.is_capital = true;
        state.cities.push(city);
        state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap().cities.push(city_id);

        // Claim initial territory (center + ring-1).
        let initial: Vec<HexCoord> = std::iter::once(coord)
            .chain(state.board.neighbors(coord))
            .collect();
        for &c in &initial {
            if let Some(tile) = state.board.tile_mut(c) { tile.owner = Some(civ_id); }
        }
        if let Some(c) = state.cities.iter_mut().find(|c| c.id == city_id) {
            c.territory = initial.into_iter().collect();
        }

        // Starting warrior.
        state.units.push(BasicUnit {
            id: state.id_gen.next_unit_id(), unit_type: warrior_type, owner: civ_id,
            coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200, combat_strength: Some(20),
            promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2,
            charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        civ_ids.push(civ_id);
        agents.push(HeuristicAgent::new(civ_id));
        civ_names.push(name.to_string());
    }

    // Spectator label.
    civ_names.push("Spectator".to_string());

    // ── Initial visibility ───────────────────────────────────────────────
    for &cid in &civ_ids {
        recalculate_visibility(&mut state, cid);
    }

    // Make all tiles explored for all civs (spectator-friendly).
    let all_coords: Vec<HexCoord> = state.board.all_coords().into_iter().collect();
    for civ in &mut state.civilizations {
        for &coord in &all_coords {
            civ.explored_tiles.insert(coord);
            civ.visible_tiles.insert(coord);
        }
    }

    // ── Helper: build spectator view ─────────────────────────────────────
    // Use the first civ's projection (all tiles already visible) as spectator.
    let spectator_view = |state: &GameState, civ_ids: &[CivId]| -> GameView {
        project_game_view(state, civ_ids[0])
    };

    // ── AI + rules ───────────────────────────────────────────────────────
    let rules = DefaultRulesEngine;
    let engine = TurnEngine::new();

    // ── Record initial state ─────────────────────────────────────────────
    let mut turns = Vec::with_capacity(num_turns as usize + 1);
    {
        let mut views: Vec<GameView> = civ_ids.iter().map(|&cid| project_game_view(&state, cid)).collect();
        views.push(spectator_view(&state, &civ_ids));
        turns.push(DemoTurnSnapshot { turn: state.turn, views });
    }

    // ── Game loop ────────────────────────────────────────────────────────
    for _ in 0..num_turns {
        // 1. Advance turn (production, yields, etc.)
        engine.process_turn(&mut state, &rules);

        // 2. Reset movement.
        for unit in &mut state.units {
            unit.movement_left = unit.max_movement;
        }

        // 3. AI decisions.
        for agent in &agents {
            agent.take_turn(&mut state, &rules);
        }

        // 4. Recalculate visibility — keep spectator view showing everything.
        let all_coords: Vec<HexCoord> = state.board.all_coords().into_iter().collect();
        for civ in &mut state.civilizations {
            for &coord in &all_coords {
                civ.explored_tiles.insert(coord);
                civ.visible_tiles.insert(coord);
            }
        }

        // 5. Snapshot.
        let mut views: Vec<GameView> = civ_ids.iter().map(|&cid| project_game_view(&state, cid)).collect();
        views.push(spectator_view(&state, &civ_ids));
        turns.push(DemoTurnSnapshot { turn: state.turn, views });

        if state.game_over.is_some() {
            break;
        }
    }

    DemoGameResult { turns, civ_names }
}
