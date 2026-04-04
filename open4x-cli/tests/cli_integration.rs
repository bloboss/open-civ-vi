//! Integration tests for the non-REPL CLI.
//!
//! These tests invoke the `open4x` binary as a subprocess and verify JSON output.

use std::path::PathBuf;
use std::process::Command;

fn open4x() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open4x"))
}

fn temp_game_file() -> PathBuf {
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("test_game_{id}_{ts}.json"))
}

// ── 5A: Single-player game cycle ────────────────────────────────────────────

#[test]
fn new_game_creates_valid_file() {
    let path = temp_game_file();
    let output = open4x()
        .args(["new-game", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "--ai", "Babylon",
               "--width", "20", "--height", "12"])
        .output()
        .expect("failed to run open4x");

    assert!(output.status.success(), "new-game failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(path.exists(), "game file should exist");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"success\": true"), "should report success");
    assert!(stdout.contains("\"turn\": 0"), "should be turn 0");

    std::fs::remove_file(&path).ok();
}

#[test]
fn list_units_returns_json_array() {
    let path = temp_game_file();
    open4x()
        .args(["new-game", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "--width", "20", "--height", "12"])
        .output().unwrap();

    let output = open4x()
        .args(["list", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "units"])
        .output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with('['), "should be a JSON array: {stdout}");
    assert!(stdout.contains("Warrior"), "should list a Warrior unit");

    std::fs::remove_file(&path).ok();
}

#[test]
fn end_turn_advances_game() {
    let path = temp_game_file();
    open4x()
        .args(["new-game", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "--width", "20", "--height", "12"])
        .output().unwrap();

    let output = open4x()
        .args(["end-turn", "--game-file", path.to_str().unwrap(),
               "--player", "Rome"])
        .output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"turn\": 1"), "should advance to turn 1");

    std::fs::remove_file(&path).ok();
}

#[test]
fn view_returns_player_view() {
    let path = temp_game_file();
    open4x()
        .args(["new-game", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "--width", "20", "--height", "12"])
        .output().unwrap();

    let output = open4x()
        .args(["view", "--game-file", path.to_str().unwrap(),
               "--player", "Rome"])
        .output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"civ_name\": \"Rome\""), "should show Rome's view");
    assert!(stdout.contains("\"visible_tiles\""), "should have visible tiles");

    std::fs::remove_file(&path).ok();
}

// ── 5B: Multiplayer turn flow ───────────────────────────────────────────────

#[test]
fn multiplayer_both_must_end_turn() {
    let path = temp_game_file();
    open4x()
        .args(["new-game", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "--player", "Babylon",
               "--width", "20", "--height", "12"])
        .output().unwrap();

    // Rome ends turn — should NOT advance (Babylon hasn't ended)
    let output1 = open4x()
        .args(["end-turn", "--game-file", path.to_str().unwrap(),
               "--player", "Rome"])
        .output().unwrap();
    assert!(output1.status.success());
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    assert!(stdout1.contains("\"turn\": 0"), "turn should still be 0 (Babylon hasn't ended)");

    // Babylon ends turn — now it should advance
    let output2 = open4x()
        .args(["end-turn", "--game-file", path.to_str().unwrap(),
               "--player", "Babylon"])
        .output().unwrap();
    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("\"turn\": 1"), "turn should advance to 1 after both end");

    std::fs::remove_file(&path).ok();
}

// ── 5C: Status queries ──────────────────────────────────────────────────────

#[test]
fn status_scores_returns_json() {
    let path = temp_game_file();
    open4x()
        .args(["new-game", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "--width", "20", "--height", "12"])
        .output().unwrap();

    let output = open4x()
        .args(["status", "--game-file", path.to_str().unwrap(),
               "--player", "Rome", "scores"])
        .output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rome"), "scores should mention Rome");

    std::fs::remove_file(&path).ok();
}
