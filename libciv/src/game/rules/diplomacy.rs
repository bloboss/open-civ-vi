//! Diplomacy handlers: `declare_war`, `make_peace`, `form_alliance`, `assign_policy`.

use crate::{CivId, PolicyId, PolicyType};
use crate::civ::DiplomaticStatus;
use crate::civ::GrievanceRecord;
use crate::civ::grievance::DeclaredWarGrievance;
use crate::civ::diplomacy::{AllianceType, GrievanceTrigger};

use super::RulesError;
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::rules_helpers::{find_or_create_relation, status_from_score};
use super::super::state::GameState;

/// Declare war between `aggressor` and `target`.
pub(crate) fn declare_war(
    state: &mut GameState,
    aggressor: CivId,
    target: CivId,
) -> Result<GameStateDiff, RulesError> {
    if aggressor == target { return Err(RulesError::SameCivilization); }
    if state.civ(aggressor).is_none() { return Err(RulesError::CivNotFound); }
    if state.civ(target).is_none()    { return Err(RulesError::CivNotFound); }

    let rel_idx = find_or_create_relation(state, aggressor, target);

    if state.diplomatic_relations[rel_idx].status == DiplomaticStatus::War {
        return Err(RulesError::AlreadyAtWar);
    }

    let grievance_id = state.id_gen.next_grievance_id();
    let trigger = DeclaredWarGrievance;
    let record = GrievanceRecord {
        grievance_id,
        description: trigger.description(),
        amount: trigger.grievance_amount(),
        visibility: trigger.visibility(),
        recorded_turn: state.turn,
    };
    state.diplomatic_relations[rel_idx].add_grievance(target, record);
    state.diplomatic_relations[rel_idx].status = DiplomaticStatus::War;

    let (civ_a, civ_b) = (
        state.diplomatic_relations[rel_idx].civ_a,
        state.diplomatic_relations[rel_idx].civ_b,
    );
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::DiplomacyChanged { civ_a, civ_b, new_status: DiplomaticStatus::War });
    Ok(diff)
}

/// End the war between `civ_a` and `civ_b`.
pub(crate) fn make_peace(
    state: &mut GameState,
    civ_a: CivId,
    civ_b: CivId,
) -> Result<GameStateDiff, RulesError> {
    if civ_a == civ_b { return Err(RulesError::SameCivilization); }

    let rel_idx = state.diplomatic_relations.iter().position(|r| {
        (r.civ_a == civ_a && r.civ_b == civ_b) ||
        (r.civ_a == civ_b && r.civ_b == civ_a)
    }).ok_or(RulesError::RelationNotFound)?;

    if state.diplomatic_relations[rel_idx].status != DiplomaticStatus::War {
        return Err(RulesError::NotAtWar);
    }

    let rel = &mut state.diplomatic_relations[rel_idx];
    rel.turns_at_war = 0;
    let score = (rel.opinion_score_a_toward_b() + rel.opinion_score_b_toward_a()) / 2;
    let new_status = status_from_score(score, &rel.active_agreements);
    rel.status = new_status;

    let (a, b) = (rel.civ_a, rel.civ_b);
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::DiplomacyChanged { civ_a: a, civ_b: b, new_status });
    Ok(diff)
}

/// Form an alliance between two civilizations.
pub(crate) fn form_alliance(
    state: &mut GameState,
    civ_a: CivId,
    civ_b: CivId,
    alliance_type: AllianceType,
) -> Result<GameStateDiff, RulesError> {
    if civ_a == civ_b { return Err(RulesError::SameCivilization); }
    if state.civ(civ_a).is_none() { return Err(RulesError::CivNotFound); }
    if state.civ(civ_b).is_none() { return Err(RulesError::CivNotFound); }

    let rel_idx = find_or_create_relation(state, civ_a, civ_b);
    let rel = &state.diplomatic_relations[rel_idx];

    if rel.status == DiplomaticStatus::War {
        return Err(RulesError::CannotAllyAtWar);
    }
    if rel.status != DiplomaticStatus::Friendly && rel.status != DiplomaticStatus::Neutral {
        return Err(RulesError::InvalidAllianceStatus);
    }

    let rel = &mut state.diplomatic_relations[rel_idx];
    rel.status = DiplomaticStatus::Alliance;
    rel.alliance_type = Some(alliance_type);
    rel.alliance_level = 1;
    rel.alliance_turns = 0;

    let (a, b) = (rel.civ_a, rel.civ_b);
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::AllianceFormed { civ_a: a, civ_b: b, alliance_type });
    diff.push(StateDelta::DiplomacyChanged { civ_a: a, civ_b: b, new_status: DiplomaticStatus::Alliance });
    Ok(diff)
}

/// Validate and assign a policy to the civilization's active slots.
pub(crate) fn assign_policy(
    state: &mut GameState,
    civ_id: CivId,
    policy_id: PolicyId,
) -> Result<GameStateDiff, RulesError> {
    let civ_idx = state.civilizations.iter().position(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;

    let policy = state.policies.iter().find(|p| p.id == policy_id)
        .cloned()
        .ok_or(RulesError::PolicyNotFound)?;

    let civ = &state.civilizations[civ_idx];

    if !civ.unlocked_policies.contains(&policy.name) {
        return Err(RulesError::PolicyNotUnlocked);
    }

    let gov_id = civ.current_government.ok_or(RulesError::NoGovernment)?;
    let gov = state.governments.iter().find(|g| g.id == gov_id)
        .cloned()
        .ok_or(RulesError::NoGovernment)?;

    let active = civ.active_policies.clone();
    let (used_mil, used_eco, used_dip, used_wc) = active.iter().fold(
        (0u8, 0u8, 0u8, 0u8),
        |(m, e, d, w), pid| {
            match state.policies.iter().find(|p| p.id == *pid).map(|p| p.policy_type) {
                Some(PolicyType::Military)   => (m + 1, e, d, w),
                Some(PolicyType::Economic)   => (m, e + 1, d, w),
                Some(PolicyType::Diplomatic) => (m, e, d + 1, w),
                Some(PolicyType::Wildcard)   => (m, e, d, w + 1),
                None => (m, e, d, w),
            }
        },
    );

    let has_slot = match policy.policy_type {
        PolicyType::Military   => used_mil  < gov.slots.military,
        PolicyType::Economic   => used_eco  < gov.slots.economic,
        PolicyType::Diplomatic => used_dip  < gov.slots.diplomatic,
        PolicyType::Wildcard   => used_wc   < gov.slots.wildcard,
    };
    if !has_slot {
        return Err(RulesError::InsufficientPolicySlots);
    }

    let civ = &state.civilizations[civ_idx];
    if civ.gold < policy.maintenance as i32 {
        return Err(RulesError::InsufficientGold);
    }

    state.civilizations[civ_idx].active_policies.push(policy_id);
    state.civilizations[civ_idx].gold -= policy.maintenance as i32;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::PolicyAssigned { civ: civ_id, policy: policy_id });
    Ok(diff)
}
