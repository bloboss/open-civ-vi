//! Fog-of-war filtered game state view for a specific player.

use libciv::game::state::GameState;
use libciv::{CivId, RulesEngine};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use serde::Serialize;

/// A fog-of-war filtered view of the game state for one player.
#[derive(Serialize)]
pub struct PlayerView {
    pub turn: u32,
    pub civ_name: String,

    // Own-civ economy
    pub gold: i32,
    pub faith: u32,
    pub science_per_turn: i32,
    pub culture_per_turn: i32,
    pub diplomatic_favor: u32,

    // Own cities
    pub cities: Vec<CityView>,

    // Own units
    pub units: Vec<UnitView>,

    // Visible tiles (fog-of-war filtered)
    pub visible_tiles: Vec<TileView>,

    // Explored-but-not-visible tiles (fog memory)
    pub explored_tiles: Vec<HexCoord>,

    // Diplomacy (always visible)
    pub diplomacy: Vec<DiplomacyView>,

    // Victory
    pub game_over: bool,
    pub score: u32,
}

#[derive(Serialize)]
pub struct CityView {
    pub id: String,
    pub name: String,
    pub coord: HexCoord,
    pub population: u32,
    pub production_stored: u32,
}

#[derive(Serialize)]
pub struct UnitView {
    pub id: String,
    pub type_name: String,
    pub coord: HexCoord,
    pub health: u32,
    pub movement_left: u32,
}

#[derive(Serialize)]
pub struct TileView {
    pub coord: HexCoord,
    pub terrain: String,
    pub feature: Option<String>,
    pub resource: Option<String>,
    pub improvement: Option<String>,
    pub owner: Option<String>,
    pub has_city: bool,
    pub units: Vec<String>,
}

#[derive(Serialize)]
pub struct DiplomacyView {
    pub civ_name: String,
    pub status: String,
}

/// Build a `PlayerView` for the given civ, respecting fog of war.
pub fn build_player_view(state: &GameState, civ_id: CivId) -> PlayerView {
    let civ = state.civilizations.iter().find(|c| c.id == civ_id)
        .expect("civ not found");
    let rules = libciv::DefaultRulesEngine;
    let yields = rules.compute_yields(state, civ_id);
    let score = libciv::compute_score(state, civ_id);

    // Own cities
    let cities: Vec<CityView> = state.cities.iter()
        .filter(|c| c.owner == civ_id)
        .map(|c| CityView {
            id: format!("{:?}", c.id),
            name: c.name.clone(),
            coord: c.coord,
            population: c.population,
            production_stored: c.production_stored,
        })
        .collect();

    // Own units
    let units: Vec<UnitView> = state.units.iter()
        .filter(|u| u.owner == civ_id)
        .map(|u| {
            let type_name = state.unit_type_defs.iter()
                .find(|d| d.id == u.unit_type)
                .map(|d| d.name.to_string())
                .unwrap_or_else(|| "?".to_string());
            UnitView {
                id: format!("{:?}", u.id),
                type_name,
                coord: u.coord,
                health: u.health,
                movement_left: u.movement_left,
            }
        })
        .collect();

    // Visible tiles
    let visible_tiles: Vec<TileView> = civ.visible_tiles.iter()
        .filter_map(|&coord| {
            let tile = state.board.tile(coord)?;
            let owner_name = tile.owner.and_then(|oid| {
                state.civilizations.iter().find(|c| c.id == oid).map(|c| c.name.to_string())
            });
            let has_city = state.cities.iter().any(|c| c.coord == coord);
            let tile_units: Vec<String> = state.units.iter()
                .filter(|u| u.coord == coord)
                .map(|u| {
                    let owner = state.civilizations.iter()
                        .find(|c| c.id == u.owner)
                        .map(|c| c.name.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    format!("{}({})", state.unit_type_defs.iter()
                        .find(|d| d.id == u.unit_type)
                        .map(|d| d.name)
                        .unwrap_or("?"), owner)
                })
                .collect();
            Some(TileView {
                coord,
                terrain: tile.terrain.name().to_string(),
                feature: tile.feature.map(|f| f.name().to_string()),
                resource: tile.resource.map(|r| format!("{r:?}")),
                improvement: tile.improvement.map(|i| i.name().to_string()),
                owner: owner_name,
                has_city,
                units: tile_units,
            })
        })
        .collect();

    // Explored but not visible
    let explored_tiles: Vec<HexCoord> = civ.explored_tiles.iter()
        .filter(|c| !civ.visible_tiles.contains(c))
        .copied()
        .collect();

    // Diplomacy
    let diplomacy: Vec<DiplomacyView> = state.diplomatic_relations.iter()
        .filter(|r| r.civ_a == civ_id || r.civ_b == civ_id)
        .map(|r| {
            let other_id = if r.civ_a == civ_id { r.civ_b } else { r.civ_a };
            let other_name = state.civilizations.iter()
                .find(|c| c.id == other_id)
                .map(|c| c.name.to_string())
                .unwrap_or_else(|| "?".to_string());
            DiplomacyView {
                civ_name: other_name,
                status: format!("{:?}", r.status),
            }
        })
        .collect();

    PlayerView {
        turn: state.turn,
        civ_name: civ.name.to_string(),
        gold: civ.gold,
        faith: civ.faith,
        science_per_turn: yields.science,
        culture_per_turn: yields.culture,
        diplomatic_favor: civ.diplomatic_favor,
        cities,
        units,
        visible_tiles,
        explored_tiles,
        diplomacy,
        game_over: state.game_over.is_some(),
        score,
    }
}
