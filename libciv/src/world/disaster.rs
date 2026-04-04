//! Environmental disaster types (GS-2).

/// The kind of natural disaster that can occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DisasterKind {
    VolcanicEruption,
    Flood,
    Blizzard,
    DustStorm,
    Tornado,
    Hurricane,
    Drought,
}

impl DisasterKind {
    /// All disaster variants, for random selection.
    pub const ALL: [DisasterKind; 7] = [
        DisasterKind::VolcanicEruption,
        DisasterKind::Flood,
        DisasterKind::Blizzard,
        DisasterKind::DustStorm,
        DisasterKind::Tornado,
        DisasterKind::Hurricane,
        DisasterKind::Drought,
    ];
}
