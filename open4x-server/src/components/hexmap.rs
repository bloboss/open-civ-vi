/// SVG hex-grid renderer backed by `GameView`.
///
/// Renders the board as pointy-top hexagons coloured by terrain type.
/// Fog of war is applied via TileVisibility on each TileView.
/// Territory borders are derived from tile ownership.
///
/// Coordinate system: axial (q, r) → pixel using pointy-top layout:
///   px = size * sqrt(3) * (q + r / 2)
///   py = size * 3/2     * r
use std::collections::HashMap;
use std::sync::Arc;
use leptos::prelude::*;
use crate::types::enums::{BuiltinTerrain, TileVisibility};
use crate::types::ids::{CivId, UnitId};
use crate::types::coord::HexCoord;
use crate::types::view::GameView;

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

fn corners_to_points(corners: &[(f64, f64); 6]) -> String {
    corners.iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn svg_dimensions(board_w: u32, board_h: u32) -> (f64, f64) {
    let (max_x, max_y) = axial_to_pixel(board_w as i32 - 1, board_h as i32 - 1);
    (max_x + HEX_SIZE * 2.0, max_y + HEX_SIZE * 2.0)
}

// ---------------------------------------------------------------------------
// Territory rendering helpers
// ---------------------------------------------------------------------------

const NEIGHBOR_OFFSETS: [(i32, i32); 6] = [
    ( 1,  0), ( 1, -1), ( 0, -1), (-1,  0), (-1,  1), ( 0,  1),
];

const BORDER_CORNER_PAIRS: [(usize, usize); 6] = [
    (0, 1), (5, 0), (4, 5), (3, 4), (2, 3), (1, 2),
];

fn civ_territory_color(civ_index: usize) -> (&'static str, &'static str) {
    const PALETTE: &[(&str, &str)] = &[
        ("#4e7df4", "#3a6de0"),
        ("#e05050", "#c83030"),
        ("#d4a800", "#b88e00"),
        ("#40b840", "#289028"),
        ("#b040c0", "#8c28a0"),
        ("#e07030", "#c05010"),
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
    game_view: ReadSignal<Option<GameView>>,
    selected_tile: RwSignal<Option<HexCoord>>,
    selected_unit: RwSignal<Option<UnitId>>,
    on_hex_click: impl Fn(HexCoord) + Send + Sync + 'static,
) -> impl IntoView {
    let on_hex_click: Arc<dyn Fn(HexCoord) + Send + Sync> = Arc::new(on_hex_click);

    let hexes = move || {
        let on_hex_click = on_hex_click.clone();
        let Some(gv) = game_view.get() else {
            return Vec::new();
        };

        let board_w = gv.board.width;
        let board_h = gv.board.height;
        let my_civ = gv.my_civ_id;

        // Build a tile lookup by coord for fast access.
        let tile_map: HashMap<(i32, i32), &crate::types::view::TileView> = gv.board.tiles.iter()
            .map(|t| ((t.coord.q, t.coord.r), t))
            .collect();

        // Build civ-index map from cities for territory rendering.
        // Collect all unique civ owners, assign index by order of appearance.
        let mut civ_indices: HashMap<CivId, usize> = HashMap::new();
        civ_indices.insert(my_civ, 0);
        let mut next_idx = 1usize;
        for city in &gv.cities {
            if let std::collections::hash_map::Entry::Vacant(e) = civ_indices.entry(city.owner) {
                e.insert(next_idx);
                next_idx += 1;
            }
        }

        // Territory mask from city territories.
        let mut territory_mask: HashMap<(i32, i32), usize> = HashMap::new();
        for city in &gv.cities {
            let idx = *civ_indices.get(&city.owner).unwrap_or(&0);
            for coord in &city.territory {
                territory_mask.insert((coord.q, coord.r), idx);
            }
        }

        // Unit lookup by coord.
        let unit_map: HashMap<(i32, i32), &crate::types::view::UnitView> = gv.units.iter()
            .map(|u| ((u.coord.q, u.coord.r), u))
            .collect();

        // City lookup by coord.
        let city_map: HashMap<(i32, i32), &crate::types::view::CityView> = gv.cities.iter()
            .map(|c| ((c.coord.q, c.coord.r), c))
            .collect();

        let (svg_w, svg_h) = svg_dimensions(board_w, board_h);
        let _ = (svg_w, svg_h); // used by parent

        let mut elems = Vec::new();

        for r in 0..board_h as i32 {
            for q in 0..board_w as i32 {
                let coord = HexCoord::from_qr(q, r);
                let (cx, cy) = axial_to_pixel(q, r);
                let corners = hex_corners(cx, cy);
                let points = corners_to_points(&corners);

                // Check if tile is in the view (explored).
                let tile_opt = tile_map.get(&(q, r)).copied();

                let (fill, label, is_visible) = if let Some(tile) = tile_opt {
                    let vis = matches!(tile.visibility, TileVisibility::Visible);
                    (terrain_fill(tile.terrain), terrain_label(tile.terrain), vis)
                } else {
                    // Unexplored tile — dark.
                    ("#0a0a14", "", false)
                };

                let is_explored = tile_opt.is_some();
                let is_selected = selected_tile.get_untracked() == Some(coord);
                let stroke = if is_selected { "#ffffff" } else { "#000000" };
                let stroke_w = if is_selected { "2.5" } else { "0.8" };

                let unit_here = if is_visible {
                    unit_map.get(&(q, r)).map(|u| (u.id, u.is_own))
                } else {
                    None
                };

                let city_here = if is_explored {
                    city_map.get(&(q, r)).map(|c| c.is_own)
                } else {
                    None
                };

                let sel_unit = selected_unit;
                let click_fn = on_hex_click.clone();

                let fog_opacity: Option<&'static str> = if is_visible {
                    None
                } else if is_explored {
                    Some("0.50")
                } else {
                    Some("0.88")
                };

                let territory_civ_idx = territory_mask.get(&(q, r)).copied();
                let territory_colors = territory_civ_idx.map(civ_territory_color);

                let border_line_views = match territory_civ_idx {
                    None => Vec::new(),
                    Some(own_idx) => {
                        let (_, stroke_color) = territory_colors.unwrap();
                        (0..6usize).filter_map(|d| {
                            let (dq, dr) = NEIGHBOR_OFFSETS[d];
                            let nb_key = (q + dq, r + dr);
                            let nb_idx = territory_mask.get(&nb_key).copied();
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

                        {city_here.map(|is_friendly| {
                            let city_stroke = if is_friendly { "#ffffff" } else { "#e05050" };
                            view! {
                                <polygon
                                    class="city-marker"
                                    points=format!(
                                        "{cx},{top} {rx},{cy} {cx},{bot} {lx},{cy}",
                                        cx=cx, cy=cy,
                                        top=cy - 10.0, bot=cy + 10.0,
                                        lx=cx - 10.0, rx=cx + 10.0,
                                    )
                                    fill="none"
                                    stroke=city_stroke
                                    stroke-width="1.5"
                                    pointer-events="none"
                                />
                            }
                        })}

                        {unit_here.map(|(uid, is_friendly)| {
                            let is_sel = sel_unit.get_untracked() == Some(uid);
                            let dot_fill = if is_sel {
                                "#ffe066"
                            } else if is_friendly {
                                "#4e7df4"
                            } else {
                                "#e05050"
                            };
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

                        {fog_opacity.map(|opacity| view! {
                            <polygon
                                points=points.clone()
                                fill="#060810"
                                fill-opacity=opacity
                                stroke="none"
                                pointer-events="none"
                            />
                        })}

                        {territory_colors.map(|(fill_color, _)| view! {
                            <polygon
                                points=points.clone()
                                fill=fill_color
                                fill-opacity="0.20"
                                stroke="none"
                                pointer-events="none"
                            />
                        })}

                        {border_line_views}
                    </g>
                });
            }
        }
        elems
    };

    // We need the dimensions for the SVG; read from the game view.
    let dims = move || {
        game_view.get().map(|gv| svg_dimensions(gv.board.width, gv.board.height))
            .unwrap_or((800.0, 600.0))
    };

    view! {
        <svg
            width=move || dims().0
            height=move || dims().1
            viewBox=move || format!("0 0 {:.0} {:.0}", dims().0, dims().1)
            xmlns="http://www.w3.org/2000/svg"
        >
            {hexes}
        </svg>
    }
}
