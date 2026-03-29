use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::coord::HexCoord;
use super::enums::*;
use super::ids::*;
use super::messages::GameStatus;
use super::view::{CityView, YieldBundleView};

// ── City reports ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityReportRow {
    pub id: CityId,
    pub name: String,
    pub population: u32,
    pub food_per_turn: i32,
    pub production_per_turn: i32,
    pub gold_per_turn: i32,
    pub current_production: Option<String>,
    pub districts_count: u32,
    pub buildings_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityDetailReport {
    pub city: CityView,
    pub tile_yields: Vec<(HexCoord, YieldBundleView)>,
    pub available_production: Vec<ProductionOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOption {
    pub item: ProductionItemView,
    pub name: String,
    pub cost: u32,
}

// ── Resource reports ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReport {
    pub resources: Vec<ResourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub name: String,
    pub category: ResourceCategory,
    pub total_count: u32,
    pub improved_count: u32,
}

// ── Unit reports ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitReport {
    pub id: UnitId,
    pub type_name: String,
    pub location: HexCoord,
    pub health: u32,
    pub movement_left: u32,
    pub max_movement: u32,
    pub combat_strength: Option<u32>,
    pub category: UnitCategory,
}

// ── Map statistics ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapStatistics {
    pub terrain_counts: HashMap<String, u32>,
    pub feature_counts: HashMap<String, u32>,
    pub resource_counts: HashMap<String, u32>,
    pub enemy_cities_visible: u32,
    pub enemy_units_visible: u32,
    pub tiles_explored: u32,
    pub tiles_visible: u32,
}

// ── Player reports ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerReport {
    pub id: CivId,
    pub name: String,
    pub leader_name: String,
    pub score: u32,
    pub diplomatic_status: DiplomaticStatus,
    pub known_cities: u32,
    pub known_units: u32,
}

// ── Turn status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStatus {
    pub game_id: GameId,
    pub current_turn: u32,
    pub game_status: GameStatus,
    pub players_submitted: Vec<(CivId, bool)>,
}

// ── Science / Culture reports ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScienceReport {
    pub tech_tree: super::view::TechTreeView,
    pub researched_techs: Vec<TechId>,
    pub research_queue: Vec<super::view::TechProgressView>,
    pub science_per_turn: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultureReport {
    pub civic_tree: super::view::CivicTreeView,
    pub completed_civics: Vec<CivicId>,
    pub civic_in_progress: Option<super::view::CivicProgressView>,
    pub culture_per_turn: i32,
}
