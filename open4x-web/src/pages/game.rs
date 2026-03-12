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
use libciv::civ::Unit;
use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::{StateDelta, RulesError, recalculate_visibility};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::hexmap::HexMap;
use crate::session::{Session, build_session};

// ---------------------------------------------------------------------------
// GamePage
// ---------------------------------------------------------------------------

#[component]
pub fn GamePage(on_quit: impl Fn() + 'static) -> impl IntoView {
    // Non-Clone game session held in StoredValue; tick drives re-renders.
    let session      = StoredValue::new(build_session());
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
                            if let StateDelta::UnitMoved { unit, to, cost, .. } = delta {
                                if let Some(u) = s.state.unit_mut(*unit) {
                                    u.coord = *to;
                                    u.movement_left = u.movement_left.saturating_sub(*cost);
                                }
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

    // End-turn handler: process turn, reset movement, recalculate visibility.
    let on_end_turn = move || {
        session.update_value(|s| {
            use libciv::TurnEngine;
            let engine = TurnEngine::new();
            let rules  = DefaultRulesEngine;
            engine.process_turn(&mut s.state, &rules);
            // Reset movement for the new turn.
            for unit in &mut s.state.units {
                unit.movement_left = unit.max_movement;
            }
            let civ_id = s.civ_id;
            recalculate_visibility(&mut s.state, civ_id);
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

    view! {
        <div class="game-topbar">
            <span class="civ-name">{civ_name}</span>
            <span class="turn-label">{turn_label}</span>
            <span class="turn-label">{gold_label}</span>
            <div style="flex:1" />
            <button class="btn btn-primary" on:click=move |_| on_end_turn()>
                "End Turn"
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
            <UnitInfo tick=tick set_tick=set_tick session=session selected_unit=selected_unit />
            <TechPanel tick=tick session=session />
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
                .map_or(false, |c| c.explored_tiles.contains(&coord));

            if !is_explored {
                return view! { <p class="no-selection">"Unexplored."</p> }.into_any();
            }

            let terrain    = tile.terrain.as_def().name().to_string();
            let (q, r)     = (coord.q, coord.r);
            let food       = tile.total_yields().food;
            let prod       = tile.total_yields().production;
            let gold       = tile.total_yields().gold;

            let city_name: String = s.state.cities.iter()
                .find(|c| c.coord == coord)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            let owner_name: String = tile.owner
                .and_then(|id| s.state.civilizations.iter().find(|c| c.id == id))
                .map(|c| c.name.to_string())
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
                    {(!city_name.is_empty()).then(|| view! {
                        <div class="info-row">
                            <span class="info-label">"City"</span>
                            <span>{city_name}</span>
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
// UnitInfo panel
// ---------------------------------------------------------------------------

#[component]
fn UnitInfo(
    tick: ReadSignal<u32>,
    set_tick: WriteSignal<u32>,
    session: StoredValue<Session>,
    selected_unit: RwSignal<Option<libciv::UnitId>>,
) -> impl IntoView {
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
            let has_movement = unit.movement_left() > 0;

            let owner_name = s.state.civilizations.iter()
                .find(|c| c.id == unit.owner)
                .map(|c| c.name)
                .unwrap_or("?");
            let is_owned = unit.owner == s.civ_id;

            view! {
                <div>
                    <h3>"Unit"</h3>
                    <div class="info-row">
                        <span class="info-label">"Type"</span>
                        <span>{type_name}</span>
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

                    // Move hint for owned units.
                    {(is_owned && has_movement && !can_settle).then(|| view! {
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
// TechPanel — available technologies
// ---------------------------------------------------------------------------

#[component]
fn TechPanel(
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

            // Available techs: prerequisites met and not yet researched.
            let mut available: Vec<&'static str> = s.state.tech_tree.nodes.values()
                .filter(|node| {
                    !civ.researched_techs.contains(&node.id)
                        && s.state.tech_tree.prerequisites_met(node.id, &civ.researched_techs)
                })
                .map(|node| node.name)
                .collect();
            available.sort_unstable();

            // Current research progress.
            let in_progress: Option<String> = civ.research_queue.front().and_then(|prog| {
                s.state.tech_tree.get(prog.tech_id).map(|node| {
                    format!("{} ({}/{})", node.name, prog.progress, node.cost)
                })
            });

            // Completed count.
            let done = civ.researched_techs.len();

            view! {
                <div style="margin-top:12px">
                    <h3 style="margin-bottom:4px">"Technologies"</h3>
                    <p style="font-size:11px;color:#888;margin:0 0 6px">
                        {format!("{} researched", done)}
                    </p>
                    {in_progress.map(|label| view! {
                        <div class="info-row" style="color:#ffe066">
                            <span class="info-label">"Researching"</span>
                            <span>{label}</span>
                        </div>
                    })}
                    {if available.is_empty() {
                        view! {
                            <p style="font-size:11px;color:#888">"No techs available."</p>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                <p style="font-size:11px;color:#aaa;margin:4px 0 2px">"Available:"</p>
                                <ul style="margin:0;padding-left:14px;font-size:12px">
                                    {available.into_iter().map(|name| view! {
                                        <li>{name}</li>
                                    }).collect::<Vec<_>>()}
                                </ul>
                            </div>
                        }.into_any()
                    }}
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}
