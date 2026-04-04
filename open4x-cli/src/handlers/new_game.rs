//! Handler for the `new-game` CLI command.
//!
//! Creates a new game with map generation, civilizations, starting units,
//! and victory conditions, then saves the initial state to disk.

use std::path::Path;

use libciv::civ::{BasicUnit, City, Civilization, Leader};
use libciv::civ::civilization::BuiltinAgenda;
use libciv::game::state::PlayerSlot;
use libciv::game::visibility::recalculate_visibility;
use libciv::world::mapgen::{MapGenConfig, generate as mapgen_generate};
use libciv::{
    BuiltinVictoryCondition, GameState, UnitCategory, UnitDomain,
};

use crate::state_io;

/// Create a new game and save it to `out`.
pub fn handle_new_game(
    out: &Path,
    seed: u64,
    width: u32,
    height: u32,
    players: &[String],
    ai_players: &[String],
) -> Result<(), String> {
    let total = players.len() + ai_players.len();
    if total == 0 {
        return Err("at least one player or AI player is required".into());
    }

    let mut state = GameState::new(seed, width, height);

    // ── Map generation ──────────────────────────────────────────────────────
    let mapgen_result = mapgen_generate(
        &MapGenConfig {
            width,
            height,
            seed,
            land_fraction: None,
            num_continents: None,
            num_zone_seeds: None,
            num_starts: total as u32,
        },
        &mut state.board,
    );
    let starts = &mapgen_result.starting_positions;

    // Look up a warrior-type UnitTypeDef from the built-in registry.
    let warrior_type_id = state
        .unit_type_defs
        .iter()
        .find(|d| d.name == "Warrior")
        .map(|d| d.id)
        .ok_or("no 'warrior' unit type in registry")?;
    let settler_type_id = state
        .unit_type_defs
        .iter()
        .find(|d| d.name == "Settler")
        .map(|d| d.id)
        .ok_or("no 'settler' unit type in registry")?;

    let warrior_def = state
        .unit_type_defs
        .iter()
        .find(|d| d.name == "Warrior")
        .cloned()
        .unwrap();
    let settler_def = state
        .unit_type_defs
        .iter()
        .find(|d| d.name == "Settler")
        .cloned()
        .unwrap();

    // ── Create civilizations ────────────────────────────────────────────────
    // For the CLI we use the player name as the civ name and adjective.
    // A real game would let the user pick from built-in civ identities.
    let all_names: Vec<(&str, bool)> = players
        .iter()
        .map(|n| (n.as_str(), false))
        .chain(ai_players.iter().map(|n| (n.as_str(), true)))
        .collect();

    for (i, &(name, is_ai)) in all_names.iter().enumerate() {
        let civ_id = state.id_gen.next_civ_id();
        let leader = Leader {
            name: "Leader",
            civ_id,
            agenda: BuiltinAgenda::Default,
        };
        // Leak name to &'static str — CLI strings live for the process lifetime.
        let static_name: &'static str = Box::leak(name.to_string().into_boxed_str());
        state
            .civilizations
            .push(Civilization::new(civ_id, static_name, static_name, leader));

        // Capital city at starting position.
        let city_coord = starts
            .get(i)
            .copied()
            .ok_or_else(|| format!("not enough starting positions for player {i}"))?;
        let city_id = state.id_gen.next_city_id();
        let mut city = City::new(
            city_id,
            format!("{name} Capital"),
            civ_id,
            city_coord,
        );
        city.is_capital = true;
        state.cities.push(city);
        state
            .civilizations
            .iter_mut()
            .find(|c| c.id == civ_id)
            .unwrap()
            .cities
            .push(city_id);

        // Starting Warrior at capital.
        let warrior_id = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id: warrior_id,
            unit_type: warrior_type_id,
            owner: civ_id,
            coord: city_coord,
            domain: UnitDomain::Land,
            category: UnitCategory::Combat,
            movement_left: warrior_def.max_movement,
            max_movement: warrior_def.max_movement,
            combat_strength: warrior_def.combat_strength,
            promotions: Vec::new(),
            experience: 0,
            health: 100,
            range: warrior_def.range,
            vision_range: warrior_def.vision_range,
            charges: None,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
        });

        // Starting Settler at capital.
        let settler_id = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id: settler_id,
            unit_type: settler_type_id,
            owner: civ_id,
            coord: city_coord,
            domain: UnitDomain::Land,
            category: UnitCategory::Civilian,
            movement_left: settler_def.max_movement,
            max_movement: settler_def.max_movement,
            combat_strength: settler_def.combat_strength,
            promotions: Vec::new(),
            experience: 0,
            health: 100,
            range: settler_def.range,
            vision_range: settler_def.vision_range,
            charges: None,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
        });

        // Player config slot.
        state.player_config.push(PlayerSlot {
            civ_name: name.to_string(),
            is_ai,
        });
    }

    // ── Victory conditions ──────────────────────────────────────────────────
    // Default: score victory at turn 500.
    let score_vc_id = state.id_gen.next_victory_id();
    state
        .victory_conditions
        .push(BuiltinVictoryCondition::Score {
            id: score_vc_id,
            turn_limit: 500,
        });

    // ── Initial visibility ──────────────────────────────────────────────────
    let civ_ids: Vec<_> = state.civilizations.iter().map(|c| c.id).collect();
    for cid in civ_ids {
        recalculate_visibility(&mut state, cid);
    }

    // ── Save ────────────────────────────────────────────────────────────────
    state_io::save_game_file(out, &state)?;

    let result = serde_json::json!({
        "success": true,
        "game_file": out.display().to_string(),
        "seed": seed,
        "map_size": [width, height],
        "players": players,
        "ai_players": ai_players,
        "turn": state.turn,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    Ok(())
}
