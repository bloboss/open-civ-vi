//! Structured observation extracted from [`GameState`] for a single civilization.

use crate::CivId;
use crate::civ::CityKind;
use crate::game::compute_score;
use crate::game::state::GameState;

/// Flat observation vector for one civilization at one point in time.
///
/// Designed to be easy to convert into a tensor for RL training.
/// All counts are simple scalars; per-city / per-tile details are omitted
/// in this first version to keep the observation space small.
#[derive(Debug, Clone)]
pub struct Observation {
    pub turn: u32,
    pub gold: i32,
    pub faith: u32,
    pub science_per_turn: i32,
    pub culture_per_turn: i32,
    pub num_cities: u32,
    pub num_units: u32,
    pub score: u32,
    pub techs_researched: u32,
    pub civics_completed: u32,
    pub diplomatic_favor: u32,
    pub science_milestones: u32,
    pub game_over: bool,
    pub is_winner: bool,
}

/// Extract an [`Observation`] from the current game state for `civ_id`.
pub fn observe(state: &GameState, civ_id: CivId) -> Observation {
    let civ = state.civ(civ_id);

    let gold = civ.map_or(0, |c| c.gold);
    let faith = civ.map_or(0, |c| c.faith);
    let techs_researched = civ.map_or(0, |c| c.researched_techs.len() as u32);
    let civics_completed = civ.map_or(0, |c| c.completed_civics.len() as u32);
    let diplomatic_favor = civ.map_or(0, |c| c.diplomatic_favor);
    let science_milestones = civ.map_or(0, |c| c.science_milestones_completed);

    let num_cities = state
        .cities
        .iter()
        .filter(|c| c.owner == civ_id && !matches!(c.kind, CityKind::CityState(_)))
        .count() as u32;

    let num_units = state.units.iter().filter(|u| u.owner == civ_id).count() as u32;

    let score = compute_score(state, civ_id);

    // Yield-per-turn figures require the rules engine; for now we store 0
    // and let the caller fill them in if needed. This avoids coupling
    // observation to the rules engine.
    let science_per_turn = 0;
    let culture_per_turn = 0;

    let game_over = state.game_over.is_some();
    let is_winner = state
        .game_over
        .as_ref()
        .is_some_and(|go| go.winner == civ_id);

    Observation {
        turn: state.turn,
        gold,
        faith,
        science_per_turn,
        culture_per_turn,
        num_cities,
        num_units,
        score,
        techs_researched,
        civics_completed,
        diplomatic_favor,
        science_milestones,
        game_over,
        is_winner,
    }
}
