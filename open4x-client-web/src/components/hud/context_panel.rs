//! Context panel — bottom-center HUD overlay.
//!
//! Mirrors `Open4X.html` lines 152-154 + `open4x.js::renderContext`
//! (lines 375-456). Resolves the currently selected tile against the
//! `WorldSnapshot` LocalResource and renders Tile / City / Unit /
//! Wonder sections plus an action row.

use leptos::prelude::*;

use crate::components::shell::Tab;
use open4x_protocol::v1::web::world::{TileView, WorldSnapshot};

#[component]
pub fn ContextPanel(
    selected: RwSignal<Option<(i32, i32)>>,
    snapshot: Signal<Option<WorldSnapshot>>,
    /// Tab switcher — used for the "Open city" / "Open unit" actions.
    on_open_tab: Callback<Tab>,
) -> impl IntoView {
    let on_close    = move |_| selected.set(None);
    let on_open_city = move |_| on_open_tab.run(Tab::City);
    let on_open_unit = move |_| on_open_tab.run(Tab::Units);

    view! {
        <div class="context-panel" id="context-panel">
            {move || {
                let Some((q, r)) = selected.get() else {
                    return view! {
                        <div class="context-empty">
                            "Click a tile to inspect · drag to pan · scroll to zoom"
                        </div>
                    }.into_any();
                };
                let snap = snapshot.get();
                let tile = snap.as_ref().and_then(|s| {
                    s.tiles.iter().find(|t| t.q == q && t.r == r).cloned()
                });
                match tile {
                    None => view! {
                        <div class="context-empty">
                            {format!("Undiscovered · {q},{r}")}
                        </div>
                    }.into_any(),
                    Some(t) => render_sections(t, on_open_city, on_open_unit, on_close).into_any(),
                }
            }}
        </div>
    }
}

fn render_sections(
    t: TileView,
    on_open_city: impl Fn(leptos::ev::MouseEvent) + 'static,
    on_open_unit: impl Fn(leptos::ev::MouseEvent) + 'static,
    on_close: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    let q = t.q;
    let r = t.r;

    let yields = {
        let ys = &t.yields;
        let mut spans: Vec<AnyView> = Vec::new();
        if let Some(v) = ys.f.filter(|v| *v != 0) { spans.push(view! { <span class="y y-f">{format!("{v} F")}</span> }.into_any()); }
        if let Some(v) = ys.p.filter(|v| *v != 0) { spans.push(view! { <span class="y y-p">{format!("{v} P")}</span> }.into_any()); }
        if let Some(v) = ys.g.filter(|v| *v != 0) { spans.push(view! { <span class="y y-g">{format!("{v} G")}</span> }.into_any()); }
        if spans.is_empty() {
            view! { <span class="ctx-meta">"no yields"</span> }.into_any()
        } else {
            view! { <span class="yields">{spans}</span> }.into_any()
        }
    };

    let resource = t.resource.as_ref().map(|res| view! {
        <div class="ctx-meta" style="margin-top:4px;">
            <span class="chip accent">{res.clone()}</span>
        </div>
    }.into_any()).unwrap_or_else(|| ().into_any());

    let city_section = t.city.as_ref().map(|c| {
        let name = c.name.clone();
        let cap_star = if c.capital { " \u{2605}" } else { "" };
        view! {
            <div class="ctx-section" style="min-width:200px;">
                <div class="ctx-title">"City"</div>
                <div class="ctx-h">
                    {name}{cap_star}
                    " "
                    <span class="coord">{format!("pop {}", c.pop)}</span>
                </div>
                <div class="ctx-meta">{c.id.clone()}</div>
            </div>
        }.into_any()
    }).unwrap_or_else(|| ().into_any());

    let unit_section = t.unit.as_ref().map(|u| view! {
        <div class="ctx-section" style="min-width:200px;">
            <div class="ctx-title">"Unit"</div>
            <div class="ctx-h">
                {u.kind.clone()}
                {(!u.name.is_empty()).then(|| format!(" · {}", u.name))}
            </div>
            <div class="ctx-meta">{format!("HP {}", u.hp)}</div>
        </div>
    }.into_any()).unwrap_or_else(|| ().into_any());

    let has_city = t.city.is_some();
    let has_unit = t.unit.is_some();

    view! {
        <>
            <div class="ctx-section" style="min-width:200px;">
                <div class="ctx-title">"Tile"</div>
                <div class="ctx-h">
                    {t.terrain.clone()}
                    " "
                    <span class="coord">{format!("{q},{r}")}</span>
                </div>
                <div class="ctx-meta">{yields}</div>
                {resource}
            </div>
            {city_section}
            {unit_section}
            <div class="ctx-actions">
                {has_city.then(|| view! {
                    <button class="btn" on:click=on_open_city>"Open city"</button>
                }.into_any()).unwrap_or_else(|| ().into_any())}
                {has_unit.then(|| view! {
                    <button class="btn" on:click=on_open_unit>"Open unit"</button>
                }.into_any()).unwrap_or_else(|| ().into_any())}
                <button class="btn ghost" on:click=on_close>"Close"</button>
            </div>
        </>
    }
}
