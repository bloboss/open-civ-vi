//! Leptos wrapper around [`WebglRenderer`].
//!
//! Owns the `<canvas>` node, instantiates the renderer on mount,
//! subscribes to the `WorldSnapshot` + `Camera` signals, and drives
//! pan / zoom / pick interactions. Mirrors the HUD shell in
//! `Open4X.html` lines 106-109 (`hud-map > hex-canvas`).

use std::cell::RefCell;
use std::rc::Rc;

use leptos::ev::{MouseEvent, PointerEvent, WheelEvent};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use open4x_protocol::v1::web::world::WorldSnapshot;

use super::webgl_renderer::{Camera, WebglRenderer, HEX_R};

const SQRT3: f32 = 1.732_050_8;
const ZOOM_MIN: f32 = 0.4;
const ZOOM_MAX: f32 = 1.8;

#[derive(Clone, Copy)]
struct DragState {
    /// Cursor position when drag started (CSS px, page coords).
    start_x: f32,
    start_y: f32,
    /// Camera position when drag started.
    cam_x: f32,
    cam_y: f32,
    moved: bool,
}

/// Shared renderer + drag state — the Leptos component, effects, and
/// event handlers all reach into the same `Rc<RefCell<_>>` so the
/// renderer survives re-renders and signal updates.
#[derive(Clone, Default)]
struct Shared {
    renderer: Rc<RefCell<Option<WebglRenderer>>>,
    drag: Rc<RefCell<Option<DragState>>>,
}

#[component]
pub fn WebglMap(
    snapshot: Signal<Option<WorldSnapshot>>,
    camera: RwSignal<Camera>,
    selected: RwSignal<Option<(i32, i32)>>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let shared = Shared::default();

    // ── Init the renderer once the <canvas> has mounted ──────────────────
    {
        let shared = shared.clone();
        Effect::new(move |_| {
            let Some(canvas_el) = canvas_ref.get() else { return };
            let canvas: HtmlCanvasElement = (*canvas_el).clone().unchecked_into();
            if shared.renderer.borrow().is_some() { return; }
            match WebglRenderer::create(canvas) {
                Ok(r) => *shared.renderer.borrow_mut() = Some(r),
                Err(e) => web_sys::console::error_1(&format!("[webgl_map] init failed: {e}").into()),
            }
        });
    }

    // ── Repaint on every (snapshot, camera) change ───────────────────────
    {
        let shared = shared.clone();
        Effect::new(move |_| {
            let snap = snapshot.get();
            let cam  = camera.get();
            let mut slot = shared.renderer.borrow_mut();
            let Some(r) = slot.as_mut() else { return };
            let Some(s) = snap else { return };
            r.render(&s, cam);
        });
    }

    // ── Camera defaults: when the snapshot first lands, recenter on the
    //    server's camera selection (or the centre of the board). ───────
    {
        let initialised = RwSignal::new(false);
        Effect::new(move |_| {
            if initialised.get_untracked() { return; }
            let Some(s) = snapshot.get() else { return };
            let (cx, cy, sel) = match &s.camera.selection {
                Some(c) => (c.q as f32, c.r as f32, Some((c.q, c.r))),
                None => (s.world.width  as f32 * 0.5,
                         s.world.height as f32 * 0.5,
                         None),
            };
            camera.set(Camera { x: cx, y: cy, zoom: 1.0, sel });
            if let Some(p) = sel { selected.set(Some(p)); }
            initialised.set(true);
        });
    }

    // Keep selected ↔ camera.sel in sync (camera.sel drives the shader's
    // selection ring; selected drives the context-panel resolver).
    {
        Effect::new(move |_| {
            let s = selected.get();
            camera.update(|c| c.sel = s);
        });
    }

    // ── Event handlers ───────────────────────────────────────────────────
    let on_pointer_down = {
        let shared = shared.clone();
        move |ev: PointerEvent| {
            if ev.button() != 0 { return; }
            let cam = camera.get_untracked();
            *shared.drag.borrow_mut() = Some(DragState {
                start_x: ev.client_x() as f32,
                start_y: ev.client_y() as f32,
                cam_x: cam.x, cam_y: cam.y,
                moved: false,
            });
            // Try to capture the pointer to keep getting moves outside the canvas.
            if let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                let _ = target.set_pointer_capture(ev.pointer_id());
            }
        }
    };

    let on_pointer_move = {
        let shared = shared.clone();
        move |ev: PointerEvent| {
            let mut drag_slot = shared.drag.borrow_mut();
            let Some(drag) = drag_slot.as_mut() else { return };
            let dx = ev.client_x() as f32 - drag.start_x;
            let dy = ev.client_y() as f32 - drag.start_y;
            if dx.abs() + dy.abs() > 4.0 { drag.moved = true; }
            let zoom = camera.get_untracked().zoom;
            let r = HEX_R * zoom;
            let hw = SQRT3 * r;
            let hh = 1.5 * r;
            let new_x = drag.cam_x - dx / hw;
            let mut new_y = drag.cam_y - dy / hh;
            if let Some(s) = snapshot.get_untracked() {
                let hmax = s.world.height as f32 + 2.0;
                new_y = new_y.clamp(-3.0, hmax);
            }
            camera.update(|c| { c.x = new_x; c.y = new_y; });
        }
    };

    let end_drag = {
        let shared = shared.clone();
        move |_ev: PointerEvent| {
            // Defer-clearing matches the JS: an immediate following click
            // should still see `drag.moved` so the pick is suppressed.
            let shared = shared.clone();
            let win = web_sys::window().expect("window");
            let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                *shared.drag.borrow_mut() = None;
            });
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.unchecked_ref(), 0,
            );
        }
    };

    let on_wheel = move |ev: WheelEvent| {
        ev.prevent_default();
        let delta = -ev.delta_y().signum() as f32 * 0.12;
        camera.update(|c| c.zoom = (c.zoom + delta).clamp(ZOOM_MIN, ZOOM_MAX));
    };

    let on_click = {
        let shared = shared.clone();
        move |ev: MouseEvent| {
            // Suppress click if this was the end of a drag.
            if let Some(d) = *shared.drag.borrow() { if d.moved { return; } }
            let Some(r) = shared.renderer.borrow().as_ref().map(|r| r.canvas.clone()) else { return };
            let rect = r.get_bounding_client_rect();
            let sx = ev.client_x() as f32 - rect.left() as f32;
            let sy = ev.client_y() as f32 - rect.top()  as f32;
            let cam = camera.get_untracked();
            let Some(snap) = snapshot.get_untracked() else { return };
            let pick = shared.renderer.borrow().as_ref().and_then(|r| r.pick(sx, sy, cam, &snap));
            if let Some(p) = pick {
                selected.set(Some(p));
            }
        }
    };

    view! {
        <div
            class="hud-map"
            id="hud-map"
            style="position:absolute; inset:0; overflow:hidden;"
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=end_drag.clone()
            on:pointercancel=end_drag.clone()
            on:pointerleave=end_drag
            on:wheel=on_wheel
            on:click=on_click
        >
            <canvas class="hex-canvas" id="hud-canvas" node_ref=canvas_ref
                    style="width:100%; height:100%; display:block; touch-action:none;" />
        </div>
    }
}

/// Camera helpers exposed to the rest of the page so the ZoomStack
/// buttons and the recenter action don't have to know our clamp range.
pub fn zoom_in(camera: RwSignal<Camera>) {
    camera.update(|c| c.zoom = (c.zoom + 0.18).clamp(ZOOM_MIN, ZOOM_MAX));
}

pub fn zoom_out(camera: RwSignal<Camera>) {
    camera.update(|c| c.zoom = (c.zoom - 0.18).clamp(ZOOM_MIN, ZOOM_MAX));
}

pub fn recenter(camera: RwSignal<Camera>, target: (i32, i32)) {
    camera.update(|c| { c.x = target.0 as f32; c.y = target.1 as f32; c.zoom = 1.0; });
}
