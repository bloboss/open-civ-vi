pub mod coord;
pub mod enums;
pub mod ids;
pub mod messages;
pub mod profile;
pub mod view;

// Re-export key types at crate root for convenience.
pub use coord::{HexCoord, HexDir};
pub use enums::*;
pub use ids::*;
pub use messages::{ClientMessage, GameAction, ServerMessage};
pub use profile::{CivTemplate, ProfileView};
pub use view::GameView;
