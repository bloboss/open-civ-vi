mod common;

use common::{build_scenario, advance_turn};

#[test]
fn congress_session_fires_at_interval() {
    let mut s = build_scenario();
    // Default interval is 30; next_session_turn starts at 30.
    assert_eq!(s.state.world_congress.next_session_turn, 30);

    // The congress check uses `state.turn >= next_session_turn`. The turn
    // counter is incremented at the END of advance_turn, so on the 31st call
    // state.turn is 30 when the check runs (the 30th call incremented it to 30).
    for _ in 0..31 {
        advance_turn(&mut s);
    }

    // After 31 advance_turn calls, state.turn == 31 and the session should
    // have fired during the turn where state.turn was 30.
    assert_eq!(
        s.state.world_congress.next_session_turn, 60,
        "next_session_turn should advance by interval (30 -> 60)"
    );
}

#[test]
fn winner_gets_diplomatic_vp() {
    let mut s = build_scenario();
    // Give Rome more diplomatic favor than Babylon.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .diplomatic_favor = 100;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .diplomatic_favor = 10;

    // Set session to fire on next turn.
    s.state.world_congress.next_session_turn = s.state.turn;

    advance_turn(&mut s);

    // Rome should have 1 VP.
    let rome_vp = s.state.world_congress.diplomatic_victory_points
        .get(&s.rome_id).copied().unwrap_or(0);
    assert_eq!(rome_vp, 1, "Rome (highest favor) should earn 1 diplomatic VP");

    // Babylon should have 0 VP.
    let babylon_vp = s.state.world_congress.diplomatic_victory_points
        .get(&s.babylon_id).copied().unwrap_or(0);
    assert_eq!(babylon_vp, 0, "Babylon should have 0 diplomatic VP");
}

#[test]
fn congress_schedules_next_session() {
    let mut s = build_scenario();
    // Set a custom interval and trigger point.
    s.state.world_congress.session_interval = 10;
    s.state.world_congress.next_session_turn = 5;

    // Advance past turn 5.
    for _ in 0..6 {
        advance_turn(&mut s);
    }

    // next_session_turn should be 5 + 10 = 15.
    assert_eq!(
        s.state.world_congress.next_session_turn, 15,
        "next_session_turn should be original (5) + interval (10) = 15"
    );
}
