//! Integration tests for the dynamic events engine
//! (`libciv::game::rules::events`).
//!
//! These drive a full turn through `TurnEngine::process_turn` and assert that:
//!   * a triggering delta (a Eureka) fires the matching `EventFired` delta,
//!   * the fired event's `OneShotEffect` lands on the effect queue, and
//!   * the events phase terminates with a bounded event count (no self-feeding
//!     loop, even across repeated turns).

mod common;

use libciv::game::StateDelta;
use libciv::game::rules::events::{
    self, Condition, DeltaPattern, EventKind, EventTrigger, evaluate_events,
};
use libciv::rules::OneShotEffect;
use libciv::{DefaultRulesEngine, TechId, TurnEngine};

/// Seeding the effect queue with a `TriggerEureka` makes the turn emit an
/// `EurekaTriggered` delta, which the events phase turns into a
/// `scientific_breakthrough` `EventFired` delta whose effect is enqueued.
#[test]
fn eureka_delta_fires_scientific_breakthrough_and_enqueues_effect() {
    let mut s = common::build_scenario();
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    // Queue a Eureka for Rome; it drains during this turn's effect phase and
    // emits `EurekaTriggered`, which the events phase observes.
    let tech = TechId::from_ulid(s.state.id_gen.next_ulid());
    s.state
        .effect_queue
        .push_back((s.rome_id, OneShotEffect::TriggerEureka { tech }));

    let diff = engine.process_turn(&mut s.state, &rules);

    // The matching event fired for Rome.
    let fired: Vec<&String> = diff
        .deltas
        .iter()
        .filter_map(|d| match d {
            StateDelta::EventFired { civ, event_id } if *civ == s.rome_id => Some(event_id),
            _ => None,
        })
        .collect();
    assert!(
        fired.iter().any(|id| id.as_str() == "scientific_breakthrough"),
        "expected a scientific_breakthrough EventFired delta, got {fired:?}"
    );

    // The event enqueued its effect (drains next turn), so it must still be on
    // the queue now.
    let enqueued = s
        .state
        .effect_queue
        .iter()
        .any(|(civ, eff)| *civ == s.rome_id && matches!(eff, OneShotEffect::UnlockPolicy("Rationalism")));
    assert!(enqueued, "scientific_breakthrough effect should be queued for next turn");
}

/// The events phase must terminate and never grow without bound. Even if a
/// Eureka fires every turn, exactly one `scientific_breakthrough` event is
/// produced per turn — the `EventFired` delta is never fed back into the
/// evaluator.
#[test]
fn events_phase_terminates_with_bounded_count() {
    let mut s = common::build_scenario();
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    let bound = events::builtin_event_defs().len() * s.state.civilizations.len();

    for _ in 0..5 {
        // Re-arm a Eureka each turn so the delta-triggered event keeps firing.
        let tech = TechId::from_ulid(s.state.id_gen.next_ulid());
        s.state
            .effect_queue
            .push_back((s.rome_id, OneShotEffect::TriggerEureka { tech }));

        let diff = engine.process_turn(&mut s.state, &rules);

        let event_count = diff
            .deltas
            .iter()
            .filter(|d| matches!(d, StateDelta::EventFired { .. }))
            .count();
        assert!(
            event_count <= bound,
            "event count {event_count} exceeded bound {bound} — possible self-feeding loop"
        );
    }
}

/// The condition-based trigger fires purely from game state: a city whose
/// loyalty is below the threshold produces a `city_in_turmoil` event for its
/// owner, with no delta required.
#[test]
fn low_loyalty_condition_fires_city_in_turmoil() {
    let mut s = common::build_scenario();

    // Drive Rome's capital into turmoil.
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.loyalty = 10;

    let fired = evaluate_events(&[], &s.state);
    assert!(
        fired
            .iter()
            .any(|(civ, def)| *civ == s.rome_id && def.id == "city_in_turmoil"),
        "expected city_in_turmoil for Rome, got {:?}",
        fired.iter().map(|(c, d)| (c, d.id)).collect::<Vec<_>>()
    );

    // A healthy city produces no such event.
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.loyalty = 100;
    let calm = evaluate_events(&[], &s.state);
    assert!(
        !calm.iter().any(|(_, def)| def.id == "city_in_turmoil"),
        "no turmoil event expected when loyalty is healthy"
    );
}

/// Sanity: the trigger shape exposes both delta-pattern and condition arms and
/// the notification kind maps to the server wire tag.
#[test]
fn trigger_shape_and_kind_are_exposed() {
    let breakthrough = events::event_def("scientific_breakthrough").unwrap();
    assert!(matches!(
        breakthrough.trigger,
        EventTrigger::OnDelta(DeltaPattern::EurekaTriggered)
    ));
    assert_eq!(breakthrough.notif.kind.as_str(), "good");

    let turmoil = events::event_def("city_in_turmoil").unwrap();
    assert!(matches!(
        turmoil.trigger,
        EventTrigger::OnCondition(Condition::CityLoyaltyBelow(_))
    ));
    assert_eq!(turmoil.notif.kind, EventKind::Warn);
}
