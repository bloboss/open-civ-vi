use serde::{Deserialize, Serialize};

use crate::ids::*;

// ── Terrain ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinTerrain {
    Grassland,
    Plains,
    Desert,
    Tundra,
    Snow,
    Coast,
    Ocean,
    Mountain,
}

// ── Features ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinFeature {
    Forest,
    Rainforest,
    Marsh,
    Floodplain,
    Reef,
    Ice,
    VolcanicSoil,
    Oasis,
}

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinResource {
    // Bonus
    Wheat,
    Rice,
    Cattle,
    Sheep,
    Fish,
    Stone,
    Copper,
    Deer,
    // Luxury
    Wine,
    Silk,
    Spices,
    Incense,
    Cotton,
    Ivory,
    Sugar,
    Salt,
    // Strategic
    Horses,
    Iron,
    Coal,
    Oil,
    Aluminum,
    Niter,
    Uranium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceCategory {
    Bonus,
    Luxury,
    Strategic,
}

// ── Improvements ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinImprovement {
    Farm,
    Mine,
    LumberMill,
    TradingPost,
    Fort,
    Airstrip,
    MissileSilo,
    Sphinx,
    Stepwell,
}

// ── Roads ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinRoad {
    Ancient,
    Medieval,
    Industrial,
    Railroad,
}

// ── Districts ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinDistrict {
    Campus,
    TheaterSquare,
    CommercialHub,
    Harbor,
    HolySite,
    Encampment,
    IndustrialZone,
    EntertainmentComplex,
    WaterPark,
    Aqueduct,
    Dam,
    Canal,
}

// ── Units ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitDomain {
    Land,
    Sea,
    Air,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitCategory {
    Civilian,
    Combat,
    Support,
    Religious,
    GreatPerson,
    Trader,
}

// ── Misc game enums ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgeType {
    Ancient,
    Classical,
    Medieval,
    Renaissance,
    Industrial,
    Modern,
    Atomic,
    Information,
    Future,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyType {
    Military,
    Economic,
    Diplomatic,
    Wildcard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiplomaticStatus {
    War,
    Denounced,
    Neutral,
    Friendly,
    Alliance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CityOwnership {
    Normal,
    Occupied,
    Puppet,
    Razed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WallLevel {
    None,
    Ancient,
    Medieval,
    Renaissance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum YieldType {
    Food,
    Production,
    Gold,
    Science,
    Culture,
    Faith,
    Housing,
    Amenities,
    Tourism,
    GreatPersonPoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    Melee,
    Ranged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileVisibility {
    Visible,
    Foggy,
}

// ── Production ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProductionItemView {
    Unit(UnitTypeId),
    Building(BuildingId),
    District(BuiltinDistrict),
    Wonder(WonderId),
}

// ── Victory ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VictoryKind {
    ImmediateWin,
    TurnLimit { turn_limit: u32 },
}
