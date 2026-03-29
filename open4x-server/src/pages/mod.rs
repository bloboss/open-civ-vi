pub mod game;
pub mod replay;

use leptos::prelude::*;
use crate::components::session::{GameConfig, DemoConfig};

// ---------------------------------------------------------------------------
// Home page
// ---------------------------------------------------------------------------

#[component]
pub fn HomePage(
    on_new_game: impl Fn() + 'static,
    on_settings: impl Fn() + 'static,
    on_players:  impl Fn() + 'static,
    on_demo:     impl Fn() + 'static,
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
                <button class="btn btn-ghost" on:click=move |_| on_demo()>
                    "Watch AI Demo"
                </button>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Map config page  (shown between Home and Game)
// ---------------------------------------------------------------------------

#[component]
pub fn MapConfigPage(
    on_start: impl Fn(GameConfig) + 'static,
    on_back:  impl Fn() + 'static,
) -> impl IntoView {
    let size    = RwSignal::new("medium");
    let seed_str = RwSignal::new("42".to_string());
    let num_ai  = RwSignal::new(1u32);

    let start = move || {
        let (w, h) = match size.get() {
            "small" => (20u32, 14u32),
            "large" => (60u32, 36u32),
            _       => (40u32, 24u32),
        };
        let seed: u64 = seed_str.get().parse().unwrap_or(42);
        on_start(GameConfig { width: w, height: h, seed, num_ai: num_ai.get() });
    };

    view! {
        <div style="height:100vh; display:flex; flex-direction:column;">
            <div class="page-header">
                <button class="btn-back" on:click=move |_| on_back()>"← Back"</button>
                <h2>"New Game"</h2>
            </div>
            <div class="panel-body" style="overflow-y:auto">
                <div class="config-card">
                    <p class="config-label">"Map Size"</p>
                    <div class="preset-row">
                        {["small", "medium", "large"].map(|preset| {
                            let label = match preset {
                                "small"  => "Small  (20 × 14)",
                                "large"  => "Large  (60 × 36)",
                                _        => "Medium (40 × 24)",
                            };
                            view! {
                                <button
                                    class="btn btn-ghost preset-btn"
                                    class:preset-active=move || size.get() == preset
                                    on:click=move |_| size.set(preset)
                                >
                                    {label}
                                </button>
                            }
                        })}
                    </div>

                    <p class="config-label" style="margin-top:1.2rem">"Map Seed"</p>
                    <input
                        class="config-input"
                        type="number"
                        min="0"
                        prop:value=move || seed_str.get()
                        on:input=move |ev| {
                            use wasm_bindgen::JsCast as _;
                            if let Some(el) = ev.target()
                                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            {
                                seed_str.set(el.value());
                            }
                        }
                    />

                    <p class="config-label" style="margin-top:1.2rem">"AI Opponents"</p>
                    <div class="preset-row">
                        {[(0u32, "None (solo)"), (1, "1 AI"), (2, "2 AI"), (3, "3 AI")].map(|(n, label)| {
                            view! {
                                <button
                                    class="btn btn-ghost preset-btn"
                                    class:preset-active=move || num_ai.get() == n
                                    on:click=move |_| num_ai.set(n)
                                >
                                    {label}
                                </button>
                            }
                        })}
                    </div>

                    <button
                        class="btn btn-primary"
                        style="margin-top:2rem;width:100%;font-size:1rem;padding:0.7rem"
                        on:click=move |_| start()
                    >
                        "Start Game"
                    </button>
                </div>
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
                <p class="empty-note">"Server: ws://127.0.0.1:3001/ws"</p>
                <p class="empty-note">"Auth: Ed25519 keypair (auto-generated, stored in localStorage)"</p>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Players page
// ---------------------------------------------------------------------------

#[component]
pub fn PlayersPage(on_back: impl Fn() + 'static) -> impl IntoView {
    view! {
        <div style="height:100vh; display:flex; flex-direction:column;">
            <div class="page-header">
                <button class="btn-back" on:click=move |_| on_back()>"← Back"</button>
                <h2>"Profile"</h2>
            </div>
            <div class="panel-body">
                <p class="empty-note">"Profile management coming soon."</p>
                <p class="empty-note">"Your identity is an Ed25519 keypair stored in your browser."</p>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Demo config page  (configure AI-vs-AI game before watching)
// ---------------------------------------------------------------------------

#[component]
pub fn DemoConfigPage(
    on_start: impl Fn(DemoConfig) + 'static,
    on_back:  impl Fn() + 'static,
) -> impl IntoView {
    let size        = RwSignal::new("small");
    let seed_str    = RwSignal::new("42".to_string());
    let turns_str   = RwSignal::new("100".to_string());
    let num_players = RwSignal::new(2u32);

    let start = move || {
        let (w, h) = match size.get() {
            "medium" => (40u32, 24u32),
            "large"  => (60u32, 36u32),
            _        => (20u32, 14u32),
        };
        let seed: u64 = seed_str.get().parse().unwrap_or(42);
        let num_turns: u32 = turns_str.get().parse().unwrap_or(100);
        on_start(DemoConfig { width: w, height: h, seed, num_turns, num_players: num_players.get() });
    };

    view! {
        <div style="height:100vh; display:flex; flex-direction:column;">
            <div class="page-header">
                <button class="btn-back" on:click=move |_| on_back()>"← Back"</button>
                <h2>"AI Demo Setup"</h2>
            </div>
            <div class="panel-body" style="overflow-y:auto">
                <div class="config-card">
                    <p class="config-label">"Map Size"</p>
                    <div class="preset-row">
                        {["small", "medium", "large"].map(|preset| {
                            let label = match preset {
                                "medium" => "Medium (40 × 24)",
                                "large"  => "Large  (60 × 36)",
                                _        => "Small  (20 × 14)",
                            };
                            view! {
                                <button
                                    class="btn btn-ghost preset-btn"
                                    class:preset-active=move || size.get() == preset
                                    on:click=move |_| size.set(preset)
                                >
                                    {label}
                                </button>
                            }
                        })}
                    </div>

                    <p class="config-label" style="margin-top:1.2rem">"Map Seed"</p>
                    <input
                        class="config-input"
                        type="number"
                        min="0"
                        prop:value=move || seed_str.get()
                        on:input=move |ev| {
                            use wasm_bindgen::JsCast as _;
                            if let Some(el) = ev.target()
                                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            {
                                seed_str.set(el.value());
                            }
                        }
                    />

                    <p class="config-label" style="margin-top:1.2rem">"Number of Turns"</p>
                    <div class="preset-row">
                        {[("50", "50"), ("100", "100"), ("200", "200"), ("300", "300")].map(|(val, label)| {
                            view! {
                                <button
                                    class="btn btn-ghost preset-btn"
                                    class:preset-active=move || turns_str.get() == val
                                    on:click=move |_| turns_str.set(val.to_string())
                                >
                                    {label}
                                </button>
                            }
                        })}
                    </div>

                    <p class="config-label" style="margin-top:1.2rem">"Number of AI Players"</p>
                    <div class="preset-row">
                        {[(2u32, "2"), (3, "3"), (4, "4"), (6, "6"), (8, "8")].map(|(n, label)| {
                            view! {
                                <button
                                    class="btn btn-ghost preset-btn"
                                    class:preset-active=move || num_players.get() == n
                                    on:click=move |_| num_players.set(n)
                                >
                                    {label}
                                </button>
                            }
                        })}
                    </div>

                    <div style="margin-top:1.2rem; padding:0.8rem; background:#0f1117; border-radius:6px; border:1px solid #2e3248;">
                        <p style="font-size:0.85rem; color:#7a7f99; margin-bottom:0.4rem;">
                            {move || {
                                let civs: Vec<&str> = ["Rome", "Babylon", "Greece", "Egypt",
                                    "Germany", "Japan", "India", "Arabia"]
                                    .into_iter().take(num_players.get() as usize).collect();
                                format!("Match: {}", civs.join(" vs "))
                            }}
                        </p>
                        <p style="font-size:0.8rem; color:#555;">"Deterministic AI agents (HeuristicAgent) will play a full game. You can watch the replay turn-by-turn from any civ's perspective or as a spectator."</p>
                    </div>

                    <button
                        class="btn btn-primary"
                        style="margin-top:2rem;width:100%;font-size:1rem;padding:0.7rem"
                        on:click=move |_| start()
                    >
                        "Run & Watch"
                    </button>
                </div>
            </div>
        </div>
    }
}
