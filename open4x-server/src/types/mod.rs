pub mod coord;
pub mod enums;
pub mod ids;
pub mod messages;
pub mod profile;
pub mod reports;
pub mod view;

// Re-export key types at crate root for convenience.
pub use coord::{HexCoord, HexDir};
pub use enums::*;
pub use ids::*;
pub use messages::{ClientMessage, GameAction, GameStatus, ServerMessage};
pub use profile::{CivTemplate, ProfileView};
pub use reports::*;
pub use view::GameView;
