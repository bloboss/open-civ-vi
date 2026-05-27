//! REST-driven single-player view.
//!
//! On mount this page calls `POST /api/v1/games/new` to bootstrap a game and
//! receive a bearer token; from then on every read uses
//! [`open4x_sdk::endpoints`] against `/api/v1/*`. Mutations bump a refresh
//! tick that drives every `LocalResource` to refetch in parallel. No
//! WebSocket is involved.
//!
//! The layout mirrors `open4x-server/static/vi/Open4X.html` — a three-row
//! `.app` grid (topbar / tabbar / screen-root). The HUD board is the
//! WebGL2 renderer in [`crate::components::hud::webgl_map`].

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use open4x_sdk::endpoints as api;
use open4x_sdk::wasm::WasmClient;

use crate::components::hud::{
    Camera, ContextPanel, MinimapPanel, NotificationsPanel, TurnQueuePanel, WebglMap, ZoomStack,
    recenter, zoom_in, zoom_out,
};
use crate::components::shell::{ScreenStub, Tab, Tabbar, Topbar};
use open4x_protocol::v1::web::world::WorldSnapshot;

/// Build an SDK client rooted at the page origin. We talk to the same
/// host the WASM bundle was served from, so the base URL is empty (the
/// `fetch` API resolves request paths against `window.location`).
fn client(token: Option<&str>) -> WasmClient {
    let c = WasmClient::new("");
    match token {
        Some(t) => c.with_token(t),
        None => c,
    }
}

#[component]
pub fn RestGamePage() -> impl IntoView {
    // ── Session state ────────────────────────────────────────────────────────
    let token = RwSignal::new(None::<String>);
    let bootstrap_error = RwSignal::new(None::<String>);
    let action_error = RwSignal::new(None::<String>);
    let tick = RwSignal::new(0u64);
    let active_tab = RwSignal::new(Tab::Hud);
    let selected_tile = RwSignal::new(None::<(i32, i32)>);
    let camera = RwSignal::new(Camera::default());

    // Bootstrap once on mount.
    //
    // If the URL carries `?token=…` (the lobby's Resume flow drops us
    // here with a pre-minted bearer), use it directly. Otherwise fall
    // back to the anonymous `POST /games/new` path so guest play
    // still works without going through the lobby.
    Effect::new(move |_| {
        if token.get_untracked().is_some() {
            return;
        }
        if let Some(t) = read_token_from_query() {
            token.set(Some(t));
            return;
        }
        spawn_local(async move {
            let body = api::games::NewGameRequest::default();
            match api::games::new_game(&client(None), &body).await {
                Ok(resp) => token.set(Some(resp.token)),
                Err(e) => bootstrap_error.set(Some(e.to_string())),
            }
        });
    });

    // ── Resources keyed on (token, tick) ────────────────────────────────────
    let player_state = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::player_state::get(&client(Some(&tok))).await.ok()
        }
    });

    let snapshot = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::world::snapshot(&client(Some(&tok)), None, None, Some(0))
                .await
                .ok()
        }
    });

    let cities = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::cities::list(&client(Some(&tok))).await.ok()
        }
    });

    let units = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::units::list(&client(Some(&tok))).await.ok()
        }
    });

    let tech = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::tech::get(&client(Some(&tok))).await.ok()
        }
    });

    let civics = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::civics::get(&client(Some(&tok))).await.ok()
        }
    });

    let turn_queue = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::turn::queue(&client(Some(&tok))).await.ok()
        }
    });

    let notifs = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::notifications::list(&client(Some(&tok))).await.ok()
        }
    });

    // ── Mutations ───────────────────────────────────────────────────────────
    let on_end_turn = Callback::new(move |_: ()| {
        let tok = token.get();
        action_error.set(None);
        spawn_local(async move {
            let Some(tok) = tok else { return };
            match api::turn::end(&client(Some(&tok))).await {
                Ok(_) => tick.update(|t| *t += 1),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    });

    view! {
        <div class="app">
            <Topbar player_state=player_state />
            <Tabbar
                active=active_tab
                cities=cities units=units tech=tech civics=civics
                turn_queue=turn_queue
                on_end_turn=on_end_turn
            />
            <main class="screen-root">
                {move || bootstrap_error.get().map(|e| view! {
                    <div class="screen-stub">
                        <div class="screen-stub-card">
                            <span class="kicker" style="color:var(--bad)">"Bootstrap failed"</span>
                            <p>{e}</p>
                        </div>
                    </div>
                })}
                {move || action_error.get().map(|e| view! {
                    <div style="position:absolute; top:8px; left:50%; transform:translateX(-50%);
                                background:var(--bad-soft); color:var(--bad); padding:6px 12px;
                                border-radius:var(--r-2); font-size:12px; z-index:60;">
                        {format!("Action failed: {e}")}
                    </div>
                })}

                <section
                    class="screen"
                    class:active=move || active_tab.get() == Tab::Hud
                    attr:data-screen="hud"
                >
                    <HudScreen
                        snapshot=snapshot
                        selected=selected_tile
                        camera=camera
                        cities=cities
                        notifs=notifs
                        turn_queue=turn_queue
                        token=token
                        tick=tick
                        on_open_tab=Callback::new(move |t| active_tab.set(t))
                    />
                </section>

                {STUB_TABS.iter().copied().map(|(tab, title, desc)| view! {
                    <section
                        class="screen"
                        class:active=move || active_tab.get() == tab
                        attr:data-screen=tab.key()
                    >
                        <ScreenStub title=title desc=desc />
                    </section>
                }).collect::<Vec<_>>()}
            </main>
        </div>
    }
}

#[component]
fn HudScreen(
    snapshot: LocalResource<Option<WorldSnapshot>>,
    selected: RwSignal<Option<(i32, i32)>>,
    camera: RwSignal<Camera>,
    cities: LocalResource<Option<open4x_protocol::v1::web::city_data::CityData>>,
    notifs: LocalResource<Option<open4x_protocol::v1::web::notifications::Notifications>>,
    turn_queue: LocalResource<Option<open4x_protocol::v1::web::turn_queue::TurnQueue>>,
    token: RwSignal<Option<String>>,
    tick: RwSignal<u64>,
    on_open_tab: Callback<Tab>,
) -> impl IntoView {
    let snap_signal = Signal::derive(move || -> Option<WorldSnapshot> {
        snapshot.get().and_then(|w| (*w).clone())
    });

    let on_recenter = Callback::new(move |_| {
        // Prefer the player's capital, then the first known city, then the
        // world centre.
        let target = cities.get().as_deref()
            .and_then(|w| w.as_ref().map(|c| {
                c.cities.iter().find(|x| x.capital && x.is_own)
                    .or_else(|| c.cities.iter().find(|x| x.is_own))
                    .or_else(|| c.cities.first())
                    .map(|x| (x.position.q, x.position.r))
            }))
            .flatten();
        if let Some(pos) = target {
            recenter(camera, pos);
            selected.set(Some(pos));
        }
    });
    let on_zoom_in  = Callback::new(move |_| zoom_in(camera));
    let on_zoom_out = Callback::new(move |_| zoom_out(camera));

    view! {
        <>
            <WebglMap snapshot=snap_signal camera=camera selected=selected />

            <MinimapPanel snapshot=snap_signal />
            <NotificationsPanel notifs=notifs token=token tick=tick />
            <TurnQueuePanel turn_queue=turn_queue />
            <ContextPanel selected=selected snapshot=snap_signal on_open_tab=on_open_tab />
            <ZoomStack
                on_zoom_in=on_zoom_in
                on_zoom_out=on_zoom_out
                on_recenter=on_recenter
            />
        </>
    }
}

/// (tab, screen title, sub-copy) — text is copied verbatim from
/// Open4X.html's own "Coming next" stub treatment (lines 177-232).
const STUB_TABS: &[(Tab, &str, &str)] = &[
    (Tab::City,       "City management",       "Three-column layout: cities · district & yields & population · production queue."),
    (Tab::Units,      "Units & Armies",        "Unit roster + per-unit detail with promotions, actions, combat stats."),
    (Tab::Tech,       "Technology tree",       "Era columns · prereq edges · queue · research focus."),
    (Tab::Civics,     "Civics tree",           "Same layout as tech, plus policy preview."),
    (Tab::Diplomacy,  "Diplomacy",             "Civ list · relations & modifiers · deal builder · city-states."),
    (Tab::Government, "Government & Policies", "Current government · slot composition · policy catalogue."),
    (Tab::Empire,     "Empire overview",       "Summary stats · city table · resources · trade routes · religion · yields chart."),
    (Tab::Victory,    "Victory conditions",    "Per-condition progress · steps · leaderboard."),
];

/// Pull the `token=…` value out of the current URL's query string.
/// Used by the Resume flow: the lobby navigates the user-agent to
/// `<this-server>/?token=<lobby-stored-bearer>` and the SPA picks
/// the token up here instead of bootstrapping a fresh game.
fn read_token_from_query() -> Option<String> {
    let win = web_sys::window()?;
    let search = win.location().search().ok()?;
    let q = search.strip_prefix('?').unwrap_or(&search);
    for pair in q.split('&') {
        if let Some(rest) = pair.strip_prefix("token=") {
            return Some(decode_uri_component(rest));
        }
    }
    None
}

/// Minimal `decodeURIComponent` shim — handles the `%XX` escapes we
/// might see in URL-safe base64 round-tripped through cookies and
/// the `+` → space convention. Sufficient for `lobby_<base64url>`
/// tokens which are already URL-safe.
fn decode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                if let Some(h) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    out.push(h as char);
                    i += 3;
                    continue;
                }
                out.push('%');
                i += 1;
            }
            b'+' => {
                out.push(' ');
                i += 1;
            }
            other => {
                out.push(other as char);
                i += 1;
            }
        }
    }
    out
}
