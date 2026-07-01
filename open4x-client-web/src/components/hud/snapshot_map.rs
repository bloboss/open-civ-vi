//! SVG hex renderer driven by [`open4x_protocol::v1::web::world::WorldSnapshot`].
//!
//! Pointy-top axial layout, mirror of `components/hexmap.rs`'s geometry but
//! reading the REST wire shape directly so the HexMap can render without a
//! `GameView` projection on the client.

use std::sync::Arc;

use leptos::prelude::*;

use open4x_protocol::v1::web::world::{TileView, WorldSnapshot};

const HEX_SIZE: f64 = 28.0;
const OFFSET_X: f64 = 32.0;
const OFFSET_Y: f64 = 32.0;

/// Axial (q, r) → SVG pixel centre (pointy-top).
fn axial_to_pixel(q: i32, r: i32) -> (f64, f64) {
    let x = HEX_SIZE * (3.0_f64.sqrt() * q as f64 + 3.0_f64.sqrt() / 2.0 * r as f64) + OFFSET_X;
    let y = HEX_SIZE * (3.0 / 2.0 * r as f64) + OFFSET_Y;
    (x, y)
}

fn hex_corners(cx: f64, cy: f64) -> [(f64, f64); 6] {
    std::array::from_fn(|i| {
        let angle = std::f64::consts::PI / 180.0 * (60.0 * i as f64 - 30.0);
        (cx + HEX_SIZE * angle.cos(), cy + HEX_SIZE * angle.sin())
    })
}

fn corners_to_points(corners: &[(f64, f64); 6]) -> String {
    corners
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn svg_dimensions(board_w: u32, board_h: u32) -> (f64, f64) {
    let (max_x, max_y) = axial_to_pixel(board_w as i32 - 1, board_h as i32 - 1);
    (max_x + HEX_SIZE * 2.0, max_y + HEX_SIZE * 2.0)
}

/// Map a server-side terrain string (e.g. `"Grassland"`, `"Plains+Hills"`) to
/// its base terrain. Hills modifier handled via [`is_hills`].
fn base_terrain(s: &str) -> &str {
    s.split_once('+').map(|(b, _)| b).unwrap_or(s)
}

fn is_hills(s: &str) -> bool {
    s.contains("Hills")
}

fn terrain_fill(s: &str) -> &'static str {
    match base_terrain(s) {
        "Grassland" => "#3a6b45",
        "Plains" => "#8a7d3a",
        "Desert" => "#c8a84b",
        "Tundra" => "#6b7a5e",
        "Snow" => "#d8e0e8",
        "Coast" => "#3a7a9e",
        "Ocean" => "#1a3a5e",
        "Mountain" => "#5e5e5e",
        _ => "#444",
    }
}

fn terrain_glyph(s: &str) -> &'static str {
    match base_terrain(s) {
        "Grassland" => "G",
        "Plains" => "P",
        "Desert" => "D",
        "Tundra" => "T",
        "Snow" => "S",
        "Coast" => "C",
        "Ocean" => "~",
        "Mountain" => "M",
        _ => "?",
    }
}

/// Render a hex map for a [`WorldSnapshot`].
///
/// `selected` is a `RwSignal<Option<(q, r)>>` shared with the parent so the
/// sidebar can react to clicks. `on_click` is a fall-through hook for things
/// the parent wants to do beyond setting `selected` (e.g. clearing other
/// selections).
#[component]
pub fn SnapshotMap(
    #[prop(into)] snapshot: Signal<Option<WorldSnapshot>>,
    selected: RwSignal<Option<(i32, i32)>>,
    #[prop(optional, into)] on_click: Option<Callback<(i32, i32)>>,
) -> impl IntoView {
    let on_click_arc: Arc<dyn Fn(i32, i32) + Send + Sync> = match on_click {
        Some(cb) => Arc::new(move |q, r| cb.run((q, r))),
        None => Arc::new(|_, _| {}),
    };

    let dims = move || {
        snapshot
            .get()
            .map(|s| svg_dimensions(s.world.width, s.world.height))
            .unwrap_or((800.0, 600.0))
    };

    let elems = move || {
        let on_click_arc = on_click_arc.clone();
        let Some(snap) = snapshot.get() else {
            return Vec::new();
        };

        let tile_lookup: std::collections::HashMap<(i32, i32), TileView> =
            snap.tiles.iter().map(|t| ((t.q, t.r), t.clone())).collect();

        let mut out = Vec::new();

        for r in 0..snap.world.height as i32 {
            for q in 0..snap.world.width as i32 {
                let (cx, cy) = axial_to_pixel(q, r);
                let corners = hex_corners(cx, cy);
                let points = corners_to_points(&corners);

                let tile = tile_lookup.get(&(q, r));
                let (fill, glyph, foggy, explored) = match tile {
                    None => ("#0a0a14", "", true, false),
                    Some(t) => {
                        let f = terrain_fill(&t.terrain);
                        let g = terrain_glyph(&t.terrain);
                        (f, g, t.fog, true)
                    }
                };

                let is_selected = selected.get_untracked() == Some((q, r));
                let stroke = if is_selected { "#ffffff" } else { "#000000" };
                let stroke_w = if is_selected { "2.5" } else { "0.6" };

                let click_fn = on_click_arc.clone();
                let sel = selected;

                let label_view = if explored && !foggy {
                    Some(view! {
                        <text
                            x=cx
                            y={cy + HEX_SIZE * 0.55}
                            text-anchor="middle"
                            font-size="9"
                            fill="rgba(0,0,0,0.45)"
                            pointer-events="none"
                        >
                            {glyph}
                        </text>
                    })
                } else {
                    None
                };

                let hills_marker = tile
                    .map(|t| is_hills(&t.terrain))
                    .unwrap_or(false)
                    .then(|| {
                        view! {
                            <circle
                                cx=cx
                                cy={cy - HEX_SIZE * 0.25}
                                r="2.4"
                                fill="rgba(0,0,0,0.5)"
                                pointer-events="none"
                            />
                        }
                    });

                let resource_marker = tile
                    .and_then(|t| t.resource.as_deref().map(|s| s.to_string()))
                    .map(|_| {
                        view! {
                            <circle
                                cx={cx + HEX_SIZE * 0.4}
                                cy={cy - HEX_SIZE * 0.4}
                                r="3"
                                fill="#e8c87a"
                                stroke="#000"
                                stroke-width="0.6"
                                pointer-events="none"
                            />
                        }
                    });

                let city_marker = tile.and_then(|t| t.city.as_ref()).map(|c| {
                    let stroke_color = if c.capital { "#ffffff" } else { "#cccccc" };
                    view! {
                        <polygon
                            class="city-marker"
                            points=format!(
                                "{cx},{top} {rx},{cy} {cx},{bot} {lx},{cy}",
                                cx = cx, cy = cy,
                                top = cy - 10.0, bot = cy + 10.0,
                                lx = cx - 10.0, rx = cx + 10.0,
                            )
                            fill="#1a1d27"
                            stroke=stroke_color
                            stroke-width="1.6"
                            pointer-events="none"
                        />
                    }
                });

                let unit_marker = tile.and_then(|t| t.unit.as_ref()).map(|_u| {
                    view! {
                        <circle
                            class="unit-dot"
                            cx=cx
                            cy=cy
                            r="6"
                            fill="#4e7df4"
                            stroke="#fff"
                            stroke-width="1.2"
                            pointer-events="none"
                        />
                    }
                });

                let owner_tint = tile.and_then(|t| t.owner.as_deref().map(|s| s.to_string()));
                let territory_layer = owner_tint.map(|_| {
                    view! {
                        <polygon
                            points=points.clone()
                            fill="#4e7df4"
                            fill-opacity="0.18"
                            stroke="none"
                            pointer-events="none"
                        />
                    }
                });

                let fog_layer = (!explored).then(|| {
                    view! {
                        <polygon
                            points=points.clone()
                            fill="#060810"
                            fill-opacity="0.85"
                            stroke="none"
                            pointer-events="none"
                        />
                    }
                });

                let foggy_layer = (explored && foggy).then(|| {
                    view! {
                        <polygon
                            points=points.clone()
                            fill="#060810"
                            fill-opacity="0.45"
                            stroke="none"
                            pointer-events="none"
                        />
                    }
                });

                out.push(view! {
                    <g>
                        <polygon
                            class="hex-cell"
                            points=points.clone()
                            fill=fill
                            stroke=stroke
                            stroke-width=stroke_w
                            on:click=move |_| {
                                sel.set(Some((q, r)));
                                click_fn(q, r);
                            }
                        />
                        {label_view}
                        {hills_marker}
                        {territory_layer}
                        {resource_marker}
                        {city_marker}
                        {unit_marker}
                        {foggy_layer}
                        {fog_layer}
                    </g>
                });
            }
        }
        out
    };

    view! {
        <svg
            width=move || dims().0
            height=move || dims().1
            viewBox=move || format!("0 0 {:.0} {:.0}", dims().0, dims().1)
            xmlns="http://www.w3.org/2000/svg"
        >
            {elems}
        </svg>
    }
}
