use serde::{Deserialize, Serialize};

/// Axial hex coordinate (cube coords with q + r + s = 0 invariant).
///
/// This is a serializable mirror of `libhexgrid::coord::HexCoord`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

impl HexCoord {
    pub fn from_qr(q: i32, r: i32) -> Self {
        Self { q, r, s: -q - r }
    }

    pub fn zero() -> Self {
        Self { q: 0, r: 0, s: 0 }
    }
}

impl std::fmt::Display for HexCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.q, self.r)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HexDir {
    E,
    NE,
    NW,
    W,
    SW,
    SE,
}
