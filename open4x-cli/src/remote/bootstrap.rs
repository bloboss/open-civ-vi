//! `new-game` over REST: maps the CLI flags to `POST /games/new` and
//! writes the bearer token + game/civ IDs to the session file so
//! subsequent subcommands can authenticate.
//!
//! The server bootstrap accepts only one human player + N AI; if the
//! caller passed multiple `--player` entries the CLI errors out
//! before hitting the network. See `book/src/roadmap/cli-server-mode.md`
//! for the full parity matrix.

use std::path::Path;

use serde_json::json;

use super::client::ApiClient;
use super::session::Session;

pub fn new_game(
    server: &str,
    token_file: &Path,
    seed: u64,
    width: u32,
    height: u32,
    players: &[String],
    ai: &[String],
) -> Result<(), String> {
    if players.len() > 1 {
        return Err(format!(
            "server mode supports only one human player; got {} (--player {:?})",
            players.len(),
            players,
        ));
    }
    let display_name = players
        .first()
        .cloned()
        .unwrap_or_else(|| "Player".to_string());

    let body = json!({
        "display_name": display_name,
        "width":  width,
        "height": height,
        "seed":   seed,
        "num_ai": ai.len() as u32,
    });

    let client = ApiClient::new(server);
    let resp = client.post_json("/api/v1/games/new", &body)?;

    let game_id = resp
        .get("game_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("response missing game_id: {resp}"))?
        .to_string();
    let civ_id = resp
        .get("civ_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("response missing civ_id: {resp}"))?
        .to_string();
    let token = resp
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("response missing token: {resp}"))?
        .to_string();
    let turn = resp.get("turn").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let session = Session {
        server: server.to_string(),
        game_id,
        civ_id,
        token,
        turn,
    };
    session.save(token_file)?;

    let out = json!({
        "success": true,
        "mode":    "remote",
        "server":  server,
        "session_file": token_file.display().to_string(),
        "game_id": session.game_id,
        "civ_id":  session.civ_id,
        "turn":    session.turn,
        "players": players,
        "ai":      ai,
        "map_size": [width, height],
        "seed":     seed,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}
