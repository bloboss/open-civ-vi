//! `<MiniMap>` — SVG continent thumbnail. Direct port of the
//! `MiniMap` JSX in `hifi/components.jsx`. Used by the New-Game
//! preview pane and the ongoing-games tile thumbnails.
//!
//! When a real-game `ThumbnailGrid` is supplied via the `cells`
//! prop (Phase 5 polish), renders a proper per-tile SVG grid
//! sourced from `<server_url>/api/v1/world/snapshot`. Otherwise
//! falls back to the deterministic LCG-blob layout used by the
//! design — same output the JSX prototype produced.

use leptos::prelude::*;

#[cfg(feature = "csr")]
use crate::components::thumbnail::{ThumbnailGrid, cell_class};

#[component]
pub fn MiniMap(
    #[prop(default = 1)] seed: u64,
    #[prop(optional, into)] class: &'static str,
    #[prop(optional, into)] style: String,
    /// When `Some`, render this real-game grid instead of the
    /// seeded continent blobs. Cells are positioned in a
    /// 100×64 viewBox by their (q, r) coords mapped through the
    /// world's width/height.
    #[prop(default = None)]
    cells: Option<ThumbnailGrid>,
) -> impl IntoView {
    if let Some(grid) = cells {
        return render_grid(grid, class, style);
    }
    // Deterministic LCG matching the JSX source so the same seed yields the
    // same continent layout cross-impl.
    let mut s = seed.wrapping_mul(9301).wrapping_add(49297);
    let mut rnd = || {
        s = (s.wrapping_mul(9301).wrapping_add(49297)) % 233280;
        s as f32 / 233280.0
    };

    let mut blobs: Vec<(f32, f32, f32, bool)> = Vec::new();
    let count = 5 + (seed % 4) as usize;
    for i in 0..count {
        let x = rnd() * 90.0 + 5.0;
        let y = rnd() * 60.0 + 8.0;
        let r = rnd() * 10.0 + 5.0;
        blobs.push((x, y, r, i == 0));
    }

    let v_lines: Vec<_> = (0..8)
        .map(|i| {
            let x = i as f32 * 12.5;
            view! { <line x1=x y1="0" x2=x y2="64" /> }
        })
        .collect();
    let h_lines: Vec<_> = (0..6)
        .map(|i| {
            let y = i as f32 * 10.6;
            view! { <line x1="0" y1=y x2="100" y2=y /> }
        })
        .collect();

    let blob_views: Vec<_> = blobs
        .into_iter()
        .map(|(x, y, r, is_self)| {
            let class = if is_self { "land-self" } else { "land" };
            view! {
                <ellipse cx=x cy=y rx=r ry={r * 0.7} class=class />
            }
        })
        .collect();

    let class = format!("svg-map {class}");
    view! {
        <svg viewBox="0 0 100 64" preserveAspectRatio="none" class=class style=style>
            <rect class="water" width="100" height="64" />
            <g class="grid">
                {v_lines}
                {h_lines}
            </g>
            {blob_views}
        </svg>
    }
    .into_any()
}

#[cfg(feature = "csr")]
fn render_grid(
    grid: ThumbnailGrid,
    class: &'static str,
    style: String,
) -> leptos::prelude::AnyView {
    use leptos::prelude::*;
    // Project each (q, r) into the design's 100×64 viewBox. The
    // game world uses offset coordinates internally, but for a
    // 100×64-pixel thumbnail a straight axial projection is
    // visually indistinguishable from the proper hex offset and
    // saves the client the hex-math.
    let w = grid.width.max(1) as f32;
    let h = grid.height.max(1) as f32;
    let cell_w = 100.0 / w;
    let cell_h = 64.0 / h;
    let rects: Vec<_> = grid
        .cells
        .iter()
        .map(|c| {
            let x = c.q as f32 * cell_w;
            let y = c.r as f32 * cell_h;
            let cls = cell_class(c);
            view! {
                <rect class=cls x=x y=y width=cell_w height=cell_h />
            }
        })
        .collect();
    let class = format!("svg-map {class}");
    view! {
        <svg viewBox="0 0 100 64" preserveAspectRatio="none" class=class style=style>
            <rect class="water" width="100" height="64" />
            {rects}
        </svg>
    }
    .into_any()
}
