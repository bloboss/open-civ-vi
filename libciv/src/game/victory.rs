use crate::{CivId, VictoryId};
use crate::rules::VictoryProgress;
use super::score::compute_score;
use super::state::GameState;

/// Determines how and when a victory condition is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VictoryKind {
    /// The first civ to satisfy the condition wins immediately (e.g. Domination, Science).
    /// Evaluated every turn.
    ImmediateWin,
    /// Only evaluated once `state.turn >= turn_limit`; the highest-scoring civ wins.
    TurnLimit { turn_limit: u32 },
}

/// Returned by `advance_turn` when a victory has been declared.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
pub struct GameOver {
    pub winner: CivId,
    /// Display name of the winning condition (e.g. `"Score Victory"`).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
    pub condition: &'static str,
    pub turn: u32,
}

/// The four milestones a civilization must complete for a Science Victory.
pub const SCIENCE_MILESTONES: [&str; 4] = [
    "Launch Earth Satellite",
    "Land on Moon",
    "Establish Mars Colony",
    "Launch Exoplanet Expedition",
];

/// Concrete enum replacing the old `Box<dyn VictoryCondition>` trait object.
///
/// Each variant carries the data previously held by individual victory structs
/// (`ScoreVictory`, `CultureVictory`, etc.). Register instances on
/// `GameState::victory_conditions` before the game loop starts.
///
/// # Evaluation order
/// `advance_turn` checks `ImmediateWin` conditions each turn (first match wins),
/// then checks `TurnLimit` conditions when the turn limit is reached (highest scorer wins).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinVictoryCondition {
    Score { id: VictoryId, turn_limit: u32 },
    Culture { id: VictoryId },
    Domination { id: VictoryId },
    Science { id: VictoryId },
    /// VP-based check: wins when `world_congress.diplomatic_victory_points >= threshold` (default 20).
    Diplomatic { id: VictoryId, threshold: u32 },
    Religious { id: VictoryId },
}

impl BuiltinVictoryCondition {
    pub fn id(&self) -> VictoryId {
        match self {
            Self::Score { id, .. }
            | Self::Culture { id }
            | Self::Domination { id }
            | Self::Science { id }
            | Self::Diplomatic { id, .. }
            | Self::Religious { id } => *id,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Score { .. } => "Score Victory",
            Self::Culture { .. } => "Culture Victory",
            Self::Domination { .. } => "Domination Victory",
            Self::Science { .. } => "Science Victory",
            Self::Diplomatic { .. } => "Diplomatic Victory",
            Self::Religious { .. } => "Religious Victory",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Score { .. } => {
                "The civilization with the highest score when the turn limit is reached wins."
            }
            Self::Culture { .. } => {
                "Achieve cultural dominance over all other civilizations by generating \
                 more tourism toward each civ than their domestic tourists."
            }
            Self::Domination { .. } => {
                "Win by capturing every other civilization's original capital city."
            }
            Self::Science { .. } => {
                "Win by completing all four space-race milestones: Launch Earth Satellite, \
                 Land on Moon, Establish Mars Colony, and Launch Exoplanet Expedition."
            }
            Self::Diplomatic { .. } => {
                "Win by accumulating 20 diplomatic victory points through World Congress \
                 resolutions, aid competitions, and other diplomatic actions."
            }
            Self::Religious { .. } => {
                "Win by converting every other civilization to your religion."
            }
        }
    }

    pub fn kind(&self) -> VictoryKind {
        match self {
            Self::Score { turn_limit, .. } => VictoryKind::TurnLimit { turn_limit: *turn_limit },
            Self::Culture { .. }
            | Self::Domination { .. }
            | Self::Science { .. }
            | Self::Diplomatic { .. }
            | Self::Religious { .. } => VictoryKind::ImmediateWin,
        }
    }

    pub fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        match self {
            Self::Score { id, turn_limit } => {
                VictoryProgress {
                    victory_id: *id,
                    civ_id,
                    current: compute_score(state, civ_id),
                    target: *turn_limit,
                }
            }
            Self::Culture { id } => check_culture(*id, civ_id, state),
            Self::Domination { id } => check_domination(*id, civ_id, state),
            Self::Science { id } => {
                let completed = state.civ(civ_id)
                    .map(|c| c.science_milestones_completed)
                    .unwrap_or(0);
                VictoryProgress {
                    victory_id: *id,
                    civ_id,
                    current: completed,
                    target: SCIENCE_MILESTONES.len() as u32,
                }
            }
            Self::Diplomatic { id, threshold } => {
                let vp = state.world_congress.diplomatic_victory_points
                    .get(&civ_id).copied().unwrap_or(0);
                VictoryProgress {
                    victory_id: *id,
                    civ_id,
                    current: vp,
                    target: *threshold,
                }
            }
            Self::Religious { id } => check_religious(*id, civ_id, state),
        }
    }
}

// ── Private helper functions ──────────────────────────────────────────────────

fn check_culture(id: VictoryId, civ_id: CivId, state: &GameState) -> VictoryProgress {
    use crate::civ::tourism::{has_cultural_dominance, domestic_tourists};

    let other_civs: Vec<_> = state.civilizations.iter()
        .filter(|c| c.id != civ_id)
        .collect();

    if other_civs.is_empty() {
        return VictoryProgress {
            victory_id: id, civ_id, current: 0, target: 1,
        };
    }

    let dominated = if let Some(civ) = state.civ(civ_id) {
        other_civs.iter().filter(|other| {
            let sent = civ.tourism_accumulated
                .get(&other.id).copied().unwrap_or(0);
            sent > domestic_tourists(other)
        }).count() as u32
    } else {
        0
    };

    let target = other_civs.len() as u32;
    let won = has_cultural_dominance(state, civ_id);
    VictoryProgress {
        victory_id: id,
        civ_id,
        current: if won { target } else { dominated },
        target,
    }
}

fn check_domination(id: VictoryId, civ_id: CivId, state: &GameState) -> VictoryProgress {
    let foreign_capitals: Vec<&crate::civ::City> = state.cities.iter()
        .filter(|c| c.is_capital && c.founded_by != civ_id)
        .collect();

    let total = foreign_capitals.len() as u32;
    let controlled = foreign_capitals.iter()
        .filter(|c| c.owner == civ_id)
        .count() as u32;

    VictoryProgress {
        victory_id: id,
        civ_id,
        current: controlled,
        target: if total == 0 { 1 } else { total },
    }
}

fn check_religious(id: VictoryId, civ_id: CivId, state: &GameState) -> VictoryProgress {
    let civ = state.civilizations.iter().find(|c| c.id == civ_id);
    let religion_id = civ.and_then(|c| c.founded_religion);

    let Some(rid) = religion_id else {
        return VictoryProgress {
            victory_id: id, civ_id, current: 0, target: 1,
        };
    };

    let other_civs: Vec<CivId> = state.civilizations.iter()
        .filter(|c| c.id != civ_id)
        .map(|c| c.id)
        .collect();
    let total = other_civs.len() as u32;

    let mut converted = 0u32;
    for other_civ_id in &other_civs {
        let civ_cities: Vec<&crate::civ::City> = state.cities.iter()
            .filter(|c| c.owner == *other_civ_id)
            .collect();
        if civ_cities.is_empty() { continue; }
        let following = civ_cities.iter()
            .filter(|c| c.majority_religion() == Some(rid))
            .count();
        if following * 2 > civ_cities.len() {
            converted += 1;
        }
    }

    VictoryProgress {
        victory_id: id,
        civ_id,
        current: converted,
        target: if total == 0 { 1 } else { total },
    }
}
