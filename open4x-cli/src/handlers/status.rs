//! Handler for the `status` CLI command.
//!
//! Read-only queries against the game state, output as JSON.

use std::path::Path;

use libciv::{
    CityId, DefaultRulesEngine, PendingActionKind, PolicyCardStatus, RulesEngine, UnitActionKind,
    all_scores,
};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use serde_json::json;

use crate::cli::StatusKind;
use crate::state_io;

use super::{find_civ_by_name, parse_ulid};

/// Execute a read-only status query and print the result as JSON.
pub fn handle_status(game_file: &Path, player: &str, kind: &StatusKind) -> Result<(), String> {
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
            let buildings: Vec<&str> = city
                .buildings
                .iter()
                .map(|b| {
                    state
                        .building_defs
                        .iter()
                        .find(|d| d.id == *b)
                        .map(|d| d.name)
                        .unwrap_or("?")
                })
                .collect();
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
                .filter_map(|tid| state.tech_tree.get(*tid).map(|n| n.name))
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
        StatusKind::Policies => {
            let rules = DefaultRulesEngine;
            let cat = rules.policy_catalogue(&state, civ_id);
            let entries: Vec<_> = cat
                .into_iter()
                .map(|e| {
                    let status = match e.status {
                        PolicyCardStatus::Active => "active",
                        PolicyCardStatus::Available => "available",
                        PolicyCardStatus::Locked => "locked",
                    };
                    json!({
                        "id":           e.policy_id.as_ulid().to_string(),
                        "name":         e.name,
                        "type":         format!("{:?}", e.policy_type).to_lowercase(),
                        "prereq_civic": e.prereq_civic,
                        "status":       status,
                    })
                })
                .collect();
            json!(entries)
        }
        StatusKind::CombatPreview { attacker, q, r } => {
            let attacker_id = parse_ulid(attacker).map(libciv::UnitId::from_ulid)?;
            let coord = HexCoord::from_qr(*q, *r);
            let rules = DefaultRulesEngine;
            match rules.preview_combat(&state, attacker_id, coord) {
                Some(p) => json!({
                    "attacker":                  p.attacker.as_ulid().to_string(),
                    "defender":                  p.defender.as_ulid().to_string(),
                    "attack_type":               format!("{:?}", p.attack_type),
                    "attacker_effective_cs":     p.attacker_effective_cs,
                    "defender_effective_cs":     p.defender_effective_cs,
                    "predicted_attacker_damage": p.predicted_attacker_damage,
                    "predicted_defender_damage": p.predicted_defender_damage,
                }),
                None => json!(null),
            }
        }
        StatusKind::UnitActions { id } => {
            let unit_id = parse_ulid(id).map(libciv::UnitId::from_ulid)?;
            let rules = DefaultRulesEngine;
            let actions = rules.available_unit_actions(&state, unit_id);
            let entries: Vec<_> = actions
                .into_iter()
                .map(|a| {
                    let kind = match a.kind {
                        UnitActionKind::Move => "move",
                        UnitActionKind::Attack => "attack",
                        UnitActionKind::Fortify => "fortify",
                        UnitActionKind::Sleep => "sleep",
                        UnitActionKind::FoundCity => "found_city",
                        UnitActionKind::Build => "build",
                        UnitActionKind::TradeRoute => "trade_route",
                        UnitActionKind::SpreadReligion => "spread_religion",
                    };
                    json!({ "kind": kind, "enabled": a.enabled })
                })
                .collect();
            json!(entries)
        }
        StatusKind::Victory => {
            let rules = DefaultRulesEngine;
            let progress = rules.victory_progress(&state, civ_id);
            let entries: Vec<_> = progress
                .iter()
                .zip(state.victory_conditions.iter())
                .map(|(p, cond)| {
                    json!({
                        "condition": cond.name(),
                        "current":   p.current,
                        "target":    p.target,
                        "pct":       p.percentage(),
                        "won":       p.is_won(),
                    })
                })
                .collect();
            json!(entries)
        }
        StatusKind::Pending => {
            let rules = DefaultRulesEngine;
            let pending = rules.pending_actions(&state, civ_id);
            let entries: Vec<_> = pending
                .into_iter()
                .map(|a| {
                    let (kind, detail) = match a.kind {
                        PendingActionKind::ChooseResearch => ("choose_research", json!(null)),
                        PendingActionKind::ChooseCivic => ("choose_civic", json!(null)),
                        PendingActionKind::UnitNeedsOrders { unit_id, coord } => (
                            "unit_needs_orders",
                            json!({
                                "unit_id": unit_id.as_ulid().to_string(),
                                "coord":   format!("({}, {})", coord.q, coord.r),
                            }),
                        ),
                        PendingActionKind::CityNeedsProduction { city_id } => (
                            "city_needs_production",
                            json!({ "city_id": city_id.as_ulid().to_string() }),
                        ),
                    };
                    json!({
                        "kind":     kind,
                        "required": a.required,
                        "detail":   detail,
                    })
                })
                .collect();
            json!(entries)
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
                .filter_map(|cid| state.civic_tree.get(*cid).map(|n| n.name))
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
