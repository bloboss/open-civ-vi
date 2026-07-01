//! Per-civ policy-card catalogue.
//!
//! Walks `state.policies` (registered by `GameState::new`) and labels
//! each entry as Active / Available / Locked from the civ's
//! `active_policies` and `unlocked_policies`. Used by the web
//! `/government` endpoint and the CLI `status policies` subcommand.

use std::collections::HashSet;

use crate::{CivId, PolicyId, PolicyType};

use super::super::state::GameState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyCardEntry {
    pub policy_id: PolicyId,
    pub name: &'static str,
    pub policy_type: PolicyType,
    /// Name of the civic that unlocks this policy.
    pub prereq_civic: &'static str,
    pub status: PolicyCardStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyCardStatus {
    /// Currently slotted into the civ's `active_policies`.
    Active,
    /// Unlocked (the civ has completed `prereq_civic`) but not slotted.
    Available,
    /// Not yet unlocked.
    Locked,
}

pub(crate) fn policy_catalogue(state: &GameState, civ: CivId) -> Vec<PolicyCardEntry> {
    let Some(civ_data) = state.civilizations.iter().find(|c| c.id == civ) else {
        return Vec::new();
    };

    let active: HashSet<PolicyId> = civ_data.active_policies.iter().copied().collect();
    let unlocked: HashSet<&'static str> = civ_data.unlocked_policies.iter().copied().collect();

    state
        .policies
        .iter()
        .map(|p| {
            let status = if active.contains(&p.id) {
                PolicyCardStatus::Active
            } else if unlocked.contains(p.name) {
                PolicyCardStatus::Available
            } else {
                PolicyCardStatus::Locked
            };
            PolicyCardEntry {
                policy_id: p.id,
                name: p.name,
                policy_type: p.policy_type,
                prereq_civic: p.prereq_civic,
                status,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::civilization::Civilization;
    use crate::civ::{BuiltinAgenda, Leader};
    use crate::{DefaultRulesEngine, RulesEngine};

    fn make_state() -> (GameState, CivId) {
        let mut state = GameState::new(17, 10, 10);
        let civ_id = state.id_gen.next_civ_id();
        let leader = Leader {
            name: "L",
            civ_id,
            agenda: BuiltinAgenda::Default,
        };
        state
            .civilizations
            .push(Civilization::new(civ_id, "TestCiv", "Test", leader));
        (state, civ_id)
    }

    #[test]
    fn fresh_civ_has_all_policies_locked() {
        let (state, civ) = make_state();
        let cat = DefaultRulesEngine.policy_catalogue(&state, civ);
        assert_eq!(
            cat.len(),
            state.policies.len(),
            "one entry per registered policy"
        );
        assert!(cat.iter().all(|e| e.status == PolicyCardStatus::Locked));
    }

    #[test]
    fn unlocking_a_policy_flips_to_available() {
        let (mut state, civ) = make_state();
        // Pretend "Discipline" was unlocked by completing Code of Laws.
        state.civilizations[0].unlocked_policies.push("Discipline");
        let cat = DefaultRulesEngine.policy_catalogue(&state, civ);
        let discipline = cat
            .iter()
            .find(|e| e.name == "Discipline")
            .expect("Discipline must be in the registry");
        assert_eq!(discipline.status, PolicyCardStatus::Available);
    }

    #[test]
    fn slotted_policy_shows_as_active() {
        let (mut state, civ) = make_state();
        state.civilizations[0].unlocked_policies.push("Discipline");
        let pid = state
            .policies
            .iter()
            .find(|p| p.name == "Discipline")
            .unwrap()
            .id;
        state.civilizations[0].active_policies.push(pid);
        let cat = DefaultRulesEngine.policy_catalogue(&state, civ);
        let discipline = cat.iter().find(|e| e.name == "Discipline").unwrap();
        assert_eq!(discipline.status, PolicyCardStatus::Active);
    }

    #[test]
    fn unknown_civ_yields_empty() {
        let (state, _) = make_state();
        let bogus = CivId::from_ulid(ulid::Ulid::new());
        assert!(
            DefaultRulesEngine
                .policy_catalogue(&state, bogus)
                .is_empty()
        );
    }
}
