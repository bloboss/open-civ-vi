//! Pending-action enumeration: what a civ owes the turn engine before
//! `advance_turn` can run cleanly.
//!
//! Exposed via [`super::RulesEngine::pending_actions`]. The web projector
//! and CLI `status pending` subcommand both read this — keep wire shapes
//! out of here, since the same data needs to flow into both.

use libhexgrid::coord::HexCoord;

use crate::{CityId, CivId, UnitId};

use super::super::state::GameState;

/// What a civ owes the turn engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAction {
    pub kind: PendingActionKind,
    /// `true` for actions the player must resolve before the next
    /// `advance_turn`; `false` for advisory items the engine can ignore.
    pub required: bool,
    pub civ_id: CivId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingActionKind {
    /// `civ.research_queue` is empty while the tech tree still has nodes.
    ChooseResearch,
    /// `civ.civic_in_progress` is `None` while the civic tree still has
    /// nodes.
    ChooseCivic,
    /// Unit has movement remaining and no automated orders.
    UnitNeedsOrders { unit_id: UnitId, coord: HexCoord },
    /// City has an empty production queue.
    CityNeedsProduction { city_id: CityId },
}

/// Compute the pending-action list for `civ`. Required items come first,
/// then unit/city items in `state` order.
pub(crate) fn pending_actions(state: &GameState, civ: CivId) -> Vec<PendingAction> {
    let Some(civ_data) = state.civilizations.iter().find(|c| c.id == civ) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    if civ_data.research_queue.is_empty() && !state.tech_tree.nodes.is_empty() {
        out.push(PendingAction {
            kind: PendingActionKind::ChooseResearch,
            required: true,
            civ_id: civ,
        });
    }

    if civ_data.civic_in_progress.is_none() && !state.civic_tree.nodes.is_empty() {
        out.push(PendingAction {
            kind: PendingActionKind::ChooseCivic,
            required: true,
            civ_id: civ,
        });
    }

    for u in state
        .units
        .iter()
        .filter(|u| u.owner == civ && u.movement_left > 0)
    {
        out.push(PendingAction {
            kind: PendingActionKind::UnitNeedsOrders {
                unit_id: u.id,
                coord: u.coord,
            },
            required: false,
            civ_id: civ,
        });
    }

    for c in state.cities.iter().filter(|c| c.owner == civ) {
        if c.production_queue.is_empty() {
            out.push(PendingAction {
                kind: PendingActionKind::CityNeedsProduction { city_id: c.id },
                required: false,
                civ_id: civ,
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DefaultRulesEngine;
    use crate::RulesEngine;
    use crate::civ::civilization::Civilization;
    use crate::civ::{BuiltinAgenda, Leader};

    fn fresh_state() -> (GameState, CivId) {
        let mut state = GameState::new(7, 10, 10);
        let civ_id = state.id_gen.next_civ_id();
        let leader = Leader {
            name: "TestLeader",
            civ_id,
            agenda: BuiltinAgenda::Default,
        };
        state
            .civilizations
            .push(Civilization::new(civ_id, "TestCiv", "Test", leader));
        (state, civ_id)
    }

    #[test]
    fn fresh_civ_owes_research_and_civic_choices() {
        let (state, civ) = fresh_state();
        let actions = DefaultRulesEngine.pending_actions(&state, civ);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a.kind, PendingActionKind::ChooseResearch) && a.required)
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a.kind, PendingActionKind::ChooseCivic) && a.required)
        );
    }

    #[test]
    fn unknown_civ_yields_empty() {
        let (state, _) = fresh_state();
        let bogus = CivId::from_ulid(ulid::Ulid::new());
        assert!(DefaultRulesEngine.pending_actions(&state, bogus).is_empty());
    }

    #[test]
    fn victory_progress_returns_one_entry_per_registered_condition() {
        use crate::VictoryId;
        use crate::game::victory::BuiltinVictoryCondition;

        let (mut state, civ) = fresh_state();
        // No conditions registered → empty.
        assert!(DefaultRulesEngine.victory_progress(&state, civ).is_empty());

        // Register a Score condition (turn_limit:500) and a Domination one;
        // expect one VictoryProgress per condition.
        state
            .victory_conditions
            .push(BuiltinVictoryCondition::Score {
                id: VictoryId::from_ulid(ulid::Ulid::new()),
                turn_limit: 500,
            });
        state
            .victory_conditions
            .push(BuiltinVictoryCondition::Domination {
                id: VictoryId::from_ulid(ulid::Ulid::new()),
            });
        let progress = DefaultRulesEngine.victory_progress(&state, civ);
        assert_eq!(progress.len(), 2);
    }
}
