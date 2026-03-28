/// History viewer / replay page.
///
/// Fetches a full AI-vs-AI demo game from the server and allows
/// stepping through turns with the hex map.
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Response;

use open4x_api::view::GameView;
use open4x_api::coord::HexCoord;
use open4x_api::ids::UnitId;

use crate::hexmap::{HexMap, svg_dimensions};

// ── Types matching server DemoGameResult ─────────────────────────────────────

#[derive(serde::Deserialize, Clone)]
struct DemoGameResult {
    turns: Vec<DemoTurnSnapshot>,
    civ_names: Vec<String>,
}

#[derive(serde::Deserialize, Clone)]
struct DemoTurnSnapshot {
    #[allow(dead_code)]
    turn: u32,
    views: Vec<GameView>,
}

// ── ReplayPage component ────────────────────────────────────────────────────

#[component]
pub fn ReplayPage(
    on_back: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let on_back = std::sync::Arc::new(on_back);
    let on_back_topbar = on_back.clone();
    let on_back_error = on_back.clone();
    let demo_data: RwSignal<Option<DemoGameResult>> = RwSignal::new(None);
    let current_turn = RwSignal::new(0usize);
    let viewing_civ = RwSignal::new(0usize); // 0 = first civ, 1 = second civ
    let loading = RwSignal::new(true);
    let error_msg: RwSignal<Option<String>> = RwSignal::new(None);
    let auto_playing = RwSignal::new(false);

    // Selected tile/unit for hex map interaction.
    let selected_tile: RwSignal<Option<HexCoord>> = RwSignal::new(None);
    let selected_unit: RwSignal<Option<UnitId>> = RwSignal::new(None);

    // Fetch the demo game on mount.
    {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_demo_game().await {
                Ok(data) => {
                    demo_data.set(Some(data));
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to load demo: {e}")));
                    loading.set(false);
                }
            }
        });
    }

    // Derive the current GameView from turn index + viewing civ.
    let game_view_signal = {
        let (read_gv, write_gv) = signal::<Option<GameView>>(None);
        Effect::new(move || {
            let idx = current_turn.get();
            let civ_idx = viewing_civ.get();
            if let Some(data) = demo_data.get().filter(|d| idx < d.turns.len()) {
                let snap = &data.turns[idx];
                if civ_idx < snap.views.len() {
                    write_gv.set(Some(snap.views[civ_idx].clone()));
                    return;
                }
            }
            write_gv.set(None);
        });
        read_gv
    };

    let max_turn = move || {
        demo_data.get().map(|d| d.turns.len().saturating_sub(1)).unwrap_or(0)
    };

    let civ_names = move || {
        demo_data.get().map(|d| d.civ_names.clone()).unwrap_or_default()
    };

    // Get scores for the current turn.
    let scores_text = move || {
        let Some(_gv) = game_view_signal.get() else { return String::new() };
        let names = civ_names();
        let Some(data) = demo_data.get() else { return String::new() };
        let idx = current_turn.get();
        if idx >= data.turns.len() { return String::new(); }

        let mut parts = Vec::new();
        for (i, name) in names.iter().enumerate() {
            if i < data.turns[idx].views.len() {
                let view = &data.turns[idx].views[i];
                let score: u32 = view.scores.iter().map(|(_, s)| s).sum();
                parts.push(format!("{name}: {score}"));
            }
        }
        parts.join("  |  ")
    };

    // Autoplay timer.
    {
        Effect::new(move || {
            if auto_playing.get() {
                let mt = max_turn();
                let ct = current_turn.get_untracked();
                if ct < mt {
                    // Schedule next turn via setTimeout.
                    let cb = Closure::once(move || {
                        if auto_playing.get_untracked() {
                            let next = current_turn.get_untracked() + 1;
                            let mt = max_turn();
                            if next <= mt {
                                current_turn.set(next);
                            }
                            if next >= mt {
                                auto_playing.set(false);
                            }
                        }
                    });
                    let _ = web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(), 300,
                    );
                    cb.forget();
                } else {
                    auto_playing.set(false);
                }
            }
        });
    }

    let on_hex_click = move |coord: HexCoord| {
        if selected_tile.get_untracked() == Some(coord) {
            selected_tile.set(None);
            selected_unit.set(None);
        } else {
            selected_tile.set(Some(coord));
            // Select any unit at this coord.
            if let Some(gv) = game_view_signal.get_untracked() {
                let unit = gv.units.iter().find(|u| u.coord == coord);
                selected_unit.set(unit.map(|u| u.id));
            }
        }
    };

    let _dims = move || {
        game_view_signal.get().map(|gv| svg_dimensions(gv.board.width, gv.board.height))
            .unwrap_or((800.0, 600.0))
    };

    view! {
        <div style="height:100vh; display:flex; flex-direction:column; background:#0f1117; color:#d8d8d8;">
            // ── Top bar ──────────────────────────────────────────────────
            <div class="replay-topbar">
                <button class="btn-back" on:click=move |_| on_back_topbar()>"← Back"</button>
                <h2 style="margin:0; font-size:1.1rem;">"Game History Viewer"</h2>
                <div style="font-size:0.85rem; color:#999;">
                    {move || scores_text()}
                </div>
            </div>

            // ── Loading / Error states ───────────────────────────────────
            <Show when=move || loading.get()>
                <div class="page-center">
                    <div class="card">
                        <div class="title">"Running AI Demo Game..."</div>
                        <div class="subtitle">"Simulating 100 turns between Rome and Babylon"</div>
                    </div>
                </div>
            </Show>

            <Show when=move || error_msg.get().is_some()>
                <div class="page-center">
                    <div class="card">
                        <div class="title" style="color:#e05050;">"Error"</div>
                        <div class="subtitle">{move || error_msg.get().unwrap_or_default()}</div>
                        <button class="btn btn-ghost" on:click={let cb = on_back_error.clone(); move |_| cb()}>"Back to Menu"</button>
                    </div>
                </div>
            </Show>

            // ── Main replay content ──────────────────────────────────────
            <Show when=move || !loading.get() && error_msg.get().is_none()>
                <div style="flex:1; display:flex; flex-direction:column; overflow:hidden;">
                    // ── Hex map viewport ─────────────────────────────────
                    <div class="hex-viewport" style="flex:1; overflow:auto;">
                        <HexMap
                            game_view=game_view_signal
                            selected_tile=selected_tile
                            selected_unit=selected_unit
                            on_hex_click=on_hex_click
                        />
                    </div>

                    // ── Turn controls bar ────────────────────────────────
                    <div class="replay-controls">
                        <div class="replay-controls-inner">
                            <button class="btn btn-ghost replay-btn"
                                on:click=move |_| { current_turn.set(0); auto_playing.set(false); }
                                disabled=move || current_turn.get() == 0
                            >"⏮"</button>

                            <button class="btn btn-ghost replay-btn"
                                on:click=move |_| {
                                    auto_playing.set(false);
                                    let t = current_turn.get();
                                    if t > 0 { current_turn.set(t - 1); }
                                }
                                disabled=move || current_turn.get() == 0
                            >"◀"</button>

                            <button class="btn btn-primary replay-btn"
                                on:click=move |_| {
                                    if auto_playing.get() {
                                        auto_playing.set(false);
                                    } else {
                                        auto_playing.set(true);
                                    }
                                }
                            >
                                {move || if auto_playing.get() { "⏸" } else { "▶" }}
                            </button>

                            <button class="btn btn-ghost replay-btn"
                                on:click=move |_| {
                                    auto_playing.set(false);
                                    let t = current_turn.get();
                                    if t < max_turn() { current_turn.set(t + 1); }
                                }
                                disabled=move || current_turn.get() >= max_turn()
                            >"▶"</button>

                            <button class="btn btn-ghost replay-btn"
                                on:click=move |_| { current_turn.set(max_turn()); auto_playing.set(false); }
                                disabled=move || current_turn.get() >= max_turn()
                            >"⏭"</button>

                            <span class="replay-turn-label">
                                {move || format!("Turn {}", current_turn.get())}
                                " / "
                                {move || format!("{}", max_turn())}
                            </span>

                            <span style="margin-left:auto; display:flex; gap:4px;">
                                {move || civ_names().into_iter().enumerate().map(|(i, name)| {
                                    let active = viewing_civ.get() == i;
                                    let class = if active { "btn btn-primary replay-civ-btn" } else { "btn btn-ghost replay-civ-btn" };
                                    view! {
                                        <button
                                            class=class
                                            on:click=move |_| viewing_civ.set(i)
                                        >
                                            {name}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </span>
                        </div>
                    </div>

                    // ── Info sidebar ─────────────────────────────────────
                    <Show when=move || selected_tile.get().is_some()>
                        <div class="replay-info-panel">
                            {move || {
                                let coord = selected_tile.get()?;
                                let gv = game_view_signal.get()?;

                                let tile = gv.board.tiles.iter().find(|t| t.coord == coord);
                                let city = gv.cities.iter().find(|c| c.coord == coord);
                                let unit = gv.units.iter().find(|u| u.coord == coord);

                                let mut info = vec![format!("({}, {})", coord.q, coord.r)];
                                if let Some(t) = tile {
                                    info.push(format!("{:?}", t.terrain));
                                    if let Some(f) = &t.feature { info.push(format!("{f:?}")); }
                                    if let Some(imp) = &t.improvement { info.push(format!("{imp:?}")); }
                                }
                                if let Some(c) = city {
                                    info.push(format!("{} (pop {})", c.name, c.population));
                                }
                                if let Some(u) = unit {
                                    let utype = gv.unit_type_defs.iter()
                                        .find(|d| d.id == u.unit_type)
                                        .map(|d| d.name.as_str())
                                        .unwrap_or("?");
                                    info.push(format!("{utype} HP:{}", u.health));
                                }

                                Some(view! {
                                    <div>{info.join(" | ")}</div>
                                })
                            }}
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

// ── Fetch helper ────────────────────────────────────────────────────────────

async fn fetch_demo_game() -> Result<DemoGameResult, String> {
    let base = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://127.0.0.1:3001".to_string());
    let url = format!("{base}/api/demo-game?seed=42&width=20&height=14&turns=100");

    let resp_value = wasm_bindgen_futures::JsFuture::from(
        web_sys::window().unwrap().fetch_with_str(&url),
    )
    .await
    .map_err(|e| format!("{e:?}"))?;

    let resp: Response = resp_value.dyn_into().map_err(|_| "not a Response")?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(
        resp.json().map_err(|e| format!("{e:?}"))?,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;

    let data: DemoGameResult = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("parse error: {e}"))?;

    Ok(data)
}
