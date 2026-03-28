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

/// Configuration for an AI-vs-AI demo game.
#[derive(Clone, Debug, PartialEq)]
pub struct DemoConfig {
    pub width:     u32,
    pub height:    u32,
    pub seed:      u64,
    pub num_turns: u32,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self { width: 20, height: 14, seed: 42, num_turns: 100 }
    }
}
