//! Minimap panel — top-left HUD overlay.
//!
//! Mirrors `Open4X.html` lines 112-117. The viewport rectangle and
//! the actual minimap canvas paint land alongside the WebGL renderer;
//! this commit ships the visual frame with a world-size label so the
//! layout reads correctly before the renderer port.

use leptos::prelude::*;

use open4x_protocol::v1::web::world::WorldSnapshot;

#[component]
pub fn MinimapPanel(
    snapshot: Signal<Option<WorldSnapshot>>,
) -> impl IntoView {
    let dims = move || {
        snapshot.get()
            .map(|s| format!("{}\u{00D7}{}", s.world.width, s.world.height))
            .unwrap_or_else(|| "—".into())
    };

    view! {
        <div class="minimap-panel" id="minimap-panel">
            <canvas id="minimap-canvas" width="220" height="132" />
            <div class="mm-vp" id="minimap-vp" />
            <div class="mm-overlay" id="minimap-overlay" />
            <span class="mm-label">{move || format!("Minimap · {}", dims())}</span>
        </div>
    }
}
