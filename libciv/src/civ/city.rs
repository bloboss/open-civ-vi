use super::city_state::CityStateData;
use super::district::BuiltinDistrict;
use crate::rules::modifier::{EffectType, Modifier, ModifierSource, StackingRule, TargetSelector};
use crate::{BuildingId, CityId, CivId, ProjectId, ReligionId, UnitTypeId, WonderId, YieldType};
use libhexgrid::coord::HexCoord;
use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Domestic unrest tiers
// ---------------------------------------------------------------------------

/// Unrest at or above this value places a city in the first (Discontent) tier:
/// a mild production/gold penalty. Below it, unrest is inert.
pub const UNREST_DISCONTENT_TIER: i32 = 25;
/// Unrest at or above this value places a city in the second (Rioting) tier and
/// is the threshold at which the "riot" domestic-crisis event fires.
pub const UNREST_RIOT_TIER: i32 = 50;
/// Unrest at or above this value places a city in the third (Revolt) tier and
/// is the threshold at which the "strike" domestic-crisis event fires.
pub const UNREST_REVOLT_TIER: i32 = 75;
/// Upper clamp for a city's unrest score.
pub const UNREST_MAX: i32 = 100;

/// Yield penalties imposed on a civilization's cities by a single city's
/// domestic `unrest`, scaled by tier. These are collected into the same
/// modifier set [`crate::game::rules::city::compute_yields`] resolves — the
/// base is frozen before modifiers are applied, so folding these per owned
/// city is recursion-safe and each city's penalty is counted exactly once,
/// exactly mirroring [`crate::civ::era::era_age_modifiers`].
///
/// Tiers (non-cumulative — the highest reached tier applies):
/// * `>= UNREST_REVOLT_TIER`   — Revolt: -4 Production, -4 Gold, -2 Science, -2 Culture
/// * `>= UNREST_RIOT_TIER`     — Rioting: -2 Production, -2 Gold, -1 Science
/// * `>= UNREST_DISCONTENT_TIER` — Discontent: -1 Production, -1 Gold
/// * below                     — inert
pub fn city_unrest_modifiers(unrest: i32) -> Vec<Modifier> {
    let make = |yt: YieldType, amount: i32| {
        Modifier::new(
            ModifierSource::Custom("city_unrest"),
            TargetSelector::Global,
            EffectType::YieldFlat(yt, amount),
            StackingRule::Additive,
        )
    };
    if unrest >= UNREST_REVOLT_TIER {
        vec![
            make(YieldType::Production, -4),
            make(YieldType::Gold, -4),
            make(YieldType::Science, -2),
            make(YieldType::Culture, -2),
        ]
    } else if unrest >= UNREST_RIOT_TIER {
        vec![
            make(YieldType::Production, -2),
            make(YieldType::Gold, -2),
            make(YieldType::Science, -1),
        ]
    } else if unrest >= UNREST_DISCONTENT_TIER {
        vec![make(YieldType::Production, -1), make(YieldType::Gold, -1)]
    } else {
        Vec::new()
    }
}

/// Whether this city is a regular player city or an independent city-state.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CityKind {
    /// A standard player or AI city.
    Regular,
    /// An independent city-state. Suzerain/influence mechanics live in `CityStateData`.
    CityState(CityStateData),
}

/// Political/ownership state of a city.
///
/// Transient conditions (Starving, LowHousing, UnderSiege) are computed each
/// turn by the rules engine — they are not stored here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CityOwnership {
    /// Owned and fully managed by the current civilization.
    Normal,
    /// Captured; owner manages production queue but suffers loyalty/amenity penalties.
    Occupied,
    /// Captured but not annexed; AI manages production queue on the owner's behalf.
    /// Still generates yields and counts toward empire size. Distinct from `Occupied`
    /// in that the owner does not directly control production choices.
    Puppet,
    /// Being razed; removed from the map when raze_turns reaches zero (Phase 2).
    Razed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WallLevel {
    None,
    Ancient,
    Medieval,
    Renaissance,
}

/// City production / citizen-assignment focus. `Default` means the engine's
/// standard auto-assignment heuristic (max total yield) drives citizen
/// placement; the others bias citizen selection toward the named yield.
///
/// Behavior note: the focus value is currently surfaced through the wire
/// API and stored on the city, but the engine's auto-assignment heuristic
/// does not yet consult it (a follow-up will switch
/// `auto_assign_citizen` to score by focus when set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CityFocus {
    #[default]
    Default,
    Food,
    Production,
    Gold,
    Science,
    Culture,
    Faith,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProductionItem {
    Unit(UnitTypeId),
    Building(BuildingId),
    District(BuiltinDistrict),
    Wonder(WonderId),
    Project(ProjectId),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct City {
    pub id: CityId,
    pub name: String,
    pub owner: CivId,
    pub founded_by: CivId,
    pub coord: HexCoord,
    pub kind: CityKind,
    pub ownership: CityOwnership,
    pub is_capital: bool,
    pub population: u32,
    pub food_stored: u32,
    pub food_to_grow: u32,
    pub production_stored: u32,
    /// Ordered production queue. `advance_turn` works on `front()` and calls
    /// `pop_front()` when an item completes (requires registry — Part 6.2).
    pub production_queue: VecDeque<ProductionItem>,
    pub walls: WallLevel,
    pub wall_hp: u32,
    pub buildings: Vec<BuildingId>,
    /// World wonders completed in this city. Each wonder's `WonderDef.effects`
    /// are folded into the owning civilization's yields by `compute_yields`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub wonders: Vec<WonderId>,
    /// District types present in this city. Each `BuiltinDistrict` may appear at most once.
    /// The corresponding `PlacedDistrict` (with coord) lives in `GameState::placed_districts`.
    pub districts: Vec<BuiltinDistrict>,
    /// Tiles currently being worked by citizens. Always includes the city center
    /// (set at founding). Citizens are auto-assigned on population growth and can
    /// be overridden via `RulesEngine::assign_citizen`.
    pub worked_tiles: Vec<HexCoord>,
    /// Tiles pinned by player/AI override; survive auto-reassignment.
    pub locked_tiles: HashSet<HexCoord>,
    /// All tiles claimed for this city (city center + ring-1 at founding, plus
    /// tiles acquired via cultural expansion). Used by the border expansion phase
    /// to track which tiles belong to which city (WorldTile.owner only tracks CivId).
    pub territory: HashSet<HexCoord>,
    /// Accumulated per-city shadow culture used exclusively for automatic border
    /// expansion. Does NOT affect the civilization's culture pool (civic research).
    /// Increases each turn by this city's culture output; spent when a tile is claimed.
    pub culture_border: u32,
    /// Whether this city has already performed its ranged bombardment this turn.
    /// Reset to false at the start of each `advance_turn`.
    pub has_attacked_this_turn: bool,
    /// Per-city loyalty score. Range 0–100. Cities at 0 loyalty revolt and
    /// may flip to the civilization exerting the most loyalty pressure, or
    /// become a Free City (independent). Starts at 100 for normally founded
    /// cities; Occupied cities start at 50.
    pub loyalty: i32,
    /// Per-city domestic **unrest** score. Range 0–100. Distinct from
    /// [`loyalty`](Self::loyalty) (which governs ownership/flipping): unrest is
    /// a happiness/discontent proxy that accumulates from war-weariness, dark
    /// ages, nearby disasters, low loyalty and city size, and decays with high
    /// loyalty plus a base rate. Elevated unrest penalizes yields (via
    /// [`city_unrest_modifiers`]) and, at high tiers, fires domestic-crisis
    /// events. Recomputed each turn by the domestic-politics phase. Starts 0.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unrest: i32,
    /// Great work slots provided by buildings in this city.
    pub great_work_slots: Vec<super::great_works::GreatWorkSlot>,
    /// Followers of each religion in this city. Source of truth for religion
    /// follower counts; `Religion` computes totals by querying cities.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_hashmap_as_vec"))]
    pub religious_followers: HashMap<ReligionId, u32>,
    /// Sum of `power_cost` from all buildings in this city. Recomputed each turn.
    #[cfg_attr(feature = "serde", serde(default))]
    pub power_consumed: u32,
    /// Sum of `power_generated` from all power-plant buildings in this city. Recomputed each turn.
    #[cfg_attr(feature = "serde", serde(default))]
    pub power_generated: u32,
    /// Player-selected city focus. See [`CityFocus`]. `Default` lets the
    /// engine's standard auto-assignment heuristic pick worked tiles.
    #[cfg_attr(feature = "serde", serde(default))]
    pub focus: CityFocus,
}

impl City {
    pub fn new(id: CityId, name: String, owner: CivId, coord: HexCoord) -> Self {
        Self {
            id,
            name,
            owner,
            founded_by: owner,
            coord,
            kind: CityKind::Regular,
            ownership: CityOwnership::Normal,
            is_capital: false,
            population: 1,
            food_stored: 0,
            food_to_grow: 15,
            production_stored: 0,
            production_queue: VecDeque::new(),
            walls: WallLevel::None,
            wall_hp: WallLevel::None.max_hp(),
            buildings: Vec::new(),
            wonders: Vec::new(),
            districts: Vec::new(),
            worked_tiles: vec![coord],
            locked_tiles: HashSet::new(),
            territory: HashSet::new(),
            culture_border: 0,
            has_attacked_this_turn: false,
            loyalty: 100,
            unrest: 0,
            great_work_slots: Vec::new(),
            religious_followers: HashMap::new(),
            power_consumed: 0,
            power_generated: 0,
            focus: CityFocus::default(),
        }
    }

    pub fn is_capital(&self) -> bool {
        self.is_capital
    }

    /// Returns the majority religion in this city, if any.
    /// A religion is majority when it has more than 50% of the population as followers.
    pub fn majority_religion(&self) -> Option<ReligionId> {
        let threshold = self.population / 2;
        self.religious_followers
            .iter()
            .filter(|&(_, count)| *count > threshold)
            .max_by_key(|&(_, count)| *count)
            .map(|(&rid, _)| rid)
    }

    pub fn growth_progress(&self) -> f32 {
        if self.food_to_grow == 0 {
            return 1.0;
        }
        self.food_stored as f32 / self.food_to_grow as f32
    }
}

impl WallLevel {
    /// Combat strength bonus granted to the city's ranged attack and defense.
    pub fn defense_bonus(&self) -> i32 {
        match self {
            WallLevel::None => 0,
            WallLevel::Ancient => 3,
            WallLevel::Medieval => 5,
            WallLevel::Renaissance => 8,
        }
    }

    /// Maximum HP of walls at this tier.
    pub fn max_hp(&self) -> u32 {
        match self {
            WallLevel::None => 0,
            WallLevel::Ancient => 50,
            WallLevel::Medieval => 100,
            WallLevel::Renaissance => 200,
        }
    }
}
