use crate::CivId;
use crate::civ::CityKind;
use super::state::GameState;

/// Compute a score for one civilization from current game state.
///
/// Components:
/// - Cities owned (excluding city-states): ×5
/// - Total population across owned cities: ×1
/// - Technologies researched: ×3
/// - Civics completed: ×2
/// - Territory tiles claimed across owned cities: ÷5
pub fn compute_score(state: &GameState, civ_id: CivId) -> u32 {
    let civ = match state.civ(civ_id) {
        Some(c) => c,
        None => return 0,
    };

    let cities_owned: Vec<_> = state.cities.iter()
        .filter(|c| c.owner == civ_id && !matches!(c.kind, CityKind::CityState(_)))
        .collect();

    let city_score      = cities_owned.len() as u32 * 5;
    let pop_score: u32  = cities_owned.iter().map(|c| c.population).sum();
    let tech_score      = civ.researched_techs.len() as u32 * 3;
    let civic_score     = civ.completed_civics.len() as u32 * 2;
    let territory_score = cities_owned.iter()
        .map(|c| c.territory.len() as u32)
        .sum::<u32>()
        / 5;

    city_score + pop_score + tech_score + civic_score + territory_score
}

/// Returns scores for all non-city-state civilizations, sorted highest first.
pub fn all_scores(state: &GameState) -> Vec<(CivId, u32)> {
    let mut scores: Vec<(CivId, u32)> = state.civilizations.iter()
        .filter(|civ| {
            // Include only civs that own at least one regular city.
            state.cities.iter().any(|city| {
                city.owner == civ.id && !matches!(city.kind, CityKind::CityState(_))
            })
        })
        .map(|civ| (civ.id, compute_score(state, civ.id)))
        .collect();
    scores.sort_by(|a, b| b.1.cmp(&a.1));
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::state::GameState;

    fn make_state() -> GameState {
        GameState::new(42, 10, 10)
    }

    #[test]
    fn test_compute_score_unknown_civ() {
        use crate::ids::CivId;
        use ulid::Ulid;
        let state = make_state();
        let fake_id = CivId::from_ulid(Ulid::nil());
        assert_eq!(compute_score(&state, fake_id), 0);
    }

    #[test]
    fn test_all_scores_empty() {
        let state = make_state();
        assert!(all_scores(&state).is_empty());
    }
}
