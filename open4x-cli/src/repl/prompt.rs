//! Configurable prompt for the REPL.
//!
//! The prompt format is a string with `{placeholder}` tokens that are expanded
//! at render time. Available placeholders:
//!
//! | Placeholder | Expands to |
//! |-------------|------------|
//! | `{turn}`    | Current turn number |
//! | `{civ}`     | Civilization name |
//! | `{gold}`    | Current gold |
//! | `{score}`   | Current score |
//! | `{city}`    | Selected city name (or empty) |
//! | `{unit}`    | Selected unit short ID + type (or empty) |
//! | `{district}`| Selected district name (or empty) |
//!
//! The default format is: `T{turn} {city} {unit}> `

use libciv::civ::district::BuiltinDistrict;
use libciv::{compute_score, CityId, CivId, GameState, UnitId};

use super::short_ids::ShortIds;

/// Default prompt format string.
pub const DEFAULT_PROMPT: &str = "T{turn} {city} {unit}> ";

/// Configuration for the REPL prompt.
#[derive(Debug, Clone)]
pub struct PromptConfig {
    pub format: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            format: DEFAULT_PROMPT.to_string(),
        }
    }
}

/// Context needed to render the prompt.
pub struct PromptContext<'a> {
    pub state: &'a GameState,
    pub civ_id: CivId,
    pub civ_name: &'static str,
    pub selected_unit: Option<UnitId>,
    pub selected_city: Option<CityId>,
    pub selected_district: Option<BuiltinDistrict>,
    pub unit_short_ids: &'a ShortIds<UnitId>,
}

/// Render the prompt string by expanding placeholders.
pub fn render_prompt(config: &PromptConfig, ctx: &PromptContext<'_>) -> String {
    let mut result = config.format.clone();

    // Only compute values for placeholders actually present.
    if result.contains("{turn}") {
        result = result.replace("{turn}", &ctx.state.turn.to_string());
    }
    if result.contains("{civ}") {
        result = result.replace("{civ}", ctx.civ_name);
    }
    if result.contains("{gold}") {
        let gold = ctx.state.civilizations.iter()
            .find(|c| c.id == ctx.civ_id)
            .map(|c| c.gold)
            .unwrap_or(0);
        result = result.replace("{gold}", &gold.to_string());
    }
    if result.contains("{score}") {
        let score = compute_score(ctx.state, ctx.civ_id);
        result = result.replace("{score}", &score.to_string());
    }
    if result.contains("{city}") {
        let city_str = ctx.selected_city
            .and_then(|cid| ctx.state.cities.iter().find(|c| c.id == cid))
            .map(|c| c.name.as_str())
            .unwrap_or("");
        result = result.replace("{city}", city_str);
    }
    if result.contains("{unit}") {
        let unit_str = ctx.selected_unit
            .and_then(|uid| {
                let unit = ctx.state.units.iter().find(|u| u.id == uid)?;
                let type_name = ctx.state.unit_type_defs.iter()
                    .find(|d| d.id == unit.unit_type)
                    .map(|d| d.name)
                    .unwrap_or("?");
                let short = ctx.unit_short_ids.short(uid);
                Some(format!("[{short} {type_name}]"))
            })
            .unwrap_or_default();
        result = result.replace("{unit}", &unit_str);
    }
    if result.contains("{district}") {
        let dist_str = ctx.selected_district
            .map(|d| d.name())
            .unwrap_or("");
        result = result.replace("{district}", dist_str);
    }

    // Collapse runs of multiple spaces into one (from empty placeholders).
    let mut prev_space = false;
    result = result
        .chars()
        .filter(|&c| {
            if c == ' ' {
                if prev_space { return false; }
                prev_space = true;
            } else {
                prev_space = false;
            }
            true
        })
        .collect();

    result
}
