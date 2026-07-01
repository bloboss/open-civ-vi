//! Espionage / intrigue — a minimal covert-operations system.
//!
//! Mirrors the autonomous barbarian phase pattern
//! ([`super::barbarians::process_barbarian_turn`]): a [`GameState`]-level
//! registry of in-flight operations is ticked once per turn by a dedicated
//! phase in `turn_phase.rs`, and completed operations resolve deterministically
//! and surface their outcomes through the events engine
//! ([`super::events`]) via the [`StateDelta::EspionageResolved`] delta.
//!
//! ## Missions
//! * [`MissionKind::GatherIntel`] — information only (always low-risk).
//! * [`MissionKind::StealTech`] — transfers research progress from the target
//!   city's owner's current research to the operative's civ.
//! * [`MissionKind::SabotageProduction`] — halves the target city's stored
//!   production.
//! * [`MissionKind::FomentUnrest`] — raises the target city's domestic
//!   [`unrest`](crate::civ::city::City::unrest) (ties into the politics layer).
//! * [`MissionKind::CounterSpy`] — defensive; while active it lowers the
//!   success chance of hostile operations run against its owner's cities.
//!
//! ## Determinism
//! Success is a single draw from the existing seeded RNG
//! (`state.id_gen.next_f32()`), exactly as the disaster system rolls, so a
//! given seed always produces the same outcome. No new RNG is introduced.

use crate::game::diff::{GameStateDiff, StateDelta};
use crate::game::state::GameState;
use crate::{CityId, CivId, EspionageOpId};

/// Number of turns an operation spends in transit before it resolves.
pub const ESPIONAGE_MISSION_TURNS: u32 = 3;

/// Unrest added to the target city when a `FomentUnrest` mission succeeds.
pub const FOMENT_UNREST_AMOUNT: i32 = 20;

/// Success-chance reduction applied per active enemy `CounterSpy` operation
/// defending the targeted civ.
pub const COUNTERSPY_REDUCTION: f32 = 0.25;

/// The kind of covert operation an [`EspionageOp`] carries out on resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MissionKind {
    /// Information only; no state change on success.
    GatherIntel,
    /// Steal research progress from the target city's owner.
    StealTech,
    /// Cut the target city's stored production.
    SabotageProduction,
    /// Raise the target city's domestic unrest.
    FomentUnrest,
    /// Defensive: while active, lowers the success chance of hostile ops run
    /// against the owning civ's cities.
    CounterSpy,
}

/// An in-flight covert operation. Registered on [`GameState::espionage_ops`] by
/// [`dispatch_espionage`] and ticked down each turn by the espionage phase.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EspionageOp {
    pub id: EspionageOpId,
    /// The civ that launched the operation.
    pub owner: CivId,
    /// The foreign city the operation targets.
    pub target_city: CityId,
    pub mission: MissionKind,
    /// Turns left before the operation resolves. Decremented each turn.
    pub turns_remaining: u32,
}

/// Base success chance for a mission before counter-espionage is applied.
pub fn mission_base_success(mission: MissionKind) -> f32 {
    match mission {
        MissionKind::GatherIntel => 0.90,
        MissionKind::StealTech => 0.50,
        MissionKind::SabotageProduction => 0.60,
        MissionKind::FomentUnrest => 0.60,
        // Defensive operations always "resolve"; their value is the passive
        // protection they grant while in flight.
        MissionKind::CounterSpy => 1.0,
    }
}

/// Launch a covert operation and register it on `state`. Returns the new op's
/// id, or `None` if the target is invalid (the city does not exist or is owned
/// by `owner` — espionage requires a *foreign* target).
pub fn dispatch_espionage(
    state: &mut GameState,
    owner: CivId,
    target_city: CityId,
    mission: MissionKind,
) -> Option<EspionageOpId> {
    // Validate a foreign target: the city must exist and belong to someone else.
    let target_owner = state.city(target_city)?.owner;
    if target_owner == owner {
        return None;
    }

    let id = state.id_gen.next_espionage_op_id();
    state.espionage_ops.push(EspionageOp {
        id,
        owner,
        target_city,
        mission,
        turns_remaining: ESPIONAGE_MISSION_TURNS,
    });
    Some(id)
}

// ── Turn Phase: espionage resolution ─────────────────────────────────────────

/// Tick every in-flight operation down by one turn; resolve those that reach
/// zero. Called from `advance_turn` just before the dynamic-events phase so the
/// resulting [`StateDelta::EspionageResolved`] deltas are scanned by
/// `evaluate_events` this same turn.
pub(crate) fn process_espionage_turn(state: &mut GameState, diff: &mut GameStateDiff) {
    if state.espionage_ops.is_empty() {
        return;
    }

    // Advance all timers.
    for op in &mut state.espionage_ops {
        op.turns_remaining = op.turns_remaining.saturating_sub(1);
    }

    // Snapshot the operations that have completed this turn, preserving order
    // so the RNG draw sequence is deterministic.
    let resolved: Vec<EspionageOp> = state
        .espionage_ops
        .iter()
        .filter(|op| op.turns_remaining == 0)
        .cloned()
        .collect();
    if resolved.is_empty() {
        return;
    }

    // Remove the completed operations; only still-in-flight ops remain (these
    // include the active CounterSpy ops that provide defensive protection).
    state.espionage_ops.retain(|op| op.turns_remaining > 0);

    for op in resolved {
        // The defender is whoever currently owns the targeted city.
        let defender = state.city(op.target_city).map(|c| c.owner);

        // Success = deterministic RNG draw below the (counter-spy-adjusted)
        // mission chance.
        let mut chance = mission_base_success(op.mission);
        if let Some(def) = defender {
            let counters = state
                .espionage_ops
                .iter()
                .filter(|o| o.owner == def && matches!(o.mission, MissionKind::CounterSpy))
                .count();
            chance -= counters as f32 * COUNTERSPY_REDUCTION;
        }
        let chance = chance.clamp(0.05, 1.0);
        let roll = state.id_gen.next_f32();
        let success = roll < chance;

        if success {
            apply_mission_effect(state, &op);
        }

        diff.push(StateDelta::EspionageResolved {
            owner: op.owner,
            target_city: op.target_city,
            mission: op.mission,
            success,
        });
    }
}

/// Apply a successful operation's effect to `state`.
fn apply_mission_effect(state: &mut GameState, op: &EspionageOp) {
    match op.mission {
        MissionKind::StealTech => {
            // Steal half of the target civ's current research progress.
            let Some(defender) = state.city(op.target_city).map(|c| c.owner) else {
                return;
            };
            let stolen = state
                .civ(defender)
                .and_then(|c| c.research_queue.front())
                .map(|t| t.progress / 2)
                .unwrap_or(0);
            if stolen == 0 {
                return;
            }
            if let Some(dc) = state.civilizations.iter_mut().find(|c| c.id == defender)
                && let Some(front) = dc.research_queue.front_mut()
            {
                front.progress = front.progress.saturating_sub(stolen);
            }
            if let Some(oc) = state.civilizations.iter_mut().find(|c| c.id == op.owner)
                && let Some(front) = oc.research_queue.front_mut()
            {
                front.progress = front.progress.saturating_add(stolen);
            }
        }
        MissionKind::SabotageProduction => {
            if let Some(city) = state.cities.iter_mut().find(|c| c.id == op.target_city) {
                city.production_stored /= 2;
            }
        }
        MissionKind::FomentUnrest => {
            if let Some(city) = state.cities.iter_mut().find(|c| c.id == op.target_city) {
                city.unrest = (city.unrest + FOMENT_UNREST_AMOUNT).min(crate::civ::city::UNREST_MAX);
            }
        }
        MissionKind::GatherIntel | MissionKind::CounterSpy => {
            // Information-only; the EspionageResolved delta records the outcome.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::civilization::{BuiltinAgenda, Leader, TechProgress};
    use crate::civ::{City, Civilization};
    use crate::game::state::IdGenerator;
    use crate::{CivId, TechId};
    use libhexgrid::coord::HexCoord;
    use ulid::Ulid;

    /// Build a bare two-civ game state: `attacker` owns no city, `defender`
    /// owns one city. The RNG is reseeded to a value that forces success
    /// (first `next_f32()` == 0.3246, below every offensive mission chance).
    fn setup() -> (GameState, CivId, CivId, crate::CityId) {
        let mut state = GameState::new(1, 8, 8);

        let attacker = state.id_gen.next_civ_id();
        let defender = state.id_gen.next_civ_id();
        state.civilizations.push(Civilization::new(
            attacker,
            "Attacker",
            "Attacker",
            Leader { name: "A", civ_id: attacker, agenda: BuiltinAgenda::Default },
        ));
        state.civilizations.push(Civilization::new(
            defender,
            "Defender",
            "Defender",
            Leader { name: "D", civ_id: defender, agenda: BuiltinAgenda::Default },
        ));

        let city_id = state.id_gen.next_city_id();
        state.cities.push(City::new(
            city_id,
            "Capital".to_string(),
            defender,
            HexCoord::from_qr(0, 0),
        ));

        // Force a successful roll: 0.3246 < any offensive base chance.
        state.id_gen = IdGenerator::new(0);
        (state, attacker, defender, city_id)
    }

    fn tech_progress(progress: u32) -> TechProgress {
        TechProgress { tech_id: TechId::from_ulid(Ulid::nil()), progress, boosted: false }
    }

    #[test]
    fn dispatch_rejects_own_city() {
        let (mut state, _attacker, defender, city_id) = setup();
        // A civ cannot spy on its own city.
        assert!(dispatch_espionage(&mut state, defender, city_id, MissionKind::StealTech).is_none());
        assert!(state.espionage_ops.is_empty());
    }

    #[test]
    fn steal_tech_transfers_research_on_success() {
        let (mut state, attacker, defender, city_id) = setup();

        // Defender has 100 progress on their current research; attacker 10.
        state
            .civilizations
            .iter_mut()
            .find(|c| c.id == defender)
            .unwrap()
            .research_queue
            .push_back(tech_progress(100));
        state
            .civilizations
            .iter_mut()
            .find(|c| c.id == attacker)
            .unwrap()
            .research_queue
            .push_back(tech_progress(10));

        dispatch_espionage(&mut state, attacker, city_id, MissionKind::StealTech)
            .expect("foreign target accepted");
        assert_eq!(state.espionage_ops.len(), 1);

        // Resolve on the next tick (turns_remaining 1 -> 0).
        state.espionage_ops[0].turns_remaining = 1;
        let mut diff = GameStateDiff::new();
        process_espionage_turn(&mut state, &mut diff);

        // Op consumed.
        assert!(state.espionage_ops.is_empty());

        // Half (50) of the defender's progress moved to the attacker.
        let def_prog = state
            .civ(defender)
            .unwrap()
            .research_queue
            .front()
            .unwrap()
            .progress;
        let atk_prog = state
            .civ(attacker)
            .unwrap()
            .research_queue
            .front()
            .unwrap()
            .progress;
        assert_eq!(def_prog, 50, "defender lost half its research progress");
        assert_eq!(atk_prog, 60, "attacker gained the stolen research progress");

        // A successful EspionageResolved delta was emitted for the attacker.
        let resolved = diff.deltas.iter().find_map(|d| match d {
            StateDelta::EspionageResolved { owner, mission, success, .. } => {
                Some((*owner, *mission, *success))
            }
            _ => None,
        });
        assert_eq!(resolved, Some((attacker, MissionKind::StealTech, true)));

        // The events engine picks up the delta and fires for the attacker.
        let fired = crate::game::rules::events::evaluate_events(&diff.deltas, &state);
        assert!(
            fired
                .iter()
                .any(|(civ, def)| *civ == attacker && def.id == "espionage_resolved"),
            "espionage_resolved event fires for the operation's owner"
        );
    }

    #[test]
    fn foment_unrest_raises_target_unrest() {
        let (mut state, attacker, _defender, city_id) = setup();

        let before = state.city(city_id).unwrap().unrest;
        let _id = dispatch_espionage(&mut state, attacker, city_id, MissionKind::FomentUnrest)
            .expect("foreign target accepted");
        state.espionage_ops[0].turns_remaining = 1;

        let mut diff = GameStateDiff::new();
        process_espionage_turn(&mut state, &mut diff);

        let after = state.city(city_id).unwrap().unrest;
        assert_eq!(after, before + FOMENT_UNREST_AMOUNT, "unrest raised on success");
        assert!(state.espionage_ops.is_empty());
        assert!(diff.deltas.iter().any(|d| matches!(
            d,
            StateDelta::EspionageResolved { mission: MissionKind::FomentUnrest, success: true, .. }
        )));
    }

    #[test]
    fn op_only_resolves_when_timer_hits_zero() {
        let (mut state, attacker, _defender, city_id) = setup();
        dispatch_espionage(&mut state, attacker, city_id, MissionKind::GatherIntel)
            .expect("foreign target accepted");
        assert_eq!(state.espionage_ops[0].turns_remaining, ESPIONAGE_MISSION_TURNS);

        // One tick: still in flight, no resolution delta.
        let mut diff = GameStateDiff::new();
        process_espionage_turn(&mut state, &mut diff);
        assert_eq!(state.espionage_ops.len(), 1);
        assert_eq!(state.espionage_ops[0].turns_remaining, ESPIONAGE_MISSION_TURNS - 1);
        assert!(!diff.deltas.iter().any(|d| matches!(d, StateDelta::EspionageResolved { .. })));
    }
}
