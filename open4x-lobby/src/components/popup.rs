//! Gwern-style popup — port of `docs/open4x-landing/project/hifi/popup.jsx`.
//!
//! Behaviour:
//! - **Hover triggers** show after a 180 ms delay. Leaving the trigger or
//!   the popup itself schedules a 140 ms hide.
//! - **Click triggers** show immediately and pin (only Esc / click-outside
//!   / closing the popup dismisses).
//! - **Pin button** in the head: turns a hover preview into a sticky
//!   popup. Pressing again (now an `×`) closes.
//! - **Esc** dismisses any popup (pinned or not).
//! - **Click outside** the popup or any popup-trigger dismisses pinned.
//! - **Smart positioning**: prefer below the anchor; flip above when the
//!   popup would clip the viewport bottom. Horizontal clamp into the
//!   viewport with an 8 px margin.
//!
//! One [`PopupProvider`] mounts at the app root and owns a single
//! `RwSignal<Option<PopupState>>`. Each [`Popup`] wrapper captures its
//! anchor's `DOMRect` on hover/click and asks the provider to show.
//!
//! For the rationale and the alternative approach (CSS-only :hover),
//! see book/src/roadmap/accounts-and-login.md §1.

use std::cell::RefCell;
use std::sync::Arc;

use gloo_timers::callback::Timeout;
use leptos::ev;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use web_sys::{Element, KeyboardEvent, MouseEvent};

// ────────────────────────────── State types ──────────────────────────────────

/// Shared state describing the active popup. `view_fn` is a renderer
/// closure: the popup body is rebuilt on each render rather than stored
/// as an `AnyView` (which is `!Clone`). Wrapped in `Arc` so `PopupState`
/// itself is cheap-Clone and can sit inside a Leptos signal.
#[derive(Clone)]
pub struct PopupState {
    pub title: String,
    pub size: PopupSize,
    pub view_fn: Arc<dyn Fn() -> AnyView + 'static>,
    pub anchor: AnchorRect,
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopupSize {
    #[default]
    Default,
    Wide,
    Narrow,
}

impl PopupSize {
    fn class(self) -> &'static str {
        match self {
            PopupSize::Default => "",
            PopupSize::Wide => "wide",
            PopupSize::Narrow => "narrow",
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct AnchorRect {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

impl AnchorRect {
    fn from_element(el: &Element) -> Self {
        let r = el.get_bounding_client_rect();
        Self {
            top: r.top(),
            bottom: r.bottom(),
            left: r.left(),
            right: r.right(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Trigger {
    Hover,
    Click,
}

// ────────────────────────────── Context shape ────────────────────────────────

type PopupSignal = RwSignal<Option<PopupState>, LocalStorage>;
type Cell = SendWrapper<RefCell<Option<Timeout>>>;

#[derive(Clone)]
struct PopupCtx {
    state: PopupSignal,
    show_timer: Arc<Cell>,
    hide_timer: Arc<Cell>,
}

const SHOW_DELAY_MS: u32 = 180;
const HIDE_DELAY_MS: u32 = 140;

impl PopupCtx {
    fn new() -> Self {
        Self {
            state: RwSignal::new_local(None),
            show_timer: Arc::new(SendWrapper::new(RefCell::new(None))),
            hide_timer: Arc::new(SendWrapper::new(RefCell::new(None))),
        }
    }

    fn cancel_show(&self) {
        self.show_timer.borrow_mut().take();
    }

    fn cancel_hide(&self) {
        self.hide_timer.borrow_mut().take();
    }

    /// Schedule a popup to appear after the standard hover delay (or
    /// immediately for click triggers).
    fn show(&self, state: PopupState, delay_ms: u32) {
        self.cancel_hide();
        if let Some(p) = self.state.get_untracked() {
            if p.pinned {
                return;
            }
        }
        let target = self.state;
        let timer = Timeout::new(delay_ms, move || target.set(Some(state)));
        *self.show_timer.borrow_mut() = Some(timer);
    }

    fn schedule_hide(&self) {
        self.cancel_show();
        if let Some(p) = self.state.get_untracked() {
            if p.pinned {
                return;
            }
        }
        let target = self.state;
        let timer = Timeout::new(HIDE_DELAY_MS, move || target.set(None));
        *self.hide_timer.borrow_mut() = Some(timer);
    }

    fn pin(&self) {
        if let Some(mut p) = self.state.get_untracked() {
            p.pinned = true;
            self.state.set(Some(p));
        }
    }

    fn close(&self) {
        self.cancel_show();
        self.cancel_hide();
        self.state.set(None);
    }
}

// ────────────────────────────── Provider ──────────────────────────────────────

#[component]
pub fn PopupProvider(children: Children) -> impl IntoView {
    let ctx = PopupCtx::new();
    provide_context(ctx.clone());

    // Esc to close.
    let esc_ctx = ctx.clone();
    window_event_listener(ev::keydown, move |ev: KeyboardEvent| {
        if ev.key() == "Escape" {
            esc_ctx.close();
        }
    });

    // Click outside (only matters when pinned). The trigger has
    // data-popup-trigger; the popup has class .popup. We dismiss
    // anything that lands outside both.
    let click_ctx = ctx.clone();
    window_event_listener(ev::click, move |ev: MouseEvent| {
        let Some(state) = click_ctx.state.get_untracked() else {
            return;
        };
        if !state.pinned {
            return;
        }
        let Some(target) = ev.target() else {
            return;
        };
        if let Ok(el) = target.dyn_into::<Element>() {
            if el.closest(".popup").ok().flatten().is_some() {
                return;
            }
            if el.closest("[data-popup-trigger]").ok().flatten().is_some() {
                return;
            }
        }
        click_ctx.close();
    });

    let render_state = ctx.state;
    view! {
        {children()}
        {move || render_state.get().map(|state| {
            view! { <PopupRender state /> }
        })}
    }
}

// ────────────────────────────── Renderer ──────────────────────────────────────

#[component]
fn PopupRender(state: PopupState) -> impl IntoView {
    let ctx = use_context::<PopupCtx>().expect("PopupRender outside PopupProvider");

    // Position is computed after the popup mounts so we can measure its
    // rect; until then it's parked off-screen at top/left = -9999.
    let pos = RwSignal::new((-9999_f64, -9999_f64));
    let popup_ref = NodeRef::<leptos::html::Div>::new();
    let anchor = state.anchor;
    let popup_ref_for_load = popup_ref;
    Effect::new(move |_| {
        let Some(el) = popup_ref_for_load.get() else {
            return;
        };
        let el: &web_sys::Element = el.as_ref();
        let r = el.get_bounding_client_rect();
        let win = web_sys::window().unwrap();
        let vw = win
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1024.0);
        let vh = win
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(768.0);
        let margin = 8.0;
        let mut top = anchor.bottom + 6.0;
        let mut left = anchor.left;
        if top + r.height() + margin > vh && anchor.top - r.height() - 6.0 > margin {
            top = anchor.top - r.height() - 6.0;
        }
        if left + r.width() + margin > vw {
            left = vw - r.width() - margin;
        }
        if left < margin {
            left = margin;
        }
        pos.set((top, left));
    });

    let class = format!("popup {}", state.size.class());
    let title = state.title;
    let pinned = state.pinned;
    let view = (state.view_fn)();

    let cancel_hide_ctx = ctx.clone();
    let schedule_hide_ctx = ctx.clone();
    let pin_or_close_ctx = ctx.clone();

    let on_pin_click = move |ev: MouseEvent| {
        ev.stop_propagation();
        if pinned {
            pin_or_close_ctx.close();
        } else {
            pin_or_close_ctx.pin();
        }
    };

    view! {
        <div
            class=class
            node_ref=popup_ref
            style=move || {
                let (t, l) = pos.get();
                format!("top: {t:.1}px; left: {l:.1}px")
            }
            on:mouseenter=move |_| cancel_hide_ctx.cancel_hide()
            on:mouseleave=move |_| schedule_hide_ctx.schedule_hide()
        >
            {(!title.is_empty()).then(|| view! {
                <div class="popup-head">
                    <span class="title">{title.clone()}</span>
                    <button
                        class="pin"
                        type="button"
                        title=if pinned { "close (esc)" } else { "pin" }
                        on:click=on_pin_click
                    >
                        {if pinned { "×" } else { "⌶" }}
                    </button>
                </div>
            })}
            {view}
        </div>
    }
}

// ────────────────────────────── Trigger wrapper ──────────────────────────────

/// Wraps any inline element with a popup. `content` is rendered into a
/// floating panel anchored to the wrapped element on hover (default) or
/// click.
///
/// Replaces the `<Trigger>` placeholder once call sites are migrated.
#[component]
pub fn Popup(
    #[prop(into, optional)] title: String,
    #[prop(optional, default = PopupSize::Default)] size: PopupSize,
    #[prop(optional, default = Trigger::Hover)] trigger: Trigger,
    /// Renderer for the popup body. Called every time the popup
    /// re-renders inside `PopupProvider`. Wrapped in `Arc` for cheap
    /// clones into the context state.
    content: Arc<dyn Fn() -> AnyView + 'static>,
    children: Children,
) -> impl IntoView {
    let ctx = use_context::<PopupCtx>().expect("Popup used outside PopupProvider");
    let trigger_ref = NodeRef::<leptos::html::Span>::new();
    let title = Arc::new(title);

    let open_ctx = ctx.clone();
    let title_open = title.clone();
    let content_open = content.clone();
    let open = move |delay_ms: u32| {
        let Some(el) = trigger_ref.get() else {
            return;
        };
        let el: &web_sys::Element = el.as_ref();
        let anchor = AnchorRect::from_element(el);
        let state = PopupState {
            title: (*title_open).clone(),
            size,
            view_fn: content_open.clone(),
            anchor,
            pinned: false,
        };
        open_ctx.show(state, delay_ms);
    };

    let on_enter_open = open.clone();
    let on_enter = move |_: MouseEvent| match trigger {
        Trigger::Click => {}
        Trigger::Hover => on_enter_open(SHOW_DELAY_MS),
    };
    let leave_ctx = ctx.clone();
    let on_leave = move |_: MouseEvent| match trigger {
        Trigger::Click => {}
        Trigger::Hover => leave_ctx.schedule_hide(),
    };
    let click_ctx = ctx.clone();
    let on_click_open = open;
    let on_click = move |ev: MouseEvent| {
        ev.stop_propagation();
        on_click_open(0);
        // Pin on the next tick so the show timeout fires first. Tiny
        // delay is fine — gloo's Timeout(10) gives the state machine
        // time to set the popup before we mark it pinned.
        let pin_ctx = click_ctx.clone();
        Timeout::new(20, move || pin_ctx.pin()).forget();
    };

    view! {
        <span
            node_ref=trigger_ref
            data-popup-trigger=""
            style="display:inline-flex"
            on:mouseenter=on_enter
            on:mouseleave=on_leave
            on:click=on_click
        >
            {children()}
        </span>
    }
}
