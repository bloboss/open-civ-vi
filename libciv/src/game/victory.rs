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
pub struct GameOver {
    pub winner: CivId,
    /// Display name of the winning condition (e.g. `"Score Victory"`).
    pub condition: &'static str,
    pub turn: u32,
}

/// Generic victory condition interface.
///
/// Implement this trait to add new victory types. Register instances on
/// `GameState::victory_conditions` before the game loop starts.
///
/// # Evaluation order
/// `advance_turn` checks `ImmediateWin` conditions each turn (first match wins),
/// then checks `TurnLimit` conditions when the turn limit is reached (highest scorer wins).
pub trait VictoryCondition: std::fmt::Debug + Send + Sync {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn kind(&self) -> VictoryKind;
    /// Returns the current progress for `civ_id`. `is_won()` signals an immediate win
    /// for `ImmediateWin` conditions; for `TurnLimit` conditions the score is used
    /// by the engine to pick the winner when the limit is reached.
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress;
}

// ── Score Victory ─────────────────────────────────────────────────────────────

/// Score-based victory: the civilization with the highest score when the turn
/// limit is reached wins. Score is computed by `game::score::compute_score`.
#[derive(Debug)]
pub struct ScoreVictory {
    pub id: VictoryId,
    /// The game ends on this turn (inclusive).
    pub turn_limit: u32,
}

impl VictoryCondition for ScoreVictory {
    fn id(&self) -> VictoryId { self.id }
    fn name(&self) -> &'static str { "Score Victory" }
    fn description(&self) -> &'static str {
        "The civilization with the highest score when the turn limit is reached wins."
    }
    fn kind(&self) -> VictoryKind { VictoryKind::TurnLimit { turn_limit: self.turn_limit } }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        VictoryProgress {
            victory_id: self.id,
            civ_id,
            // `current` carries the score; `target` is the turn limit for display.
            current: compute_score(state, civ_id),
            target: self.turn_limit,
        }
    }
}

// ── Culture Victory ──────────────────────────────────────────────────────────

/// Culture victory: a civ wins when its accumulated tourism toward every other
/// civ exceeds that civ's domestic tourists (derived from lifetime culture).
///
/// Evaluated every turn as an `ImmediateWin` condition.
#[derive(Debug)]
pub struct CultureVictory {
    pub id: VictoryId,
}

impl VictoryCondition for CultureVictory {
    fn id(&self) -> VictoryId { self.id }
    fn name(&self) -> &'static str { "Culture Victory" }
    fn description(&self) -> &'static str {
        "Achieve cultural dominance over all other civilizations by generating \
         more tourism toward each civ than their domestic tourists."
    }
    fn kind(&self) -> VictoryKind { VictoryKind::ImmediateWin }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        use crate::civ::tourism::{has_cultural_dominance, domestic_tourists};

        let other_civs: Vec<_> = state.civilizations.iter()
            .filter(|c| c.id != civ_id)
            .collect();

        if other_civs.is_empty() {
            return VictoryProgress {
                victory_id: self.id, civ_id, current: 0, target: 1,
            };
        }

        // Count how many civs we have cultural dominance over.
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
        // Also check full dominance for the `is_won()` shortcut.
        let won = has_cultural_dominance(state, civ_id);
        VictoryProgress {
            victory_id: self.id,
            civ_id,
            current: if won { target } else { dominated },
            target,
        }
    }
}

// ── Domination Victory ───────────────────────────────────────────────────────

/// Domination victory: a civilization wins when it controls every other
/// civilization's original capital city.
///
/// "Original capital" = the first city a civ founded (`is_capital == true` at
/// founding). Captured capitals retain `founded_by` pointing to the original
/// owner. A civ "controls" a capital when `city.owner == civ_id`.
///
/// A civ always controls its own capital, so the check only considers capitals
/// whose `founded_by != civ_id`.
#[derive(Debug)]
pub struct DominationVictory {
    pub id: VictoryId,
}

impl VictoryCondition for DominationVictory {
    fn id(&self) -> VictoryId { self.id }
    fn name(&self) -> &'static str { "Domination Victory" }
    fn description(&self) -> &'static str {
        "Win by capturing every other civilization's original capital city."
    }
    fn kind(&self) -> VictoryKind { VictoryKind::ImmediateWin }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        // Find all original capitals (is_capital == true) not founded by civ_id.
        let foreign_capitals: Vec<&crate::civ::City> = state.cities.iter()
            .filter(|c| c.is_capital && c.founded_by != civ_id)
            .collect();

        let total = foreign_capitals.len() as u32;
        let controlled = foreign_capitals.iter()
            .filter(|c| c.owner == civ_id)
            .count() as u32;

        VictoryProgress {
            victory_id: self.id,
            civ_id,
            current: controlled,
            target: if total == 0 { 1 } else { total },
        }
    }
}
