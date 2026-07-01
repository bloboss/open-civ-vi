//! Shared Leptos primitives ported from
//! `docs/open4x-landing/project/hifi/components.jsx` and `primitives.jsx`.
//!
//! Each primitive renders the same DOM the design CSS expects (class names
//! `.btn`, `.tag`, `.toggle`, `.seg`, `.panel`, …) so visual styling lives
//! in `styles.css` alone. Behaviour is reactive via Leptos signals.

pub mod api;
pub mod btn;
pub mod crypto;
pub mod i18n;
pub mod minimap;
pub mod panel;
pub mod popup;
pub mod popup_body;
pub mod qr;
pub mod segmented;
pub mod slider;
pub mod tag;
pub mod thumbnail;
pub mod toggle;
pub mod tweaks_panel;

pub use btn::Btn;
pub use minimap::MiniMap;
pub use panel::{Panel, PanelHead};
pub use popup::{AnchorRect, Popup, PopupProvider, PopupSize, PopupState, Trigger as PopupTrigger};
pub use popup_body::{PopupActions, PopupBody, PopupList, PopupListItem};
pub use segmented::Segmented;
pub use slider::{FormatFn, Slider};
pub use tag::Tag;
pub use toggle::Toggle;
pub use tweaks_panel::TweaksPanel;
