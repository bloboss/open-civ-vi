pub mod game;

use leptos::prelude::*;
use crate::session::GameConfig;

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
