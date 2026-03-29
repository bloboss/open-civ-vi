use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::coord::HexCoord;
use super::enums::*;
use super::ids::*;

// ── Top-level game view ──────────────────────────────────────────────────────

/// Serializable projection of GameState visible to one player.
/// Respects fog-of-war: only explored tiles included, only visible units shown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameView {
    pub turn: u32,
    pub my_civ_id: CivId,
    pub board: BoardView,
    pub my_civ: CivView,
    pub other_civs: Vec<PublicCivView>,
    pub cities: Vec<CityView>,
    pub units: Vec<UnitView>,
    pub tech_tree: TechTreeView,
    pub civic_tree: CivicTreeView,
    pub trade_routes: Vec<TradeRouteView>,
    pub unit_type_defs: Vec<UnitTypeDefView>,
    pub building_defs: Vec<BuildingDefView>,
    pub scores: Vec<(CivId, u32)>,
    pub game_over: Option<GameOverView>,
}

// ── Board ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardView {
    pub width: u32,
    pub height: u32,
    pub topology: BoardTopology,
    /// Only explored tiles are included.
    pub tiles: Vec<TileView>,
    /// River edge endpoints (both coords of each river edge).
    pub river_edges: Vec<(HexCoord, HexCoord)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileView {
    pub coord: HexCoord,
    pub terrain: BuiltinTerrain,
    pub hills: bool,
    pub feature: Option<BuiltinFeature>,
    /// Only present if the player's civ has the reveal tech for this resource.
    pub resource: Option<BuiltinResource>,
    pub improvement: Option<BuiltinImprovement>,
    pub road: Option<BuiltinRoad>,
    pub owner: Option<CivId>,
    pub visibility: TileVisibility,
}

// ── Civilizations ────────────────────────────────────────────────────────────

/// Full detail for the player's own civilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivView {
    pub id: CivId,
    pub name: String,
    pub adjective: String,
    pub leader_name: String,
    pub gold: i32,
    pub current_era: AgeType,
    pub researched_techs: Vec<TechId>,
    pub research_queue: Vec<TechProgressView>,
    pub completed_civics: Vec<CivicId>,
    pub civic_in_progress: Option<CivicProgressView>,
    pub current_government: Option<String>,
    pub active_policies: Vec<PolicyId>,
    pub unlocked_units: Vec<String>,
    pub unlocked_buildings: Vec<String>,
    pub unlocked_improvements: Vec<String>,
    pub strategic_resources: HashMap<String, u32>,
    pub yields: YieldBundleView,
}

/// Limited information about other civilizations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicCivView {
    pub id: CivId,
    pub name: String,
    pub leader_name: String,
    pub score: u32,
    pub diplomatic_status: DiplomaticStatus,
}

// ── Cities ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityView {
    pub id: CityId,
    pub name: String,
    pub owner: CivId,
    pub coord: HexCoord,
    pub is_capital: bool,
    pub population: u32,
    pub food_stored: u32,
    pub food_to_grow: u32,
    pub production_stored: u32,
    pub production_queue: Vec<ProductionItemView>,
    pub buildings: Vec<BuildingId>,
    pub worked_tiles: Vec<HexCoord>,
    pub territory: Vec<HexCoord>,
    pub ownership: CityOwnership,
    pub walls: WallLevel,
    /// True if this is the player's own city (full detail); false = foreign.
    pub is_own: bool,
}

// ── Units ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitView {
    pub id: UnitId,
    pub unit_type: UnitTypeId,
    pub owner: CivId,
    pub coord: HexCoord,
    pub domain: UnitDomain,
    pub category: UnitCategory,
    pub movement_left: u32,
    pub max_movement: u32,
    pub combat_strength: Option<u32>,
    pub health: u32,
    pub range: u8,
    pub vision_range: u8,
    /// True if owned by the viewing player.
    pub is_own: bool,
}

// ── Tech / Civic trees ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechTreeView {
    pub nodes: Vec<TechNodeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechNodeView {
    pub id: TechId,
    pub name: String,
    pub cost: u32,
    pub prerequisites: Vec<TechId>,
    pub eureka_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivicTreeView {
    pub nodes: Vec<CivicNodeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivicNodeView {
    pub id: CivicId,
    pub name: String,
    pub cost: u32,
    pub prerequisites: Vec<CivicId>,
    pub inspiration_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechProgressView {
    pub tech_id: TechId,
    pub progress: u32,
    pub boosted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivicProgressView {
    pub civic_id: CivicId,
    pub progress: u32,
    pub inspired: bool,
}

// ── Trade routes ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRouteView {
    pub id: TradeRouteId,
    pub origin: CityId,
    pub destination: CityId,
    pub owner: CivId,
    pub origin_yields: YieldBundleView,
    pub destination_yields: YieldBundleView,
    pub turns_remaining: Option<u32>,
}

// ── Registries ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTypeDefView {
    pub id: UnitTypeId,
    pub name: String,
    pub production_cost: u32,
    pub domain: UnitDomain,
    pub category: UnitCategory,
    pub max_movement: u32,
    pub combat_strength: Option<u32>,
    pub range: u8,
    pub vision_range: u8,
    pub can_found_city: bool,
    pub resource_cost: Option<(BuiltinResource, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDefView {
    pub id: BuildingId,
    pub name: String,
    pub cost: u32,
    pub maintenance: u32,
    pub yields: YieldBundleView,
}

// ── Yields ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct YieldBundleView {
    pub food: i32,
    pub production: i32,
    pub gold: i32,
    pub science: i32,
    pub culture: i32,
    pub faith: i32,
    pub housing: i32,
    pub amenities: i32,
    pub tourism: i32,
    pub great_person_points: i32,
}

// ── Victory ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameOverView {
    pub winner: CivId,
    pub condition: String,
    pub turn: u32,
}
