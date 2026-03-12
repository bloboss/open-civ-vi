pub mod game;

use leptos::prelude::*;

// ---------------------------------------------------------------------------
// Home page
// ---------------------------------------------------------------------------

#[component]
pub fn HomePage(
    on_new_game: impl Fn() + 'static,
    on_settings: impl Fn() + 'static,
    on_players:  impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <div class="page-center">
            <div class="card">
                <div class="title">"Open 4X"</div>
                <div class="subtitle">"An open-source Civilization-style engine"</div>

                <button class="btn btn-primary" on:click=move |_| on_new_game()>
                    "New Game"
                </button>
                <button class="btn btn-ghost" on:click=move |_| on_players()>
                    "Players"
                </button>
                <button class="btn btn-ghost" on:click=move |_| on_settings()>
                    "Settings"
                </button>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Settings page
// ---------------------------------------------------------------------------

#[component]
pub fn SettingsPage(on_back: impl Fn() + 'static) -> impl IntoView {
    view! {
        <div style="height:100vh; display:flex; flex-direction:column;">
            <div class="page-header">
                <button class="btn-back" on:click=move |_| on_back()>"← Back"</button>
                <h2>"Settings"</h2>
            </div>
            <div class="panel-body">
                <p class="empty-note">"No settings available yet."</p>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Players page
// ---------------------------------------------------------------------------

/// A single player entry.  Extensible: add more entries to `PLAYERS` or
/// replace the static list with a reactive `Vec<PlayerConfig>`.
struct PlayerConfig {
    name:     &'static str,
    civ_name: &'static str,
    is_human: bool,
}

// FIXME: Player list should come from the server lobby/game-config endpoint.
//        Right now it's hard-coded to mirror the single human player that
//        `session::build_session` creates.
const PLAYERS: &[PlayerConfig] = &[
    PlayerConfig { name: "Player 1", civ_name: "Rome (Caesar)", is_human: true },
];

#[component]
pub fn PlayersPage(on_back: impl Fn() + 'static) -> impl IntoView {
    let rows = PLAYERS.iter().map(|p| {
        let initial = &p.name[..1];
        let name    = p.name;
        let civ     = p.civ_name;
        let tag     = if p.is_human { "Human" } else { "AI" };
        view! {
            <div class="player-row">
                <div class="player-avatar">{initial}</div>
                <div class="player-info">
                    <div class="player-name">{name}</div>
                    <div class="player-civ">{civ}" · "{tag}</div>
                </div>
            </div>
        }
    }).collect_view();

    view! {
        <div style="height:100vh; display:flex; flex-direction:column;">
            <div class="page-header">
                <button class="btn-back" on:click=move |_| on_back()>"← Back"</button>
                <h2>"Players"</h2>
            </div>
            <div class="panel-body">
                {rows}
            </div>
        </div>
    }
}
