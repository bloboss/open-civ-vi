//! HUD components for the REST-driven single-player client.
//!
//! Mirrors the floating-overlay layout in `Open4X.html` lines 104-174:
//! minimap (top-left), notifications (right), turn queue (bottom-left),
//! context panel (bottom-center), zoom stack (bottom-right). The hex
//! board itself lives in `snapshot_map` (SVG, placeholder) and will be
//! replaced by `webgl_map` in a follow-up commit.

pub mod context_panel;
pub mod minimap_panel;
pub mod notifications_panel;
pub mod snapshot_map;
pub mod turn_queue_panel;
pub mod zoom_stack;

pub use context_panel::ContextPanel;
pub use minimap_panel::MinimapPanel;
pub use notifications_panel::NotificationsPanel;
pub use turn_queue_panel::TurnQueuePanel;
pub use zoom_stack::ZoomStack;
