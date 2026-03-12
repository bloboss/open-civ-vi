/// SVG hex-grid renderer.
///
/// Renders the WorldBoard as pointy-top hexagons.  Each hex is a `<polygon>`
/// coloured by terrain type.  Units are shown as small filled circles.
/// City locations are shown as a white diamond outline.
/// Clicking a hex fires `on_select(coord)`.
///
/// Coordinate system: axial (q, r) → pixel using pointy-top layout:
///   px = size * sqrt(3) * (q + r / 2)
///   py = size * 3/2     * r
use leptos::prelude::*;
use libciv::world::terrain::BuiltinTerrain;
use libciv::UnitId;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::session::Session;

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

const HEX_SIZE: f64 = 28.0;
const OFFSET_X: f64 = 32.0;
const OFFSET_Y: f64 = 32.0;

/// Axial (q, r) → SVG pixel centre (pointy-top).
pub fn axial_to_pixel(q: i32, r: i32) -> (f64, f64) {
    let x = HEX_SIZE * (3.0_f64.sqrt() * q as f64 + 3.0_f64.sqrt() / 2.0 * r as f64) + OFFSET_X;
    let y = HEX_SIZE * (3.0 / 2.0 * r as f64) + OFFSET_Y;
    (x, y)
}

/// Six corner points for a pointy-top hex centred at (cx, cy).
fn hex_corners(cx: f64, cy: f64) -> [(f64, f64); 6] {
    std::array::from_fn(|i| {
        let angle = std::f64::consts::PI / 180.0 * (60.0 * i as f64 - 30.0);
        (cx + HEX_SIZE * angle.cos(), cy + HEX_SIZE * angle.sin())
    })
}

/// Format corner points as an SVG `points` attribute string.
fn corners_to_points(corners: &[(f64, f64); 6]) -> String {
    corners.iter()
        .map(|(x, y)| format!("{:.1},{:.1}", x, y))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Compute the total SVG canvas size needed for a board of given width/height.
pub fn svg_dimensions(board_w: u32, board_h: u32) -> (f64, f64) {
    let (max_x, max_y) = axial_to_pixel(board_w as i32 - 1, board_h as i32 - 1);
    (max_x + HEX_SIZE * 2.0, max_y + HEX_SIZE * 2.0)
}

// ---------------------------------------------------------------------------
// Terrain colours
// ---------------------------------------------------------------------------

// FIXME: Terrain-to-colour mapping belongs in frontend display logic only.
//        Once the server sends a `GameView`, the client picks colours by
//        matching the terrain name string returned by the view model.
fn terrain_fill(t: BuiltinTerrain) -> &'static str {
    match t {
        BuiltinTerrain::Grassland(_) => "#3a6b45",
        BuiltinTerrain::Plains(_)    => "#8a7d3a",
        BuiltinTerrain::Desert(_)    => "#c8a84b",
        BuiltinTerrain::Tundra(_)    => "#6b7a5e",
        BuiltinTerrain::Snow(_)      => "#d8e0e8",
        BuiltinTerrain::Coast(_)     => "#3a7a9e",
        BuiltinTerrain::Ocean(_)     => "#1a3a5e",
        BuiltinTerrain::Mountain(_)  => "#5e5e5e",
    }
}

fn terrain_label(t: BuiltinTerrain) -> &'static str {
    match t {
        BuiltinTerrain::Grassland(_) => "G",
        BuiltinTerrain::Plains(_)    => "P",
        BuiltinTerrain::Desert(_)    => "D",
        BuiltinTerrain::Tundra(_)    => "T",
        BuiltinTerrain::Snow(_)      => "S",
        BuiltinTerrain::Coast(_)     => "C",
        BuiltinTerrain::Ocean(_)     => "~",
        BuiltinTerrain::Mountain(_)  => "M",
    }
}

// ---------------------------------------------------------------------------
// HexMap component
// ---------------------------------------------------------------------------

#[component]
pub fn HexMap(
    /// Tick signal — increment after every state mutation to redraw the map.
    tick: ReadSignal<u32>,
    /// Shared game session (non-Clone; accessed via StoredValue).
    session: StoredValue<Session>,
    /// Currently selected hex tile.
    selected_tile: RwSignal<Option<HexCoord>>,
    /// Currently selected unit.
    selected_unit: RwSignal<Option<UnitId>>,
) -> impl IntoView {
    // Board dimensions are fixed for the lifetime of the session.
    let (board_w, board_h, svg_w, svg_h) = session.with_value(|s| {
        let w = s.state.board.width();
        let h = s.state.board.height();
        let (sw, sh) = svg_dimensions(w, h);
        (w, h, sw, sh)
    });

    // Build one <polygon> per tile.  We re-derive on every tick.
    let hexes = move || {
        tick.get(); // reactive dependency

        session.with_value(|s| {
            let mut elems: Vec<_> = Vec::new();

            for r in 0..board_h as i32 {
                for q in 0..board_w as i32 {
                    let coord = HexCoord::from_qr(q, r);
                    let Some(tile) = s.state.board.tile(coord) else { continue };

                    let (cx, cy) = axial_to_pixel(q, r);
                    let corners  = hex_corners(cx, cy);
                    let points   = corners_to_points(&corners);
                    let fill     = terrain_fill(tile.terrain);
                    let label    = terrain_label(tile.terrain);

                    let is_selected = selected_tile.get_untracked() == Some(coord);
                    let stroke      = if is_selected { "#ffffff" } else { "#000000" };
                    let stroke_w    = if is_selected { "2.5" } else { "0.8" };

                    // Unit on this tile (first owned unit wins for display).
                    let unit_here: Option<UnitId> = s.state.units.iter()
                        .find(|u| u.coord == coord)
                        .map(|u| u.id);

                    // City on this tile.
                    let city_here = s.state.cities.iter().any(|c| c.coord == coord);

                    let sel_tile = selected_tile;
                    let sel_unit = selected_unit;

                    elems.push(view! {
                        <g>
                            <polygon
                                class="hex-cell"
                                points=points.clone()
                                fill=fill
                                stroke=stroke
                                stroke-width=stroke_w
                                on:click=move |_| {
                                    sel_tile.set(Some(coord));
                                    sel_unit.set(unit_here);
                                }
                            />
                            // Terrain letter (tiny, bottom of hex).
                            <text
                                x=cx y={cy + HEX_SIZE * 0.58}
                                text-anchor="middle"
                                font-size="9"
                                fill="rgba(0,0,0,0.45)"
                                pointer-events="none"
                            >
                                {label}
                            </text>

                            // City marker: white diamond outline.
                            {city_here.then(|| view! {
                                <polygon
                                    class="city-marker"
                                    points=format!(
                                        "{cx},{top} {rx},{cy} {cx},{bot} {lx},{cy}",
                                        cx=cx, cy=cy,
                                        top=cy - 10.0, bot=cy + 10.0,
                                        lx=cx - 10.0, rx=cx + 10.0,
                                    )
                                    fill="none"
                                    stroke="#ffffff"
                                    stroke-width="1.5"
                                />
                            })}

                            // Unit dot.
                            {unit_here.map(|uid| {
                                let is_sel = selected_unit.get_untracked() == Some(uid);
                                let dot_fill = if is_sel { "#ffe066" } else { "#4e7df4" };
                                view! {
                                    <circle
                                        class="unit-dot"
                                        cx=cx cy=cy r="7"
                                        fill=dot_fill
                                        stroke="#fff"
                                        stroke-width="1.2"
                                    />
                                }
                            })}
                        </g>
                    });
                }
            }
            elems
        })
    };

    view! {
        <svg
            width=svg_w height=svg_h
            viewBox=format!("0 0 {:.0} {:.0}", svg_w, svg_h)
            xmlns="http://www.w3.org/2000/svg"
        >
            {hexes}
        </svg>
    }
}
