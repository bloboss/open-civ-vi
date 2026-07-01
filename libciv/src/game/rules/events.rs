//! Dynamic events engine.
//!
//! A reusable, data-driven generalization of the historic-moment observer
//! (`crate::civ::historic_moments::observe_deltas`). Where that observer only
//! knows about era score, this registry lets any subsystem — politics,
//! espionage, disasters, diplomacy — declare an [`EventDef`] that fires when
//! either (a) a matching [`StateDelta`] appears in the turn's batch, or (b) a
//! game-state predicate (a [`Condition`]) holds. A fired event enqueues its
//! [`OneShotEffect`]s and surfaces a player-facing notification.
//!
//! # Purity & termination
//! [`evaluate_events`] is a **pure** scan (like `observe_deltas`): it never
//! mutates state and never enqueues effects. The caller (a turn phase in
//! `turn_phase.rs`) performs the mutation. Crucially, evaluation runs over the
//! *pre-events* delta snapshot and the caller does **not** feed the resulting
//! `EventFired` deltas back into a second evaluation this turn, so the phase
//! always terminates with a bounded event count (at most one per
//! `(civ, EventDef)` pair).

use crate::CivId;
use crate::game::diff::StateDelta;
use crate::game::state::GameState;
use crate::rules::effect::OneShotEffect;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Notification metadata
// ---------------------------------------------------------------------------

/// Tone/severity of an event, mirroring the server-side `NotificationKind`
/// (`open4x_server::server::state::NotificationKind`). Kept libciv-side so the
/// engine is self-describing; the server maps 1:1 via [`EventKind::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Accent,
    Good,
    Warn,
    Bad,
    Neutral,
}

impl EventKind {
    /// Lower-case wire tag, matching `NotificationKind::as_wire` on the server.
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Accent => "accent",
            EventKind::Good => "good",
            EventKind::Warn => "warn",
            EventKind::Bad => "bad",
            EventKind::Neutral => "neutral",
        }
    }
}

/// Player-facing metadata attached to an event.
#[derive(Debug, Clone)]
pub struct EventNotif {
    pub kind: EventKind,
    pub category: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
}

// ---------------------------------------------------------------------------
// Triggers
// ---------------------------------------------------------------------------

/// A lightweight tag matched against a single [`StateDelta`] in the turn's
/// batch. `resolve_delta_owner` maps a matching delta to the owning [`CivId`].
///
/// This is a tag rather than a full delta so `EventDef`s stay cheap and
/// `Copy`; add a variant here (plus an arm in `resolve_delta_owner`) when a
/// new subsystem wants to fire on a new delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaPattern {
    EurekaTriggered,
    InspirationTriggered,
    WonderBuilt,
    NaturalWonderDiscovered,
    CityRevolted,
    EspionageResolved,
}

/// A predicate evaluated against the whole [`GameState`] (threshold-style).
/// Returns every civ for which the condition currently holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// Any city whose loyalty is strictly below this value fires for its owner.
    CityLoyaltyBelow(i32),
    /// Any city whose domestic unrest is at or above this value fires for its
    /// owner. Drives the domestic-crisis events (riot/strike).
    CityUnrestAbove(i32),
}

/// How an event fires: either a matching delta appeared this turn, or a
/// game-state predicate holds right now.
#[derive(Debug, Clone)]
pub enum EventTrigger {
    /// Fires when a [`StateDelta`] matching the pattern is present in the batch.
    OnDelta(DeltaPattern),
    /// Fires when a [`Condition`] holds for a civ in the current state.
    OnCondition(Condition),
}

// ---------------------------------------------------------------------------
// Event definitions
// ---------------------------------------------------------------------------

/// A dynamic event: a trigger, the effects it enqueues, and the notification
/// it surfaces. Registered in [`builtin_event_defs`].
#[derive(Debug, Clone)]
pub struct EventDef {
    /// Stable identifier, referenced by `StateDelta::EventFired` and resolved
    /// back to this def (for notification text) via [`event_def`].
    pub id: &'static str,
    pub trigger: EventTrigger,
    /// Effects enqueued onto `GameState::effect_queue` when the event fires.
    /// They drain in the next turn's effect-drain phase.
    pub effects: Vec<OneShotEffect>,
    pub notif: EventNotif,
}

/// The builtin event registry. Lazily built once (needs owned `Vec`s of
/// effects, which cannot live in a `const`).
static BUILTIN_EVENTS: LazyLock<Vec<EventDef>> = LazyLock::new(|| {
    vec![
        // A Eureka insight ripples into a broader research boon.
        EventDef {
            id: "scientific_breakthrough",
            trigger: EventTrigger::OnDelta(DeltaPattern::EurekaTriggered),
            effects: vec![OneShotEffect::UnlockPolicy("Rationalism")],
            notif: EventNotif {
                kind: EventKind::Good,
                category: "research",
                title: "Scientific Breakthrough",
                desc: "A sudden insight accelerates your researchers.",
            },
        },
        // A city's loyalty is collapsing; warn the player before it revolts.
        EventDef {
            id: "city_in_turmoil",
            trigger: EventTrigger::OnCondition(Condition::CityLoyaltyBelow(30)),
            effects: Vec::new(),
            notif: EventNotif {
                kind: EventKind::Warn,
                category: "loyalty",
                title: "City in Turmoil",
                desc: "Loyalty is collapsing; the city may soon revolt.",
            },
        },
        // Domestic unrest has boiled over into open rioting. The mechanical
        // bite is delivered by the hardened yield pipeline
        // (`city_unrest_modifiers`); this event surfaces the player-facing
        // crisis. Effects are intentionally empty (mirroring `city_in_turmoil`)
        // so nothing is re-enqueued every turn the condition persists.
        EventDef {
            id: "riot",
            trigger: EventTrigger::OnCondition(Condition::CityUnrestAbove(
                crate::civ::city::UNREST_RIOT_TIER,
            )),
            effects: Vec::new(),
            notif: EventNotif {
                kind: EventKind::Warn,
                category: "unrest",
                title: "Riots Break Out",
                desc: "Discontent has spilled into the streets; your cities riot.",
            },
        },
        // Unrest has escalated to a general strike — the most severe tier.
        EventDef {
            id: "strike",
            trigger: EventTrigger::OnCondition(Condition::CityUnrestAbove(
                crate::civ::city::UNREST_REVOLT_TIER,
            )),
            effects: Vec::new(),
            notif: EventNotif {
                kind: EventKind::Bad,
                category: "unrest",
                title: "General Strike",
                desc: "A general strike grips your empire; production grinds to a halt.",
            },
        },
        // A covert operation this civ launched has concluded (success or
        // failure). Fires for the operation's owner so the player learns the
        // outcome; the mechanical effect was already applied by the espionage
        // phase.
        EventDef {
            id: "espionage_resolved",
            trigger: EventTrigger::OnDelta(DeltaPattern::EspionageResolved),
            effects: Vec::new(),
            notif: EventNotif {
                kind: EventKind::Accent,
                category: "espionage",
                title: "Covert Operation Concluded",
                desc: "One of your covert operations has run its course.",
            },
        },
    ]
});

/// All registered event definitions.
pub fn builtin_event_defs() -> &'static [EventDef] {
    &BUILTIN_EVENTS
}

/// Look up an event definition by its `id`. Used by the server to resolve a
/// `StateDelta::EventFired { event_id, .. }` back to notification text.
pub fn event_def(id: &str) -> Option<&'static EventDef> {
    BUILTIN_EVENTS.iter().find(|d| d.id == id)
}

// ---------------------------------------------------------------------------
// Evaluation (pure)
// ---------------------------------------------------------------------------

/// Resolve the owning [`CivId`] for a delta matching `pattern`.
fn resolve_delta_owner(pattern: DeltaPattern, delta: &StateDelta) -> Option<CivId> {
    match (pattern, delta) {
        (DeltaPattern::EurekaTriggered, StateDelta::EurekaTriggered { civ, .. }) => Some(*civ),
        (DeltaPattern::InspirationTriggered, StateDelta::InspirationTriggered { civ, .. }) => {
            Some(*civ)
        }
        (DeltaPattern::WonderBuilt, StateDelta::WonderBuilt { civ, .. }) => Some(*civ),
        (
            DeltaPattern::NaturalWonderDiscovered,
            StateDelta::NaturalWonderDiscovered { civ, .. },
        ) => Some(*civ),
        (DeltaPattern::CityRevolted, StateDelta::CityRevolted { old_owner, .. }) => {
            Some(*old_owner)
        }
        (DeltaPattern::EspionageResolved, StateDelta::EspionageResolved { owner, .. }) => {
            Some(*owner)
        }
        _ => None,
    }
}

/// Return every civ for which `cond` currently holds.
fn condition_matches(cond: Condition, state: &GameState) -> Vec<CivId> {
    match cond {
        Condition::CityLoyaltyBelow(threshold) => state
            .cities
            .iter()
            .filter(|c| c.loyalty < threshold)
            .map(|c| c.owner)
            .collect(),
        Condition::CityUnrestAbove(threshold) => state
            .cities
            .iter()
            .filter(|c| c.unrest >= threshold)
            .map(|c| c.owner)
            .collect(),
    }
}

/// Scan the turn's `deltas` plus the current `state` and return the events
/// that fire, as `(CivId, &EventDef)` pairs — deduplicated per `(civ, id)`.
///
/// PURE: no mutation, no effect enqueuing. Mirrors `observe_deltas`. The
/// returned references borrow the `'static` builtin registry, so they are
/// independent of the `state`/`deltas` borrows and the caller may mutate
/// `state` immediately after.
pub fn evaluate_events(
    deltas: &[StateDelta],
    state: &GameState,
) -> Vec<(CivId, &'static EventDef)> {
    let mut fired: Vec<(CivId, &'static EventDef)> = Vec::new();

    let mut push_unique = |civ: CivId, def: &'static EventDef| {
        if !fired.iter().any(|(c, d)| *c == civ && d.id == def.id) {
            fired.push((civ, def));
        }
    };

    for def in builtin_event_defs() {
        match &def.trigger {
            EventTrigger::OnDelta(pattern) => {
                for delta in deltas {
                    if let Some(civ) = resolve_delta_owner(*pattern, delta) {
                        push_unique(civ, def);
                    }
                }
            }
            EventTrigger::OnCondition(cond) => {
                for civ in condition_matches(*cond, state) {
                    push_unique(civ, def);
                }
            }
        }
    }

    fired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_round_trips_builtins() {
        for def in builtin_event_defs() {
            assert_eq!(event_def(def.id).map(|d| d.id), Some(def.id));
        }
        assert!(event_def("no_such_event").is_none());
    }

    #[test]
    fn event_kind_wire_tags_match_server() {
        assert_eq!(EventKind::Good.as_str(), "good");
        assert_eq!(EventKind::Warn.as_str(), "warn");
        assert_eq!(EventKind::Bad.as_str(), "bad");
        assert_eq!(EventKind::Accent.as_str(), "accent");
    }
}
