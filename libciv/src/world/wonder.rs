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

/// NOTE: Krakatoa is **not** in the Civ VI base game — it was added in DLC.
/// Retained for backward compatibility; tag with `is_base_game: false` if a
/// content-gating flag is added later.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// NOTE: Grand Mesa is **not** in the Civ VI base game — it was added in DLC.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CliffsOfDover { pub id: NaturalWonderId }
impl NaturalWonder for CliffsOfDover {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Cliffs of Dover" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 2)
            .with(crate::YieldType::Culture, 3)
            .with(crate::YieldType::Gold, 3)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

/// NOTE: Uluru is **not** in the Civ VI base game — it was added in DLC.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Yosemite { pub id: NaturalWonderId }
impl NaturalWonder for Yosemite {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Yosemite" }
    fn appeal_bonus(&self) -> i32 { 4 }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

// ── 7 Gathering Storm natural wonders ─────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChocolateHills { pub id: NaturalWonderId }
impl NaturalWonder for ChocolateHills {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Chocolate Hills" }
    fn appeal_bonus(&self) -> i32 { 2 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Production, 2)
            .with(crate::YieldType::Science, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::TWO }  // passable, hills-like
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DevilsTower { pub id: NaturalWonderId }
impl NaturalWonder for DevilsTower {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Devil's Tower" }
    fn appeal_bonus(&self) -> i32 { 2 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Faith, 1)
            .with(crate::YieldType::Production, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gobustan { pub id: NaturalWonderId }
impl NaturalWonder for Gobustan {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Gobustan" }
    fn appeal_bonus(&self) -> i32 { 2 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Culture, 3)
            .with(crate::YieldType::Production, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Ik-Kil is an adjacent-yield wonder: the tile itself has no yields,
/// but adjacent tiles get bonuses. Simplified here as tile yields until
/// the adjacent-yield system is implemented.
pub struct IkKil { pub id: NaturalWonderId }
impl NaturalWonder for IkKil {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Ik-Kil" }
    fn appeal_bonus(&self) -> i32 { 2 }
    fn yield_bonus(&self) -> YieldBundle {
        // XML: no tile yields; adjacent tiles get culture+faith.
        // Simplified: place yields on the wonder tile itself.
        YieldBundle::new()
            .with(crate::YieldType::Faith, 2)
            .with(crate::YieldType::Culture, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Pamukkale is an adjacent-yield wonder: adjacent districts get +2 Culture,
/// +1 Faith, +2 Gold, +2 Science. Simplified here as tile yields.
pub struct Pamukkale { pub id: NaturalWonderId }
impl NaturalWonder for Pamukkale {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Pamukkale" }
    fn appeal_bonus(&self) -> i32 { 2 }
    fn yield_bonus(&self) -> YieldBundle {
        // XML: adjacent districts get culture, faith, gold, science.
        // Simplified: place representative yields on the wonder tile.
        YieldBundle::new()
            .with(crate::YieldType::Culture, 2)
            .with(crate::YieldType::Faith, 1)
            .with(crate::YieldType::Gold, 2)
            .with(crate::YieldType::Science, 2)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }  // passable, provides fresh water
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WhiteDesert { pub id: NaturalWonderId }
impl NaturalWonder for WhiteDesert {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "White Desert" }
    fn appeal_bonus(&self) -> i32 { 2 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Culture, 1)
            .with(crate::YieldType::Gold, 4)
            .with(crate::YieldType::Science, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Vesuvius is a volcanic natural wonder — can erupt in the GS disaster system.
pub struct Vesuvius { pub id: NaturalWonderId }
impl NaturalWonder for Vesuvius {
    fn id(&self) -> NaturalWonderId { self.id }
    fn name(&self) -> &'static str { "Vesuvius" }
    fn appeal_bonus(&self) -> i32 { 2 }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Production, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn impassable(&self) -> bool { true }
}

// ── Builtin enum ──────────────────────────────────────────────────────────────

/// Enum wrapping all built-in natural wonders for direct inline storage.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    ChocolateHills(ChocolateHills),
    DevilsTower(DevilsTower),
    Gobustan(Gobustan),
    IkKil(IkKil),
    Pamukkale(Pamukkale),
    WhiteDesert(WhiteDesert),
    Vesuvius(Vesuvius),
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
            BuiltinNaturalWonder::ChocolateHills(w)    => w,
            BuiltinNaturalWonder::DevilsTower(w)       => w,
            BuiltinNaturalWonder::Gobustan(w)          => w,
            BuiltinNaturalWonder::IkKil(w)             => w,
            BuiltinNaturalWonder::Pamukkale(w)         => w,
            BuiltinNaturalWonder::WhiteDesert(w)       => w,
            BuiltinNaturalWonder::Vesuvius(w)          => w,
        }
    }
}
