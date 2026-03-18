/// Game page: top bar, hex viewport, and detail sidebar.
///
/// Architecture note:
/// The game session is stored in a `StoredValue<Session>` so that the
/// non-Clone GameState can be held in a Leptos reactive context without
/// requiring Clone.  A lightweight `tick: RwSignal<u32>` is incremented
/// after every mutation to drive re-renders.
///
/// FIXME: In a client-server split:
///   - `StoredValue<Session>` becomes a `RwSignal<GameView>` populated by
///     API responses.
///   - Every mutation (end turn, move unit, …) sends a POST to the server
///     and updates the `GameView` signal with the returned snapshot.
use leptos::prelude::*;
use libciv::ai::{Agent, HeuristicAgent};
use libciv::civ::{ProductionItem, TechProgress, Unit};
use libciv::world::improvement::BuiltinImprovement;
use libciv::{DefaultRulesEngine, RulesEngine, UnitCategory};
use libciv::game::{StateDelta, RulesError, recalculate_visibility};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::hexmap::HexMap;
use crate::session::{GameConfig, Session, build_session};

// ---------------------------------------------------------------------------
// Debug save helper
// ---------------------------------------------------------------------------

/// Trigger a browser file-download of `content` as a plain-text `.txt` file.
///
/// Uses the standard Blob → object-URL → hidden `<a>` click pattern so the
/// file ends up in the user's Downloads folder without opening a new tab.
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

    let document = web_sys::window()
        .expect("no window")
        .document()
        .expect("no document");

    let a = document
        .create_element("a")
        .expect("create_element('a') failed")
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .expect("dyn_into::<HtmlAnchorElement> failed");
    a.set_href(&url);
    a.set_download(filename);

    let body = document.body().expect("no <body>");
    body.append_child(&a).expect("append_child failed");
    a.click();
    let _ = body.remove_child(&a);

    web_sys::Url::revoke_object_url(&url).expect("revokeObjectURL failed");
}

// ---------------------------------------------------------------------------
// GamePage
// ---------------------------------------------------------------------------

#[component]
pub fn GamePage(
    game_config: RwSignal<GameConfig>,
    on_quit: impl Fn() + 'static,
) -> impl IntoView {
    // Non-Clone game session held in StoredValue; tick drives re-renders.
    let session = StoredValue::new(build_session(&game_config.get_untracked()));
    let (tick, set_tick) = signal(0u32);

    // UI selection state (lightweight signals — always Clone-able).
    let selected_tile = RwSignal::new(None::<HexCoord>);
    let selected_unit = RwSignal::new(
        session.with_value(|s| s.selected_unit)
    );

    // Sync selected_unit back into the session whenever it changes.
    Effect::new(move |_| {
        let uid = selected_unit.get();
        session.update_value(|s| s.selected_unit = uid);
    });

    // Hex click handler: move, attack, or select depending on context.
    let on_hex_click = move |coord: HexCoord| {
        // Determine the action to take (read-only pass first).
        #[derive(Clone, Copy)]
        enum Action {
            Move,
            Attack(libciv::UnitId),
            Select,
        }

        let action = session.with_value(|s| {
            let Some(uid) = s.selected_unit else { return Action::Select };
            let Some(unit) = s.state.unit(uid) else { return Action::Select };
            if unit.coord == coord { return Action::Select }

            // Check for enemy unit at dest.
            if let Some(enemy) = s.state.units.iter()
                .find(|u| u.coord == coord && u.owner != s.civ_id)
            {
                return Action::Attack(enemy.id);
            }

            // No friendly unit at dest → move.
            let friendly_at_dest = s.state.units.iter()
                .any(|u| u.coord == coord && u.owner == s.civ_id);
            if friendly_at_dest { Action::Select } else { Action::Move }
        });

        match action {
            Action::Move => {
                let uid = session.with_value(|s| s.selected_unit).unwrap();
                session.update_value(|s| {
                    let civ_id = s.civ_id;
                    let rules  = DefaultRulesEngine;
                    let diff = match rules.move_unit(&s.state, uid, coord) {
                        Ok(d)                                       => Some(d),
                        Err(RulesError::InsufficientMovement(d))
                            if !d.is_empty()                        => Some(d),
                        _                                           => None,
                    };
                    if let Some(diff) = diff {
                        for delta in &diff.deltas {
                            if let StateDelta::UnitMoved { unit, to, cost, .. } = delta
                                && let Some(u) = s.state.unit_mut(*unit)
                            {
                                u.coord = *to;
                                u.movement_left = u.movement_left.saturating_sub(*cost);
                            }
                        }
                        recalculate_visibility(&mut s.state, civ_id);
                    }
                });
                selected_tile.set(Some(coord));
                set_tick.update(|n| *n += 1);
            }
            Action::Attack(enemy_id) => {
                let uid = session.with_value(|s| s.selected_unit).unwrap();
                session.update_value(|s| {
                    let rules = DefaultRulesEngine;
                    let _ = rules.attack(&mut s.state, uid, enemy_id);
                    if s.state.unit(uid).is_none() {
                        s.selected_unit = None;
                    }
                });
                // Sync selected_unit signal if unit was destroyed.
                let still_alive = session.with_value(|s| s.state.unit(uid).is_some());
                if !still_alive { selected_unit.set(None); }
                selected_tile.set(Some(coord));
                set_tick.update(|n| *n += 1);
            }
            Action::Select => {
                // Select the friendly unit on the clicked tile (if any).
                let unit_here = session.with_value(|s| {
                    s.state.units.iter()
                        .find(|u| u.coord == coord && u.owner == s.civ_id)
                        .map(|u| u.id)
                });
                selected_tile.set(Some(coord));
                selected_unit.set(unit_here);
            }
        }
    };

    // Save handler: dump the full GameState debug snapshot to a .txt download.
    let on_save = move || {
        session.with_value(|s| {
            let turn = s.state.turn;
            let content = format!("{:#?}", s.state);
            let filename = format!("gamestate-turn{turn}.txt");
            download_text(&filename, &content);
        });
    };

    // End-turn handler: process turn, run AI, reset movement, recalculate visibility.
    let on_end_turn = move || {
        session.update_value(|s| {
            use libciv::TurnEngine;
            let engine = TurnEngine::new();
            let rules  = DefaultRulesEngine;
            engine.process_turn(&mut s.state, &rules);

            // Reset movement for all units (player + AI).
            for unit in &mut s.state.units {
                unit.movement_left = unit.max_movement;
            }

            let civ_id = s.civ_id;
            recalculate_visibility(&mut s.state, civ_id);

            // AI adversary takes its turn.
            if let Some(ai_id) = s.ai_civ_id {
                let ai_agent = HeuristicAgent::new(ai_id);
                let _ai_diff = ai_agent.take_turn(&mut s.state, &rules);
                recalculate_visibility(&mut s.state, ai_id);
                // Re-check player visibility (AI may have moved into view).
                recalculate_visibility(&mut s.state, civ_id);
            }
        });
        set_tick.update(|n| *n += 1);
    };

    view! {
        <div class="game-layout">
            // Top bar
            <TopBar
                tick=tick
                session=session
                on_end_turn=move || on_end_turn()
                on_save=move || on_save()
                on_quit=on_quit
            />

            <div class="game-main">
                // Scrollable hex viewport
                <div class="hex-viewport">
                    <HexMap
                        tick=tick
                        session=session
                        selected_tile=selected_tile
                        selected_unit=selected_unit
                        on_hex_click=on_hex_click
                    />
                </div>

                // Detail sidebar
                <Sidebar
                    tick=tick
                    set_tick=set_tick
                    session=session
                    selected_tile=selected_tile
                    selected_unit=selected_unit
                />
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// TopBar
// ---------------------------------------------------------------------------

#[component]
fn TopBar(
    tick: ReadSignal<u32>,
    session: StoredValue<Session>,
    on_end_turn: impl Fn() + 'static,
    on_save: impl Fn() + 'static,
    on_quit: impl Fn() + 'static,
) -> impl IntoView {
    let turn_label = move || {
        tick.get();
        session.with_value(|s| format!("Turn {}", s.state.turn))
    };
    let civ_name = session.with_value(|s| {
        s.state.civilizations
            .iter()
            .find(|c| c.id == s.civ_id)
            .map(|c| c.name)
            .unwrap_or("Unknown")
    });
    let gold_label = move || {
        tick.get();
        session.with_value(|s| {
            s.state.civilizations
                .iter()
                .find(|c| c.id == s.civ_id)
                .map(|c| format!("{} gold", c.gold))
                .unwrap_or_default()
        })
    };

    let ai_status = move || {
        tick.get();
        session.with_value(|s| {
            s.ai_civ_id.map(|ai_id| {
                let name    = s.state.civilizations.iter().find(|c| c.id == ai_id)
                    .map(|c| c.name).unwrap_or("AI");
                let cities  = s.state.cities.iter().filter(|c| c.owner == ai_id).count();
                let units   = s.state.units.iter().filter(|u| u.owner == ai_id).count();
                format!("{name}: {cities}c {units}u")
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
    tick: ReadSignal<u32>,
    set_tick: WriteSignal<u32>,
    session: StoredValue<Session>,
    selected_tile: RwSignal<Option<HexCoord>>,
    selected_unit: RwSignal<Option<libciv::UnitId>>,
) -> impl IntoView {
    view! {
        <div class="sidebar">
            <TileInfo tick=tick session=session selected_tile=selected_tile />
            <CityPanel tick=tick set_tick=set_tick session=session selected_tile=selected_tile />
            <UnitInfo tick=tick set_tick=set_tick session=session selected_unit=selected_unit />
            <TradePanel tick=tick set_tick=set_tick session=session selected_unit=selected_unit />
            <YieldsPanel tick=tick session=session />
            <TechPanel tick=tick set_tick=set_tick session=session />
            <CivicsPanel tick=tick session=session />
        </div>
    }
}

// ---------------------------------------------------------------------------
// TileInfo panel
// ---------------------------------------------------------------------------

#[component]
fn TileInfo(
    tick: ReadSignal<u32>,
    session: StoredValue<Session>,
    selected_tile: RwSignal<Option<HexCoord>>,
) -> impl IntoView {
    let content = move || {
        tick.get();
        let Some(coord) = selected_tile.get() else {
            return view! {
                <p class="no-selection">"Click a tile to inspect."</p>
            }.into_any();
        };

        session.with_value(|s| {
            let Some(tile) = s.state.board.tile(coord) else {
                return view! { <p class="no-selection">"Off map."</p> }.into_any();
            };

            // Only show tile details if explored.
            let civ_id = s.civ_id;
            let is_explored = s.state.civilizations.iter()
                .find(|c| c.id == civ_id)
                .is_some_and(|c| c.explored_tiles.contains(&coord));

            if !is_explored {
                return view! { <p class="no-selection">"Unexplored."</p> }.into_any();
            }

            let terrain    = tile.terrain.name().to_string();
            let (q, r)     = (coord.q, coord.r);
            let food       = tile.total_yields().food;
            let prod       = tile.total_yields().production;
            let gold       = tile.total_yields().gold;

            let owner_name: String = tile.owner
                .and_then(|id| s.state.civilizations.iter().find(|c| c.id == id))
                .map(|c| c.name.to_string())
                .unwrap_or_default();

            // Skip city name here — CityPanel handles city tiles.
            let improvement_name: String = tile.improvement
                .map(|i| i.name().to_string())
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
                    <div class="info-row">
                        <span class="info-label">"Food"</span>
                        <span>{food}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Prod"</span>
                        <span>{prod}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Gold"</span>
                        <span>{gold}</span>
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
        })
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// CityPanel — shown when the selected tile has a city
// ---------------------------------------------------------------------------

#[component]
fn CityPanel(
    tick: ReadSignal<u32>,
    set_tick: WriteSignal<u32>,
    session: StoredValue<Session>,
    selected_tile: RwSignal<Option<HexCoord>>,
) -> impl IntoView {
    let content = move || {
        tick.get();
        let Some(coord) = selected_tile.get() else {
            return view! { <span /> }.into_any();
        };

        // Only render if a city sits on this tile and it's explored.
        let city_data = session.with_value(|s| {
            let civ_id = s.civ_id;
            let is_explored = s.state.civilizations.iter()
                .find(|c| c.id == civ_id)
                .is_some_and(|c| c.explored_tiles.contains(&coord));
            if !is_explored { return None; }
            s.state.cities.iter()
                .find(|c| c.coord == coord)
                .map(|c| c.id)
        });

        let Some(city_id) = city_data else {
            return view! { <span /> }.into_any();
        };

        session.with_value(|s| {
            let Some(city) = s.state.cities.iter().find(|c| c.id == city_id) else {
                return view! { <span /> }.into_any();
            };

            let city_name  = city.name.clone();
            let population = city.population;
            let food_stored = city.food_stored;
            let food_to_grow = city.food_to_grow;
            let prod_stored = city.production_stored;
            let capital_tag = if city.is_capital { " ★" } else { "" };

            // Production queue front item name and cost.
            let prod_info: Option<(String, u32)> = city.production_queue.front()
                .and_then(|item| match item {
                    ProductionItem::Unit(tid) => s.state.unit_type_defs.iter()
                        .find(|d| d.id == *tid)
                        .map(|d| (capitalize(d.name), d.production_cost)),
                    _ => None,
                });

            let prod_label = prod_info.as_ref()
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| "idle".to_string());
            let prod_cost_str = prod_info.as_ref()
                .map(|(_, cost)| format!("{prod_stored}/{cost}"))
                .unwrap_or_else(|| format!("{prod_stored}/—"));

            // Worked tiles count.
            let worked_count = city.worked_tiles.len();

            // Buildings.
            let building_count = city.buildings.len();

            // Available unit types for production.
            let unit_defs: Vec<(libciv::UnitTypeId, String, u32)> = s.state.unit_type_defs.iter()
                .map(|d| (d.id, capitalize(d.name), d.production_cost))
                .collect();

            // Available tiles to assign (rings 1-3, not yet worked, on-map).
            let available_tiles: Vec<HexCoord> = (1u32..=3)
                .flat_map(|r| city.coord.ring(r))
                .filter(|c| {
                    s.state.board.tile(*c).is_some() && !city.worked_tiles.contains(c)
                })
                .collect();

            // Worked tiles for unassign.
            let worked_tiles: Vec<HexCoord> = city.worked_tiles.clone();
            let city_center = city.coord;

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
                    {(building_count > 0).then(|| view! {
                        <div class="info-row">
                            <span class="info-label">"Buildings"</span>
                            <span>{building_count}</span>
                        </div>
                    })}

                    // Production queue management
                    <p style="font-size:11px;color:#aaa;margin:6px 0 2px">"Queue unit:"</p>
                    <div style="display:flex;flex-wrap:wrap;gap:3px">
                        {unit_defs.into_iter().map(|(type_id, name, cost)| {
                            let name_clone = name.clone();
                            view! {
                                <button
                                    class="btn btn-ghost"
                                    style="font-size:10px;padding:2px 5px"
                                    on:click=move |_| {
                                        session.update_value(|s| {
                                            if let Some(city) = s.state.cities.iter_mut()
                                                .find(|c| c.id == city_id)
                                            {
                                                city.production_queue
                                                    .push_back(ProductionItem::Unit(type_id));
                                            }
                                        });
                                        set_tick.update(|n| *n += 1);
                                    }
                                >
                                    {format!("{name_clone} ({cost})")}
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    // Cancel front production item
                    <button
                        class="btn btn-ghost"
                        style="font-size:10px;margin-top:3px;width:100%"
                        on:click=move |_| {
                            session.update_value(|s| {
                                if let Some(city) = s.state.cities.iter_mut()
                                    .find(|c| c.id == city_id)
                                {
                                    city.production_queue.pop_front();
                                    city.production_stored = 0;
                                }
                            });
                            set_tick.update(|n| *n += 1);
                        }
                    >
                        "Cancel Production"
                    </button>

                    // Citizen assignment
                    {(!available_tiles.is_empty()).then(|| {
                        let tiles = available_tiles.clone();
                        view! {
                            <div>
                                <p style="font-size:11px;color:#aaa;margin:6px 0 2px">"Assign citizen:"</p>
                                <div style="display:flex;flex-wrap:wrap;gap:3px">
                                    {tiles.into_iter().map(|tile_coord| {
                                        view! {
                                            <button
                                                class="btn btn-ghost"
                                                style="font-size:10px;padding:2px 5px"
                                                on:click=move |_| {
                                                    session.update_value(|s| {
                                                        let rules = DefaultRulesEngine;
                                                        let _ = rules.assign_citizen(
                                                            &mut s.state, city_id, tile_coord, false
                                                        );
                                                    });
                                                    set_tick.update(|n| *n += 1);
                                                }
                                            >
                                                {format!("({},{})", tile_coord.q, tile_coord.r)}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }
                    })}

                    // Citizen unassignment
                    {(!worked_tiles.is_empty()).then(|| {
                        let tiles = worked_tiles.clone();
                        view! {
                            <div>
                                <p style="font-size:11px;color:#aaa;margin:6px 0 2px">"Unassign citizen:"</p>
                                <div style="display:flex;flex-wrap:wrap;gap:3px">
                                    {tiles.into_iter()
                                        .filter(|c| *c != city_center)
                                        .map(|tile_coord| {
                                            view! {
                                                <button
                                                    class="btn btn-ghost"
                                                    style="font-size:10px;padding:2px 5px"
                                                    on:click=move |_| {
                                                        session.update_value(|s| {
                                                            if let Some(city) = s.state.cities.iter_mut()
                                                                .find(|c| c.id == city_id)
                                                            {
                                                                city.worked_tiles.retain(|c| *c != tile_coord);
                                                                city.locked_tiles.remove(&tile_coord);
                                                            }
                                                        });
                                                        set_tick.update(|n| *n += 1);
                                                    }
                                                >
                                                    {format!("({},{})", tile_coord.q, tile_coord.r)}
                                                </button>
                                            }
                                        }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }
                    })}
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// UnitInfo panel
// ---------------------------------------------------------------------------

#[component]
fn UnitInfo(
    tick: ReadSignal<u32>,
    set_tick: WriteSignal<u32>,
    session: StoredValue<Session>,
    selected_unit: RwSignal<Option<libciv::UnitId>>,
) -> impl IntoView {
    let improve_msg = RwSignal::new(None::<String>);

    let content = move || {
        tick.get();
        let Some(uid) = selected_unit.get() else {
            return view! { <span /> }.into_any();
        };

        session.with_value(|s| {
            let Some(unit) = s.state.unit(uid) else {
                return view! { <span /> }.into_any();
            };
            let (q, r) = (unit.coord.q, unit.coord.r);

            let type_def = s.state.unit_type_defs.iter()
                .find(|d| d.id == unit.unit_type);
            let type_name   = type_def.map(|d| d.name).unwrap_or("Unknown");
            let can_settle  = type_def.map(|d| d.can_found_city).unwrap_or(false);
            let is_builder  = type_name == "builder";
            let is_trader   = type_def.map(|d| d.category == UnitCategory::Trader).unwrap_or(false);
            let has_movement = unit.movement_left() > 0;

            let owner_name = s.state.civilizations.iter()
                .find(|c| c.id == unit.owner)
                .map(|c| c.name)
                .unwrap_or("?");
            let is_owned = unit.owner == s.civ_id;
            let unit_coord = unit.coord;
            let civ_id = s.civ_id;

            view! {
                <div>
                    <h3>"Unit"</h3>
                    <div class="info-row">
                        <span class="info-label">"Type"</span>
                        <span>{capitalize(type_name)}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Owner"</span>
                        <span>{owner_name}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Pos"</span>
                        <span>{format!("({q},{r})")}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"HP"</span>
                        <span>{unit.health}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Move"</span>
                        <span>{format!("{}/{}", unit.movement_left(), unit.max_movement())}</span>
                    </div>
                    {unit.combat_strength.map(|cs| view! {
                        <div class="info-row">
                            <span class="info-label">"Combat"</span>
                            <span>{cs}</span>
                        </div>
                    })}

                    // "Found City" button for settlers with movement remaining.
                    {(is_owned && can_settle && has_movement).then(|| {
                        view! {
                            <button
                                class="btn btn-primary"
                                style="margin-top:6px;width:100%"
                                on:click=move |_| {
                                    session.update_value(|s| {
                                        let civ_id = s.civ_id;
                                        let rules = DefaultRulesEngine;
                                        if let Ok(diff) = rules.found_city(
                                            &mut s.state, uid,
                                            format!("City {}", s.city_ids.len() + 1)
                                        ) {
                                            for delta in &diff.deltas {
                                                if let StateDelta::CityFounded { city, .. } = delta {
                                                    s.city_ids.push(*city);
                                                }
                                            }
                                            s.selected_unit = None;
                                            recalculate_visibility(&mut s.state, civ_id);
                                        }
                                    });
                                    selected_unit.set(None);
                                    set_tick.update(|n| *n += 1);
                                }
                            >
                                "Found City"
                            </button>
                        }
                    })}

                    // Improvement buttons for builder units.
                    {(is_owned && is_builder).then(|| {
                        let improvements = [
                            ("Farm",         BuiltinImprovement::Farm),
                            ("Mine",         BuiltinImprovement::Mine),
                            ("Lumber Mill",  BuiltinImprovement::LumberMill),
                            ("Trading Post", BuiltinImprovement::TradingPost),
                            ("Fort",         BuiltinImprovement::Fort),
                            ("Airstrip",     BuiltinImprovement::Airstrip),
                            ("Missile Silo", BuiltinImprovement::MissileSilo),
                        ];
                        view! {
                            <div>
                                <p style="font-size:11px;color:#aaa;margin:6px 0 2px">"Build improvement:"</p>
                                <div style="display:flex;flex-wrap:wrap;gap:3px">
                                    {improvements.into_iter().map(|(label, imp)| {
                                        view! {
                                            <button
                                                class="btn btn-ghost"
                                                style="font-size:10px;padding:2px 5px"
                                                on:click=move |_| {
                                                    let mut ok = false;
                                                    let mut err = String::new();
                                                    session.update_value(|s| {
                                                        let rules = DefaultRulesEngine;
                                                        match rules.place_improvement(
                                                            &mut s.state, civ_id, unit_coord, imp, None
                                                        ) {
                                                            Ok(_)  => { ok = true; }
                                                            Err(e) => { err = format!("{e}"); }
                                                        }
                                                    });
                                                    let msg = if ok {
                                                        format!("{label} built!")
                                                    } else {
                                                        format!("Error: {err}")
                                                    };
                                                    improve_msg.set(Some(msg));
                                                    set_tick.update(|n| *n += 1);
                                                }
                                            >
                                                {label}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                                {move || improve_msg.get().map(|m| view! {
                                    <p style="font-size:11px;color:#ffe066;margin-top:4px">{m}</p>
                                })}
                            </div>
                        }
                    })}

                    // Trader hint.
                    {(is_owned && is_trader).then(|| view! {
                        <p style="font-size:11px;color:#aaa;margin-top:4px">
                            "Move to a city, then use the Trade panel to establish a route."
                        </p>
                    })}

                    // Move hint for combat units.
                    {(is_owned && has_movement && !can_settle && !is_builder && !is_trader).then(|| view! {
                        <p style="font-size:11px;color:#aaa;margin-top:4px">
                            "Click a tile to move or attack."
                        </p>
                    })}
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// TradePanel — active trade routes + establish-route UI for traders
// ---------------------------------------------------------------------------

#[component]
fn TradePanel(
    tick: ReadSignal<u32>,
    set_tick: WriteSignal<u32>,
    session: StoredValue<Session>,
    selected_unit: RwSignal<Option<libciv::UnitId>>,
) -> impl IntoView {
    let trade_msg = RwSignal::new(None::<String>);

    let content = move || {
        tick.get();
        session.with_value(|s| {
            let civ_id = s.civ_id;

            // Active routes owned by this civ.
            let routes: Vec<(String, String, i32, Option<u32>)> = s.state.trade_routes.iter()
                .filter(|r| r.owner == civ_id)
                .map(|r| {
                    let orig = s.state.cities.iter().find(|c| c.id == r.origin)
                        .map(|c| c.name.clone()).unwrap_or_default();
                    let dest = s.state.cities.iter().find(|c| c.id == r.destination)
                        .map(|c| c.name.clone()).unwrap_or_default();
                    (orig, dest, r.origin_yields.gold, r.turns_remaining)
                })
                .collect();

            // Determine if the selected unit is an owned Trader at a city.
            let trader_at_city: Option<(libciv::UnitId, libciv::CityId)> =
                selected_unit.get_untracked()
                    .and_then(|uid| s.state.unit(uid))
                    .and_then(|u| {
                        if u.owner != civ_id { return None; }
                        let def = s.state.unit_type_defs.iter().find(|d| d.id == u.unit_type)?;
                        if def.category != UnitCategory::Trader { return None; }
                        let origin = s.state.cities.iter()
                            .find(|c| c.coord == u.coord && c.owner == civ_id)?;
                        Some((u.id, origin.id))
                    });

            // Available destinations: all explored cities except origin.
            let explored = s.state.civilizations.iter()
                .find(|c| c.id == civ_id)
                .map(|c| c.explored_tiles.clone())
                .unwrap_or_default();

            let dest_cities: Vec<(libciv::CityId, String)> =
                if let Some((_, origin_id)) = trader_at_city {
                    s.state.cities.iter()
                        .filter(|c| c.id != origin_id && explored.contains(&c.coord))
                        .map(|c| (c.id, c.name.clone()))
                        .collect()
                } else {
                    Vec::new()
                };

            // Only show establish UI when trader is at a city AND there are dests.
            let establish_uid: Option<libciv::UnitId> =
                trader_at_city.filter(|_| !dest_cities.is_empty()).map(|(id, _)| id);

            view! {
                <div style="margin-top:10px;border-top:1px solid #333;padding-top:8px">
                    <h3>"Trade"</h3>

                    // Active route list.
                    {if routes.is_empty() {
                        view! {
                            <p class="no-selection">"No active routes."</p>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                {routes.into_iter().map(|(orig, dest, gold, turns)| {
                                    let turns_str = match turns {
                                        Some(t) => format!("{t}t"),
                                        None    => "perm".to_string(),
                                    };
                                    view! {
                                        <div class="info-row" style="font-size:12px">
                                            <span>{format!("{orig}→{dest}")}</span>
                                            <span style="color:#aaa">{format!("+{gold}g {turns_str}")}</span>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }}

                    // Establish-route buttons (only when trader is at an origin city).
                    {establish_uid.map(|uid| {
                        view! {
                            <div>
                                <p style="font-size:11px;color:#aaa;margin:6px 0 2px">
                                    "Establish route to:"
                                </p>
                                <div style="display:flex;flex-wrap:wrap;gap:3px">
                                    {dest_cities.into_iter().map(|(dest_id, dest_name)| {
                                        let name_clone = dest_name.clone();
                                        view! {
                                            <button
                                                class="btn btn-ghost"
                                                style="font-size:10px;padding:2px 5px"
                                                on:click=move |_| {
                                                    let mut msg = String::new();
                                                    session.update_value(|s| {
                                                        let rules = DefaultRulesEngine;
                                                        match rules.establish_trade_route(
                                                            &mut s.state, uid, dest_id
                                                        ) {
                                                            Ok(_) => {
                                                                msg = format!(
                                                                    "Route to {} established!",
                                                                    dest_name
                                                                );
                                                                s.selected_unit = None;
                                                            }
                                                            Err(e) => {
                                                                msg = format!("Error: {e}");
                                                            }
                                                        }
                                                    });
                                                    trade_msg.set(Some(msg));
                                                    set_tick.update(|n| *n += 1);
                                                }
                                            >
                                                {name_clone}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }
                    })}

                    {move || trade_msg.get().map(|m| view! {
                        <p style="font-size:11px;color:#ffe066;margin-top:4px">{m}</p>
                    })}
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// YieldsPanel — per-turn yield breakdown
// ---------------------------------------------------------------------------

#[component]
fn YieldsPanel(
    tick: ReadSignal<u32>,
    session: StoredValue<Session>,
) -> impl IntoView {
    let content = move || {
        tick.get();
        session.with_value(|s| {
            let rules = DefaultRulesEngine;
            let y = rules.compute_yields(&s.state, s.civ_id);
            let civ = s.state.civilizations.iter().find(|c| c.id == s.civ_id);
            let treasury = civ.map(|c| c.gold).unwrap_or(0);

            view! {
                <div style="margin-top:10px;border-top:1px solid #333;padding-top:8px">
                    <h3 style="margin-bottom:4px">"Yields / turn"</h3>
                    <div class="info-row">
                        <span class="info-label">"Food"</span>
                        <span>{format!("{:+}", y.food)}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Production"</span>
                        <span>{format!("{:+}", y.production)}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Gold"</span>
                        <span>{format!("{:+} (treasury: {})", y.gold, treasury)}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Science"</span>
                        <span>{format!("{:+}", y.science)}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Culture"</span>
                        <span>{format!("{:+}", y.culture)}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Faith"</span>
                        <span>{format!("{:+}", y.faith)}</span>
                    </div>
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// TechPanel — available technologies with research queuing
// ---------------------------------------------------------------------------

#[component]
fn TechPanel(
    tick: ReadSignal<u32>,
    set_tick: WriteSignal<u32>,
    session: StoredValue<Session>,
) -> impl IntoView {
    let content = move || {
        tick.get();
        session.with_value(|s| {
            let civ = match s.state.civilizations.iter().find(|c| c.id == s.civ_id) {
                Some(c) => c,
                None    => return view! { <span /> }.into_any(),
            };

            // Available techs: prerequisites met and not yet researched or queued.
            let queued_ids: Vec<libciv::TechId> = civ.research_queue.iter()
                .map(|p| p.tech_id)
                .collect();
            let mut available: Vec<(libciv::TechId, &'static str)> = s.state.tech_tree.nodes.values()
                .filter(|node| {
                    !civ.researched_techs.contains(&node.id)
                        && !queued_ids.contains(&node.id)
                        && s.state.tech_tree.prerequisites_met(node.id, &civ.researched_techs)
                })
                .map(|node| (node.id, node.name))
                .collect();
            available.sort_unstable_by_key(|(_, name)| *name);

            // Current research progress.
            let in_progress: Option<String> = civ.research_queue.front().and_then(|prog| {
                s.state.tech_tree.get(prog.tech_id).map(|node| {
                    format!("{} ({}/{})", node.name, prog.progress, node.cost)
                })
            });

            // Queued (beyond front).
            let queue_tail: Vec<String> = civ.research_queue.iter().skip(1)
                .filter_map(|prog| s.state.tech_tree.get(prog.tech_id).map(|n| n.name.to_string()))
                .collect();

            // Completed count.
            let done = civ.researched_techs.len();

            view! {
                <div style="margin-top:12px;border-top:1px solid #333;padding-top:8px">
                    <h3 style="margin-bottom:4px">"Technologies"</h3>
                    <p style="font-size:11px;color:#888;margin:0 0 6px">
                        {format!("{done} researched")}
                    </p>
                    {in_progress.map(|label| view! {
                        <div class="info-row" style="color:#ffe066">
                            <span class="info-label">"Researching"</span>
                            <span>{label}</span>
                        </div>
                    })}
                    {(!queue_tail.is_empty()).then(|| view! {
                        <div>
                            <p style="font-size:11px;color:#aaa;margin:4px 0 2px">"Queued:"</p>
                            <ul style="margin:0;padding-left:14px;font-size:12px;color:#aaa">
                                {queue_tail.into_iter().map(|name| view! {
                                    <li>{name}</li>
                                }).collect::<Vec<_>>()}
                            </ul>
                        </div>
                    })}
                    {if available.is_empty() {
                        view! {
                            <p style="font-size:11px;color:#888">"No techs available."</p>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                <p style="font-size:11px;color:#aaa;margin:4px 0 2px">"Available (click to queue):"</p>
                                <div style="display:flex;flex-wrap:wrap;gap:3px">
                                    {available.into_iter().map(|(tech_id, name)| {
                                        view! {
                                            <button
                                                class="btn btn-ghost"
                                                style="font-size:10px;padding:2px 5px"
                                                on:click=move |_| {
                                                    session.update_value(|s| {
                                                        if let Some(civ) = s.state.civilizations
                                                            .iter_mut().find(|c| c.id == s.civ_id)
                                                            && !civ.researched_techs.contains(&tech_id)
                                                            && !civ.research_queue.iter()
                                                                .any(|p| p.tech_id == tech_id)
                                                        {
                                                            civ.research_queue.push_back(
                                                                TechProgress {
                                                                    tech_id,
                                                                    progress: 0,
                                                                    boosted: false,
                                                                }
                                                            );
                                                        }
                                                    });
                                                    set_tick.update(|n| *n += 1);
                                                }
                                            >
                                                {name}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }.into_any()
                    }}
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// CivicsPanel — completed civics and in-progress civic
// ---------------------------------------------------------------------------

#[component]
fn CivicsPanel(
    tick: ReadSignal<u32>,
    session: StoredValue<Session>,
) -> impl IntoView {
    let content = move || {
        tick.get();
        session.with_value(|s| {
            let civ = match s.state.civilizations.iter().find(|c| c.id == s.civ_id) {
                Some(c) => c,
                None    => return view! { <span /> }.into_any(),
            };

            let done = civ.completed_civics.len();

            let in_progress: Option<String> = civ.civic_in_progress.as_ref()
                .and_then(|prog| {
                    s.state.civic_tree.get(prog.civic_id).map(|node| {
                        format!("{} ({}/{})", node.name, prog.progress, node.cost)
                    })
                });

            view! {
                <div style="margin-top:12px;border-top:1px solid #333;padding-top:8px">
                    <h3 style="margin-bottom:4px">"Civics"</h3>
                    <p style="font-size:11px;color:#888;margin:0 0 6px">
                        {format!("{done} completed")}
                    </p>
                    {if let Some(label) = in_progress {
                        view! {
                            <div class="info-row" style="color:#ffe066">
                                <span class="info-label">"In progress"</span>
                                <span>{label}</span>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="info-row">
                                <span class="info-label">"In progress"</span>
                                <span style="color:#888">"none"</span>
                            </div>
                        }.into_any()
                    }}
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None    => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
