//! Ongoing-games screen — wired to `GET /api/v1/games`.
//!
//! Reads live data on mount via `LocalResource`; until the resource
//! resolves the screen shows a "Loading…" placeholder. Empty-state
//! copy nudges the user toward `+ New game`. Tile actions
//! (Resume / Notes / ⋯) are still inert beyond the Resume CTA, which
//! requires the Phase 4.3 orchestrator to populate `server_url`.

use std::sync::Arc;

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::api::games as games_api;
use crate::components::{
    Btn, MiniMap, Popup, PopupActions, PopupBody, PopupList, PopupListItem, PopupSize,
    PopupTrigger, Tag,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Filter {
    All,
    YourTurn,
    Waiting,
    Completed,
    Multiplayer,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Sort {
    /// `last_played_at` desc, falling back to `created_at` desc.
    Recent,
    /// `created_at` asc.
    Oldest,
    /// `score` desc, then turn desc.
    Score,
    /// `turn` desc.
    Turn,
}

impl Sort {
    fn label(self) -> &'static str {
        match self {
            Sort::Recent => "recent ↓",
            Sort::Oldest => "oldest",
            Sort::Score => "score ↓",
            Sort::Turn => "turn ↓",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Sort::Recent => "recent",
            Sort::Oldest => "oldest",
            Sort::Score => "score",
            Sort::Turn => "turn",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "recent" => Some(Sort::Recent),
            "oldest" => Some(Sort::Oldest),
            "score" => Some(Sort::Score),
            "turn" => Some(Sort::Turn),
            _ => None,
        }
    }
}

impl Filter {
    fn slug(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::YourTurn => "your_turn",
            Filter::Waiting => "waiting",
            Filter::Completed => "completed",
            Filter::Multiplayer => "multiplayer",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Filter::All),
            "your_turn" => Some(Filter::YourTurn),
            "waiting" => Some(Filter::Waiting),
            "completed" => Some(Filter::Completed),
            "multiplayer" => Some(Filter::Multiplayer),
            _ => None,
        }
    }
}

#[component]
pub fn OngoingGames(on_new: Callback<()>) -> impl IntoView {
    let tick = RwSignal::new(0u32);
    let games = LocalResource::new(move || {
        let _ = tick.get();
        async move { games_api::list().await.ok() }
    });

    // Seed filter/sort/search from the URL query string so the back
    // button + reload restore the user's view.
    let initial = read_query_state();
    let filter = RwSignal::new(initial.filter);
    let search = RwSignal::new(initial.q);
    let sort = RwSignal::new(initial.sort);

    // Push state whenever any of the three change. push_state replaces
    // the URL without a navigation, which is what we want here.
    Effect::new(move |_| {
        let f = filter.get();
        let s = sort.get();
        let q = search.get();
        push_query_state(&QueryState {
            filter: f,
            sort: s,
            q,
        });
    });

    let chip_class = move |target: Filter| -> &'static str {
        if filter.get() == target {
            "chip active"
        } else {
            "chip"
        }
    };

    view! {
        <div style="flex:1; display:flex; flex-direction:column; min-height:0">
            <div class="content-header">
                <div class="title">"Ongoing games"</div>
                <span class="crumbs">
                    {move || games
                        .get()
                        .and_then(|wrap| (*wrap).clone())
                        .map(|resp| {
                            let total = resp.games.len();
                            let yt = resp.games.iter().filter(|g| g.status == "your_turn").count();
                            format!("{total} games · {yt} awaiting you")
                        })
                        .unwrap_or_else(|| "loading…".into())}
                </span>
                <div class="actions">
                    <span class="muted xsmall">"view"</span>
                    <Btn variant="accent"
                         on_click=Callback::new(move |_| on_new.run(()))>
                        "+ New game"
                    </Btn>
                </div>
            </div>

            <div class="filter-bar">
                <span class="muted xsmall" style="padding-left:4px">"⌕"</span>
                <input
                    class="filter-search"
                    placeholder="search games and notes…"
                    prop:value=move || search.get()
                    on:input=move |ev| {
                        use wasm_bindgen::JsCast as _;
                        if let Some(el) = ev.target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        {
                            search.set(el.value());
                        }
                    }
                />
                <span class="sep-v"></span>
                <button
                    class=move || chip_class(Filter::YourTurn)
                    on:click=move |_| filter.set(Filter::YourTurn)
                >
                    "your turn"
                    {move || (filter.get() == Filter::YourTurn).then(|| view! { " " <span class="x">"×"</span> })}
                </button>
                <button class=move || chip_class(Filter::Waiting)
                        on:click=move |_| filter.set(Filter::Waiting)>"waiting"</button>
                <button class=move || chip_class(Filter::Completed)
                        on:click=move |_| filter.set(Filter::Completed)>"completed"</button>
                <button class=move || chip_class(Filter::Multiplayer)
                        on:click=move |_| filter.set(Filter::Multiplayer)>"multiplayer"</button>
                <button class=move || chip_class(Filter::All)
                        on:click=move |_| filter.set(Filter::All)>"all"</button>
                <span style="margin-left:auto" class="muted xsmall">"sort"</span>
                <Popup
                    title="Sort"
                    size=PopupSize::Narrow
                    trigger=PopupTrigger::Click
                    content=Arc::new(move || view! {
                        <div class="popup-list">
                            {[Sort::Recent, Sort::Oldest, Sort::Score, Sort::Turn].iter().copied().map(|s| {
                                let active = sort.get() == s;
                                view! {
                                    <button
                                        class="item"
                                        type="button"
                                        on:click=move |_| sort.set(s)
                                    >
                                        <span class="icon">{if active { "✓" } else { " " }}</span>
                                        <span>{s.label()}</span>
                                    </button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any())
                >
                    <button class="chip">{move || sort.get().label()}</button>
                </Popup>
            </div>

            <div style="flex:1; overflow:auto">
                <Suspense fallback=move || view! { <p class="muted xsmall" style="padding:16px">"Loading…"</p> }>
                    {move || games.get().map(|wrap| {
                        let Some(resp) = (*wrap).clone() else {
                            return view! {
                                <p class="muted xsmall" style="padding:16px">
                                    "Couldn't load games — try refreshing."
                                </p>
                            }.into_any();
                        };
                        if resp.games.is_empty() {
                            return view! {
                                <div style="padding:24px; text-align:center; color:var(--dim)">
                                    <p class="small" style="margin-bottom:12px">
                                        "No games yet. Click "
                                        <strong>"+ New game"</strong>
                                        " above to start your first."
                                    </p>
                                </div>
                            }.into_any();
                        }
                        let f = filter.get();
                        let q = search.get().to_lowercase();
                        let s = sort.get();
                        let mut filtered: Vec<_> = resp.games.into_iter()
                            .filter(|g| match f {
                                Filter::All => true,
                                Filter::YourTurn => g.status == "your_turn",
                                Filter::Waiting => g.status == "waiting",
                                Filter::Completed => g.status == "completed",
                                Filter::Multiplayer => g.players_human > 1,
                            })
                            .filter(|g| {
                                if q.is_empty() { return true; }
                                g.name.to_lowercase().contains(&q)
                                    || g.leader.to_lowercase().contains(&q)
                                    || g.civ_id.to_lowercase().contains(&q)
                            })
                            .collect();
                        match s {
                            Sort::Recent => filtered.sort_by(|a, b| {
                                let key = |g: &games_api::GameView| {
                                    g.last_played_at
                                        .clone()
                                        .unwrap_or_else(|| g.created_at.clone())
                                };
                                key(b).cmp(&key(a))
                            }),
                            Sort::Oldest => filtered.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
                            Sort::Score => filtered.sort_by(|a, b| {
                                b.score.cmp(&a.score).then(b.turn.cmp(&a.turn))
                            }),
                            Sort::Turn => filtered.sort_by(|a, b| b.turn.cmp(&a.turn)),
                        }
                        if filtered.is_empty() {
                            return view! {
                                <div style="padding:24px; text-align:center; color:var(--dim)">
                                    <p class="small">"No games match the current filter."</p>
                                </div>
                            }.into_any();
                        }
                        let rows = filtered.into_iter().enumerate().map(|(i, g)| {
                            let is_yours = g.status == "your_turn";
                            let tile_class = if is_yours { "game-tile your-turn" } else { "game-tile" };
                            let resume_label = match g.status.as_str() {
                                "your_turn" => "→ Resume",
                                "waiting" => "Open",
                                "completed" => "Review",
                                _ => "Open",
                            };
                            let resume_variant = if is_yours { "accent" } else { "primary" };
                            let game_id_for_resume = g.game_id.clone();
                            let resume_disabled = g.server_url.is_empty();
                            let on_resume = Callback::new(move |_: ()| {
                                let id = game_id_for_resume.clone();
                                spawn_local(async move {
                                    let Ok(resp) = games_api::resume(&id).await else { return };
                                    if let Some(win) = web_sys::window() {
                                        let target = format!(
                                            "{}/?token={}",
                                            resp.url.trim_end_matches('/'),
                                            resp.token,
                                        );
                                        let _ = win.location().set_href(&target);
                                    }
                                });
                            });
                            let disabled_signal = Signal::derive(move || resume_disabled);
                            let players = format!("{}H · {}AI", g.players_human, g.players_ai);
                            // Seed the MiniMap from the actual world seed when
                            // possible; fall back to row index otherwise so
                            // every tile still gets unique decoration.
                            let seed = g
                                .seed
                                .bytes()
                                .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
                                .max(1)
                                .saturating_add(i as u64);
                            // Real-game thumbnail: kick off a fetch
                            // (no-op if already cached) and pass any
                            // ready grid into the MiniMap as a `cells`
                            // override. Falls back to the seeded blob
                            // layout while pending or on error.
                            let thumb_cache =
                                crate::components::thumbnail::use_thumbnail_cache();
                            let thumb_game_id = g.game_id.clone();
                            // Only kick off the fetch when the game has
                            // a non-empty server_url (orchestrator
                            // already produced a backing instance).
                            if !g.server_url.is_empty() {
                                crate::components::thumbnail::ensure_fetched(
                                    &thumb_cache,
                                    thumb_game_id.clone(),
                                );
                            }
                            let cells = match thumb_cache.get_entry(&thumb_game_id) {
                                Some(crate::components::thumbnail::ThumbnailEntry::Ready(g)) => {
                                    Some(g)
                                }
                                _ => None,
                            };
                            view! {
                                <div class=tile_class style="width:100%">
                                    <div class="tile-head">
                                        <div>
                                            <div class="tile-name">{g.name.clone()}</div>
                                            <div class="leader">{format!("{} · {}", g.leader, g.civ_id)}</div>
                                        </div>
                                        <div class="row gap-xs">
                                            {(g.players_human > 1).then(|| view! { <Tag>"MP"</Tag> })}
                                        </div>
                                    </div>
                                    <div class="map-thumb">
                                        <MiniMap
                                            seed=seed
                                            style="position:absolute; inset:0; width:100%; height:100%"
                                            cells=cells
                                        />
                                    </div>
                                    <div class="stats">
                                        <div class="row-stat"><span class="k">"turn"</span><span class="v">{g.turn.to_string()}</span></div>
                                        <div class="row-stat"><span class="k">"era"</span><span class="v">{g.era.clone()}</span></div>
                                        <div class="row-stat"><span class="k">"diff"</span><span class="v">{g.difficulty.clone()}</span></div>
                                        <div class="row-stat"><span class="k">"score"</span><span class="v">{g.score.to_string()}</span></div>
                                        <div class="row-stat"><span class="k">"players"</span><span class="v">{players}</span></div>
                                        <div class="row-stat"><span class="k">"last"</span><span class="v">{g.last_played_at.clone().unwrap_or_else(|| "—".into())}</span></div>
                                    </div>
                                    <div class="actions">
                                        {notes_popup(g.game_id.clone(), g.name.clone(), g.notes.clone(), tick)}
                                        <span style="flex:1"></span>
                                        <Btn
                                            variant=resume_variant
                                            size="sm"
                                            disabled=disabled_signal
                                            on_click=on_resume
                                        >{resume_label}</Btn>
                                        {tile_menu_popup(g.clone(), tick)}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>();
                        view! { <div class="games-grid">{rows}</div> }.into_any()
                    })}
                </Suspense>
            </div>
        </div>
    }
}

/// Click-trigger popup wrapping a tile's "···" button.
///
/// Items: View summary (TODO popup-in-popup) / Copy game ID
/// (writes to navigator.clipboard) / Share invite link (TODO) /
/// Archive (TODO — needs a status mutation route) / Resign
/// (DELETE /games/{id} + bump tick).
///
/// Resign and Copy game ID are wired today; Archive / View summary
/// / Share invite are visible-but-inert pending their underlying
/// routes / popups.
fn tile_menu_popup(g: games_api::GameView, tick: RwSignal<u32>) -> impl IntoView {
    let game = Arc::new(g);

    let content = {
        let game = game.clone();
        Arc::new(move || {
            // Local state inside the popup content lets us flip between
            // the action menu and the summary view without re-rendering
            // the whole popup. The popup's pinned state survives the
            // flip because PopupRender re-runs the renderer.
            let view_mode = RwSignal::new(SummaryMode::Menu);
            let game = game.clone();

            // Suppress dead_code on PopupList — used elsewhere
            // (StepPlayers slot menu).
            let _ = (PopupListItem::sep, PopupList);

            view! {
                {move || match view_mode.get() {
                    SummaryMode::Menu => render_menu_rows(game.clone(), view_mode, tick),
                    SummaryMode::Summary => render_summary_kv(game.clone(), view_mode),
                }}
            }
            .into_any()
        })
    };

    view! {
        <Popup
            title="Game"
            size=PopupSize::Narrow
            trigger=PopupTrigger::Click
            content=content
        >
            <Btn variant="ghost" size="sm">"···"</Btn>
        </Popup>
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum SummaryMode {
    Menu,
    Summary,
}

fn render_menu_rows(
    game: Arc<games_api::GameView>,
    view_mode: RwSignal<SummaryMode>,
    tick: RwSignal<u32>,
) -> AnyView {
    let id_resign = game.game_id.clone();
    let id_copy = game.game_id.clone();

    view! {
        <div class="popup-list">
            <button
                class="item"
                type="button"
                on:click=move |_| {
                    let id = id_copy.clone();
                    if let Some(win) = web_sys::window() {
                        let nav = win.navigator();
                        let _ = nav.clipboard().write_text(&id);
                    }
                }
            >
                <span class="icon">"⎘"</span>
                <span>"Copy game ID"</span>
            </button>
            <button
                class="item"
                type="button"
                on:click=move |_| view_mode.set(SummaryMode::Summary)
            >
                <span class="icon">"◑"</span>
                <span>"View summary"</span>
            </button>
            <button class="item" type="button">
                <span class="icon">"↗"</span>
                <span>"Share invite link"</span>
                <span class="desc">"(TODO)"</span>
            </button>
            <div class="sep"></div>
            <button class="item" type="button">
                <span class="icon">"⊟"</span>
                <span>"Archive"</span>
                <span class="desc">"(TODO)"</span>
            </button>
            <button
                class="item"
                type="button"
                on:click=move |_| {
                    let id = id_resign.clone();
                    spawn_local(async move {
                        let _ = games_api::delete_game(&id).await;
                        tick.update(|t| *t += 1);
                    });
                }
            >
                <span class="icon">"⊗"</span>
                <span>"Resign / delete"</span>
            </button>
        </div>
    }
    .into_any()
}

fn render_summary_kv(game: Arc<games_api::GameView>, view_mode: RwSignal<SummaryMode>) -> AnyView {
    let players = format!("{}H · {}AI", game.players_human, game.players_ai);
    let last = game.last_played_at.clone().unwrap_or_else(|| "—".into());
    view! {
        <div class="popup-body">
            <div class="kv xsmall">
                <span class="k">"name"</span><span>{game.name.clone()}</span>
                <span class="k">"id"</span><span style="font-family:var(--font-mono)">{game.game_id.clone()}</span>
                <span class="k">"leader"</span><span>{format!("{} · {}", game.leader, game.civ_id)}</span>
                <span class="k">"map"</span>
                <span>{format!("{} · {}", game.map_type, game.map_size)}</span>
                <span class="k">"seed"</span>
                <span style="font-family:var(--font-mono); font-size:10px">{game.seed.clone()}</span>
                <span class="k">"difficulty"</span><span>{game.difficulty.clone()}</span>
                <span class="k">"turn"</span><span>{game.turn.to_string()}</span>
                <span class="k">"era"</span><span>{game.era.clone()}</span>
                <span class="k">"score"</span><span>{game.score.to_string()}</span>
                <span class="k">"status"</span><span>{game.status.clone()}</span>
                <span class="k">"players"</span><span>{players}</span>
                <span class="k">"created"</span><span>{game.created_at.clone()}</span>
                <span class="k">"last played"</span><span>{last}</span>
            </div>
        </div>
        <PopupActions>
            <Btn
                variant="bare"
                size="sm"
                on_click=Callback::new(move |_| view_mode.set(SummaryMode::Menu))
            >"← back"</Btn>
        </PopupActions>
    }
    .into_any()
}

/// Click-trigger popup wrapping the tile's "📝 Notes" button.
/// The popup's textarea is seeded with `initial_notes` from the
/// loaded GameView; clicking Save posts /api/v1/games/{id}/notes
/// and bumps `tick` so the list reloads with the persisted value.
fn notes_popup(
    game_id: String,
    game_name: String,
    initial_notes: String,
    tick: RwSignal<u32>,
) -> impl IntoView {
    let id = Arc::new(game_id);
    let title = Arc::new(game_name);
    let initial = Arc::new(initial_notes);

    let content = Arc::new(move || {
        let id = id.clone();
        let title = title.clone();
        let draft = RwSignal::new((*initial).clone());
        let save_state = RwSignal::new(NotesSaveState::Idle);
        let id_for_save = id.clone();
        let on_save = move |_| {
            save_state.set(NotesSaveState::Saving);
            let id = id_for_save.clone();
            let body = draft.get_untracked();
            spawn_local(async move {
                match games_api::set_notes(&id, body).await {
                    Ok(()) => {
                        save_state.set(NotesSaveState::Saved);
                        tick.update(|t| *t += 1);
                    }
                    Err(e) => save_state.set(NotesSaveState::Error(e.to_string())),
                }
            });
        };
        let saving = Signal::derive(move || matches!(save_state.get(), NotesSaveState::Saving));

        view! {
            <PopupBody>
                <p class="xsmall muted" style="letter-spacing:0.06em; text-transform:uppercase; margin-bottom:6px">
                    "Notes — " {(*title).clone()}
                </p>
                <textarea
                    class="input"
                    rows="6"
                    placeholder="markdown ok — your private notes for this game"
                    prop:value=move || draft.get()
                    on:input=move |ev| {
                        use wasm_bindgen::JsCast as _;
                        if let Some(el) = ev.target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
                        {
                            draft.set(el.value());
                        }
                    }
                ></textarea>
                {move || match save_state.get() {
                    NotesSaveState::Saved => view! {
                        <p class="xsmall" style="color:var(--good); margin-top:6px">"saved ✓"</p>
                    }.into_any(),
                    NotesSaveState::Error(msg) => view! {
                        <p class="xsmall" style="color:var(--accent); margin-top:6px">{msg}</p>
                    }.into_any(),
                    _ => view! { <span /> }.into_any(),
                }}
            </PopupBody>
            <PopupActions right=true>
                <Btn
                    variant="primary"
                    size="sm"
                    disabled=saving
                    on_click=Callback::new(on_save)
                >
                    {move || if saving.get() { "saving…" } else { "save" }}
                </Btn>
            </PopupActions>
        }
        .into_any()
    });

    view! {
        <Popup
            title="Notes"
            trigger=PopupTrigger::Click
            content=content
        >
            <Btn variant="ghost" size="sm">"📝 Notes"</Btn>
        </Popup>
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum NotesSaveState {
    #[default]
    Idle,
    Saving,
    Saved,
    Error(String),
}

// ───────────────────────────── URL query state ────────────────────────────────

#[derive(Clone, Debug)]
struct QueryState {
    filter: Filter,
    sort: Sort,
    q: String,
}

impl Default for QueryState {
    fn default() -> Self {
        Self {
            filter: Filter::YourTurn,
            sort: Sort::Recent,
            q: String::new(),
        }
    }
}

fn read_query_state() -> QueryState {
    let Some(win) = web_sys::window() else {
        return QueryState::default();
    };
    let Ok(search) = win.location().search() else {
        return QueryState::default();
    };
    let q_str = search.strip_prefix('?').unwrap_or(&search);
    let mut state = QueryState::default();
    for pair in q_str.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        let v = decode_query(v);
        match k {
            "filter" => {
                if let Some(f) = Filter::parse(&v) {
                    state.filter = f;
                }
            }
            "sort" => {
                if let Some(s) = Sort::parse(&v) {
                    state.sort = s;
                }
            }
            "q" => state.q = v,
            _ => {}
        }
    }
    state
}

fn push_query_state(state: &QueryState) {
    let Some(win) = web_sys::window() else { return };
    let Ok(history) = win.history() else { return };

    let mut parts = Vec::with_capacity(3);
    // Only include non-default values to keep the URL short.
    if state.filter != Filter::YourTurn {
        parts.push(format!("filter={}", state.filter.slug()));
    }
    if state.sort != Sort::Recent {
        parts.push(format!("sort={}", state.sort.slug()));
    }
    if !state.q.is_empty() {
        parts.push(format!("q={}", encode_query(&state.q)));
    }

    let path = win.location().pathname().unwrap_or_else(|_| "/".into());
    let url = if parts.is_empty() {
        path
    } else {
        format!("{path}?{}", parts.join("&"))
    };

    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
}

/// Minimal percent-encoder for the query-value half — covers the
/// characters likely to appear in a search string without dragging in
/// the `url` crate.
fn encode_query(s: &str) -> String {
    const SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if SAFE.contains(&b) {
            out.push(b as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

fn decode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(h) =
                    u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
                {
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
