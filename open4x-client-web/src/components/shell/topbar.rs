//! Topbar — brand + turn/era + resource row + topbar-right icons.
//!
//! Pixel-mirrors `Open4X.html` lines 27-73 and the resource rendering
//! in `open4x.js::bootTopbar` (lines 145-205).

use leptos::prelude::*;
use open4x_protocol::v1::web::player_state::{Bucket, PlayerState};

#[component]
pub fn Topbar(
    /// Reactive source of truth for the topbar; `None` while the page is
    /// still bootstrapping the bearer token.
    player_state: LocalResource<Option<PlayerState>>,
) -> impl IntoView {
    view! {
        <header class="topbar">
            <div class="brand">
                <span class="brand-mark" aria-hidden="true"></span>
                <span class="brand-name">"Open4X"</span>
                <span class="brand-sep">"/"</span>
                <span class="brand-game" id="brand-civ">"Single player"</span>

                <div class="turn-info">
                    <div>
                        <span class="label">"Turn"</span>
                        <span class="val mono" id="turn-val">
                            {move || ps_field(player_state, |p| p.turn.to_string())}
                        </span>
                        <span class="val mono" style="color: var(--ink-4);">
                            "/"
                            <span id="turn-max">
                                {move || ps_field(player_state, |p| p.turn_max.to_string())}
                            </span>
                        </span>
                    </div>
                    <div style="display:flex; align-items:center; gap:8px;">
                        <span class="label" style="margin-right:0;">"Era"</span>
                        <span class="val" id="era-val"
                              style="font-family: var(--font-sans); font-size:12px;">
                            {move || ps_field(player_state, |p| p.era.clone())}
                        </span>
                        <div class="era-bar" title="Era progress">
                            <i id="era-bar-fill"
                               style:width=move || ps_field(player_state, |p| {
                                   format!("{}%", (p.era_progress.clamp(0.0, 1.0) * 100.0).round() as i32)
                               })
                            />
                        </div>
                    </div>
                </div>
            </div>

            <div class="resources" id="resources">
                {move || render_resources(player_state)}
            </div>

            <div class="topbar-right">
                <button class="icon-btn" title="Search (Ctrl+K)">
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none"
                         stroke="currentColor" stroke-width="1.4">
                        <circle cx="7" cy="7" r="4.5" />
                        <path d="M10.5 10.5 L13 13" stroke-linecap="round" />
                    </svg>
                </button>
                <button class="icon-btn" title="Settings">
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none"
                         stroke="currentColor" stroke-width="1.4">
                        <circle cx="8" cy="8" r="2" />
                        <path d="M8 1.5v2M8 12.5v2M14.5 8h-2M3.5 8h-2M12.6 3.4l-1.4 1.4M4.8 11.2l-1.4 1.4M12.6 12.6l-1.4-1.4M4.8 4.8 3.4 3.4"
                              stroke-linecap="round" />
                    </svg>
                </button>
                <button class="icon-btn" title="Profile">
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none"
                         stroke="currentColor" stroke-width="1.4">
                        <circle cx="8" cy="5.5" r="2.5" />
                        <path d="M2.5 14c0-2.5 2.5-4.5 5.5-4.5s5.5 2 5.5 4.5" />
                    </svg>
                </button>
            </div>
        </header>
    }
}

/// Render the resource strip (gold/sci/cul/faith/food/prod/happy).
/// Order, glyphs, and class names match `open4x.js::bootTopbar`.
fn render_resources(player_state: LocalResource<Option<PlayerState>>) -> AnyView {
    let Some(wrap) = player_state.get() else { return ().into_any() };
    let Some(ps) = wrap.as_ref() else { return ().into_any() };

    let rows: [(&str, &str, &str, bool); 6] = [
        ("gold",       "gold",       "G", true),
        ("science",    "science",    "S", false),
        ("culture",    "culture",    "C", false),
        ("faith",      "faith",      "F", false),
        ("food",       "food",       "f", false),
        ("production", "production", "P", false),
    ];

    let items = rows.iter().map(|(key, cls, glyph, show_value)| {
        let b: &Bucket = match *key {
            "gold"       => &ps.resources.gold,
            "science"    => &ps.resources.science,
            "culture"    => &ps.resources.culture,
            "faith"      => &ps.resources.faith,
            "food"       => &ps.resources.food,
            "production" => &ps.resources.production,
            _ => unreachable!(),
        };
        let delta = b.per_turn;
        let delta_cls = if delta > 0 { "pos" } else if delta < 0 { "neg" } else { "" };
        let sign = if delta > 0 { "+" } else { "" };
        let cls = format!("res {cls}");
        let delta_cls = format!("res-delta {delta_cls}");
        let value_block = if *show_value && b.value.is_some() {
            let v = b.value.unwrap();
            view! { <span class="res-val">{format_thousands(v)}</span> }.into_any()
        } else {
            ().into_any()
        };
        view! {
            <div class=cls>
                <span class="res-glyph">{*glyph}</span>
                {value_block}
                <span class=delta_cls>{format!("{sign}{delta}")}</span>
            </div>
        }
    }).collect::<Vec<_>>();

    let happy = ps.happiness;
    let happy_cls = if happy > 0 { "pos" } else if happy < 0 { "neg" } else { "" };
    let happy_sign = if happy >= 0 { "+" } else { "" };
    let happy_glyph = if happy >= 0 { ":)" } else { ":(" };

    view! {
        <>
            {items}
            <div class="res happy" title="Happiness">
                <span class="res-glyph">{happy_glyph}</span>
                <span class=format!("res-val {happy_cls}")>
                    {format!("{happy_sign}{happy}")}
                </span>
            </div>
        </>
    }.into_any()
}

fn ps_field(
    res: LocalResource<Option<PlayerState>>,
    f: impl Fn(&PlayerState) -> String,
) -> String {
    res.get()
        .as_deref()
        .and_then(|wrap| wrap.as_ref().map(f))
        .unwrap_or_else(|| "—".to_string())
}

/// Pretty-print an integer with thousand separators (1234 → "1,234"),
/// matching `Number.toLocaleString()` in the JS source.
fn format_thousands(n: i32) -> String {
    let neg = n < 0;
    let mut s = n.unsigned_abs().to_string();
    let bytes = s.as_bytes().to_vec();
    s.clear();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            s.push(',');
        }
        s.push(*b as char);
    }
    if neg { format!("-{s}") } else { s }
}
