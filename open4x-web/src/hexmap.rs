/// SVG hex-grid renderer.
///
/// Renders the WorldBoard as pointy-top hexagons.  Each hex is a `<polygon>`
/// coloured by terrain type.  Fog of war is applied as a dark overlay:
///   - Unexplored tiles: nearly-opaque black overlay, no markers.
///   - Explored-but-foggy: semi-transparent overlay, city outline still shown.
///   - Fully visible: no overlay, all markers shown.
///
/// Territory is rendered on top of the fog overlay so it is always legible:
///   - A semi-transparent tinted polygon fills each owned tile.
///   - Thick coloured lines are drawn along every edge that separates an
///     owned tile from an unowned (or off-map) tile, forming the civ border.
///     Each civilization gets a deterministic colour from a fixed palette.
///
/// Clicking a hex fires `on_hex_click(coord)`.
///
/// Coordinate system: axial (q, r) → pixel using pointy-top layout:
///   px = size * sqrt(3) * (q + r / 2)
///   py = size * 3/2     * r
use std::collections::HashMap;
use std::sync::Arc;
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
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Compute the total SVG canvas size needed for a board of given width/height.
pub fn svg_dimensions(board_w: u32, board_h: u32) -> (f64, f64) {
    let (max_x, max_y) = axial_to_pixel(board_w as i32 - 1, board_h as i32 - 1);
    (max_x + HEX_SIZE * 2.0, max_y + HEX_SIZE * 2.0)
}

// ---------------------------------------------------------------------------
// Territory rendering helpers
// ---------------------------------------------------------------------------

/// Axial offsets to the 6 hex neighbours, indexed 0-5:
///   0=East, 1=NE, 2=NW, 3=West, 4=SW, 5=SE
const NEIGHBOR_OFFSETS: [(i32, i32); 6] = [
    ( 1,  0),  // East
    ( 1, -1),  // NE
    ( 0, -1),  // NW
    (-1,  0),  // West
    (-1,  1),  // SW
    ( 0,  1),  // SE
];

/// For each neighbour direction (same indexing as NEIGHBOR_OFFSETS), the pair
/// of `hex_corners` indices that form the shared edge with that neighbour.
/// Corners are numbered 0-5 clockwise from the upper-right vertex:
///   0=upper-right, 1=lower-right, 2=bottom, 3=lower-left, 4=upper-left, 5=top
const BORDER_CORNER_PAIRS: [(usize, usize); 6] = [
    (0, 1),  // East  — right edge
    (5, 0),  // NE    — upper-right edge
    (4, 5),  // NW    — upper-left edge
    (3, 4),  // West  — left edge
    (2, 3),  // SW    — lower-left edge
    (1, 2),  // SE    — lower-right edge
];

/// Deterministic territory colour for a civilization by its index in the
/// `state.civilizations` vec.  The palette is chosen to be visually distinct
/// from the terrain colours and legible over the fog overlay.
fn civ_territory_color(civ_index: usize) -> (&'static str, &'static str) {
    // (fill colour, border/stroke colour)
    const PALETTE: &[(&str, &str)] = &[
        ("#4e7df4", "#3a6de0"),  // blue   (index 0 — player default)
        ("#e05050", "#c83030"),  // red
        ("#d4a800", "#b88e00"),  // gold
        ("#40b840", "#289028"),  // green
        ("#b040c0", "#8c28a0"),  // purple
        ("#e07030", "#c05010"),  // orange
    ];
    PALETTE[civ_index % PALETTE.len()]
}

// ---------------------------------------------------------------------------
// Terrain colours
// ---------------------------------------------------------------------------

fn terrain_fill(t: BuiltinTerrain) -> &'static str {
    match t {
        BuiltinTerrain::Grassland => "#3a6b45",
        BuiltinTerrain::Plains    => "#8a7d3a",
        BuiltinTerrain::Desert    => "#c8a84b",
        BuiltinTerrain::Tundra    => "#6b7a5e",
        BuiltinTerrain::Snow      => "#d8e0e8",
        BuiltinTerrain::Coast     => "#3a7a9e",
        BuiltinTerrain::Ocean     => "#1a3a5e",
        BuiltinTerrain::Mountain  => "#5e5e5e",
    }
}

fn terrain_label(t: BuiltinTerrain) -> &'static str {
    match t {
        BuiltinTerrain::Grassland => "G",
        BuiltinTerrain::Plains    => "P",
        BuiltinTerrain::Desert    => "D",
        BuiltinTerrain::Tundra    => "T",
        BuiltinTerrain::Snow      => "S",
        BuiltinTerrain::Coast     => "C",
        BuiltinTerrain::Ocean     => "~",
        BuiltinTerrain::Mountain  => "M",
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
    /// Called when the user clicks a hex (before selection is updated).
    on_hex_click: impl Fn(HexCoord) + Send + Sync + 'static,
) -> impl IntoView {
    // Board dimensions are fixed for the lifetime of the session.
    let (board_w, board_h, svg_w, svg_h) = session.with_value(|s| {
        let w = s.state.board.width();
        let h = s.state.board.height();
        let (sw, sh) = svg_dimensions(w, h);
        (w, h, sw, sh)
    });

    // Arc allows cloning the callback into each per-hex click handler.
    let on_hex_click: Arc<dyn Fn(HexCoord) + Send + Sync> = Arc::new(on_hex_click);

    // Build one <g> per tile.  Re-derived on every tick.
    let hexes = move || {
        tick.get(); // reactive dependency

        let on_hex_click = on_hex_click.clone();
        session.with_value(|s| {
            let civ_id = s.civ_id;

            // Snapshot visibility sets for the rendering pass.
            let (visible, explored) = s.state.civilizations.iter()
                .find(|c| c.id == civ_id)
                .map(|c| (c.visible_tiles.clone(), c.explored_tiles.clone()))
                .unwrap_or_default();

            // Build territory mask from city.territory (the authoritative source).
            // Maps each claimed coord → the owning civ's palette index, so that
            // adjacent cities within the same civilization share a colour and do
            // not draw a border between each other.
            let territory_mask: HashMap<HexCoord, usize> = {
                let mut map = HashMap::new();
                for city in &s.state.cities {
                    let civ_idx = s.state.civilizations.iter()
                        .position(|c| c.id == city.owner)
                        .unwrap_or(0);
                    for &coord in &city.territory {
                        map.insert(coord, civ_idx);
                    }
                }
                map
            };

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

                    let is_visible  = visible.contains(&coord);
                    let is_explored = explored.contains(&coord);

                    let is_selected = selected_tile.get_untracked() == Some(coord);
                    let stroke      = if is_selected { "#ffffff" } else { "#000000" };
                    let stroke_w    = if is_selected { "2.5" } else { "0.8" };

                    // Unit on this tile (only shown when visible).
                    let unit_here: Option<UnitId> = if is_visible {
                        s.state.units.iter()
                            .find(|u| u.coord == coord)
                            .map(|u| u.id)
                    } else {
                        None
                    };

                    // City on this tile (shown when explored or visible).
                    let city_here = is_explored && s.state.cities.iter().any(|c| c.coord == coord);

                    let sel_unit = selected_unit;
                    let click_fn = on_hex_click.clone();

                    // Fog overlay opacity: none when visible, semi when explored, solid when unknown.
                    let fog_opacity: Option<&'static str> = if is_visible {
                        None
                    } else if is_explored {
                        Some("0.50")
                    } else {
                        Some("0.88")
                    };

                    // ── Territory tint + border lines ─────────────────────────────────
                    // Derived from city.territory (the authoritative per-city coord set),
                    // not from tile.owner.  Rendered after the fog overlay so borders
                    // are always legible regardless of exploration state.
                    let territory_civ_idx: Option<usize> = territory_mask.get(&coord).copied();
                    let territory_colors: Option<(&'static str, &'static str)> =
                        territory_civ_idx.map(civ_territory_color);

                    // For each of the 6 neighbour directions, draw a border segment when
                    // the neighbour does not belong to the same civilization's territory
                    // (including off-map neighbours, which always trigger a border).
                    let border_line_views = match territory_civ_idx {
                        None => Vec::new(),
                        Some(own_idx) => {
                            let (_, stroke_color) = territory_colors.unwrap();
                            (0..6usize).filter_map(|d| {
                                let (dq, dr) = NEIGHBOR_OFFSETS[d];
                                let nb_idx = s.state.board
                                    .normalize(HexCoord::from_qr(q + dq, r + dr))
                                    .and_then(|nc| territory_mask.get(&nc).copied());
                                if nb_idx != Some(own_idx) {
                                    let (ci, cj) = BORDER_CORNER_PAIRS[d];
                                    let (ax, ay) = corners[ci];
                                    let (bx, by) = corners[cj];
                                    Some(view! {
                                        <line
                                            x1=ax y1=ay x2=bx y2=by
                                            stroke=stroke_color
                                            stroke-width="2.5"
                                            stroke-linecap="round"
                                            pointer-events="none"
                                        />
                                    })
                                } else {
                                    None
                                }
                            }).collect::<Vec<_>>()
                        }
                    };

                    elems.push(view! {
                        <g>
                            <polygon
                                class="hex-cell"
                                points=points.clone()
                                fill=fill
                                stroke=stroke
                                stroke-width=stroke_w
                                on:click=move |_| {
                                    click_fn(coord);
                                }
                            />

                            // Terrain letter (only on visible tiles).
                            {is_visible.then(|| view! {
                                <text
                                    x=cx y={cy + HEX_SIZE * 0.58}
                                    text-anchor="middle"
                                    font-size="9"
                                    fill="rgba(0,0,0,0.45)"
                                    pointer-events="none"
                                >
                                    {label}
                                </text>
                            })}

                            // City marker: white diamond outline (visible when explored).
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
                                    pointer-events="none"
                                />
                            })}

                            // Unit dot (only when tile is fully visible).
                            {unit_here.map(|uid| {
                                let is_sel = sel_unit.get_untracked() == Some(uid);
                                let dot_fill = if is_sel { "#ffe066" } else { "#4e7df4" };
                                view! {
                                    <circle
                                        class="unit-dot"
                                        cx=cx cy=cy r="7"
                                        fill=dot_fill
                                        stroke="#fff"
                                        stroke-width="1.2"
                                        pointer-events="none"
                                    />
                                }
                            })}

                            // Fog of war overlay.
                            {fog_opacity.map(|opacity| view! {
                                <polygon
                                    points=points.clone()
                                    fill="#060810"
                                    fill-opacity=opacity
                                    stroke="none"
                                    pointer-events="none"
                                />
                            })}

                            // Territory tint (semi-transparent fill over the fog).
                            {territory_colors.map(|(fill_color, _)| view! {
                                <polygon
                                    points=points.clone()
                                    fill=fill_color
                                    fill-opacity="0.20"
                                    stroke="none"
                                    pointer-events="none"
                                />
                            })}

                            // Territory border lines (drawn last to stay on top).
                            {border_line_views}
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
