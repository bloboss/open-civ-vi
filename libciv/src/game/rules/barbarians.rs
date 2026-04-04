//! Barbarian turn phase logic and player interaction methods.
//!
//! ## Turn Phase (called from `advance_turn`)
//! 1. Attempt to spawn new camps on unclaimed land tiles.
//! 2. For each camp whose scout has not yet spawned, spawn a scout.
//! 3. Move scouts: exploring scouts wander; returning scouts head home.
//! 4. When an exploring scout sees a player unit/city, mark it returning.
//! 5. When a returning scout reaches the camp, mark it returned.
//! 6. For returned camps, generate combat units on a timer.
//! 7. (Clans mode) Advance conversion progress; convert eligible camps.
//!
//! ## Player Interactions (Clans mode, called from RulesEngine methods)
//! - `hire_from_camp`: pay gold, get a unit, start cooldown.
//! - `bribe_camp`: pay gold, set non-aggression + conversion bonus.
//! - `incite_camp`: pay gold, set aggression target + conversion penalty.

use crate::civ::barbarian::{BarbarianCamp, ClanInteraction, ClanType, ScoutState};
use crate::civ::city_state::{CityStateData, CityStateType};
use crate::civ::{BasicUnit, City, CityKind};
use crate::{BarbarianCampId, CivId, UnitId};
use crate::world::terrain::BuiltinTerrain;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;
use super::RulesError;

// ── Turn Phase: barbarian processing ─────────────────────────────────────────

/// Run all barbarian processing for one turn. Called from `advance_turn`.
pub(crate) fn process_barbarian_turn(state: &mut GameState, diff: &mut GameStateDiff) {
    if !state.barbarian_config.enabled {
        return;
    }

    // Ensure barbarian civ exists.
    if state.barbarian_civ.is_none() {
        let civ_id = state.id_gen.next_civ_id();
        state.barbarian_civ = Some(civ_id);
        // The barbarian civ is not added to state.civilizations — it is a
        // virtual faction. Units owned by it participate in combat but have
        // no cities, research, or diplomacy.
    }

    spawn_camps(state, diff);
    spawn_scouts(state, diff);
    move_and_check_scouts(state, diff);
    generate_combat_units(state, diff);

    if state.barbarian_config.clans_mode {
        advance_conversion(state, diff);
    }
}

// ── Camp spawning ────────────────────────────────────────────────────────────

fn spawn_camps(state: &mut GameState, diff: &mut GameStateDiff) {
    let config = &state.barbarian_config;
    let current_camps = state.barbarian_camps.iter().filter(|c| !c.converted).count();
    let max_camps = config.max_camps_per_major_civ * (state.civilizations.len() as u32).max(1);
    if current_camps >= max_camps as usize {
        return;
    }

    let min_city_dist = config.min_distance_from_city;
    let min_camp_dist = config.min_distance_between_camps;
    let spawn_chance = config.spawn_chance_per_tile;
    let clans_mode = config.clans_mode;

    // Collect city coords and existing camp coords.
    let city_coords: Vec<HexCoord> = state.cities.iter().map(|c| c.coord).collect();
    let camp_coords: Vec<HexCoord> = state.barbarian_camps.iter()
        .filter(|c| !c.converted)
        .map(|c| c.coord)
        .collect();

    // Collect visible tiles from all player civs for fog-of-war check.
    let visible_to_players: std::collections::HashSet<HexCoord> = state.civilizations.iter()
        .flat_map(|civ| civ.visible_tiles.iter().copied())
        .collect();

    // Find candidate tiles: land, unclaimed, in fog-of-war, far from cities and other camps.
    let all_coords = state.board.all_coords();
    let mut candidates: Vec<HexCoord> = Vec::new();
    for coord in &all_coords {
        let tile = match state.board.tile(*coord) {
            Some(t) => t,
            None => continue,
        };
        // Must be passable land.
        if matches!(tile.terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast | BuiltinTerrain::Mountain) {
            continue;
        }
        // Must be unclaimed.
        if tile.owner.is_some() {
            continue;
        }
        // Must be in fog-of-war (not visible to any player civ).
        if visible_to_players.contains(coord) {
            continue;
        }
        // Distance from cities.
        if city_coords.iter().any(|c| coord.distance(c) < min_city_dist) {
            continue;
        }
        // Distance from other camps.
        if camp_coords.iter().any(|c| coord.distance(c) < min_camp_dist) {
            continue;
        }
        // No unit already here.
        if state.units.iter().any(|u| u.coord == *coord) {
            continue;
        }
        candidates.push(*coord);
    }

    if candidates.is_empty() {
        return;
    }

    // Probabilistic spawning: each candidate tile has spawn_chance_per_tile probability.
    // We pick among candidates that pass the roll.
    let mut spawnable: Vec<HexCoord> = Vec::new();
    for &coord in &candidates {
        if state.id_gen.next_f32() < spawn_chance {
            spawnable.push(coord);
        }
    }
    if spawnable.is_empty() {
        return;
    }

    // Pick one from the spawnable set.
    let idx = (state.id_gen.next_f32() * spawnable.len() as f32) as usize;
    let idx = idx.min(spawnable.len() - 1);
    let coord = spawnable[idx];

    let barb_civ = state.barbarian_civ.unwrap();
    let camp_id = state.id_gen.next_barbarian_camp_id();

    let clan_type = if clans_mode {
        let tile = state.board.tile(coord).unwrap();
        let roll = state.id_gen.next_f32();
        Some(pick_clan_type(tile.terrain, roll))
    } else {
        None
    };

    let camp = BarbarianCamp::new(camp_id, coord, barb_civ, state.turn, clan_type);
    state.barbarian_camps.push(camp);
    diff.push(StateDelta::BarbarianCampSpawned { camp: camp_id, coord });
}

fn pick_clan_type(terrain: BuiltinTerrain, roll: f32) -> ClanType {
    match terrain {
        BuiltinTerrain::Plains | BuiltinTerrain::Grassland => {
            if roll < 0.4 { ClanType::Flatland }
            else if roll < 0.7 { ClanType::Rover }
            else { ClanType::Chariot }
        }
        BuiltinTerrain::Desert | BuiltinTerrain::Tundra | BuiltinTerrain::Snow => {
            if roll < 0.5 { ClanType::Hills }
            else { ClanType::Flatland }
        }
        _ => {
            if roll < 0.4 { ClanType::Woodland }
            else if roll < 0.7 { ClanType::Jungle }
            else { ClanType::Seafaring }
        }
    }
}

// ── Scout spawning ───────────────────────────────────────────────────────────

fn spawn_scouts(state: &mut GameState, diff: &mut GameStateDiff) {
    let barb_civ = match state.barbarian_civ {
        Some(c) => c,
        None => return,
    };

    // Collect camps that need scouts.
    let camps_needing_scouts: Vec<(BarbarianCampId, HexCoord)> = state.barbarian_camps.iter()
        .filter(|c| !c.converted && c.scout_state == ScoutState::NotSpawned)
        .map(|c| (c.id, c.coord))
        .collect();

    for (camp_id, coord) in camps_needing_scouts {
        // Find a scout unit type, or use warrior as fallback.
        let scout_type = state.unit_type_defs.iter()
            .find(|d| d.name == "Scout")
            .or_else(|| state.unit_type_defs.iter().find(|d| d.name.eq_ignore_ascii_case("Warrior")));

        let def = match scout_type {
            Some(d) => d.clone(),
            None => continue,
        };

        let unit_id = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id: unit_id,
            unit_type: def.id,
            owner: barb_civ,
            coord,
            domain: def.domain,
            category: def.category,
            movement_left: def.max_movement,
            max_movement: def.max_movement,
            combat_strength: def.combat_strength,
            promotions: Vec::new(),
            health: 100,
            range: def.range,
            vision_range: def.vision_range,
            charges: None,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
        });

        if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == camp_id) {
            camp.scout_state = ScoutState::Exploring { scout_id: unit_id };
        }

        diff.push(StateDelta::BarbarianScoutSpawned { camp: camp_id, scout: unit_id, coord });
        diff.push(StateDelta::UnitCreated { unit: unit_id, coord, owner: barb_civ });

        // Also spawn a melee defender unit at the camp.
        let defender_def = state.unit_type_defs.iter()
            .find(|d| d.name.eq_ignore_ascii_case("Warrior"))
            .cloned();

        if let Some(ddef) = defender_def {
            let defender_coord = find_spawn_coord(state, coord).unwrap_or(coord);
            let defender_id = state.id_gen.next_unit_id();
            state.units.push(BasicUnit {
                id: defender_id,
                unit_type: ddef.id,
                owner: barb_civ,
                coord: defender_coord,
                domain: ddef.domain,
                category: ddef.category,
                movement_left: ddef.max_movement,
                max_movement: ddef.max_movement,
                combat_strength: ddef.combat_strength,
                promotions: Vec::new(),
                health: 100,
                range: ddef.range,
                vision_range: ddef.vision_range,
                charges: None,
                trade_origin: None,
                trade_destination: None,
                religion_id: None,
                spread_charges: None,
                religious_strength: None,
            });

            if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == camp_id) {
                camp.spawned_units.push(defender_id);
                camp.units_spawned_count += 1;
            }

            diff.push(StateDelta::BarbarianUnitGenerated { camp: camp_id, unit: defender_id, coord: defender_coord });
            diff.push(StateDelta::UnitCreated { unit: defender_id, coord: defender_coord, owner: barb_civ });
        }
    }
}

// ── Scout movement and discovery ─────────────────────────────────────────────

fn move_and_check_scouts(state: &mut GameState, diff: &mut GameStateDiff) {
    // Collect all camp scout info to avoid borrow conflicts.
    struct ScoutInfo {
        camp_id: BarbarianCampId,
        camp_coord: HexCoord,
        scout_id: UnitId,
        is_returning: bool,
        target_civ: Option<CivId>,
    }

    let scout_infos: Vec<ScoutInfo> = state.barbarian_camps.iter()
        .filter(|c| !c.converted)
        .filter_map(|c| {
            match &c.scout_state {
                ScoutState::Exploring { scout_id } => Some(ScoutInfo {
                    camp_id: c.id,
                    camp_coord: c.coord,
                    scout_id: *scout_id,
                    is_returning: false,
                    target_civ: None,
                }),
                ScoutState::Returning { scout_id, discovered_civ } => Some(ScoutInfo {
                    camp_id: c.id,
                    camp_coord: c.coord,
                    scout_id: *scout_id,
                    is_returning: true,
                    target_civ: Some(*discovered_civ),
                }),
                _ => None,
            }
        })
        .collect();

    for info in scout_infos {
        // Check if scout is still alive.
        let scout_pos = match state.units.iter().find(|u| u.id == info.scout_id) {
            Some(u) => u.coord,
            None => {
                // Scout was killed; reset camp state.
                if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == info.camp_id) {
                    camp.scout_state = ScoutState::NotSpawned;
                }
                continue;
            }
        };

        if info.is_returning {
            // Move toward camp.
            let next = step_toward(scout_pos, info.camp_coord, state);
            if let Some(next_coord) = next {
                move_barbarian_unit(state, info.scout_id, next_coord, diff);
            }

            // Check arrival.
            let current_pos = state.units.iter()
                .find(|u| u.id == info.scout_id)
                .map(|u| u.coord);
            if current_pos == Some(info.camp_coord) {
                // Scout returned! Remove the scout unit and mark camp.
                state.units.retain(|u| u.id != info.scout_id);
                diff.push(StateDelta::UnitDestroyed { unit: info.scout_id });

                let discovered_civ = info.target_civ.unwrap();
                if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == info.camp_id) {
                    camp.scout_state = ScoutState::Returned {
                        discovered_civs: vec![discovered_civ],
                    };
                }
                diff.push(StateDelta::BarbarianScoutReturned { camp: info.camp_id });
            }
        } else {
            // Exploring: move in a random-ish direction away from camp.
            let next = explore_step(scout_pos, info.camp_coord, state);
            if let Some(next_coord) = next {
                move_barbarian_unit(state, info.scout_id, next_coord, diff);
            }

            // Check if the scout can see any player civ's units or cities.
            let scout_pos = state.units.iter()
                .find(|u| u.id == info.scout_id)
                .map(|u| (u.coord, u.vision_range));

            if let Some((pos, vision)) = scout_pos {
                let barb_civ = state.barbarian_civ.unwrap();
                let discovered = find_nearby_player(state, pos, vision as u32, barb_civ);
                if let Some(civ_id) = discovered {
                    if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == info.camp_id) {
                        camp.scout_state = ScoutState::Returning {
                            scout_id: info.scout_id,
                            discovered_civ: civ_id,
                        };
                    }
                    diff.push(StateDelta::BarbarianScoutDiscovered {
                        camp: info.camp_id,
                        scout: info.scout_id,
                        discovered_civ: civ_id,
                    });
                }
            }
        }
    }
}

/// Find a player civ with a unit or city within `radius` of `center`.
fn find_nearby_player(state: &GameState, center: HexCoord, radius: u32, barb_civ: CivId) -> Option<CivId> {
    // Check units.
    for unit in &state.units {
        if unit.owner != barb_civ && center.distance(&unit.coord) <= radius {
            return Some(unit.owner);
        }
    }
    // Check cities.
    for city in &state.cities {
        if city.owner != barb_civ && center.distance(&city.coord) <= radius {
            return Some(city.owner);
        }
    }
    None
}

// ── Combat unit generation ───────────────────────────────────────────────────

fn generate_combat_units(state: &mut GameState, diff: &mut GameStateDiff) {
    let barb_civ = match state.barbarian_civ {
        Some(c) => c,
        None => return,
    };
    let boldness_per_turn = state.barbarian_config.boldness_per_turn;
    let threshold = state.barbarian_config.boldness_spawn_threshold;

    // Collect camp info to avoid borrow conflicts.
    struct GenInfo {
        camp_id: BarbarianCampId,
        coord: HexCoord,
        preferred_unit: &'static str,
        triggered: bool,
    }

    let gen_infos: Vec<GenInfo> = state.barbarian_camps.iter()
        .filter(|c| !c.converted)
        .map(|c| GenInfo {
            camp_id: c.id,
            coord: c.coord,
            preferred_unit: c.preferred_unit_type(),
            triggered: c.is_triggered(),
        })
        .collect();

    for info in gen_infos {
        // Increment boldness: +2/turn for triggered camps, +1/turn for untriggered.
        let increment = if info.triggered { boldness_per_turn } else { (boldness_per_turn / 2).max(1) };
        if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == info.camp_id) {
            camp.boldness += increment;
        }

        // Check if boldness >= threshold → spawn a unit.
        let boldness = state.barbarian_camps.iter()
            .find(|c| c.id == info.camp_id)
            .map(|c| c.boldness)
            .unwrap_or(0);
        if boldness < threshold {
            continue;
        }

        // Reset boldness after spawning.
        if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == info.camp_id) {
            camp.boldness = 0;
        }

        // Find unit type def.
        let def = state.unit_type_defs.iter()
            .find(|d| d.name.eq_ignore_ascii_case(info.preferred_unit))
            .or_else(|| state.unit_type_defs.iter().find(|d| d.name.eq_ignore_ascii_case("Warrior")));

        let def = match def {
            Some(d) => d.clone(),
            None => continue,
        };

        // Find a free adjacent tile to spawn on (or the camp tile itself).
        let spawn_coord = find_spawn_coord(state, info.coord);
        let spawn_coord = match spawn_coord {
            Some(c) => c,
            None => continue,
        };

        let unit_id = state.id_gen.next_unit_id();
        state.units.push(BasicUnit {
            id: unit_id,
            unit_type: def.id,
            owner: barb_civ,
            coord: spawn_coord,
            domain: def.domain,
            category: def.category,
            movement_left: def.max_movement,
            max_movement: def.max_movement,
            combat_strength: def.combat_strength,
            promotions: Vec::new(),
            health: 100,
            range: def.range,
            vision_range: def.vision_range,
            charges: None,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
        });

        if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == info.camp_id) {
            camp.spawned_units.push(unit_id);
            camp.units_spawned_count += 1;
        }

        diff.push(StateDelta::BarbarianUnitGenerated { camp: info.camp_id, unit: unit_id, coord: spawn_coord });
        diff.push(StateDelta::UnitCreated { unit: unit_id, coord: spawn_coord, owner: barb_civ });
    }
}

fn find_spawn_coord(state: &GameState, center: HexCoord) -> Option<HexCoord> {
    // Try center tile first.
    let occupied = state.units.iter().any(|u| u.coord == center);
    if !occupied {
        return Some(center);
    }
    // Try neighbors.
    for neighbor in center.neighbors() {
        if let Some(tile) = state.board.tile(neighbor) {
            if matches!(tile.terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast | BuiltinTerrain::Mountain) {
                continue;
            }
            if !state.units.iter().any(|u| u.coord == neighbor) {
                return Some(neighbor);
            }
        }
    }
    None
}

// ── Barbarian Clans: conversion to city-state ────────────────────────────────

fn advance_conversion(state: &mut GameState, diff: &mut GameStateDiff) {
    let config = state.barbarian_config.clone();

    // Collect camps eligible for conversion progress.
    let camp_ids: Vec<(BarbarianCampId, HexCoord)> = state.barbarian_camps.iter()
        .filter(|c| !c.converted && c.clan_type.is_some())
        .map(|c| (c.id, c.coord))
        .collect();

    for (camp_id, camp_coord) in camp_ids {
        // Probabilistic advancement: conversion_increment_chance each turn.
        let roll = state.id_gen.next_f32();
        if roll >= config.conversion_increment_chance {
            continue;
        }

        // Sum food yields of unowned tiles within radius 2 of camp.
        let mut food_total = 0i32;
        // Center tile.
        if let Some(tile) = state.board.tile(camp_coord)
            && tile.owner.is_none()
        {
            food_total += tile.terrain.base_yields().food;
        }
        // Rings 1 and 2.
        for r in 1..=2u32 {
            for tile_coord in camp_coord.ring(r) {
                if let Some(tile) = state.board.tile(tile_coord)
                    && tile.owner.is_none()
                {
                    food_total += tile.terrain.base_yields().food;
                }
            }
        }
        let increment = food_total.max(1); // at least 1

        if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == camp_id) {
            camp.conversion_progress += increment;
        }

        // Check conversion threshold.
        let progress = state.barbarian_camps.iter()
            .find(|c| c.id == camp_id)
            .map(|c| c.conversion_progress)
            .unwrap_or(0);
        if progress >= config.conversion_threshold {
            let coord = camp_coord;
            let cs_type = state.barbarian_camps.iter()
                .find(|c| c.id == camp_id)
                .map(|c| c.conversion_city_state_type())
                .unwrap_or(CityStateType::Militaristic);

            // Validate city-placement rules: no city within 3 tiles.
            let too_close = state.cities.iter().any(|c| coord.distance(&c.coord) < 4);
            if too_close {
                continue;
            }

            // Collect units to remove before mutating.
            let (units_to_remove, scout_to_remove) = {
                let camp = state.barbarian_camps.iter().find(|c| c.id == camp_id).unwrap();
                let units: Vec<UnitId> = camp.spawned_units.clone();
                let scout = match &camp.scout_state {
                    ScoutState::Exploring { scout_id } | ScoutState::Returning { scout_id, .. } => Some(*scout_id),
                    _ => None,
                };
                (units, scout)
            };

            // Mark converted.
            if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == camp_id) {
                camp.converted = true;
            }

            // Remove all barbarian units from this camp.
            for uid in &units_to_remove {
                state.units.retain(|u| u.id != *uid);
                diff.push(StateDelta::UnitDestroyed { unit: *uid });
            }

            // Also remove any scout.
            if let Some(sid) = scout_to_remove {
                state.units.retain(|u| u.id != sid);
                diff.push(StateDelta::UnitDestroyed { unit: sid });
            }

            // Create city-state civ and city.
            let cs_civ_id = state.id_gen.next_civ_id();
            let city_id = state.id_gen.next_city_id();

            let cs_name = city_state_name(cs_type);
            let mut city = City::new(city_id, cs_name.to_string(), cs_civ_id, coord);
            city.kind = CityKind::CityState(CityStateData::new(cs_type));
            city.is_capital = true;
            city.territory.insert(coord);
            // Claim ring-1 tiles.
            for neighbor in coord.neighbors() {
                if let Some(tile) = state.board.tile_mut(neighbor)
                    && tile.owner.is_none()
                {
                    tile.owner = Some(cs_civ_id);
                    city.territory.insert(neighbor);
                }
            }
            // Claim center tile.
            if let Some(tile) = state.board.tile_mut(coord) {
                tile.owner = Some(cs_civ_id);
            }

            state.cities.push(city);

            diff.push(StateDelta::BarbarianCampConverted { camp: camp_id, city: city_id, coord });
            diff.push(StateDelta::CityFounded { city: city_id, coord, owner: cs_civ_id });
        }
    }
}

fn city_state_name(cs_type: CityStateType) -> &'static str {
    match cs_type {
        CityStateType::Militaristic => "Valletta",
        CityStateType::Scientific   => "Geneva",
        CityStateType::Trade        => "Zanzibar",
        CityStateType::Industrial   => "Brussels",
        CityStateType::Cultural     => "Kumasi",
        CityStateType::Religious    => "Yerevan",
    }
}

// ── Player interactions (Clans mode) ─────────────────────────────────────────

/// Hire a unit from a barbarian camp. Clans mode only.
pub(crate) fn hire_from_camp(
    state: &mut GameState,
    camp_id: BarbarianCampId,
    civ_id: CivId,
) -> Result<GameStateDiff, RulesError> {
    let config = state.barbarian_config.clone();
    if !config.clans_mode {
        return Err(RulesError::BarbarianClansNotEnabled);
    }

    let camp = state.barbarian_camps.iter().find(|c| c.id == camp_id)
        .ok_or(RulesError::BarbarianCampNotFound)?;

    if camp.converted {
        return Err(RulesError::BarbarianCampNotFound);
    }

    if !camp.can_hire(civ_id, state.turn, config.hire_cooldown) {
        return Err(RulesError::BarbarianHireOnCooldown);
    }

    // Check gold.
    let civ = state.civilizations.iter().find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    if (civ.gold as u32) < config.hire_cost {
        return Err(RulesError::InsufficientGold);
    }

    let coord = camp.coord;
    let unit_name = camp.preferred_unit_type();
    let hire_cost = config.hire_cost;

    // Find the unit type def.
    let def = state.unit_type_defs.iter()
        .find(|d| d.name.eq_ignore_ascii_case(unit_name))
        .or_else(|| state.unit_type_defs.iter().find(|d| d.name.eq_ignore_ascii_case("Warrior")))
        .cloned()
        .ok_or(RulesError::UnitNotFound)?;

    // Find spawn coord near the camp for the hired unit.
    let spawn = find_spawn_coord(state, coord).ok_or(RulesError::TileOccupiedByUnit)?;

    let mut diff = GameStateDiff::new();

    // Deduct gold.
    if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
        civ.gold -= hire_cost as i32;
    }
    diff.push(StateDelta::GoldChanged { civ: civ_id, delta: -(hire_cost as i32) });

    // Spawn unit owned by the player.
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: unit_id,
        unit_type: def.id,
        owner: civ_id,
        coord: spawn,
        domain: def.domain,
        category: def.category,
        movement_left: 0, // just hired, no movement this turn
        max_movement: def.max_movement,
        combat_strength: def.combat_strength,
        promotions: Vec::new(),
        health: 100,
        range: def.range,
        vision_range: def.vision_range,
        charges: if def.max_charges > 0 { Some(def.max_charges) } else { None },
        trade_origin: None,
        trade_destination: None,
        religion_id: None,
        spread_charges: None,
        religious_strength: None,
    });
    diff.push(StateDelta::UnitCreated { unit: unit_id, coord: spawn, owner: civ_id });

    // Record interaction and apply one-shot conversion points.
    if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == camp_id) {
        camp.clan_interactions.push((civ_id, ClanInteraction::Hired { turn: state.turn }));
        camp.conversion_progress += config.hire_conversion_points;
    }

    diff.push(StateDelta::BarbarianClanHired {
        camp: camp_id, civ: civ_id, unit: unit_id, gold_spent: hire_cost,
    });

    Ok(diff)
}

/// Bribe a barbarian camp not to attack. Clans mode only.
pub(crate) fn bribe_camp(
    state: &mut GameState,
    camp_id: BarbarianCampId,
    civ_id: CivId,
) -> Result<GameStateDiff, RulesError> {
    let config = state.barbarian_config.clone();
    if !config.clans_mode {
        return Err(RulesError::BarbarianClansNotEnabled);
    }

    let camp = state.barbarian_camps.iter().find(|c| c.id == camp_id)
        .ok_or(RulesError::BarbarianCampNotFound)?;
    if camp.converted {
        return Err(RulesError::BarbarianCampNotFound);
    }

    let civ = state.civilizations.iter().find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    if (civ.gold as u32) < config.bribe_cost {
        return Err(RulesError::InsufficientGold);
    }

    let bribe_cost = config.bribe_cost;
    let expires = state.turn + config.bribe_duration;

    let mut diff = GameStateDiff::new();

    // Deduct gold.
    if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
        civ.gold -= bribe_cost as i32;
    }
    diff.push(StateDelta::GoldChanged { civ: civ_id, delta: -(bribe_cost as i32) });

    // Record interaction and apply one-shot conversion points.
    if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == camp_id) {
        camp.clan_interactions.push((civ_id, ClanInteraction::Bribed { expires_turn: expires }));
        camp.conversion_progress += config.bribe_conversion_points;
    }

    diff.push(StateDelta::BarbarianClanBribed { camp: camp_id, civ: civ_id, gold_spent: bribe_cost });

    Ok(diff)
}

/// Incite a barbarian camp against another player. Clans mode only.
pub(crate) fn incite_camp(
    state: &mut GameState,
    camp_id: BarbarianCampId,
    civ_id: CivId,
    target: CivId,
) -> Result<GameStateDiff, RulesError> {
    let config = state.barbarian_config.clone();
    if !config.clans_mode {
        return Err(RulesError::BarbarianClansNotEnabled);
    }

    if civ_id == target {
        return Err(RulesError::SameCivilization);
    }

    let camp = state.barbarian_camps.iter().find(|c| c.id == camp_id)
        .ok_or(RulesError::BarbarianCampNotFound)?;
    if camp.converted {
        return Err(RulesError::BarbarianCampNotFound);
    }

    // Verify target exists.
    if !state.civilizations.iter().any(|c| c.id == target) {
        return Err(RulesError::CivNotFound);
    }

    let civ = state.civilizations.iter().find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    if (civ.gold as u32) < config.incite_cost {
        return Err(RulesError::InsufficientGold);
    }

    let incite_cost = config.incite_cost;
    let expires = state.turn + config.incite_duration;

    let mut diff = GameStateDiff::new();

    // Deduct gold.
    if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
        civ.gold -= incite_cost as i32;
    }
    diff.push(StateDelta::GoldChanged { civ: civ_id, delta: -(incite_cost as i32) });

    // Record interaction and apply one-shot conversion points.
    if let Some(camp) = state.barbarian_camps.iter_mut().find(|c| c.id == camp_id) {
        camp.clan_interactions.push((civ_id, ClanInteraction::Incited { target, expires_turn: expires }));
        camp.conversion_progress += config.incite_conversion_points;
    }

    diff.push(StateDelta::BarbarianClanIncited {
        camp: camp_id, civ: civ_id, target, gold_spent: incite_cost,
    });

    Ok(diff)
}

/// Clear a barbarian camp (called when a player unit moves onto the camp tile
/// and defeats all defenders).
pub(crate) fn clear_camp(
    state: &mut GameState,
    camp_id: BarbarianCampId,
    cleared_by: CivId,
) -> GameStateDiff {
    let mut diff = GameStateDiff::new();

    let camp = match state.barbarian_camps.iter().find(|c| c.id == camp_id) {
        Some(c) => c,
        None => return diff,
    };
    let coord = camp.coord;

    // Remove all units belonging to this camp.
    let units_to_remove: Vec<UnitId> = camp.spawned_units.clone();
    let scout_id = match &camp.scout_state {
        ScoutState::Exploring { scout_id } | ScoutState::Returning { scout_id, .. } => Some(*scout_id),
        _ => None,
    };

    for uid in &units_to_remove {
        if state.units.iter().any(|u| u.id == *uid) {
            state.units.retain(|u| u.id != *uid);
            diff.push(StateDelta::UnitDestroyed { unit: *uid });
        }
    }
    if let Some(sid) = scout_id
        && state.units.iter().any(|u| u.id == sid)
    {
        state.units.retain(|u| u.id != sid);
        diff.push(StateDelta::UnitDestroyed { unit: sid });
    }

    // Grant gold reward for clearing the camp.
    let reward = state.barbarian_config.camp_clear_gold_reward;
    if reward > 0 {
        if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == cleared_by) {
            civ.gold += reward as i32;
        }
        diff.push(StateDelta::GoldChanged { civ: cleared_by, delta: reward as i32 });
    }

    diff.push(StateDelta::BarbarianCampDestroyed { camp: camp_id, coord, cleared_by });

    // Remove the camp.
    state.barbarian_camps.retain(|c| c.id != camp_id);

    diff
}

// ── Movement helpers ─────────────────────────────────────────────────────────

/// Move a barbarian unit to `to`, updating its coord directly.
fn move_barbarian_unit(state: &mut GameState, unit_id: UnitId, to: HexCoord, diff: &mut GameStateDiff) {
    // Check tile is valid and passable.
    let tile = match state.board.tile(to) {
        Some(t) => t,
        None => return,
    };
    if matches!(tile.terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast | BuiltinTerrain::Mountain) {
        return;
    }
    // Don't move into tiles occupied by other units.
    if state.units.iter().any(|u| u.coord == to && u.id != unit_id) {
        return;
    }

    let from = match state.units.iter().find(|u| u.id == unit_id) {
        Some(u) => u.coord,
        None => return,
    };

    if let Some(u) = state.units.iter_mut().find(|u| u.id == unit_id) {
        u.coord = to;
    }
    diff.push(StateDelta::UnitMoved { unit: unit_id, from, to, cost: 100 });
}

/// Step one hex toward `target` from `from`. Returns the best neighbor.
fn step_toward(from: HexCoord, target: HexCoord, state: &GameState) -> Option<HexCoord> {
    let mut best: Option<(HexCoord, u32)> = None;
    for n in from.neighbors() {
        if let Some(tile) = state.board.tile(n) {
            if matches!(tile.terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast | BuiltinTerrain::Mountain) {
                continue;
            }
            if state.units.iter().any(|u| u.coord == n) {
                continue;
            }
            let dist = n.distance(&target);
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((n, dist));
            }
        }
    }
    best.map(|(c, _)| c)
}

/// Pick an exploration step: move away from camp, biased by RNG.
fn explore_step(from: HexCoord, camp_coord: HexCoord, state: &mut GameState) -> Option<HexCoord> {
    let neighbors: Vec<HexCoord> = from.neighbors().into_iter()
        .filter(|n| {
            if let Some(tile) = state.board.tile(*n) {
                !matches!(tile.terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast | BuiltinTerrain::Mountain)
                && !state.units.iter().any(|u| u.coord == *n)
            } else {
                false
            }
        })
        .collect();

    if neighbors.is_empty() {
        return None;
    }

    // Prefer moving away from camp (exploration), but with some randomness.
    let roll = state.id_gen.next_f32();

    // Sort by distance from camp (descending) and pick from the top half with bias.
    let mut sorted = neighbors;
    sorted.sort_by_key(|c| std::cmp::Reverse(c.distance(&camp_coord)));

    // Pick from the first half (further from camp) most of the time.
    let pick_range = (sorted.len() / 2).max(1);
    let pick_idx = (roll * pick_range as f32) as usize;
    let pick_idx = pick_idx.min(pick_range - 1);
    Some(sorted[pick_idx])
}
