/// Game page: top bar, tab bar, hex viewport, and tab content panels.
///
/// All game state comes from a `GameView` signal populated by the WebSocket
/// connection.  Mutations are sent as `ClientMessage::Action(...)` over WS.
use leptos::prelude::*;
use leptos::prelude::LocalStorage;

use crate::types::coord::HexCoord;
use crate::types::enums::*;
use crate::types::ids::*;
use crate::types::messages::{ClientMessage, GameAction, ServerMessage, CreateGameRequest};
use crate::types::view::*;

use crate::components::client_auth as auth;
use crate::components::hexmap::HexMap;
use crate::components::ws::WsClient;
use crate::components::session::GameConfig;

use crate::tabs::{
    GameTab, TabBar,
    city::CityTab,
    climate::ClimateTab,
    culture::CultureTab,
    data_reports::DataReportsTab,
    governors::GovernorsTab,
    great_people::GreatPeopleTab,
    players::PlayersTab,
    science::ScienceTab,
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// ---------------------------------------------------------------------------
// GamePage
// ---------------------------------------------------------------------------

#[component]
pub fn GamePage(
    game_config: RwSignal<GameConfig>,
    on_quit: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    // Game view signal populated by WebSocket messages.
    let (game_view, set_game_view) = signal(None::<GameView>);

    // UI selection state.
    let selected_tile = RwSignal::new(None::<HexCoord>);
    let selected_unit = RwSignal::new(None::<UnitId>);
    let active_tab = RwSignal::new(GameTab::Map);

    // Connection status.
    let (connected, set_connected) = signal(false);

    // Auth keypair.
    let (signing_key, pubkey) = auth::load_or_generate_keypair();
    let signing_key = StoredValue::<_, LocalStorage>::new_local(signing_key);

    // WebSocket client (established on mount).
    let ws_client: StoredValue<Option<WsClient>, LocalStorage> = StoredValue::new_local(None);

    // Connect to server.
    let config = game_config.get_untracked();
    let pubkey_bytes = pubkey.to_vec();

    // Spawn the WebSocket connection.
    // Try localhost:3001 by default; in production this would be configurable.
    let ws_url = "ws://127.0.0.1:3001/ws";

    let ws = WsClient::connect(ws_url, move |msg| {
        match msg {
            ServerMessage::Challenge { nonce } => {
                // Sign and authenticate.
                signing_key.with_value(|sk| {
                    let sig = auth::sign_challenge(sk, &nonce);
                    ws_client.with_value(|ws| {
                        if let Some(ws) = ws {
                            ws.send(&ClientMessage::Authenticate {
                                pubkey: pubkey_bytes.clone(),
                                signature: sig,
                            });
                        }
                    });
                });
            }
            ServerMessage::AuthSuccess { .. } => {
                set_connected.set(true);
                // Create the game.
                ws_client.with_value(|ws| {
                    if let Some(ws) = ws {
                        ws.send(&ClientMessage::CreateGame(CreateGameRequest {
                            name: "Game".into(),
                            width: config.width,
                            height: config.height,
                            seed: config.seed,
                            num_ai: config.num_ai,
                            max_players: 1,
                            turn_limit: Some(300),
                        }));
                    }
                });
            }
            ServerMessage::AuthFailure { reason } => {
                web_sys::console::error_1(&format!("Auth failed: {reason}").into());
            }
            ServerMessage::GameCreated { .. } => {
                // Game will be followed by GameJoined.
            }
            ServerMessage::GameJoined { view, .. } => {
                set_game_view.set(Some(view));
            }
            ServerMessage::GameUpdate(view) => {
                set_game_view.set(Some(view));
            }
            ServerMessage::TurnResolved { view, .. } => {
                set_game_view.set(Some(view));
            }
            ServerMessage::ActionResult { ok, error } => {
                if !ok
                    && let Some(e) = error
                {
                    web_sys::console::warn_1(&format!("Action failed: {e}").into());
                }
            }
            ServerMessage::GameOver { view } => {
                set_game_view.set(Some(view));
            }
            _ => {}
        }
    });

    if let Some(ws) = ws {
        ws_client.set_value(Some(ws));
    }

    // Hex click handler.
    let on_hex_click = move |coord: HexCoord| {
        let Some(gv) = game_view.get() else { return };

        // Check if we have a selected unit.
        let sel = selected_unit.get();

        if let Some(uid) = sel {
            // Check if our unit is at this coord — if so, just deselect.
            let own_unit_here = gv.units.iter().find(|u| u.id == uid);
            if own_unit_here.is_some_and(|u| u.coord == coord) {
                selected_tile.set(Some(coord));
                return;
            }

            // Check for enemy unit at destination.
            if let Some(enemy) = gv.units.iter().find(|u| u.coord == coord && !u.is_own) {
                ws_client.with_value(|ws| {
                    if let Some(ws) = ws {
                        ws.send(&ClientMessage::Action(GameAction::Attack {
                            attacker: uid,
                            defender: enemy.id,
                        }));
                    }
                });
                selected_tile.set(Some(coord));
                return;
            }

            // Check no friendly unit at destination.
            let friendly_at_dest = gv.units.iter().any(|u| u.coord == coord && u.is_own);
            if !friendly_at_dest {
                // Move.
                ws_client.with_value(|ws| {
                    if let Some(ws) = ws {
                        ws.send(&ClientMessage::Action(GameAction::MoveUnit {
                            unit: uid,
                            to: coord,
                        }));
                    }
                });
                selected_tile.set(Some(coord));
                return;
            }
        }

        // Select friendly unit on this tile.
        let unit_here = game_view.get().and_then(|gv| {
            gv.units.iter()
                .find(|u| u.coord == coord && u.is_own)
                .map(|u| u.id)
        });
        selected_tile.set(Some(coord));
        selected_unit.set(unit_here);
    };

    // End turn handler.
    let on_end_turn = move || {
        ws_client.with_value(|ws| {
            if let Some(ws) = ws {
                ws.send(&ClientMessage::EndTurn);
            }
        });
    };

    // Save handler (download GameView as JSON).
    let on_save = move || {
        if let Some(gv) = game_view.get() {
            let content = serde_json::to_string_pretty(&gv).unwrap_or_default();
            let filename = format!("gameview-turn{}.json", gv.turn);
            download_text(&filename, &content);
        }
    };

    let on_quit_back = on_quit.clone();
    let on_quit_topbar = on_quit.clone();

    view! {
        <div class="game-layout">
            <Show when=move || game_view.get().is_none()>
                <div class="page-center">
                    <div class="card">
                        <div class="title">
                            {move || if connected.get() { "Creating game..." } else { "Connecting to server..." }}
                        </div>
                        <p class="subtitle">"Make sure the server is running on localhost:3001"</p>
                        <button class="btn btn-ghost" on:click={
                            let q = on_quit_back.clone();
                            move |_| q()
                        }>
                            "Back"
                        </button>
                    </div>
                </div>
            </Show>
            <Show when=move || game_view.get().is_some()>
                <TopBar
                    game_view=game_view
                    on_end_turn=move || on_end_turn()
                    on_save=move || on_save()
                    on_quit=on_quit_topbar.clone()
                />
                <TabBar active_tab=active_tab />

                <div class="game-main">
                    // Map is always rendered as the background layer.
                    <div class="hex-viewport">
                        <HexMap
                            game_view=game_view
                            selected_tile=selected_tile
                            selected_unit=selected_unit
                            on_hex_click=on_hex_click
                        />
                    </div>

                    // When a non-Map tab is active, render its content over the map.
                    {move || {
                        match active_tab.get() {
                            GameTab::Map => {
                                // Show the tile/unit info sidebar on Map tab.
                                view! {
                                    <Sidebar
                                        game_view=game_view
                                        ws_client=ws_client
                                        selected_tile=selected_tile
                                        selected_unit=selected_unit
                                    />
                                }.into_any()
                            }
                            GameTab::DataReports(_) => {
                                view! { <DataReportsTab game_view=game_view active_tab=active_tab /> }.into_any()
                            }
                            GameTab::Science => {
                                view! { <ScienceTab game_view=game_view ws_client=ws_client /> }.into_any()
                            }
                            GameTab::Culture => {
                                view! { <CultureTab game_view=game_view ws_client=ws_client /> }.into_any()
                            }
                            GameTab::Governors => {
                                view! { <GovernorsTab /> }.into_any()
                            }
                            GameTab::GreatPeople => {
                                view! { <GreatPeopleTab /> }.into_any()
                            }
                            GameTab::Climate => {
                                view! { <ClimateTab /> }.into_any()
                            }
                            GameTab::Players => {
                                view! { <PlayersTab game_view=game_view /> }.into_any()
                            }
                            GameTab::City(cid) => {
                                view! { <CityTab city_id=cid game_view=game_view ws_client=ws_client active_tab=active_tab /> }.into_any()
                            }
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Download helper
// ---------------------------------------------------------------------------

fn download_text(filename: &str, content: &str) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(content));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("text/plain");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)
        .expect("Blob::new failed");
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .expect("URL.createObjectURL failed");
    let document = web_sys::window().unwrap().document().unwrap();
    let a = document.create_element("a").unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
    a.set_href(&url);
    a.set_download(filename);
    let body = document.body().unwrap();
    body.append_child(&a).unwrap();
    a.click();
    let _ = body.remove_child(&a);
    web_sys::Url::revoke_object_url(&url).unwrap();
}

// ---------------------------------------------------------------------------
// TopBar
// ---------------------------------------------------------------------------

#[component]
fn TopBar(
    game_view: ReadSignal<Option<GameView>>,
    on_end_turn: impl Fn() + 'static,
    on_save: impl Fn() + 'static,
    on_quit: impl Fn() + 'static,
) -> impl IntoView {
    let turn_label = move || {
        game_view.get().map(|gv| format!("Turn {}", gv.turn)).unwrap_or_default()
    };
    let civ_name = move || {
        game_view.get().map(|gv| gv.my_civ.name.clone()).unwrap_or_default()
    };
    let gold_label = move || {
        game_view.get().map(|gv| format!("{} gold", gv.my_civ.gold)).unwrap_or_default()
    };
    let ai_status = move || {
        game_view.get().and_then(|gv| {
            gv.other_civs.first().map(|c| {
                format!("{}: score {}", c.name, c.score)
            })
        })
    };

    view! {
        <div class="game-topbar">
            <span class="civ-name">{civ_name}</span>
            <span class="turn-label">{turn_label}</span>
            <span class="turn-label">{gold_label}</span>
            {move || ai_status().map(|s| view! {
                <span class="turn-label" style="color:#e05050">{s}</span>
            })}
            <div style="flex:1" />
            <button class="btn btn-primary" on:click=move |_| on_end_turn()>
                "End Turn"
            </button>
            <button class="btn btn-ghost" on:click=move |_| on_save()>
                "Save"
            </button>
            <button class="btn btn-ghost" on:click=move |_| on_quit()>
                "Quit"
            </button>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

#[component]
fn Sidebar(
    game_view: ReadSignal<Option<GameView>>,
    ws_client: StoredValue<Option<WsClient>, LocalStorage>,
    selected_tile: RwSignal<Option<HexCoord>>,
    selected_unit: RwSignal<Option<UnitId>>,
) -> impl IntoView {
    view! {
        <div class="sidebar">
            <TileInfo game_view=game_view selected_tile=selected_tile />
            <CityPanel game_view=game_view ws_client=ws_client selected_tile=selected_tile />
            <UnitInfo game_view=game_view ws_client=ws_client selected_unit=selected_unit />
            <YieldsPanel game_view=game_view />
            <TechPanel game_view=game_view ws_client=ws_client />
        </div>
    }
}

// ---------------------------------------------------------------------------
// TileInfo
// ---------------------------------------------------------------------------

#[component]
fn TileInfo(
    game_view: ReadSignal<Option<GameView>>,
    selected_tile: RwSignal<Option<HexCoord>>,
) -> impl IntoView {
    let content = move || {
        let Some(coord) = selected_tile.get() else {
            return view! { <p class="no-selection">"Click a tile to inspect."</p> }.into_any();
        };
        let Some(gv) = game_view.get() else {
            return view! { <p class="no-selection">"Loading..."</p> }.into_any();
        };

        let tile = gv.board.tiles.iter().find(|t| t.coord == coord);
        let Some(tile) = tile else {
            return view! { <p class="no-selection">"Unexplored."</p> }.into_any();
        };

        let terrain = format!("{:?}", tile.terrain);
        let (q, r) = (coord.q, coord.r);

        let owner_name: String = tile.owner
            .and_then(|id| {
                if gv.my_civ.id == id { Some(gv.my_civ.name.clone()) }
                else { gv.other_civs.iter().find(|c| c.id == id).map(|c| c.name.clone()) }
            })
            .unwrap_or_default();

        let improvement_name: String = tile.improvement
            .map(|i| format!("{i:?}"))
            .unwrap_or_default();

        view! {
            <div>
                <h3>"Tile"</h3>
                <div class="info-row">
                    <span class="info-label">"Coord"</span>
                    <span>{format!("({q}, {r})")}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"Terrain"</span>
                    <span>{terrain}</span>
                </div>
                {(!improvement_name.is_empty()).then(|| view! {
                    <div class="info-row">
                        <span class="info-label">"Improvement"</span>
                        <span>{improvement_name}</span>
                    </div>
                })}
                {(!owner_name.is_empty()).then(|| view! {
                    <div class="info-row">
                        <span class="info-label">"Owner"</span>
                        <span>{owner_name}</span>
                    </div>
                })}
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// CityPanel
// ---------------------------------------------------------------------------

#[component]
fn CityPanel(
    game_view: ReadSignal<Option<GameView>>,
    ws_client: StoredValue<Option<WsClient>, LocalStorage>,
    selected_tile: RwSignal<Option<HexCoord>>,
) -> impl IntoView {
    let content = move || {
        let Some(coord) = selected_tile.get() else {
            return view! { <span /> }.into_any();
        };
        let Some(gv) = game_view.get() else {
            return view! { <span /> }.into_any();
        };

        let city = gv.cities.iter().find(|c| c.coord == coord);
        let Some(city) = city else {
            return view! { <span /> }.into_any();
        };

        let city_name = city.name.clone();
        let city_id = city.id;
        let population = city.population;
        let food_stored = city.food_stored;
        let food_to_grow = city.food_to_grow;
        let prod_stored = city.production_stored;
        let capital_tag = if city.is_capital { " ★" } else { "" };

        let prod_info: Option<(String, u32)> = city.production_queue.first()
            .and_then(|item| match item {
                ProductionItemView::Unit(tid) => gv.unit_type_defs.iter()
                    .find(|d| d.id == *tid)
                    .map(|d| (capitalize(&d.name), d.production_cost)),
                _ => None,
            });

        let prod_label = prod_info.as_ref()
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "idle".to_string());
        let prod_cost_str = prod_info.as_ref()
            .map(|(_, cost)| format!("{prod_stored}/{cost}"))
            .unwrap_or_else(|| format!("{prod_stored}/—"));

        let worked_count = city.worked_tiles.len();

        let unit_defs: Vec<(UnitTypeId, String, u32)> = gv.unit_type_defs.iter()
            .map(|d| (d.id, capitalize(&d.name), d.production_cost))
            .collect();

        let is_own = city.is_own;

        view! {
            <div style="margin-top:10px;border-top:1px solid #333;padding-top:8px">
                <h3>{format!("{city_name}{capital_tag}")}</h3>
                <div class="info-row">
                    <span class="info-label">"Population"</span>
                    <span>{population}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"Food"</span>
                    <span>{format!("{food_stored}/{food_to_grow}")}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"Production"</span>
                    <span>{format!("{prod_label} [{prod_cost_str}]")}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"Worked"</span>
                    <span>{format!("{worked_count}/{population} tiles")}</span>
                </div>

                {is_own.then(|| {
                    view! {
                        <div>
                            <p style="font-size:11px;color:#aaa;margin:6px 0 2px">"Queue unit:"</p>
                            <div style="display:flex;flex-wrap:wrap;gap:3px">
                                {unit_defs.into_iter().map(|(type_id, name, cost)| {
                                    let name_clone = name.clone();
                                    view! {
                                        <button
                                            class="btn btn-ghost"
                                            style="font-size:10px;padding:2px 5px"
                                            on:click=move |_| {
                                                ws_client.with_value(|ws| {
                                                    if let Some(ws) = ws {
                                                        ws.send(&ClientMessage::Action(GameAction::QueueProduction {
                                                            city: city_id,
                                                            item: ProductionItemView::Unit(type_id),
                                                        }));
                                                    }
                                                });
                                            }
                                        >
                                            {format!("{name_clone} ({cost})")}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                            <button
                                class="btn btn-ghost"
                                style="font-size:10px;margin-top:3px;width:100%"
                                on:click=move |_| {
                                    ws_client.with_value(|ws| {
                                        if let Some(ws) = ws {
                                            ws.send(&ClientMessage::Action(GameAction::CancelProduction {
                                                city: city_id,
                                                index: 0,
                                            }));
                                        }
                                    });
                                }
                            >
                                "Cancel Production"
                            </button>
                        </div>
                    }
                })}
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// UnitInfo
// ---------------------------------------------------------------------------

#[component]
fn UnitInfo(
    game_view: ReadSignal<Option<GameView>>,
    ws_client: StoredValue<Option<WsClient>, LocalStorage>,
    selected_unit: RwSignal<Option<UnitId>>,
) -> impl IntoView {
    let content = move || {
        let Some(uid) = selected_unit.get() else {
            return view! { <span /> }.into_any();
        };
        let Some(gv) = game_view.get() else {
            return view! { <span /> }.into_any();
        };
        let unit = gv.units.iter().find(|u| u.id == uid);
        let Some(unit) = unit else {
            return view! { <span /> }.into_any();
        };

        let type_name = gv.unit_type_defs.iter()
            .find(|d| d.id == unit.unit_type)
            .map(|d| capitalize(&d.name))
            .unwrap_or_else(|| "Unknown".into());

        let owner_name = if unit.is_own {
            gv.my_civ.name.clone()
        } else {
            gv.other_civs.iter().find(|c| c.id == unit.owner)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Unknown".into())
        };

        let hp = unit.health;
        let move_left = unit.movement_left;
        let move_max = unit.max_movement;
        let cs = unit.combat_strength;
        let is_own = unit.is_own;
        let can_found = gv.unit_type_defs.iter()
            .find(|d| d.id == unit.unit_type)
            .is_some_and(|d| d.can_found_city);
        let is_builder = unit.category == UnitCategory::Civilian && !can_found
            && unit.category != UnitCategory::Trader;

        let settler_uid = uid;

        view! {
            <div style="margin-top:10px;border-top:1px solid #333;padding-top:8px">
                <h3>{type_name}</h3>
                <div class="info-row">
                    <span class="info-label">"Owner"</span>
                    <span>{owner_name}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"HP"</span>
                    <span>{hp}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"Move"</span>
                    <span>{format!("{move_left}/{move_max}")}</span>
                </div>
                {cs.map(|s| view! {
                    <div class="info-row">
                        <span class="info-label">"Combat"</span>
                        <span>{s}</span>
                    </div>
                })}
                {(is_own && can_found && move_left > 0).then(|| view! {
                    <button
                        class="btn btn-primary"
                        style="font-size:10px;margin-top:4px;width:100%"
                        on:click=move |_| {
                            ws_client.with_value(|ws| {
                                if let Some(ws) = ws {
                                    ws.send(&ClientMessage::Action(GameAction::FoundCity {
                                        settler: settler_uid,
                                        name: "New City".into(),
                                    }));
                                }
                            });
                            selected_unit.set(None);
                        }
                    >
                        "Found City"
                    </button>
                })}
                {(is_own && is_builder).then(|| view! {
                    <div>
                        <p style="font-size:11px;color:#aaa;margin:4px 0 2px">"Build:"</p>
                        <div style="display:flex;flex-wrap:wrap;gap:3px">
                            {[BuiltinImprovement::Farm, BuiltinImprovement::Mine,
                              BuiltinImprovement::LumberMill, BuiltinImprovement::TradingPost]
                                .into_iter().map(|imp| {
                                    let label = format!("{imp:?}");
                                    let unit_coord = unit.coord;
                                    view! {
                                        <button
                                            class="btn btn-ghost"
                                            style="font-size:10px;padding:2px 5px"
                                            on:click=move |_| {
                                                ws_client.with_value(|ws| {
                                                    if let Some(ws) = ws {
                                                        ws.send(&ClientMessage::Action(GameAction::PlaceImprovement {
                                                            coord: unit_coord,
                                                            improvement: imp,
                                                        }));
                                                    }
                                                });
                                            }
                                        >
                                            {label}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                        </div>
                    </div>
                })}
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// YieldsPanel
// ---------------------------------------------------------------------------

#[component]
fn YieldsPanel(
    game_view: ReadSignal<Option<GameView>>,
) -> impl IntoView {
    let content = move || {
        let Some(gv) = game_view.get() else {
            return view! { <span /> }.into_any();
        };
        let y = &gv.my_civ.yields;
        let gold = gv.my_civ.gold;

        view! {
            <div style="margin-top:10px;border-top:1px solid #333;padding-top:8px">
                <h3>"Yields"</h3>
                <div class="info-row"><span class="info-label">"Food"</span><span>{format!("{:+}", y.food)}</span></div>
                <div class="info-row"><span class="info-label">"Prod"</span><span>{format!("{:+}", y.production)}</span></div>
                <div class="info-row"><span class="info-label">"Gold"</span><span>{format!("{:+}", y.gold)}</span></div>
                <div class="info-row"><span class="info-label">"Science"</span><span>{format!("{:+}", y.science)}</span></div>
                <div class="info-row"><span class="info-label">"Culture"</span><span>{format!("{:+}", y.culture)}</span></div>
                <div class="info-row"><span class="info-label">"Faith"</span><span>{format!("{:+}", y.faith)}</span></div>
                <div class="info-row"><span class="info-label">"Treasury"</span><span>{gold}</span></div>
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// TechPanel
// ---------------------------------------------------------------------------

#[component]
fn TechPanel(
    game_view: ReadSignal<Option<GameView>>,
    ws_client: StoredValue<Option<WsClient>, LocalStorage>,
) -> impl IntoView {
    let content = move || {
        let Some(gv) = game_view.get() else {
            return view! { <span /> }.into_any();
        };

        let researched = &gv.my_civ.researched_techs;
        let queue = &gv.my_civ.research_queue;
        let researched_count = researched.len();

        // Current research.
        let current_info = queue.first().map(|tp| {
            let name = gv.tech_tree.nodes.iter()
                .find(|n| n.id == tp.tech_id)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "?".into());
            let cost = gv.tech_tree.nodes.iter()
                .find(|n| n.id == tp.tech_id)
                .map(|n| n.cost)
                .unwrap_or(0);
            (name, tp.progress, cost)
        });

        // Available techs to queue.
        let queued_ids: Vec<TechId> = queue.iter().map(|tp| tp.tech_id).collect();
        let available: Vec<(TechId, String, u32)> = gv.tech_tree.nodes.iter()
            .filter(|n| {
                !researched.contains(&n.id)
                    && !queued_ids.contains(&n.id)
                    && n.prerequisites.iter().all(|p| researched.contains(p))
            })
            .map(|n| (n.id, n.name.clone(), n.cost))
            .collect();

        view! {
            <div style="margin-top:10px;border-top:1px solid #333;padding-top:8px">
                <h3>"Tech"</h3>
                <div class="info-row">
                    <span class="info-label">"Researched"</span>
                    <span>{researched_count}</span>
                </div>
                {current_info.map(|(name, progress, cost)| view! {
                    <div class="info-row">
                        <span class="info-label">"Current"</span>
                        <span>{format!("{name} [{progress}/{cost}]")}</span>
                    </div>
                })}
                {(!available.is_empty()).then(|| {
                    view! {
                        <div>
                            <p style="font-size:11px;color:#aaa;margin:6px 0 2px">"Research:"</p>
                            <div style="display:flex;flex-wrap:wrap;gap:3px">
                                {available.into_iter().map(|(tech_id, name, cost)| {
                                    let name_c = name.clone();
                                    view! {
                                        <button
                                            class="btn btn-ghost"
                                            style="font-size:10px;padding:2px 5px"
                                            on:click=move |_| {
                                                ws_client.with_value(|ws| {
                                                    if let Some(ws) = ws {
                                                        ws.send(&ClientMessage::Action(GameAction::QueueResearch {
                                                            tech: tech_id,
                                                        }));
                                                    }
                                                });
                                            }
                                        >
                                            {format!("{name_c} ({cost})")}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }
                })}
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}
