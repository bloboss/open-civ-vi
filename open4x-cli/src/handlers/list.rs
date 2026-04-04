//! Handler for the `list` CLI command.
//!
//! Enumerates available items for a player, output as JSON arrays.

use std::path::Path;

use libciv::game::{available_building_defs, available_unit_defs};
use libciv::CityId;
use serde_json::json;

use crate::cli::ListKind;
use crate::state_io;

use super::{find_civ_by_name, parse_ulid};

/// Execute a list query and print the result as JSON.
pub fn handle_list(
    game_file: &Path,
    player: &str,
    kind: &ListKind,
) -> Result<(), String> {
    let state = state_io::load_game_file(game_file)?;
    let civ_id = find_civ_by_name(&state, player)?;

    let output = match kind {
        ListKind::Units => {
            let units: Vec<_> = state
                .units
                .iter()
                .filter(|u| u.owner == civ_id)
                .map(|u| {
                    let type_name = state
                        .unit_type_defs
                        .iter()
                        .find(|d| d.id == u.unit_type)
                        .map(|d| d.name)
                        .unwrap_or("?");
                    json!({
                        "id": format!("{}", u.id.as_ulid()),
                        "type": type_name,
                        "coord": format!("({}, {})", u.coord.q, u.coord.r),
                        "health": u.health,
                        "movement_left": u.movement_left,
                    })
                })
                .collect();
            json!(units)
        }
        ListKind::Cities => {
            let cities: Vec<_> = state
                .cities
                .iter()
                .filter(|c| c.owner == civ_id)
                .map(|c| {
                    json!({
                        "id": format!("{}", c.id.as_ulid()),
                        "name": c.name,
                        "coord": format!("({}, {})", c.coord.q, c.coord.r),
                        "population": c.population,
                        "is_capital": c.is_capital,
                    })
                })
                .collect();
            json!(cities)
        }
        ListKind::Production { city } => {
            let city_id = parse_ulid(city).map(CityId::from_ulid)?;
            let _city_obj = state
                .cities
                .iter()
                .find(|c| c.id == city_id)
                .ok_or_else(|| format!("city not found: {city}"))?;

            let unit_defs: Vec<_> = available_unit_defs(&state, civ_id)
                .iter()
                .map(|d| {
                    json!({
                        "type": "unit",
                        "name": d.name,
                        "cost": d.production_cost,
                    })
                })
                .collect();
            let building_defs: Vec<_> = available_building_defs(&state, civ_id)
                .iter()
                .map(|d| {
                    json!({
                        "type": "building",
                        "name": d.name,
                        "cost": d.cost,
                    })
                })
                .collect();
            let mut all = unit_defs;
            all.extend(building_defs);
            json!(all)
        }
        ListKind::GreatPeople => {
            let people: Vec<_> = state
                .great_people
                .iter()
                .map(|gp| {
                    json!({
                        "id": format!("{}", gp.id.as_ulid()),
                        "name": gp.name,
                        "person_type": format!("{:?}", gp.person_type),
                        "owner": gp.owner.map(|cid| {
                            state.civilizations.iter()
                                .find(|c| c.id == cid)
                                .map(|c| c.name.to_string())
                                .unwrap_or_else(|| "?".to_string())
                        }),
                        "retired": gp.is_retired,
                    })
                })
                .collect();
            json!(people)
        }
        ListKind::Routes => {
            let routes: Vec<_> = state
                .trade_routes
                .iter()
                .map(|tr| {
                    let origin_name = state
                        .cities
                        .iter()
                        .find(|c| c.id == tr.origin)
                        .map(|c| c.name.as_str())
                        .unwrap_or("?");
                    let dest_name = state
                        .cities
                        .iter()
                        .find(|c| c.id == tr.destination)
                        .map(|c| c.name.as_str())
                        .unwrap_or("?");
                    json!({
                        "id": format!("{}", tr.id.as_ulid()),
                        "origin": origin_name,
                        "destination": dest_name,
                        "turns_remaining": tr.turns_remaining,
                    })
                })
                .collect();
            json!(routes)
        }
        ListKind::Governors => {
            let govs: Vec<_> = state
                .governors
                .iter()
                .filter(|g| g.owner == civ_id)
                .map(|g| {
                    let city_name = g.assigned_city.and_then(|cid| {
                        state.cities.iter().find(|c| c.id == cid).map(|c| c.name.as_str())
                    });
                    json!({
                        "id": format!("{}", g.id.as_ulid()),
                        "name": g.def_name,
                        "assigned_city": city_name,
                        "turns_to_establish": g.turns_to_establish,
                        "promotions": g.promotions,
                    })
                })
                .collect();
            json!(govs)
        }
        ListKind::Buildings => {
            let buildings: Vec<_> = state
                .building_defs
                .iter()
                .map(|d| {
                    json!({
                        "name": d.name,
                        "cost": d.cost,
                        "maintenance": d.maintenance,
                        "requires_district": d.requires_district,
                    })
                })
                .collect();
            json!(buildings)
        }
        ListKind::Improvements => {
            // List built-in improvement types with their names.
            use libciv::world::improvement::BuiltinImprovement;
            let improvements = [
                BuiltinImprovement::Farm,
                BuiltinImprovement::Mine,
                BuiltinImprovement::LumberMill,
                BuiltinImprovement::TradingPost,
                BuiltinImprovement::Fort,
                BuiltinImprovement::Airstrip,
                BuiltinImprovement::MissileSilo,
                BuiltinImprovement::Quarry,
                BuiltinImprovement::Plantation,
                BuiltinImprovement::Camp,
                BuiltinImprovement::FishingBoats,
                BuiltinImprovement::Pasture,
                BuiltinImprovement::OilWell,
                BuiltinImprovement::OffshoreOilRig,
                BuiltinImprovement::BeachResort,
            ];
            let list: Vec<_> = improvements
                .iter()
                .map(|imp| json!({ "name": imp.name() }))
                .collect();
            json!(list)
        }
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("failed to serialize list: {e}"))?;
    println!("{json}");

    Ok(())
}
