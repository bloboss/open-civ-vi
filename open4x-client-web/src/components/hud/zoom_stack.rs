//! Zoom controls — bottom-right HUD overlay.
//!
//! Mirrors `Open4X.html` lines 156-173. The buttons drive a shared
//! camera signal; until the WebGL renderer lands the callbacks are
//! no-ops by default.

use leptos::prelude::*;

#[component]
pub fn ZoomStack(
    #[prop(optional, into)] on_zoom_in:  Option<Callback<()>>,
    #[prop(optional, into)] on_zoom_out: Option<Callback<()>>,
    #[prop(optional, into)] on_recenter: Option<Callback<()>>,
) -> impl IntoView {
    let fire = |cb: Option<Callback<()>>| move |_| { if let Some(c) = cb { c.run(()); } };

    view! {
        <div class="zoom-stack">
            <button class="icon-btn" title="Zoom in" on:click=fire(on_zoom_in)>
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none"
                     stroke="currentColor" stroke-width="1.4">
                    <path d="M8 3v10M3 8h10" stroke-linecap="round" />
                </svg>
            </button>
            <button class="icon-btn" title="Zoom out" on:click=fire(on_zoom_out)>
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none"
                     stroke="currentColor" stroke-width="1.4">
                    <path d="M3 8h10" stroke-linecap="round" />
                </svg>
            </button>
            <button class="icon-btn" title="Recenter" on:click=fire(on_recenter)>
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none"
                     stroke="currentColor" stroke-width="1.4">
                    <path d="M3 8 8 3l5 5M5 8v5h6V8" stroke-linejoin="round" />
                </svg>
            </button>
        </div>
    }
}
