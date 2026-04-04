//! Save/load functionality for GameState.
//!
//! `save_game` serializes the full game state to JSON via serde.
//! `load_game` reconstructs the state: it creates a fresh GameState from the
//! saved seed (rebuilding all definition registries), then overlays the saved
//! mutable fields.

/// Serialize the current game state to a JSON string.
#[cfg(feature = "serde")]
pub fn save_game(state: &super::state::GameState) -> Result<String, String> {
    serde_json::to_string_pretty(state).map_err(|e| e.to_string())
}

/// Deserialize a game state from a JSON string and rebuild definition registries.
///
/// Creates a fresh `GameState::new(seed, w, h)` then overlays each saved field.
/// Types containing `&'static str` are deserialized by round-tripping through
/// `serde_json::to_string` + `from_str` so the deserializer lifetime matches.
#[cfg(feature = "serde")]
pub fn load_game(json: &str) -> Result<super::state::GameState, String> {
    use serde_json::Value;

    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;

    let seed = v["seed"].as_u64().ok_or("missing seed")?;
    let board_w = v["board"]["width"].as_u64().ok_or("missing board.width")? as u32;
    let board_h = v["board"]["height"].as_u64().ok_or("missing board.height")? as u32;

    // Create a fresh state with all registries rebuilt from seed.
    let mut state = super::state::GameState::new(seed, board_w, board_h);

    // Helper: deserialize a field from a JSON Value.
    // For types implementing DeserializeOwned, use from_value directly.
    macro_rules! load_field {
        ($field:ident) => {
            if let Some(val) = v.get(stringify!($field)) {
                state.$field = serde_json::from_value(val.clone())
                    .map_err(|e| format!("failed to load {}: {}", stringify!($field), e))?;
            }
        };
    }

    load_field!(turn);
    load_field!(board);
    load_field!(id_gen);
    load_field!(cities);
    load_field!(units);
    load_field!(placed_districts);
    load_field!(diplomatic_relations);
    load_field!(religions);
    load_field!(trade_routes);
    load_field!(current_era);
    load_field!(current_era_index);
    load_field!(victory_conditions);
    load_field!(barbarian_camps);
    load_field!(barbarian_config);
    load_field!(barbarian_civ);

    // Types with &'static str fields need special treatment:
    // serde_json::from_value requires DeserializeOwned (for<'de> Deserialize<'de>)
    // but our types' Deserialize impls are bound to specific lifetimes due to
    // string leaking. We round-trip through to_string + from_str to give serde
    // a concrete input lifetime.
    macro_rules! load_static_str_field {
        ($field:ident) => {
            if let Some(val) = v.get(stringify!($field)) {
                // Leak the JSON string to give it 'static lifetime. This is
                // acceptable because game loading is a one-time operation and
                // the leaked memory is small relative to the game state.
                let json_str: &'static str = Box::leak(val.to_string().into_boxed_str());
                state.$field = serde_json::from_str(json_str)
                    .map_err(|e| format!("failed to load {}: {}", stringify!($field), e))?;
            }
        };
    }

    load_static_str_field!(civilizations);
    load_static_str_field!(great_people);
    load_static_str_field!(great_works);
    load_static_str_field!(wonder_tourism);
    load_static_str_field!(governors);

    // Reseed the ID generator RNG from the saved seed + timestamp.
    state.id_gen = {
        let ts = state.id_gen.timestamp_ms;
        let mut id_gen = super::state::IdGenerator::new(seed);
        // Fast-forward the RNG to the saved timestamp position.
        while id_gen.timestamp_ms < ts {
            id_gen.next_ulid();
        }
        id_gen
    };

    Ok(state)
}
