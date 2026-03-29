//! Build report types from GameState for a specific player.

use std::collections::HashMap;

use libciv::CivId;

use crate::types::enums::*;
use crate::types::ids as api;
use crate::types::reports::*;
use crate::types::view::*;

fn conv_civ_id(id: CivId) -> api::CivId {
    api::CivId::from_ulid(id.as_ulid())
}

/// Build city report rows from a projected game view.
pub fn build_city_report(view: &GameView) -> Vec<CityReportRow> {
    view.cities
        .iter()
        .filter(|c| c.is_own)
        .map(|c| {
            let current_production = c.production_queue.first().map(|item| match item {
                ProductionItemView::Unit(tid) => view
                    .unit_type_defs
                    .iter()
                    .find(|d| d.id == *tid)
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| "Unit".into()),
                ProductionItemView::Building(bid) => view
                    .building_defs
                    .iter()
                    .find(|d| d.id == *bid)
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| "Building".into()),
                ProductionItemView::District(d) => format!("{d:?}"),
                ProductionItemView::Wonder(_) => "Wonder".into(),
            });
            CityReportRow {
                id: c.id,
                name: c.name.clone(),
                population: c.population,
                food_per_turn: 0,       // TODO: compute from yields
                production_per_turn: 0, // TODO: compute from yields
                gold_per_turn: 0,       // TODO: compute from yields
                current_production,
                districts_count: 0, // TODO: count from city data
                buildings_count: c.buildings.len() as u32,
            }
        })
        .collect()
}

/// Build resource report from a projected game view.
pub fn build_resource_report(view: &GameView) -> ResourceReport {
    let mut resource_map: HashMap<String, (ResourceCategory, u32, u32)> = HashMap::new();

    for tile in &view.board.tiles {
        if let Some(res) = &tile.resource {
            let name = format!("{res:?}");
            let cat = resource_category(*res);
            let entry = resource_map.entry(name).or_insert((cat, 0, 0));
            entry.1 += 1;
            if tile.improvement.is_some() {
                entry.2 += 1;
            }
        }
    }

    let resources = resource_map
        .into_iter()
        .map(|(name, (category, total_count, improved_count))| ResourceEntry {
            name,
            category,
            total_count,
            improved_count,
        })
        .collect();

    ResourceReport { resources }
}

fn resource_category(r: BuiltinResource) -> ResourceCategory {
    match r {
        BuiltinResource::Wheat
        | BuiltinResource::Rice
        | BuiltinResource::Cattle
        | BuiltinResource::Sheep
        | BuiltinResource::Fish
        | BuiltinResource::Stone
        | BuiltinResource::Copper
        | BuiltinResource::Deer => ResourceCategory::Bonus,

        BuiltinResource::Wine
        | BuiltinResource::Silk
        | BuiltinResource::Spices
        | BuiltinResource::Incense
        | BuiltinResource::Cotton
        | BuiltinResource::Ivory
        | BuiltinResource::Sugar
        | BuiltinResource::Salt => ResourceCategory::Luxury,

        BuiltinResource::Horses
        | BuiltinResource::Iron
        | BuiltinResource::Coal
        | BuiltinResource::Oil
        | BuiltinResource::Aluminum
        | BuiltinResource::Niter
        | BuiltinResource::Uranium => ResourceCategory::Strategic,
    }
}

/// Build unit report from a projected game view.
pub fn build_unit_report(view: &GameView) -> Vec<UnitReport> {
    view.units
        .iter()
        .filter(|u| u.is_own)
        .map(|u| {
            let type_name = view
                .unit_type_defs
                .iter()
                .find(|d| d.id == u.unit_type)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| "Unknown".into());
            UnitReport {
                id: u.id,
                type_name,
                location: u.coord,
                health: u.health,
                movement_left: u.movement_left,
                max_movement: u.max_movement,
                combat_strength: u.combat_strength,
                category: u.category,
            }
        })
        .collect()
}

/// Build map statistics from a projected game view.
pub fn build_map_stats(view: &GameView) -> MapStatistics {
    let mut terrain_counts: HashMap<String, u32> = HashMap::new();
    let mut feature_counts: HashMap<String, u32> = HashMap::new();
    let mut resource_counts: HashMap<String, u32> = HashMap::new();
    let mut tiles_visible = 0u32;

    for tile in &view.board.tiles {
        *terrain_counts.entry(format!("{:?}", tile.terrain)).or_default() += 1;
        if let Some(f) = &tile.feature {
            *feature_counts.entry(format!("{f:?}")).or_default() += 1;
        }
        if let Some(r) = &tile.resource {
            *resource_counts.entry(format!("{r:?}")).or_default() += 1;
        }
        if tile.visibility == TileVisibility::Visible {
            tiles_visible += 1;
        }
    }

    let enemy_cities_visible = view.cities.iter().filter(|c| !c.is_own).count() as u32;
    let enemy_units_visible = view.units.iter().filter(|u| !u.is_own).count() as u32;
    let tiles_explored = view.board.tiles.len() as u32;

    MapStatistics {
        terrain_counts,
        feature_counts,
        resource_counts,
        enemy_cities_visible,
        enemy_units_visible,
        tiles_explored,
        tiles_visible,
    }
}

/// Build player reports from a projected game view.
pub fn build_player_reports(view: &GameView) -> Vec<PlayerReport> {
    view.other_civs
        .iter()
        .map(|c| {
            let known_cities = view.cities.iter().filter(|city| city.owner == c.id).count() as u32;
            let known_units = view.units.iter().filter(|u| u.owner == c.id).count() as u32;
            PlayerReport {
                id: c.id,
                name: c.name.clone(),
                leader_name: c.leader_name.clone(),
                score: c.score,
                diplomatic_status: c.diplomatic_status,
                known_cities,
                known_units,
            }
        })
        .collect()
}

/// Build science report from a projected game view.
pub fn build_science_report(view: &GameView) -> ScienceReport {
    ScienceReport {
        tech_tree: view.tech_tree.clone(),
        researched_techs: view.my_civ.researched_techs.clone(),
        research_queue: view.my_civ.research_queue.clone(),
        science_per_turn: view.my_civ.yields.science,
    }
}

/// Build culture report from a projected game view.
pub fn build_culture_report(view: &GameView) -> CultureReport {
    CultureReport {
        civic_tree: view.civic_tree.clone(),
        completed_civics: view.my_civ.completed_civics.clone(),
        civic_in_progress: view.my_civ.civic_in_progress.clone(),
        culture_per_turn: view.my_civ.yields.culture,
    }
}

/// Build turn status from a game room.
pub fn build_turn_status(
    game_id: api::GameId,
    turn: u32,
    status: crate::types::messages::GameStatus,
    players: &[crate::server::state::PlayerSlot],
) -> TurnStatus {
    let players_submitted = players
        .iter()
        .map(|s| (conv_civ_id(s.civ_id), s.submitted_turn))
        .collect();
    TurnStatus {
        game_id,
        current_turn: turn,
        game_status: status,
        players_submitted,
    }
}
