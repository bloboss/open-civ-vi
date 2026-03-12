use crate::NaturalWonderId;
use crate::YieldBundle;
use libhexgrid::types::MovementCost;

pub trait NaturalWonder: std::fmt::Debug + Send + Sync {
    fn id(&self) -> NaturalWonderId;
    fn name(&self) -> &'static str;
    fn appeal_bonus(&self) -> i32 { 0 }
    /// Yields added to the tile that hosts this wonder.
    fn yield_bonus(&self) -> YieldBundle;
    /// Movement cost override for the wonder tile.
    fn movement_cost(&self) -> MovementCost;
    /// Whether units cannot enter this tile.
    fn impassable(&self) -> bool { false }
}

// ── 5 built-in natural wonders ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct Krakatoa { pub id: NaturalWonderId }
impl NaturalWonder for Krakatoa {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Krakatoa" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Production, 4)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct GrandMesa { pub id: NaturalWonderId }
impl NaturalWonder for GrandMesa {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Grand Mesa" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Production, 2)
            .with(crate::YieldType::Food, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::TWO }  // hills movement cost
}

#[derive(Debug, Clone, Copy)]
pub struct CliffsOfDover { pub id: NaturalWonderId }
impl NaturalWonder for CliffsOfDover {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Cliffs of Dover" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Culture, 2)
            .with(crate::YieldType::Gold, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct UluruAyersRock { pub id: NaturalWonderId }
impl NaturalWonder for UluruAyersRock {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Uluru / Ayers Rock" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Faith, 3)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct GalapagosIslands { pub id: NaturalWonderId }
impl NaturalWonder for GalapagosIslands {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Galapagos Islands" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Science, 2)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }  // coast movement cost
}

// ── Builtin enum ──────────────────────────────────────────────────────────────

/// Enum wrapping all built-in natural wonders for direct inline storage.
#[derive(Debug, Clone, Copy)]
pub enum BuiltinNaturalWonder {
    Krakatoa(Krakatoa),
    GrandMesa(GrandMesa),
    CliffsOfDover(CliffsOfDover),
    UluruAyersRock(UluruAyersRock),
    GalapagosIslands(GalapagosIslands),
}

impl BuiltinNaturalWonder {
    pub fn as_def(&self) -> &dyn NaturalWonder {
        match self {
            BuiltinNaturalWonder::Krakatoa(w)         => w,
            BuiltinNaturalWonder::GrandMesa(w)        => w,
            BuiltinNaturalWonder::CliffsOfDover(w)    => w,
            BuiltinNaturalWonder::UluruAyersRock(w)   => w,
            BuiltinNaturalWonder::GalapagosIslands(w) => w,
        }
    }
}
