//! Handler for the `status` CLI command.
//!
//! Read-only queries against the game state, output as JSON.

use std::path::Path;

use libciv::{all_scores, CityId, DefaultRulesEngine, RulesEngine};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use serde_json::json;

use crate::cli::StatusKind;
use crate::state_io;

use super::{find_civ_by_name, parse_ulid};

/// Execute a read-only status query and print the result as JSON.
pub fn handle_status(
    game_file: &Path,
    player: &str,
    kind: &StatusKind,
) -> Result<(), String> {
    let state = state_io::load_game_file(game_file)?;
    let civ_id = find_civ_by_name(&state, player)?;

    let output = match kind {
        StatusKind::Scores => {
            let scores = all_scores(&state);
            let entries: Vec<_> = scores
                .iter()
                .map(|(cid, score)| {
                    let name = state
                        .civilizations
                        .iter()
                        .find(|c| c.id == *cid)
                        .map(|c| c.name)
                        .unwrap_or("?");
                    json!({ "civ": name, "score": score })
                })
                .collect();
            json!(entries)
        }
        StatusKind::City { id } => {
            let city_id = parse_ulid(id).map(CityId::from_ulid)?;
            let city = state
                .cities
                .iter()
                .find(|c| c.id == city_id)
                .ok_or_else(|| format!("city not found: {id}"))?;
            let buildings: Vec<&str> = city.buildings.iter().map(|b| {
                state
                    .building_defs
                    .iter()
                    .find(|d| d.id == *b)
                    .map(|d| d.name)
                    .unwrap_or("?")
            }).collect();
            json!({
                "id": format!("{:?}", city.id),
                "name": city.name,
                "coord": format!("({}, {})", city.coord.q, city.coord.r),
                "population": city.population,
                "food_stored": city.food_stored,
                "food_to_grow": city.food_to_grow,
                "production_stored": city.production_stored,
                "buildings": buildings,
                "worked_tiles": city.worked_tiles.iter()
                    .map(|t| format!("({}, {})", t.q, t.r))
                    .collect::<Vec<_>>(),
                "production_queue": city.production_queue.iter()
                    .map(|p| format!("{p:?}"))
                    .collect::<Vec<_>>(),
            })
        }
        StatusKind::Unit { id } => {
            let unit_id = parse_ulid(id).map(libciv::UnitId::from_ulid)?;
            let unit = state
                .units
                .iter()
                .find(|u| u.id == unit_id)
                .ok_or_else(|| format!("unit not found: {id}"))?;
            let type_name = state
                .unit_type_defs
                .iter()
                .find(|d| d.id == unit.unit_type)
                .map(|d| d.name)
                .unwrap_or("?");
            json!({
                "id": format!("{:?}", unit.id),
                "type": type_name,
                "coord": format!("({}, {})", unit.coord.q, unit.coord.r),
                "health": unit.health,
                "movement_left": unit.movement_left,
                "max_movement": unit.max_movement,
                "combat_strength": unit.combat_strength,
                "promotions": unit.promotions.iter()
                    .map(|p| format!("{p:?}"))
                    .collect::<Vec<_>>(),
                "experience": unit.experience,
            })
        }
        StatusKind::Tile { q, r } => {
            let coord = HexCoord::from_qr(*q, *r);
            let tile = state
                .board
                .tile(coord)
                .ok_or_else(|| format!("tile not found at ({q}, {r})"))?;
            let owner_name = tile.owner.and_then(|oid| {
                state
                    .civilizations
                    .iter()
                    .find(|c| c.id == oid)
                    .map(|c| c.name)
            });
            json!({
                "coord": format!("({q}, {r})"),
                "terrain": tile.terrain.name(),
                "hills": tile.hills,
                "feature": tile.feature.map(|f| f.name()),
                "resource": tile.resource.map(|r| format!("{r:?}")),
                "improvement": tile.improvement.map(|i| i.name()),
                "road": tile.road.as_ref().map(|r| format!("{r:?}")),
                "owner": owner_name,
            })
        }
        StatusKind::Diplomacy => {
            let relations: Vec<_> = state
                .diplomatic_relations
                .iter()
                .filter(|r| r.civ_a == civ_id || r.civ_b == civ_id)
                .map(|r| {
                    let other_id = if r.civ_a == civ_id { r.civ_b } else { r.civ_a };
                    let other_name = state
                        .civilizations
                        .iter()
                        .find(|c| c.id == other_id)
                        .map(|c| c.name)
                        .unwrap_or("?");
                    json!({
                        "civ": other_name,
                        "status": format!("{:?}", r.status),
                        "turns_at_war": r.turns_at_war,
                    })
                })
                .collect();
            json!(relations)
        }
        StatusKind::Congress => {
            let wc = &state.world_congress;
            let resolutions: Vec<_> = wc
                .active_resolutions
                .iter()
                .map(|r| format!("{r:?}"))
                .collect();
            let dvp: Vec<_> = wc
                .diplomatic_victory_points
                .iter()
                .map(|(cid, pts)| {
                    let name = state
                        .civilizations
                        .iter()
                        .find(|c| c.id == *cid)
                        .map(|c| c.name)
                        .unwrap_or("?");
                    json!({ "civ": name, "points": pts })
                })
                .collect();
            json!({
                "session_interval": wc.session_interval,
                "next_session_turn": wc.next_session_turn,
                "active_resolutions": resolutions,
                "diplomatic_victory_points": dvp,
            })
        }
        StatusKind::Yields => {
            let rules = DefaultRulesEngine;
            let yields = rules.compute_yields(&state, civ_id);
            json!({
                "food": yields.food,
                "production": yields.production,
                "gold": yields.gold,
                "science": yields.science,
                "culture": yields.culture,
                "faith": yields.faith,
            })
        }
        StatusKind::Techs => {
            let civ = state
                .civilizations
                .iter()
                .find(|c| c.id == civ_id)
                .ok_or("civ not found")?;
            let researched: Vec<&str> = civ
                .researched_techs
                .iter()
                .filter_map(|tid| {
                    state.tech_tree.get(*tid).map(|n| n.name)
                })
                .collect();
            let in_progress: Vec<_> = civ
                .research_queue
                .iter()
                .filter_map(|tp| {
                    state.tech_tree.get(tp.tech_id).map(|n| {
                        json!({
                            "tech": n.name,
                            "progress": tp.progress,
                            "cost": n.cost,
                        })
                    })
                })
                .collect();
            json!({
                "researched": researched,
                "in_progress": in_progress,
            })
        }
        StatusKind::Civics => {
            let civ = state
                .civilizations
                .iter()
                .find(|c| c.id == civ_id)
                .ok_or("civ not found")?;
            let completed: Vec<&str> = civ
                .completed_civics
                .iter()
                .filter_map(|cid| {
                    state.civic_tree.get(*cid).map(|n| n.name)
                })
                .collect();
            let in_progress = civ.civic_in_progress.as_ref().and_then(|cp| {
                state.civic_tree.get(cp.civic_id).map(|n| {
                    json!({
                        "civic": n.name,
                        "progress": cp.progress,
                        "cost": n.cost,
                    })
                })
            });
            json!({
                "completed": completed,
                "in_progress": in_progress,
            })
        }
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("failed to serialize status: {e}"))?;
    println!("{json}");

    Ok(())
}
