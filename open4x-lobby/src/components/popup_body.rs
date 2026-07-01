//! Popup body / actions / list containers — port of the slot shapes
//! used inside `PopupRender` in `docs/open4x-landing/project/hifi/popup.jsx`.
//!
//! These are pure layout wrappers; the surrounding `.popup` chrome lives
//! in [`crate::components::popup_stub`] (placeholder) and will land
//! properly when the Gwern hover/pin component is ported. Until then
//! they still render correctly inside any container styled with
//! `.popup-body` / `.popup-actions` / `.popup-list` (e.g. inline
//! tooltips or custom dropdowns).

use leptos::prelude::*;

/// `<div class="popup-body">{children}</div>` — the main paragraph /
/// kv-table area inside a popup.
#[component]
pub fn PopupBody(children: Children) -> impl IntoView {
    view! { <div class="popup-body">{children()}</div> }
}

/// `<div class="popup-actions">` — footer button row. Set `right=true`
/// to right-align the buttons (matches the JSX `<PopupActions right>`).
#[component]
pub fn PopupActions(#[prop(optional)] right: bool, children: Children) -> impl IntoView {
    let class = if right {
        "popup-actions right"
    } else {
        "popup-actions"
    };
    view! { <div class=class>{children()}</div> }
}

/// One row in a [`PopupList`]. The JSX accepts either a structured row
/// (`{icon, label, desc?}`) or the literal string `"sep"` for a
/// horizontal-rule separator. We model that as the [`PopupListItem`]
/// enum so call sites read clearly.
#[derive(Clone)]
pub enum PopupListItem {
    Row {
        icon: &'static str,
        label: &'static str,
        desc: Option<&'static str>,
    },
    Separator,
}

impl PopupListItem {
    pub fn row(icon: &'static str, label: &'static str) -> Self {
        Self::Row {
            icon,
            label,
            desc: None,
        }
    }

    pub fn row_with_desc(icon: &'static str, label: &'static str, desc: &'static str) -> Self {
        Self::Row {
            icon,
            label,
            desc: Some(desc),
        }
    }

    pub fn sep() -> Self {
        Self::Separator
    }
}

/// `<div class="popup-list">` — vertical list of clickable rows
/// (icon + label + optional desc). Used by every `<Popup trigger="click">`
/// menu in the design.
///
/// Items are not interactive yet (no `on_click` plumbing) — this is a
/// visual port. When the popup component lands and the menu rows need
/// click dispatch, extend the `Row` variant with an optional callback.
#[component]
pub fn PopupList(#[prop(into)] items: Vec<PopupListItem>) -> impl IntoView {
    view! {
        <div class="popup-list">
            {items.into_iter().map(|item| match item {
                PopupListItem::Separator => view! {
                    <div class="sep"></div>
                }.into_any(),
                PopupListItem::Row { icon, label, desc } => view! {
                    <button class="item" type="button">
                        <span class="icon">{icon}</span>
                        <span>{label}</span>
                        {desc.map(|d| view! { <span class="desc">{d}</span> })}
                    </button>
                }.into_any(),
            }).collect::<Vec<_>>()}
        </div>
    }
}
