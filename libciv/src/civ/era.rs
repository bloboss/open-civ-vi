use crate::{AgeType, CivId, EraId};

// ---------------------------------------------------------------------------
// Era trigger trait (existing)
// ---------------------------------------------------------------------------

pub trait EraTrigger: std::fmt::Debug {
    fn description(&self) -> &'static str;
    /// Returns true if the era transition condition is met.
    fn is_triggered(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Era definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Era {
    pub id: EraId,
    pub name: &'static str,
    pub age: AgeType,
    /// Number of techs required globally (any civ) to trigger advancement past this era.
    pub tech_count: u32,
    /// Number of civics required globally (any civ) to trigger advancement past this era.
    pub civic_count: u32,
}

impl Era {
    pub fn new(id: EraId, name: &'static str, age: AgeType) -> Self {
        Self {
            id,
            name,
            age,
            tech_count: 0,
            civic_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Era Age -- the "quality" of a civ's era (Dark/Normal/Golden/Heroic)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EraAge {
    Dark,
    Normal,
    Golden,
    /// A Golden Age that immediately follows a Dark Age.
    Heroic,
}

impl Default for EraAge {
    fn default() -> Self {
        EraAge::Normal
    }
}

// ---------------------------------------------------------------------------
// Era score thresholds
// ---------------------------------------------------------------------------

/// Era score at or below this value results in a Dark Age.
pub const DARK_AGE_THRESHOLD: u32 = 11;
/// Era score at or above this value results in a Golden (or Heroic) Age.
pub const GOLDEN_AGE_THRESHOLD: u32 = 24;

/// Determine the era age for a civ based on accumulated era score and
/// whether the civ was in a Dark Age during the previous era.
pub fn compute_era_age(era_score: u32, was_dark_age: bool) -> EraAge {
    if era_score >= GOLDEN_AGE_THRESHOLD {
        if was_dark_age {
            EraAge::Heroic
        } else {
            EraAge::Golden
        }
    } else if era_score <= DARK_AGE_THRESHOLD {
        EraAge::Dark
    } else {
        EraAge::Normal
    }
}

// ---------------------------------------------------------------------------
// Historic moment types
// ---------------------------------------------------------------------------

/// Categories of game events that can trigger a historic moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HistoricMomentKind {
    CityFounded,
    TechResearched,
    CivicCompleted,
    WonderBuilt,
    BattleWon,
    DistrictBuilt,
    BuildingCompleted,
    TradeRouteEstablished,
}

/// Static definition of a historic moment. Const data in `historic_moments.rs`.
#[derive(Debug, Clone)]
pub struct HistoricMomentDef {
    pub name: &'static str,
    pub description: &'static str,
    pub era_score: u32,
    pub kind: HistoricMomentKind,
    /// If true, this moment can only be earned once per civ per era.
    pub unique: bool,
}

/// A recorded historic moment for a specific civilization.
#[derive(Debug, Clone)]
pub struct HistoricMoment {
    pub civ: CivId,
    pub moment_name: &'static str,
    pub era_score: u32,
    pub turn: u32,
    pub era: AgeType,
}

// ---------------------------------------------------------------------------
// Era dedication (stub for future bonuses)
// ---------------------------------------------------------------------------

/// A dedication chosen when entering a new era. Stub -- modifiers not yet implemented.
#[derive(Debug, Clone)]
pub struct EraDedication {
    pub name: &'static str,
    pub description: &'static str,
    /// Which era age(s) make this dedication available.
    pub required_age: EraAge,
    // TODO: pub modifiers: Vec<crate::rules::modifier::Modifier>,
}

// ---------------------------------------------------------------------------
// Era advancement check
// ---------------------------------------------------------------------------

/// Check whether any civilization has crossed the tech/civic threshold for the
/// current era, triggering a global era advancement.
///
/// Returns `true` if the era should advance.
pub fn should_advance_era(
    current_era: &Era,
    civs: &[crate::civ::Civilization],
) -> bool {
    for civ in civs {
        let techs = civ.researched_techs.len() as u32;
        let civics = civ.completed_civics.len() as u32;
        if techs >= current_era.tech_count && civics >= current_era.civic_count
            && (current_era.tech_count > 0 || current_era.civic_count > 0)
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_era_age_dark() {
        assert_eq!(compute_era_age(0, false), EraAge::Dark);
        assert_eq!(compute_era_age(11, false), EraAge::Dark);
    }

    #[test]
    fn compute_era_age_normal() {
        assert_eq!(compute_era_age(12, false), EraAge::Normal);
        assert_eq!(compute_era_age(23, false), EraAge::Normal);
    }

    #[test]
    fn compute_era_age_golden() {
        assert_eq!(compute_era_age(24, false), EraAge::Golden);
        assert_eq!(compute_era_age(100, false), EraAge::Golden);
    }

    #[test]
    fn compute_era_age_heroic_after_dark() {
        assert_eq!(compute_era_age(24, true), EraAge::Heroic);
        assert_eq!(compute_era_age(100, true), EraAge::Heroic);
    }

    #[test]
    fn compute_era_age_dark_even_after_dark() {
        assert_eq!(compute_era_age(5, true), EraAge::Dark);
    }
}
