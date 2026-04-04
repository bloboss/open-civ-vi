//! Server-side game session builder.
//!
//! Mirrors the logic from `civsim/src/main.rs`,
//! adapted for multiplayer: creates civs from player profiles + templates.

use std::sync::Arc;

use libciv::ai::HeuristicAgent;
use libciv::civ::{BasicUnit, BuiltinAgenda, City, Civilization, Leader};
use libciv::game::recalculate_visibility;
use libciv::game::state::{GameState, UnitTypeDef};
use libciv::world::mapgen::{MapGenConfig, generate as mapgen_generate};
use libciv::{CivId, UnitCategory, UnitDomain, UnitTypeId};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::types::ids::GameId;
use crate::types::messages::CreateGameRequest;

use crate::server::state::{AppState, PlayerSlot};

/// Intermediate result from building a server session.
pub struct ServerSession {
    pub state: GameState,
    pub players: Vec<PlayerSlot>,
    pub ai_agents: Vec<(CivId, HeuristicAgent)>,
}


pub fn build_server_session(
    req: &CreateGameRequest,
    creator_pubkey: &[u8; 32],
    app_state: &Arc<AppState>,
    _game_id: GameId,
) -> ServerSession {
    let w = req.width;
    let h = req.height;
    let seed = req.seed;
    let mut state = GameState::new(seed, w, h);

    // ── Map generation ───────────────────────────────────────────────────
    let num_starts = 1 + req.num_ai;
    let mapgen_result = mapgen_generate(
        &MapGenConfig {
            width: w, height: h, seed,
            land_fraction: None,
            num_continents: None,
            num_zone_seeds: None,
            num_starts,
        },
        &mut state.board,
    );
    let starts = &mapgen_result.starting_positions;

    // ── Unit type registry ───────────────────────────────────────────────
    let warrior_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let builder_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let trader_type_id  = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef { id: warrior_type_id, name: "warrior", production_cost: 40,
                      max_movement: 200, combat_strength: Some(20),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: settler_type_id, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: builder_type_id, name: "builder", production_cost: 50,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: trader_type_id, name: "trader", production_cost: 40,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Trader,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
    ]);

    // ── Creator's civilization ────────────────────────────────────────────
    let player_record = app_state.players.get(creator_pubkey);
    let template_id = player_record.as_ref()
        .map(|r| r.selected_template)
        .unwrap_or(app_state.templates[0].id);
    let template = app_state.templates.iter()
        .find(|t| t.id == template_id)
        .unwrap_or(&app_state.templates[0]);
    let display_name = player_record.as_ref()
        .map(|r| r.display_name.clone())
        .unwrap_or_else(|| "Player".to_string());

    let civ_id = state.id_gen.next_civ_id();
    let city_coord = starts.first().copied()
        .unwrap_or(HexCoord::from_qr(w as i32 / 4, h as i32 / 2));

    // Use template for civ name; use leaked str for &'static str fields.
    let civ_name: &'static str = Box::leak(template.civ_name.clone().into_boxed_str());
    let adjective: &'static str = Box::leak(template.adjective.clone().into_boxed_str());
    let leader_name: &'static str = Box::leak(display_name.clone().into_boxed_str());

    state.civilizations.push(Civilization::new(
        civ_id, civ_name, adjective,
        Leader { name: leader_name, civ_id,
                 agenda: BuiltinAgenda::Default },
    ));

    // Capital city.
    let city_id = state.id_gen.next_city_id();
    let mut city = City::new(city_id, format!("{} Capital", template.civ_name), civ_id, city_coord);
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations[0].cities.push(city_id);

    // Claim initial territory.
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

    // Starting units.
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: unit_id, unit_type: warrior_type_id, owner: civ_id,
        coord: city_coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(20),
        promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let builder_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: builder_id, unit_type: builder_type_id, owner: civ_id,
        coord: city_coord, domain: UnitDomain::Land, category: UnitCategory::Civilian,
        movement_left: 200, max_movement: 200, combat_strength: None,
        promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    recalculate_visibility(&mut state, civ_id);

    let players = vec![
        PlayerSlot {
            civ_id,
            pubkey: *creator_pubkey,
            profile: crate::types::profile::ProfileView {
                pubkey: creator_pubkey.to_vec(),
                display_name,
                selected_template: template_id,
            },
            submitted_turn: false,
        },
    ];

    // ── AI opponents ─────────────────────────────────────────────────────
    let mut ai_agents = Vec::new();
    for i in 0..req.num_ai {
        let ai_template = &app_state.templates[((i + 1) as usize) % app_state.templates.len()];
        let ai_civ_id = state.id_gen.next_civ_id();
        let ai_coord = starts.get((i + 1) as usize).copied()
            .unwrap_or(HexCoord::from_qr(w as i32 * 3 / 4, h as i32 / 2));

        let ai_civ_name: &'static str = Box::leak(ai_template.civ_name.clone().into_boxed_str());
        let ai_adjective: &'static str = Box::leak(ai_template.adjective.clone().into_boxed_str());
        let ai_leader: &'static str = Box::leak(ai_template.leader_name.clone().into_boxed_str());

        state.civilizations.push(Civilization::new(
            ai_civ_id, ai_civ_name, ai_adjective,
            Leader { name: ai_leader, civ_id: ai_civ_id,
                     agenda: BuiltinAgenda::Default },
        ));

        let ai_city_id = state.id_gen.next_city_id();
        let mut ai_city = City::new(ai_city_id, format!("{} Capital", ai_template.civ_name),
                                     ai_civ_id, ai_coord);
        ai_city.is_capital = true;
        state.cities.push(ai_city);
        state.civilizations.iter_mut()
            .find(|c| c.id == ai_civ_id).unwrap()
            .cities.push(ai_city_id);

        // Claim territory.
        {
            let initial: Vec<HexCoord> = std::iter::once(ai_coord)
                .chain(state.board.neighbors(ai_coord))
                .collect();
            for &coord in &initial {
                if let Some(tile) = state.board.tile_mut(coord) {
                    tile.owner = Some(ai_civ_id);
                }
            }
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == ai_city_id) {
                c.territory = initial.into_iter().collect();
            }
        }

        // AI warrior.
        let ai_warrior = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id: ai_warrior, unit_type: warrior_type_id, owner: ai_civ_id,
            coord: ai_coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200, combat_strength: Some(20),
            promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        recalculate_visibility(&mut state, ai_civ_id);
        ai_agents.push((ai_civ_id, HeuristicAgent::new(ai_civ_id)));
    }

    ServerSession { state, players, ai_agents }
}
