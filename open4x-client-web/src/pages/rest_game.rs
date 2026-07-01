//! REST-driven single-player view.
//!
//! On mount this page calls `POST /api/v1/games/new` to bootstrap a game and
//! receive a bearer token; from then on every read uses
//! [`open4x_sdk::endpoints`] against `/api/v1/*`. Mutations bump a refresh
//! tick that drives every `LocalResource` to refetch in parallel. No
//! WebSocket is involved.
//!
//! Layout mirrors the legacy wireframe in
//! `docs/legacy-wireframe/4X Wireframes.html`: top resource bar, tab strip,
//! main hex viewport, right sidebar with selection + summaries, plus a
//! notifications/turn-queue drawer.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use open4x_sdk::endpoints as api;
use open4x_sdk::wasm::WasmClient;

use crate::components::hud::snapshot_map::SnapshotMap;
use open4x_protocol::v1::web::city_data::CityData;
use open4x_protocol::v1::web::empire_overview::EmpireOverview;
use open4x_protocol::v1::web::notifications::Notifications;
use open4x_protocol::v1::web::tech_tree::TechTreeView;
use open4x_protocol::v1::web::turn_queue::TurnQueue;
use open4x_protocol::v1::web::unit_data::UnitData;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tab {
    Map,
    City,
    Units,
    Tech,
    Civics,
    Government,
    Diplomacy,
    Empire,
    Victory,
}

impl Tab {
    const ALL: &'static [Tab] = &[
        Tab::Map,
        Tab::City,
        Tab::Units,
        Tab::Tech,
        Tab::Civics,
        Tab::Government,
        Tab::Diplomacy,
        Tab::Empire,
        Tab::Victory,
    ];

    fn label(self) -> &'static str {
        match self {
            Tab::Map => "Map",
            Tab::City => "Cities",
            Tab::Units => "Units",
            Tab::Tech => "Tech",
            Tab::Civics => "Civics",
            Tab::Government => "Government",
            Tab::Diplomacy => "Diplomacy",
            Tab::Empire => "Empire",
            Tab::Victory => "Victory",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Drawer {
    None,
    Notifications,
    TurnQueue,
}

#[component]
pub fn RestGamePage() -> impl IntoView {
    // ── Session state ────────────────────────────────────────────────────────
    let token = RwSignal::new(None::<String>);
    let bootstrap_error = RwSignal::new(None::<String>);
    let action_error = RwSignal::new(None::<String>);
    let tick = RwSignal::new(0u64);
    let active_tab = RwSignal::new(Tab::Map);
    let drawer = RwSignal::new(Drawer::None);
    let selected_tile = RwSignal::new(None::<(i32, i32)>);

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

    let empire = LocalResource::new(move || {
        let _ = tick.get();
        let tok = token.get();
        async move {
            let tok = tok?;
            api::empire::overview(&client(Some(&tok))).await.ok()
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
    let on_end_turn = move |_| {
        let tok = token.get();
        action_error.set(None);
        spawn_local(async move {
            let Some(tok) = tok else { return };
            match api::turn::end(&client(Some(&tok))).await {
                Ok(_) => tick.update(|t| *t += 1),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    };

    let on_pick_research = move |_| {
        let tok = token.get();
        action_error.set(None);
        spawn_local(async move {
            let Some(tok) = tok else { return };
            let c = client(Some(&tok));
            let Ok(tt) = api::tech::get(&c).await else {
                return;
            };
            let Some(first) = tt.techs.iter().find(|t| t.status == "available") else {
                return;
            };
            let body = api::tech::TechResearchBody {
                tech_id: first.id.clone(),
            };
            match api::tech::research(&c, &body).await {
                Ok(_) => tick.update(|t| *t += 1),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    };

    let on_dismiss_all = move |_| {
        let tok = token.get();
        spawn_local(async move {
            let Some(tok) = tok else { return };
            let _ = api::notifications::dismiss_all(&client(Some(&tok))).await;
            tick.update(|t| *t += 1);
        });
    };

    // ── Derived signals ─────────────────────────────────────────────────────
    let required_count = move || {
        turn_queue
            .get()
            .and_then(|w| {
                w.as_ref()
                    .map(|q| q.items.iter().filter(|i| i.required).count())
            })
            .unwrap_or(0)
    };

    let notif_count = move || {
        notifs
            .get()
            .and_then(|w| w.as_ref().map(|n| n.notifications.len()))
            .unwrap_or(0)
    };

    view! {
        <div class="game-layout">
            // ─────────────────────────── Top Bar ────────────────────────────
            <div class="game-topbar">
                <Suspense fallback=move || view! {
                    <span class="turn-label">"Connecting…"</span>
                }>
                    {move || player_state.get().map(|wrap| match wrap.as_ref() {
                        None => view! { <span class="turn-label">"…"</span> }.into_any(),
                        Some(ps) => {
                            let gold = ps.resources.gold.value.unwrap_or(0);
                            let gpt = ps.resources.gold.per_turn;
                            let spt = ps.resources.science.per_turn;
                            let cpt = ps.resources.culture.per_turn;
                            let fpt = ps.resources.faith.per_turn;
                            let turn = ps.turn;
                            let turn_max = ps.turn_max;
                            let era = ps.era.clone();
                            view! {
                                <span class="civ-name">
                                    {format!("Turn {turn}/{turn_max}")}
                                </span>
                                <span class="turn-label">{format!("Era: {era}")}</span>
                                <span class="turn-label">{format!("Gold {gold} ({gpt:+})")}</span>
                                <span class="turn-label">{format!("Sci {spt:+}")}</span>
                                <span class="turn-label">{format!("Cul {cpt:+}")}</span>
                                <span class="turn-label">{format!("Fai {fpt:+}")}</span>
                            }.into_any()
                        }
                    })}
                </Suspense>
                <div style="flex:1" />

                <button class="btn btn-ghost"
                    on:click=move |_| {
                        drawer.update(|d| *d = if *d == Drawer::Notifications {
                            Drawer::None
                        } else {
                            Drawer::Notifications
                        });
                    }>
                    {move || format!("Alerts ({})", notif_count())}
                </button>
                <button class="btn btn-ghost"
                    on:click=move |_| {
                        drawer.update(|d| *d = if *d == Drawer::TurnQueue {
                            Drawer::None
                        } else {
                            Drawer::TurnQueue
                        });
                    }>
                    {move || {
                        let r = required_count();
                        if r > 0 {
                            format!("Queue ({r}!)")
                        } else {
                            "Queue".to_string()
                        }
                    }}
                </button>
                <button class="btn btn-ghost" on:click=on_pick_research>
                    "Pick Research"
                </button>
                <button class="btn btn-primary" on:click=on_end_turn>
                    "End Turn"
                </button>
            </div>

            // ─────────────────────────── Tab Bar ────────────────────────────
            <div class="tab-bar">
                {Tab::ALL.iter().copied().map(|t| {
                    let label = t.label();
                    view! {
                        <button
                            class="tab-btn"
                            class:tab-active=move || active_tab.get() == t
                            on:click=move |_| active_tab.set(t)
                        >{label}</button>
                    }
                }).collect::<Vec<_>>()}
            </div>

            // ─────────────────────────── Body ───────────────────────────────
            <div class="game-main">
                // Main viewport (Map tab) or scrollable tab body
                <div class="hex-viewport" style="position:relative">
                    {move || bootstrap_error.get().map(|e| view! {
                        <p style="color:#e05050; padding:1rem">
                            {format!("Bootstrap failed: {e}")}
                        </p>
                    })}
                    {move || action_error.get().map(|e| view! {
                        <p style="color:#e0a050; padding:0.5rem 1rem">
                            {format!("Action failed: {e}")}
                        </p>
                    })}

                    <Suspense fallback=move || view! {
                        <p style="padding:1rem">"Loading map…"</p>
                    }>
                        {move || {
                            let snap_signal = Signal::derive(move || -> Option<WorldSnapshot> {
                                snapshot.get().and_then(|w| (*w).clone())
                            });
                            view! {
                                <div style="display:flex; height:100%;">
                                    <div style="flex:1; overflow:auto; padding:0.5rem;">
                                        <SnapshotMap snapshot=snap_signal selected=selected_tile />
                                    </div>
                                </div>
                            }
                        }}
                    </Suspense>

                    // Tab body overlay (anything but Map shows full-pane content)
                    {move || (active_tab.get() != Tab::Map).then(|| view! {
                        <div class="tab-content">
                            <div class="tab-body">
                                {move || render_tab_body(
                                    active_tab.get(),
                                    cities,
                                    units,
                                    tech,
                                    empire,
                                )}
                            </div>
                        </div>
                    })}
                </div>

                // Right sidebar (always-visible, world summary + selection info)
                <div class="sidebar">
                    <SidebarSummary
                        snapshot=snapshot
                        cities=cities
                        units=units
                        tech=tech
                        selected=selected_tile
                    />
                </div>
            </div>

            // ─────────────────────── Drawer (overlay) ───────────────────────
            {move || (drawer.get() != Drawer::None).then(|| view! {
                <div style="position:fixed; right:0; top:0; bottom:0; width:340px;
                            background:#13151e; border-left:1px solid #2e3248;
                            box-shadow: -4px 0 12px rgba(0,0,0,0.4); z-index:50;
                            display:flex; flex-direction:column;">
                    <div style="padding:0.6rem 1rem; border-bottom:1px solid #2e3248;
                                display:flex; align-items:center; gap:0.5rem;">
                        <h3 style="flex:1; margin:0">{move || match drawer.get() {
                            Drawer::Notifications => "Notifications",
                            Drawer::TurnQueue => "Turn Queue",
                            Drawer::None => "",
                        }}</h3>
                        {move || (drawer.get() == Drawer::Notifications).then(|| view! {
                            <button class="btn btn-ghost"
                                on:click=on_dismiss_all>
                                "Clear All"
                            </button>
                        })}
                        <button class="btn btn-ghost"
                            on:click=move |_| drawer.set(Drawer::None)>
                            "✕"
                        </button>
                    </div>
                    <div style="flex:1; overflow-y:auto; padding:0.5rem 1rem;">
                        {move || match drawer.get() {
                            Drawer::Notifications => render_notifications_drawer(notifs, token, tick).into_any(),
                            Drawer::TurnQueue => render_turn_queue_drawer(turn_queue).into_any(),
                            Drawer::None => view! { <span /> }.into_any(),
                        }}
                    </div>
                </div>
            })}
        </div>
    }
}

// ─────────────────── helper components / fragments ────────────────────────────

#[component]
fn SidebarSummary(
    snapshot: LocalResource<Option<WorldSnapshot>>,
    cities: LocalResource<Option<CityData>>,
    units: LocalResource<Option<UnitData>>,
    tech: LocalResource<Option<TechTreeView>>,
    selected: RwSignal<Option<(i32, i32)>>,
) -> impl IntoView {
    view! {
        <h3>"Selection"</h3>
        {move || match selected.get() {
            None => view! { <p class="no-selection">"Click a tile."</p> }.into_any(),
            Some((q, r)) => {
                let snap_opt: Option<WorldSnapshot> = snapshot.get().and_then(|w| (*w).clone());
                let tile = snap_opt.as_ref()
                    .and_then(|s| s.tiles.iter().find(|t| t.q == q && t.r == r).cloned());
                match tile {
                    None => view! {
                        <p class="no-selection">{format!("({q}, {r}) — unexplored")}</p>
                    }.into_any(),
                    Some(t) => {
                        let owner = t.owner.clone().unwrap_or_else(|| "—".into());
                        let resource = t.resource.clone().unwrap_or_else(|| "—".into());
                        let improvement = t.improvement.clone().unwrap_or_else(|| "—".into());
                        let city = t.city.as_ref().map(|c| {
                            let star = if c.capital { "★" } else { "" };
                            format!("{}{} (pop {})", star, c.name, c.pop)
                        }).unwrap_or_else(|| "—".into());
                        let unit = t.unit.as_ref().map(|u| {
                            format!("{} {}", u.kind, u.hp)
                        }).unwrap_or_else(|| "—".into());
                        view! {
                            <div class="info-row"><span class="info-label">"Coord"</span><span>{format!("{q},{r}")}</span></div>
                            <div class="info-row"><span class="info-label">"Terrain"</span><span>{t.terrain.clone()}</span></div>
                            <div class="info-row"><span class="info-label">"Owner"</span><span>{owner}</span></div>
                            <div class="info-row"><span class="info-label">"Resource"</span><span>{resource}</span></div>
                            <div class="info-row"><span class="info-label">"Improv"</span><span>{improvement}</span></div>
                            <div class="info-row"><span class="info-label">"City"</span><span>{city}</span></div>
                            <div class="info-row"><span class="info-label">"Unit"</span><span>{unit}</span></div>
                        }.into_any()
                    }
                }
            }
        }}

        <h3>"Cities"</h3>
        <Suspense fallback=move || view! { <p class="no-selection">"…"</p> }>
            {move || cities.get().map(|wrap| match wrap.as_ref() {
                None => view! { <p class="no-selection">"–"</p> }.into_any(),
                Some(c) if c.cities.is_empty() => view! {
                    <p class="no-selection">"None"</p>
                }.into_any(),
                Some(c) => {
                    let rows = c.cities.iter().map(|city| {
                        let label = if city.is_own {
                            format!("{} (pop {})", city.name, city.population)
                        } else {
                            format!("{} · {} (foreign)", city.name, city.owner)
                        };
                        view! { <div class="info-row"><span>{label}</span></div> }
                    }).collect::<Vec<_>>();
                    view! { <div>{rows}</div> }.into_any()
                }
            })}
        </Suspense>

        <h3>"Units"</h3>
        <Suspense fallback=move || view! { <p class="no-selection">"…"</p> }>
            {move || units.get().map(|wrap| match wrap.as_ref() {
                None => view! { <p class="no-selection">"–"</p> }.into_any(),
                Some(u) => {
                    let own = u.units.iter().filter(|x| x.is_own).count();
                    let total = u.units.len();
                    view! {
                        <p class="no-selection">
                            {format!("{own} own · {total} visible")}
                        </p>
                    }.into_any()
                }
            })}
        </Suspense>

        <h3>"Research"</h3>
        <Suspense fallback=move || view! { <p class="no-selection">"…"</p> }>
            {move || tech.get().map(|wrap| match wrap.as_ref() {
                None => view! { <p class="no-selection">"–"</p> }.into_any(),
                Some(tt) => {
                    let current = tt.techs.iter().find(|t| t.status == "current");
                    let label = match current {
                        Some(t) => {
                            let prog = t.progress.unwrap_or(0);
                            format!("{} {}/{}", t.name, prog, t.cost)
                        }
                        None => "(none queued)".into(),
                    };
                    let avail = tt.techs.iter().filter(|t| t.status == "available").count();
                    view! {
                        <p class="no-selection">{label}</p>
                        <p class="no-selection">{format!("{avail} available")}</p>
                    }.into_any()
                }
            })}
        </Suspense>
    }
}

fn render_tab_body(
    tab: Tab,
    cities: LocalResource<Option<CityData>>,
    units: LocalResource<Option<UnitData>>,
    tech: LocalResource<Option<TechTreeView>>,
    empire: LocalResource<Option<EmpireOverview>>,
) -> AnyView {
    match tab {
        Tab::Map => view! { <span /> }.into_any(),
        Tab::City => render_city_tab(cities).into_any(),
        Tab::Units => render_units_tab(units).into_any(),
        Tab::Tech => render_tech_tab(tech).into_any(),
        Tab::Empire => render_empire_tab(empire).into_any(),
        other => render_placeholder_tab(other.label()).into_any(),
    }
}

fn render_placeholder_tab(name: &'static str) -> impl IntoView {
    view! {
        <div class="placeholder-tab">
            <h2>{name}</h2>
            <p class="placeholder-msg">"Coming soon."</p>
            <p class="placeholder-hint">
                "This tab is wired to the REST surface but the visual port from "
                "the legacy wireframe is still pending. Track progress in "
                "book/src/roadmap/web-ui.md."
            </p>
        </div>
    }
}

fn render_city_tab(cities: LocalResource<Option<CityData>>) -> impl IntoView {
    view! {
        <div>
            <h2 style="color:#e8c87a; margin-bottom:1rem">"Cities"</h2>
            <Suspense fallback=move || view! { <p>"Loading…"</p> }>
                {move || cities.get().map(|wrap| match wrap.as_ref() {
                    None => view! { <p>"No data."</p> }.into_any(),
                    Some(c) if c.cities.is_empty() => view! {
                        <p class="empty-note">"No cities yet — found one with a Settler."</p>
                    }.into_any(),
                    Some(c) => {
                        let rows = c.cities.iter().map(|city| {
                            let cap = if city.capital { "★" } else { "" };
                            let queue = if city.production_queue.is_empty() {
                                "—".to_string()
                            } else {
                                city.production_queue.join(", ")
                            };
                            view! {
                                <tr>
                                    <td>{format!("{}{}", cap, city.name)}</td>
                                    <td>{if city.is_own { "you".into() } else { city.owner.clone() }}</td>
                                    <td>{city.population.to_string()}</td>
                                    <td>{format!("{}/{}", city.food_stored, city.food_to_grow)}</td>
                                    <td>{city.production_stored.to_string()}</td>
                                    <td>{queue}</td>
                                    <td>{format!("{}/{}", city.worked_tile_count, city.territory_count)}</td>
                                </tr>
                            }
                        }).collect::<Vec<_>>();
                        view! {
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Name"</th>
                                        <th>"Owner"</th>
                                        <th>"Pop"</th>
                                        <th>"Food / Grow"</th>
                                        <th>"Prod"</th>
                                        <th>"Queue"</th>
                                        <th>"Tiles"</th>
                                    </tr>
                                </thead>
                                <tbody>{rows}</tbody>
                            </table>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}

fn render_units_tab(units: LocalResource<Option<UnitData>>) -> impl IntoView {
    view! {
        <div>
            <h2 style="color:#e8c87a; margin-bottom:1rem">"Units"</h2>
            <Suspense fallback=move || view! { <p>"Loading…"</p> }>
                {move || units.get().map(|wrap| match wrap.as_ref() {
                    None => view! { <p>"No data."</p> }.into_any(),
                    Some(u) if u.units.is_empty() => view! {
                        <p class="empty-note">"No visible units."</p>
                    }.into_any(),
                    Some(u) => {
                        let rows = u.units.iter().map(|unit| {
                            let own = if unit.is_own { "you" } else { "foreign" };
                            let pos = format!("({}, {})", unit.position.q, unit.position.r);
                            let str_label = unit.combat_strength.map(|s| s.to_string()).unwrap_or_else(|| "—".into());
                            view! {
                                <tr>
                                    <td>{unit.name.clone()}</td>
                                    <td>{unit.kind.clone()}</td>
                                    <td>{own}</td>
                                    <td>{format!("{}/{}", unit.hp, unit.hp_max)}</td>
                                    <td>{format!("{}/{}", unit.mp, unit.mp_max)}</td>
                                    <td>{str_label}</td>
                                    <td>{pos}</td>
                                    <td>{unit.status.clone()}</td>
                                </tr>
                            }
                        }).collect::<Vec<_>>();
                        view! {
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Name"</th>
                                        <th>"Kind"</th>
                                        <th>"Owner"</th>
                                        <th>"HP"</th>
                                        <th>"MP"</th>
                                        <th>"Str"</th>
                                        <th>"Pos"</th>
                                        <th>"Status"</th>
                                    </tr>
                                </thead>
                                <tbody>{rows}</tbody>
                            </table>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}

fn render_tech_tab(tech: LocalResource<Option<TechTreeView>>) -> impl IntoView {
    view! {
        <div>
            <h2 style="color:#e8c87a; margin-bottom:1rem">"Tech Tree"</h2>
            <Suspense fallback=move || view! { <p>"Loading…"</p> }>
                {move || tech.get().map(|wrap| match wrap.as_ref() {
                    None => view! { <p>"No data."</p> }.into_any(),
                    Some(tt) => {
                        let nodes = tt.techs.iter().map(|t| {
                            let class = match t.status.as_str() {
                                "done"      => "tech-node tech-researched",
                                "current"   => "tech-node tech-in-progress",
                                "available" => "tech-node tech-available",
                                _           => "tech-node tech-locked",
                            };
                            let progress_label = match (t.status.as_str(), t.progress) {
                                ("current", Some(p)) => format!("{p}/{}", t.cost),
                                _ => format!("{} sci", t.cost),
                            };
                            view! {
                                <div class=class>
                                    <div class="tech-name">{t.name.clone()}</div>
                                    <div class="tech-cost">{progress_label}</div>
                                    <div class="tech-eureka">{t.unlocks.clone()}</div>
                                </div>
                            }
                        }).collect::<Vec<_>>();
                        view! { <div class="tech-grid">{nodes}</div> }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}

fn render_empire_tab(empire: LocalResource<Option<EmpireOverview>>) -> impl IntoView {
    view! {
        <div>
            <h2 style="color:#e8c87a; margin-bottom:1rem">"Empire Overview"</h2>
            <Suspense fallback=move || view! { <p>"Loading…"</p> }>
                {move || empire.get().map(|wrap| match wrap.as_ref() {
                    None => view! { <p>"No data."</p> }.into_any(),
                    Some(e) => {
                        let s = &e.summary;
                        let cities = e.cities.iter().map(|c| {
                            let cap = if c.capital { "★" } else { "" };
                            view! {
                                <tr>
                                    <td>{format!("{}{}", cap, c.name)}</td>
                                    <td>{c.pop.to_string()}</td>
                                </tr>
                            }
                        }).collect::<Vec<_>>();
                        view! {
                            <div class="city-stats">
                                <div class="stat-card">
                                    <div class="stat-label">"Cities"</div>
                                    <div class="stat-value">{s.cities.to_string()}</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"Population"</div>
                                    <div class="stat-value">{s.population.to_string()}</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"Treasury"</div>
                                    <div class="stat-value">
                                        {format!("{} ({:+})", s.treasury, s.treasury_per_turn)}
                                    </div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"Military"</div>
                                    <div class="stat-value">{s.military_units.to_string()}</div>
                                </div>
                            </div>
                            <table class="data-table" style="margin-top:1rem">
                                <thead>
                                    <tr><th>"City"</th><th>"Pop"</th></tr>
                                </thead>
                                <tbody>{cities}</tbody>
                            </table>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}

fn render_notifications_drawer(
    notifs: LocalResource<Option<Notifications>>,
    token: RwSignal<Option<String>>,
    tick: RwSignal<u64>,
) -> impl IntoView {
    view! {
        <Suspense fallback=move || view! { <p class="no-selection">"…"</p> }>
            {move || notifs.get().map(|wrap| match wrap.as_ref() {
                None => view! { <p class="no-selection">"–"</p> }.into_any(),
                Some(n) if n.notifications.is_empty() => view! {
                    <p class="empty-note">"No notifications."</p>
                }.into_any(),
                Some(n) => {
                    let items = n.notifications.iter().map(|item| {
                        let id = item.id.clone();
                        let on_click = move |_| {
                            let tok = token.get();
                            let id = id.clone();
                            spawn_local(async move {
                                let Some(tok) = tok else { return };
                                let _ = api::notifications::dismiss(&client(Some(&tok)), &id).await;
                                tick.update(|t| *t += 1);
                            });
                        };
                        let color = match item.kind.as_str() {
                            "good" => "#50c070",
                            "warn" => "#e0a050",
                            "bad" => "#e05050",
                            "accent" => "#e8c87a",
                            _ => "#bbb",
                        };
                        view! {
                            <div style="background:#1a1d27; border:1px solid #2e3248;
                                        border-radius:6px; padding:0.6rem; margin-bottom:0.5rem">
                                <div style="display:flex; align-items:start; gap:0.5rem">
                                    <div style:color=color style="font-weight:700">
                                        {item.title.clone()}
                                    </div>
                                    <div style="flex:1" />
                                    <button class="btn btn-ghost btn-sm" on:click=on_click>"✕"</button>
                                </div>
                                <div style="font-size:0.85rem; color:#9aa; margin-top:0.3rem">
                                    {item.desc.clone()}
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>();
                    view! { <div>{items}</div> }.into_any()
                }
            })}
        </Suspense>
    }
}

fn render_turn_queue_drawer(turn_queue: LocalResource<Option<TurnQueue>>) -> impl IntoView {
    view! {
        <Suspense fallback=move || view! { <p class="no-selection">"…"</p> }>
            {move || turn_queue.get().map(|wrap| match wrap.as_ref() {
                None => view! { <p class="no-selection">"–"</p> }.into_any(),
                Some(q) if q.items.is_empty() => view! {
                    <p class="empty-note">"Queue clear."</p>
                }.into_any(),
                Some(q) => {
                    let items = q.items.iter().map(|item| {
                        let border = if item.required { "#e05050" } else { "#2e3248" };
                        let label = if item.required { "REQUIRED" } else { "optional" };
                        let label_color = if item.required { "#e05050" } else { "#7a7f99" };
                        view! {
                            <div style="background:#1a1d27; border-radius:6px; padding:0.6rem; margin-bottom:0.5rem"
                                 style:border={format!("1px solid {border}")}>
                                <div style="display:flex; gap:0.5rem; align-items:start">
                                    <div style="font-weight:700">{item.title.clone()}</div>
                                    <div style="flex:1" />
                                    <div style:color=label_color
                                         style="font-size:0.7rem; text-transform:uppercase; letter-spacing:0.06em">
                                        {label}
                                    </div>
                                </div>
                                <div style="font-size:0.85rem; color:#9aa; margin-top:0.3rem">
                                    {item.desc.clone()}
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>();
                    view! { <div>{items}</div> }.into_any()
                }
            })}
        </Suspense>
    }
}

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
