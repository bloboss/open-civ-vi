//! Data types for civilization-specific unique components.

use crate::civ::civ_identity::BuiltinCiv;
use crate::civ::district::BuiltinDistrict;
use crate::world::resource::BuiltinResource;
use crate::{UnitCategory, UnitDomain, YieldBundle, YieldType};

// ── Unique unit ──────────────────────────────────────────────────────────────

/// Special abilities intrinsic to a unique unit type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniqueUnitAbility {
    /// Can build forts (Legion).
    CanBuildFort,
    /// Does not lose combat strength when damaged (Samurai).
    NoCombatPenaltyWhenDamaged,
    /// Heals at end of every turn regardless of action (Mamluk).
    HealEveryTurn,
    /// Adjacent enemy units receive a combat strength debuff (Varu: -5).
    DebuffAdjacentEnemies(i32),
    /// Bonus combat strength when adjacent to another unit of same type (Hoplite: +10).
    BonusAdjacentSameType(i32),
}

/// A unique unit that replaces a base unit for a specific civilization.
#[derive(Debug, Clone)]
pub struct UniqueUnitDef {
    pub civ: BuiltinCiv,
    pub name: &'static str,
    /// Name of the base unit this replaces (e.g., "swordsman"). Units with this
    /// name in `unit_type_defs` are hidden for this civ; the unique unit is
    /// shown instead.
    pub replaces: Option<&'static str>,
    pub production_cost: u32,
    pub domain: UnitDomain,
    pub category: UnitCategory,
    pub max_movement: u32,
    pub combat_strength: Option<u32>,
    pub ranged_strength: Option<u32>,
    pub range: u8,
    pub vision_range: u8,
    pub resource_cost: Option<(BuiltinResource, u32)>,
    pub abilities: Vec<UniqueUnitAbility>,
}

// ── Unique district ──────────────────────────────────────────────────────────

/// Placement restrictions for unique districts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistrictPlacementReq {
    /// Must be built on a hills tile (Acropolis).
    MustBeOnHills,
}

/// What provides adjacency bonuses for a unique district.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjacencySource {
    /// +N per adjacent district of any type.
    PerAdjacentDistrict,
    /// +N from a specific adjacent district.
    SpecificDistrict(BuiltinDistrict),
}

/// A single adjacency bonus override for a unique district.
#[derive(Debug, Clone, Copy)]
pub struct AdjacencyOverride {
    pub source: AdjacencySource,
    pub yield_type: YieldType,
    pub amount: i32,
}

/// A unique district that replaces a base district for a specific civ.
#[derive(Debug, Clone)]
pub struct UniqueDistrictDef {
    pub civ: BuiltinCiv,
    pub name: &'static str,
    pub replaces: BuiltinDistrict,
    pub base_cost: u32,
    pub extra_yields: YieldBundle,
    pub extra_housing: i32,
    pub extra_amenities: i32,
    pub placement: Option<DistrictPlacementReq>,
    pub adjacency_overrides: Vec<AdjacencyOverride>,
}

// ── Unique building ──────────────────────────────────────────────────────────

/// Special abilities for unique buildings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniqueBuildingAbility {
    /// Bonus faith equal to the Campus adjacency science bonus (Madrasa).
    FaithEqualsCampusAdjacency,
    /// Culture bonus to all cities within 6 tiles when powered (Electronics Factory).
    CultureToNearbyWhenPowered(i32),
}

/// A unique building that replaces a base building for a specific civ.
#[derive(Debug, Clone)]
pub struct UniqueBuildingDef {
    pub civ: BuiltinCiv,
    pub name: &'static str,
    pub replaces: Option<&'static str>,
    pub cost: u32,
    pub maintenance: u32,
    pub yields: YieldBundle,
    pub requires_district: Option<&'static str>,
    pub extra_housing: i32,
    pub abilities: Vec<UniqueBuildingAbility>,
}

// ── Unique improvement ───────────────────────────────────────────────────────

/// What provides adjacency bonuses for a unique improvement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementAdjacencySource {
    HolySite,
    Farm,
    Pasture,
}

/// A single adjacency bonus for a unique improvement.
#[derive(Debug, Clone, Copy)]
pub struct ImprovementAdjacencyBonus {
    pub adjacent_to: ImprovementAdjacencySource,
    pub yield_type: YieldType,
    pub amount: i32,
}

/// A unique improvement only buildable by a specific civilization.
#[derive(Debug, Clone)]
pub struct UniqueImprovementDef {
    pub civ: BuiltinCiv,
    pub name: &'static str,
    pub base_yields: YieldBundle,
    /// Appeal modifier to adjacent tiles (Sphinx: +2).
    pub appeal_modifier: i32,
    pub adjacency_bonuses: Vec<ImprovementAdjacencyBonus>,
}
