//! WebSocket handler: upgrade, auth handshake, and message routing.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use tokio::sync::broadcast;

use open4x_api::messages::{ClientMessage, GameStatus, ServerMessage};

use crate::auth::{self, AuthChallenge};
use crate::projection::project_game_view;
use crate::state::{AppState, GameRoom, GameRoomConfig};

/// HTTP handler for WebSocket upgrade.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // ── Phase 1: Auth handshake ──────────────────────────────────────────
    let challenge = AuthChallenge::generate();
    let challenge_msg = ServerMessage::Challenge {
        nonce: challenge.nonce.to_vec(),
    };
    if send_msg(&mut socket, &challenge_msg).await.is_err() {
        return;
    }

    // Wait for Authenticate message.
    let pubkey = loop {
        let Some(Ok(msg)) = socket.recv().await else { return };
        let Message::Text(text) = msg else { continue };
        let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) else { continue };

        match client_msg {
            ClientMessage::Authenticate { pubkey, signature } => {
                match auth::verify_auth(&challenge, &pubkey, &signature) {
                    Ok(key) => {
                        // Look up or create player record.
                        let profile = state.players.entry(key)
                            .or_insert_with(|| crate::state::PlayerRecord {
                                pubkey: key,
                                display_name: format!("Player_{}", hex::encode(&key[..4])),
                                selected_template: state.templates[0].id,
                                games_played: 0,
                            });
                        let profile_view = open4x_api::profile::ProfileView {
                            pubkey: key.to_vec(),
                            display_name: profile.display_name.clone(),
                            selected_template: profile.selected_template,
                        };
                        let _ = send_msg(&mut socket, &ServerMessage::AuthSuccess {
                            session_token: hex::encode(&key),
                            profile: profile_view,
                        }).await;
                        break key;
                    }
                    Err(e) => {
                        let _ = send_msg(&mut socket, &ServerMessage::AuthFailure {
                            reason: e.to_string(),
                        }).await;
                        return;
                    }
                }
            }
            ClientMessage::Ping => {
                let _ = send_msg(&mut socket, &ServerMessage::Pong).await;
            }
            _ => {
                let _ = send_msg(&mut socket, &ServerMessage::Error {
                    message: "authenticate first".into(),
                }).await;
            }
        }
    };

    // ── Phase 2: Message loop ────────────────────────────────────────────
    let mut current_game: Option<open4x_api::ids::GameId> = None;
    let mut _rx: Option<broadcast::Receiver<ServerMessage>> = None;

    loop {
        let Some(Ok(msg)) = socket.recv().await else { break };
        let Message::Text(text) = msg else { continue };
        let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) else {
            let _ = send_msg(&mut socket, &ServerMessage::Error {
                message: "invalid message format".into(),
            }).await;
            continue;
        };

        match client_msg {
            ClientMessage::Ping => {
                let _ = send_msg(&mut socket, &ServerMessage::Pong).await;
            }
            ClientMessage::ListGames => {
                let entries: Vec<_> = state.games.iter().map(|entry| {
                    let room = entry.value();
                    open4x_api::messages::GameListEntry {
                        game_id: room.game_id,
                        name: room.name.clone(),
                        players_joined: room.players.len() as u32,
                        max_players: room.config.max_players,
                        turn: room.state.turn,
                        status: room.status,
                    }
                }).collect();
                let _ = send_msg(&mut socket, &ServerMessage::GamesList(entries)).await;
            }
            ClientMessage::CreateGame(req) => {
                let game_id = open4x_api::ids::GameId::from_ulid(
                    ulid::Ulid::new()
                );
                let (tx, rx) = broadcast::channel(64);
                _rx = Some(rx);

                // Build game state using the session builder pattern from civsim.
                let session = crate::session::build_server_session(&req, &pubkey, &state, game_id);

                let room = GameRoom {
                    game_id,
                    name: req.name.clone(),
                    state: session.state,
                    rules: libciv::DefaultRulesEngine,
                    players: session.players,
                    ai_agents: session.ai_agents,
                    status: GameStatus::InProgress,
                    config: GameRoomConfig {
                        max_players: req.max_players,
                        turn_limit: req.turn_limit,
                    },
                    tx,
                };

                state.games.insert(game_id, room);
                current_game = Some(game_id);

                let _ = send_msg(&mut socket, &ServerMessage::GameCreated { game_id }).await;

                // Send initial game view.
                if let Some(room) = state.games.get(&game_id)
                    && let Some(slot) = room.players.iter().find(|s| s.pubkey == pubkey)
                {
                    let view = project_game_view(&room.state, slot.civ_id);
                    let _ = send_msg(&mut socket, &ServerMessage::GameJoined {
                        game_id, view,
                    }).await;
                }
            }
            ClientMessage::JoinGame { game_id } => {
                // TODO: full join logic with player slot allocation
                current_game = Some(game_id);
                if let Some(room) = state.games.get(&game_id) {
                    _rx = Some(room.tx.subscribe());
                }
                if let Some(room) = state.games.get(&game_id)
                    && let Some(slot) = room.players.iter().find(|s| s.pubkey == pubkey)
                {
                    let view = project_game_view(&room.state, slot.civ_id);
                    let _ = send_msg(&mut socket, &ServerMessage::GameJoined {
                        game_id, view,
                    }).await;
                }
            }
            ClientMessage::Action(action) => {
                let Some(game_id) = current_game else {
                    let _ = send_msg(&mut socket, &ServerMessage::Error {
                        message: "not in a game".into(),
                    }).await;
                    continue;
                };
                let result = if let Some(mut room) = state.games.get_mut(&game_id) {
                    let civ_id = room.players.iter()
                        .find(|s| s.pubkey == pubkey)
                        .map(|s| s.civ_id);
                    if let Some(civ_id) = civ_id {
                        room.apply_action(civ_id, &action)
                    } else {
                        Err("not a player in this game".into())
                    }
                } else {
                    Err("game not found".into())
                };

                match result {
                    Ok(()) => {
                        let _ = send_msg(&mut socket, &ServerMessage::ActionResult {
                            ok: true, error: None,
                        }).await;
                        // Send updated view.
                        if let Some(room) = state.games.get(&game_id)
                            && let Some(slot) = room.players.iter().find(|s| s.pubkey == pubkey)
                        {
                            let view = project_game_view(&room.state, slot.civ_id);
                            let _ = send_msg(&mut socket, &ServerMessage::GameUpdate(view)).await;
                        }
                    }
                    Err(e) => {
                        let _ = send_msg(&mut socket, &ServerMessage::ActionResult {
                            ok: false, error: Some(e),
                        }).await;
                    }
                }
            }
            ClientMessage::EndTurn => {
                let Some(game_id) = current_game else { continue };
                let should_resolve = if let Some(mut room) = state.games.get_mut(&game_id) {
                    if let Some(slot) = room.players.iter_mut().find(|s| s.pubkey == pubkey) {
                        slot.submitted_turn = true;
                    }
                    room.all_submitted()
                } else {
                    false
                };

                if should_resolve {
                    if let Some(mut room) = state.games.get_mut(&game_id) {
                        room.resolve_turn();
                        let new_turn = room.state.turn;

                        // Build per-player views and broadcast.
                        let mut snapshots = Vec::new();
                        for slot in &room.players {
                            let view = project_game_view(&room.state, slot.civ_id);
                            let civ_api_id = open4x_api::ids::CivId::from_ulid(slot.civ_id.as_ulid());
                            snapshots.push((civ_api_id, view.clone()));
                            let _ = room.tx.send(ServerMessage::TurnResolved {
                                new_turn, view,
                            });
                        }

                        // Persist game state to disk.
                        crate::persist::save_game_snapshot(game_id, new_turn, &snapshots);
                    }
                } else {
                    // Notify others that this player ended their turn.
                    if let Some(room) = state.games.get(&game_id)
                        && let Some(slot) = room.players.iter().find(|s| s.pubkey == pubkey)
                    {
                        let civ_id = open4x_api::ids::CivId::from_ulid(slot.civ_id.as_ulid());
                        let _ = room.tx.send(ServerMessage::PlayerEndedTurn { civ_id });
                    }
                }
            }
            ClientMessage::SetProfile(update) => {
                if let Some(mut record) = state.players.get_mut(&pubkey) {
                    record.display_name = update.display_name.clone();
                    record.selected_template = update.selected_template;
                }
                let profile = open4x_api::profile::ProfileView {
                    pubkey: pubkey.to_vec(),
                    display_name: update.display_name,
                    selected_template: update.selected_template,
                };
                let _ = send_msg(&mut socket, &ServerMessage::ProfileUpdated(profile)).await;
            }
            ClientMessage::Authenticate { .. } => {
                let _ = send_msg(&mut socket, &ServerMessage::Error {
                    message: "already authenticated".into(),
                }).await;
            }
        }
    }
}

async fn send_msg(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), axum::Error> {
    let json = serde_json::to_string(msg).expect("serialize server message");
    socket.send(Message::Text(json.into())).await
}

/// Encode bytes as hex string.
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
