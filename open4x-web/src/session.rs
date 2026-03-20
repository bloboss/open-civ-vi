/// Pre-game configuration set on the map-config screen.
#[derive(Clone, Debug, PartialEq)]
pub struct GameConfig {
    pub width:  u32,
    pub height: u32,
    pub seed:   u64,
    /// Number of AI opponents.  0 = solo play.
    pub num_ai: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self { width: 40, height: 24, seed: 42, num_ai: 1 }
    }
}
