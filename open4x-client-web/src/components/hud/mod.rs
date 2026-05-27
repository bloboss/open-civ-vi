//! HUD components for the REST-driven single-player client.
//!
//! Mirrors the floating-overlay layout in `Open4X.html` lines 104-174:
//! minimap (top-left), notifications (right), turn queue (bottom-left),
//! context panel (bottom-center), zoom stack (bottom-right). The hex
//! board is the WebGL2 renderer in [`webgl_map`] / [`webgl_renderer`].

pub mod context_panel;
pub mod minimap_panel;
pub mod notifications_panel;
pub mod turn_queue_panel;
pub mod webgl_map;
pub mod webgl_renderer;
pub mod zoom_stack;

pub use context_panel::ContextPanel;
pub use minimap_panel::MinimapPanel;
pub use notifications_panel::NotificationsPanel;
pub use turn_queue_panel::TurnQueuePanel;
pub use webgl_map::{WebglMap, recenter, zoom_in, zoom_out};
pub use webgl_renderer::Camera;
pub use zoom_stack::ZoomStack;
