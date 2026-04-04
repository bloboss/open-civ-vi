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

// ── 10 additional natural wonders ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct GreatBarrierReef { pub id: NaturalWonderId }
impl NaturalWonder for GreatBarrierReef {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Great Barrier Reef" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 3)
            .with(crate::YieldType::Science, 2)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }  // coast movement cost
}

#[derive(Debug, Clone, Copy)]
pub struct CraterLake { pub id: NaturalWonderId }
impl NaturalWonder for CraterLake {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Crater Lake" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Faith, 5)
            .with(crate::YieldType::Science, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct DeadSea { pub id: NaturalWonderId }
impl NaturalWonder for DeadSea {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Dead Sea" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Culture, 2)
            .with(crate::YieldType::Faith, 2)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct MountEverest { pub id: NaturalWonderId }
impl NaturalWonder for MountEverest {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Mount Everest" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct MountKilimanjaro { pub id: NaturalWonderId }
impl NaturalWonder for MountKilimanjaro {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Mount Kilimanjaro" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct Pantanal { pub id: NaturalWonderId }
impl NaturalWonder for Pantanal {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Pantanal" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Culture, 2)
            .with(crate::YieldType::Food, 2)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::TWO }  // marsh-like movement cost
}

#[derive(Debug, Clone, Copy)]
pub struct Piopiotahi { pub id: NaturalWonderId }
impl NaturalWonder for Piopiotahi {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Piopiotahi (Milford Sound)" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct TorresDelPaine { pub id: NaturalWonderId }
impl NaturalWonder for TorresDelPaine {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Torres del Paine" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct TsingyDeBemaraha { pub id: NaturalWonderId }
impl NaturalWonder for TsingyDeBemaraha {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Tsingy de Bemaraha" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
pub struct Yosemite { pub id: NaturalWonderId }
impl NaturalWonder for Yosemite {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Yosemite" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
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
    GreatBarrierReef(GreatBarrierReef),
    CraterLake(CraterLake),
    DeadSea(DeadSea),
    MountEverest(MountEverest),
    MountKilimanjaro(MountKilimanjaro),
    Pantanal(Pantanal),
    Piopiotahi(Piopiotahi),
    TorresDelPaine(TorresDelPaine),
    TsingyDeBemaraha(TsingyDeBemaraha),
    Yosemite(Yosemite),
}

impl BuiltinNaturalWonder {
    pub fn as_def(&self) -> &dyn NaturalWonder {
        match self {
            BuiltinNaturalWonder::Krakatoa(w)         => w,
            BuiltinNaturalWonder::GrandMesa(w)        => w,
            BuiltinNaturalWonder::CliffsOfDover(w)    => w,
            BuiltinNaturalWonder::UluruAyersRock(w)   => w,
            BuiltinNaturalWonder::GalapagosIslands(w) => w,
            BuiltinNaturalWonder::GreatBarrierReef(w) => w,
            BuiltinNaturalWonder::CraterLake(w)       => w,
            BuiltinNaturalWonder::DeadSea(w)           => w,
            BuiltinNaturalWonder::MountEverest(w)      => w,
            BuiltinNaturalWonder::MountKilimanjaro(w)  => w,
            BuiltinNaturalWonder::Pantanal(w)          => w,
            BuiltinNaturalWonder::Piopiotahi(w)        => w,
            BuiltinNaturalWonder::TorresDelPaine(w)    => w,
            BuiltinNaturalWonder::TsingyDeBemaraha(w)  => w,
            BuiltinNaturalWonder::Yosemite(w)          => w,
        }
    }
}
