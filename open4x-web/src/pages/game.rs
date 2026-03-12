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
    // (StoredValue mutation for the non-reactive field.)
    Effect::new(move |_| {
        let uid = selected_unit.get();
        session.update_value(|s| s.selected_unit = uid);
    });

    // End-turn handler.
    // FIXME: Should POST /api/games/{id}/actions/end-turn and apply returned diff.
    let on_end_turn = move || {
        session.update_value(|s| {
            use libciv::{TurnEngine, DefaultRulesEngine};
            let engine = TurnEngine::new();
            let rules  = DefaultRulesEngine;
            engine.process_turn(&mut s.state, &rules);
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
                    />
                </div>

                // Detail sidebar
                <Sidebar
                    tick=tick
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
    session: StoredValue<Session>,
    selected_tile: RwSignal<Option<HexCoord>>,
    selected_unit: RwSignal<Option<libciv::UnitId>>,
) -> impl IntoView {
    view! {
        <div class="sidebar">
            <TileInfo tick=tick session=session selected_tile=selected_tile />
            <UnitInfo tick=tick session=session selected_unit=selected_unit />
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

            let terrain    = tile.terrain.as_def().name().to_string();
            let (q, r)     = (coord.q, coord.r);
            let food       = tile.total_yields().food;
            let prod       = tile.total_yields().production;
            let gold       = tile.total_yields().gold;

            // City and ownership — owned Strings to escape the borrow.
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

            // FIXME: unit_type_name lookup duplicates civsim/src/main.rs::unit_type_name().
            //        Should be part of a shared view-model helper once the
            //        API layer is introduced.
            let type_name = s.unit_type_ids.iter()
                .position(|&tid| tid == unit.unit_type)
                .and_then(|i| s.state.unit_type_defs.get(i))
                .map(|def| def.name)
                .unwrap_or("Unknown");

            let owner_name = s.state.civilizations.iter()
                .find(|c| c.id == unit.owner)
                .map(|c| c.name)
                .unwrap_or("?");

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
                </div>
            }.into_any()
        })
    };

    view! { <div>{content}</div> }
}
