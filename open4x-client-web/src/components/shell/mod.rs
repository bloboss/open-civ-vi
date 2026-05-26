//! App shell — topbar, 9-tab nav, end-turn group, screen stub.
//!
//! Mirrors `open4x-server/static/vi/Open4X.html` lines 24-98 and lines
//! 177-232 (the "Coming next" stub treatment). The HUD overlays
//! (minimap, notifications, turn queue, context panel, zoom stack) live
//! in [`crate::components::hud`] — this module only owns the chrome.

pub mod tabbar;
pub mod topbar;

pub use tabbar::{EndTurnGroup, ScreenStub, Tab, Tabbar};
pub use topbar::Topbar;
