use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexCoordError {
    InvalidCoord,
}

impl std::fmt::Display for HexCoordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HexCoord invariant violated: q + r + s must equal 0")
    }
}

impl std::error::Error for HexCoordError {}

impl HexCoord {
    pub fn new(q: i32, r: i32, s: i32) -> Result<Self, HexCoordError> {
        if q + r + s != 0 {
            Err(HexCoordError::InvalidCoord)
        } else {
            Ok(Self { q, r, s })
        }
    }

    /// Construct from q and r; s is computed automatically.
    pub fn from_qr(q: i32, r: i32) -> Self {
        Self { q, r, s: -q - r }
    }

    pub fn zero() -> Self {
        Self { q: 0, r: 0, s: 0 }
    }

    pub fn distance(&self, other: &HexCoord) -> u32 {
        let diff = *self - *other;
        (diff.q.abs() + diff.r.abs() + diff.s.abs()) as u32 / 2
    }

    pub fn neighbors(&self) -> [HexCoord; 6] {
        HexDir::ALL.map(|d| *self + d.unit_vec())
    }

    pub fn ring(&self, radius: u32) -> Vec<HexCoord> {
        if radius == 0 {
            return vec![*self];
        }
        let mut results = Vec::with_capacity(6 * radius as usize);
        let mut cursor = *self + HexDir::SW.unit_vec() * radius as i32;
        for dir in HexDir::ALL {
            for _ in 0..radius {
                results.push(cursor);
                cursor = cursor + dir.unit_vec();
            }
        }
        results
    }
}

impl Add for HexCoord {
    type Output = HexCoord;
    fn add(self, rhs: HexCoord) -> HexCoord {
        // invariant preserved: (q1+q2)+(r1+r2)+(s1+s2) = (q1+r1+s1)+(q2+r2+s2) = 0
        HexCoord {
            q: self.q + rhs.q,
            r: self.r + rhs.r,
            s: self.s + rhs.s,
        }
    }
}

impl Sub for HexCoord {
    type Output = HexCoord;
    fn sub(self, rhs: HexCoord) -> HexCoord {
        HexCoord {
            q: self.q - rhs.q,
            r: self.r - rhs.r,
            s: self.s - rhs.s,
        }
    }
}

impl Neg for HexCoord {
    type Output = HexCoord;
    fn neg(self) -> HexCoord {
        HexCoord {
            q: -self.q,
            r: -self.r,
            s: -self.s,
        }
    }
}

impl Mul<i32> for HexCoord {
    type Output = HexCoord;
    fn mul(self, rhs: i32) -> HexCoord {
        HexCoord {
            q: self.q * rhs,
            r: self.r * rhs,
            s: self.s * rhs,
        }
    }
}

/// Six cardinal directions in a flat-top hex grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HexDir {
    /// +q, -s
    E,
    /// +q, -r
    NE,
    /// +r, -s … wait — standard flat-top axial:
    NW,
    /// -q, +s
    W,
    /// -q, +r
    SW,
    /// -r, +s
    SE,
}

impl HexDir {
    pub const ALL: [HexDir; 6] = [
        HexDir::E,
        HexDir::NE,
        HexDir::NW,
        HexDir::W,
        HexDir::SW,
        HexDir::SE,
    ];

    /// Unit vector for this direction.
    pub fn unit_vec(self) -> HexCoord {
        match self {
            HexDir::E => HexCoord { q: 1, r: 0, s: -1 },
            HexDir::NE => HexCoord { q: 1, r: -1, s: 0 },
            HexDir::NW => HexCoord { q: 0, r: -1, s: 1 },
            HexDir::W => HexCoord { q: -1, r: 0, s: 1 },
            HexDir::SW => HexCoord { q: -1, r: 1, s: 0 },
            HexDir::SE => HexCoord { q: 0, r: 1, s: -1 },
        }
    }

    pub fn opposite(self) -> HexDir {
        match self {
            HexDir::E => HexDir::W,
            HexDir::NE => HexDir::SW,
            HexDir::NW => HexDir::SE,
            HexDir::W => HexDir::E,
            HexDir::SW => HexDir::NE,
            HexDir::SE => HexDir::NW,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_coord_invariant_enforced() {
        assert!(HexCoord::new(1, 1, 1).is_err());
        assert!(HexCoord::new(0, 0, 0).is_ok());
        assert!(HexCoord::new(1, -1, 0).is_ok());
    }

    #[test]
    fn test_hex_coord_arithmetic() {
        let a = HexCoord::from_qr(1, 2);
        let b = HexCoord::from_qr(-1, 1);
        let sum = a + b;
        // invariant: q+r+s == 0
        assert_eq!(sum.q + sum.r + sum.s, 0);
        let diff = a - b;
        assert_eq!(diff.q + diff.r + diff.s, 0);
    }

    #[test]
    fn test_hex_distance_adjacent() {
        let origin = HexCoord::zero();
        for neighbor in origin.neighbors() {
            assert_eq!(origin.distance(&neighbor), 1);
        }
    }

    #[test]
    fn test_hex_distance_symmetry() {
        let a = HexCoord::from_qr(3, -1);
        let b = HexCoord::from_qr(-2, 4);
        assert_eq!(a.distance(&b), b.distance(&a));
    }

    #[test]
    fn test_neighbors_count() {
        let coord = HexCoord::from_qr(5, -3);
        assert_eq!(coord.neighbors().len(), 6);
    }
}
