//! Wire types for the `/api/v1/*` REST surface.
//!
//! These mirror the JSON schemas in `open4x-webui/*.json` (each module name
//! corresponds to a `<name>.json` file) so that the wireframe HTML can be
//! retired without re-shaping its data semantics.
//!
//! Built by `open4x_server::server::web_projection` on the server side;
//! consumed by `open4x_server::components::api` on the client side. Compile
//! for both `ssr` and `csr` targets so the bindings layer can re-use the
//! same types.
//!
//! Phase 0 (scaffolding): every type is a `Default`-constructible stub. Real
//! field schemas land in Phase 1+.

use serde::{Deserialize, Serialize};

// ── /player-state ────────────────────────────────────────────────────────────

pub mod player_state {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct PlayerState {
        pub turn: u32,
        pub turn_max: u32,
        pub era: String,
        pub era_progress: f32,
        pub resources: Resources,
        pub happiness: i32,
        pub strategic: std::collections::BTreeMap<String, u32>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Resources {
        pub gold: Bucket,
        pub science: Bucket,
        pub culture: Bucket,
        pub faith: Bucket,
        pub food: Bucket,
        pub production: Bucket,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Bucket {
        pub value: Option<i32>,
        pub per_turn: i32,
    }
}

// ── /world/snapshot ──────────────────────────────────────────────────────────

pub mod world {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct WorldSnapshot {
        pub world: WorldMeta,
        pub camera: Camera,
        pub legend: Legend,
        pub tiles: Vec<TileView>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct WorldMeta {
        pub width: u32,
        pub height: u32,
        pub wrap_x: bool,
        pub wrap_y: bool,
        pub seed: u64,
        pub turn: u32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Camera {
        pub x: i32,
        pub y: i32,
        pub zoom: f32,
        pub selection: Option<TileCoord>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TileCoord {
        pub q: i32,
        pub r: i32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Legend {
        pub terrains: Vec<String>,
        pub resources: Vec<String>,
        pub edge_kinds: Vec<String>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TileView {
        pub q: i32,
        pub r: i32,
        pub terrain: String,
        pub yields: TileYields,
        #[serde(default)]
        pub appeal: i32,
        #[serde(default)]
        pub flood: i32,
        #[serde(default)]
        pub fog: bool,
        pub owner: Option<String>,
        pub resource: Option<String>,
        pub improvement: Option<String>,
        pub city: Option<TileCity>,
        pub unit: Option<TileUnit>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TileYields {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub p: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub g: Option<i32>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TileCity {
        pub id: String,
        pub name: String,
        pub pop: u32,
        #[serde(default)]
        pub capital: bool,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TileUnit {
        pub id: String,
        pub kind: String,
        pub name: String,
        pub hp: String,
    }
}

// ── /tech ────────────────────────────────────────────────────────────────────

pub mod tech_tree {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TechTreeView {
        pub techs: Vec<TechNode>,
        pub research_queue: Vec<String>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TechNode {
        pub id: String,
        pub name: String,
        pub cost: u32,
        pub progress: Option<u32>,
        pub unlocks: String,
        pub era: String,
        pub status: String,
        pub prereqs: Vec<String>,
    }
}

// ── /civics ──────────────────────────────────────────────────────────────────

pub mod civics_tree {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CivicsTreeView {
        pub civics: Vec<CivicNode>,
        pub civic_queue: Vec<String>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CivicNode {
        pub id: String,
        pub name: String,
        pub cost: u32,
        pub progress: Option<u32>,
        pub unlocks: String,
        pub era: String,
        pub status: String,
        pub prereqs: Vec<String>,
    }
}

// ── /government ──────────────────────────────────────────────────────────────

pub mod government {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct GovernmentPolicies {
        pub government: Government,
        pub active_policies: Vec<ActivePolicy>,
        pub catalogue: Vec<PolicyCard>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Government {
        pub id: String,
        pub name: String,
        pub era: String,
        pub slots: Slots,
        pub legacy_bonus: String,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Slots {
        pub military: u32,
        pub economic: u32,
        pub diplomatic: u32,
        pub wildcard: u32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct ActivePolicy {
        pub slot: String,
        pub id: String,
        pub name: String,
        pub effect: String,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct PolicyCard {
        pub id: String,
        pub name: String,
        #[serde(rename = "type")]
        pub kind: String,
        pub era: String,
        #[serde(default)]
        pub dark_age: bool,
        pub status: String,
        pub unlock_civic: Option<String>,
        pub effect: String,
    }
}

// ── /diplomacy ───────────────────────────────────────────────────────────────

pub mod diplomacy {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Diplomacy {
        pub civs: Vec<CivRow>,
        pub city_states: Vec<CityStateRow>,
        pub active_civ: Option<String>,
        pub deal_draft: Option<DealDraft>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CivRow {
        pub id: String,
        pub name: String,
        pub leader: String,
        pub relation: String,
        pub relation_score: i32,
        pub agenda: String,
        pub government: String,
        pub cities_known: u32,
        pub treaties: Vec<String>,
        pub modifiers: Vec<RelationModifier>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct RelationModifier {
        pub kind: String,
        pub desc: String,
        pub value: i32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CityStateRow {
        pub id: String,
        pub name: String,
        pub envoys: u32,
        #[serde(default)]
        pub suzerain: bool,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct DealDraft {
        pub civ_id: String,
        pub you_give: Vec<String>,
        pub they_give: Vec<String>,
        pub balance: String,
        pub mood: String,
        pub duration_turns: Option<u32>,
    }
}

// ── /empire/overview ─────────────────────────────────────────────────────────

pub mod empire_overview {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct EmpireOverview {
        pub summary: Summary,
        pub cities: Vec<CityRow>,
        pub strategic_resources: Vec<ResourceRow>,
        pub luxury_resources: Vec<String>,
        pub trade_routes: Vec<TradeRow>,
        pub trade_slots_total: u32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Summary {
        pub cities: u32,
        pub population: u32,
        pub treasury: i32,
        pub treasury_per_turn: i32,
        pub military_units: u32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CityRow {
        pub name: String,
        #[serde(default)]
        pub capital: bool,
        pub pop: u32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct ResourceRow {
        pub name: String,
        pub value: u32,
        pub per_turn: i32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TradeRow {
        pub from: String,
        pub to: String,
        pub yields: String,
    }
}

// ── /victory ─────────────────────────────────────────────────────────────────

pub mod victory {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Victory {
        pub turn: u32,
        pub turn_max: u32,
        pub leading_condition: String,
        pub leading_pct: u32,
        pub score: i32,
        pub rank: u32,
        pub rank_of: u32,
        pub conditions: Vec<Condition>,
        pub leaderboard: Vec<LeaderRow>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Condition {
        pub id: String,
        pub name: String,
        pub player_pct: u32,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct LeaderRow {
        pub rank: u32,
        pub name: String,
        #[serde(default)]
        pub is_player: bool,
        pub score: i32,
    }
}

// ── /cities + /cities/:id/tiles ──────────────────────────────────────────────

pub mod city_data {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CityData {
        pub cities: Vec<CityRow>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CityRow {
        pub id: String,
        pub name: String,
        pub owner: String,
        #[serde(default)]
        pub capital: bool,
        #[serde(default)]
        pub is_own: bool,
        pub position: super::world::TileCoord,
        pub population: u32,
        pub food_stored: u32,
        pub food_to_grow: u32,
        pub production_stored: u32,
        /// Names of items currently in the production queue (head = active).
        pub production_queue: Vec<String>,
        pub worked_tile_count: u32,
        pub territory_count: u32,
        /// Player-selected city focus, lowercase.
        #[serde(default)]
        pub focus: String,
    }
}

pub mod city_tiles {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CityTiles {
        pub city_id: String,
        pub center: super::world::TileCoord,
        pub tiles: Vec<TileEntry>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TileEntry {
        pub q: i32,
        pub r: i32,
        #[serde(default)]
        pub worked: bool,
        #[serde(default)]
        pub locked: bool,
        #[serde(default)]
        pub is_center: bool,
    }
}

// ── /units + /armies ─────────────────────────────────────────────────────────

pub mod unit_data {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct UnitData {
        pub units: Vec<Unit>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Unit {
        pub id: String,
        pub name: String,
        pub kind: String,
        pub owner: String,
        #[serde(default)]
        pub is_own: bool,
        pub hp: u32,
        pub hp_max: u32,
        pub mp: u32,
        pub mp_max: u32,
        pub position: super::world::TileCoord,
        pub status: String,
        pub combat_strength: Option<u32>,
        pub range: u8,
        pub vision_range: u8,
        pub category: String,
        pub domain: String,
        pub actions: Vec<UnitAction>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct UnitAction {
        pub id: String,
        pub label: String,
        pub hotkey: Option<String>,
        pub enabled: bool,
    }
}

// ── /combat/preview ──────────────────────────────────────────────────────────

pub mod combat_preview {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct CombatPreview {
        pub attacker_id: String,
        pub defender: Option<DefenderInfo>,
        pub attacker_strength: i32,
        pub defender_strength: i32,
        pub predicted_attacker_damage: i32,
        pub predicted_defender_damage: i32,
        pub note: String,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct DefenderInfo {
        pub id: String,
        pub kind: String,
        pub q: i32,
        pub r: i32,
    }
}

pub mod army_data {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct ArmyData {
        pub armies: Vec<Army>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Army {
        pub id: String,
        pub name: String,
        pub units: Vec<String>,
    }
}

// ── /notifications + /turn-queue ─────────────────────────────────────────────

pub mod notifications {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Notifications {
        pub turn: u32,
        pub notifications: Vec<Notification>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Notification {
        pub id: String,
        pub kind: String,
        pub category: String,
        pub title: String,
        pub desc: String,
        pub target: Option<NotificationTarget>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct NotificationTarget {
        pub screen: String,
        pub q: Option<i32>,
        pub r: Option<i32>,
    }
}

pub mod turn_queue {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TurnQueue {
        pub turn: u32,
        pub items: Vec<TurnQueueItem>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct TurnQueueItem {
        pub id: String,
        pub kind: String,
        #[serde(default)]
        pub required: bool,
        pub title: String,
        pub desc: String,
        pub skip_label: Option<String>,
        pub target: Option<super::notifications::NotificationTarget>,
    }
}

// ── /map/overlays ────────────────────────────────────────────────────────────

pub mod map_overlays {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct MapOverlays {
        pub overlays: Vec<Overlay>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Overlay {
        pub id: String,
        pub label: String,
        pub active: bool,
    }
}

// ── /registry ────────────────────────────────────────────────────────────────

pub mod registry {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Registry {
        pub unit_types: Vec<UnitType>,
        pub buildings: Vec<Building>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct UnitType {
        pub id: String,
        pub name: String,
        pub production_cost: u32,
        pub combat_strength: Option<u32>,
        pub max_movement: u32,
        pub category: String,
        pub domain: String,
        #[serde(default)]
        pub can_found_city: bool,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Building {
        pub id: String,
        pub name: String,
        pub cost: u32,
        pub maintenance: u32,
    }
}

// ── /turn/end response ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TurnStatusBlock {
    pub turn: u32,
    pub ended: bool,
}

/// Standard mutation envelope: `{ ok, view, turn_status }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MutationResponse<T> {
    pub ok: bool,
    pub view: T,
    pub turn_status: TurnStatusBlock,
}

/// Standard error envelope returned with non-2xx statuses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiErrorBody {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ── /governors ───────────────────────────────────────────────────────────────

/// Read-only projection of the owning civilization's governor roster, mirroring
/// libciv's `GameState.governors` plus the per-civ `governor_titles` count.
pub mod governors {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct GovernorsView {
        /// Unspent governor titles available to appoint or promote with.
        pub titles_available: u32,
        pub governors: Vec<GovernorEntry>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct GovernorEntry {
        pub id: String,
        /// Governor definition name (e.g. "Magnus", "Pingala").
        pub name: String,
        /// Display name of the city this governor is assigned to, if any and
        /// resolvable.
        pub assigned_city: Option<String>,
        /// `true` once `turns_to_establish` has reached zero.
        pub established: bool,
        pub turns_to_establish: u32,
        pub promotions: Vec<String>,
    }
}

// ── /climate ─────────────────────────────────────────────────────────────────

pub mod climate {
    use super::*;

    /// Read-only projection of the global climate / sea-level simulation
    /// (libciv GS-2). Mirrors `GameState.global_co2` / `climate_level` plus a
    /// best-effort summary of the player's emissions and submerged tiles.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct ClimateView {
        /// Cumulative global CO2 emitted across all civilizations.
        pub global_co2: u32,
        /// Current sea-level rise stage (0–7).
        pub climate_level: u8,
        /// CO2 thresholds that trigger each successive sea-level stage.
        pub thresholds: Vec<u32>,
        /// CO2 the authenticated player's cities emit per turn.
        pub co2_per_turn: u32,
        /// Count of coastal-lowland tiles submerged by sea-level rise.
        pub submerged_tiles: u32,
        /// Best-effort list of recent disasters (may be empty — there is no
        /// persistent disaster log on `GameState`).
        pub recent_disasters: Vec<DisasterEntry>,
    }

    /// A single environmental disaster occurrence.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]

    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct DisasterEntry {
        /// Disaster kind as a string (e.g. "Flood", "Hurricane").
        pub kind: String,
        /// Turn on which the disaster occurred.
        pub turn: u32,
        /// Tile coordinate `[q, r]` if known.
        pub coord: Option<[i32; 2]>,
    }
}

// ── /great-people ──────────────────────────────────────────────────────────

pub mod great_people {
    use super::*;

    /// Read-only projection of the owner civ's great-people standing: per-class
    /// point progress toward the next recruitment plus the visible roster.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct GreatPeopleView {
        pub points: Vec<GreatPersonProgress>,
        pub roster: Vec<GreatPersonEntry>,
    }

    /// Accumulated points and recruitment threshold for a single great-person class.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct GreatPersonProgress {
        pub class: String,
        pub points: u32,
        pub threshold: u32,
        pub progress: f32,
    }

    /// A single great person visible to the owner: either recruited by them or
    /// still available in the pool.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct GreatPersonEntry {
        pub id: String,
        pub name: String,
        pub class: String,
        pub era: String,
        pub owned: bool,
        pub available: bool,
        pub retired: bool,
        pub ability: String,
    }
}
