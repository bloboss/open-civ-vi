use crate::CivId;
use super::civilization::Civilization;
use crate::game::state::GameState;

/// A built wonder that generates tourism. Registered on `GameState::wonder_tourism`.
#[derive(Debug, Clone)]
pub struct WonderTourism {
    pub name: &'static str,
    pub owner: CivId,
    pub tourism_per_turn: u32,
}

/// Compute the total tourism output per turn for a civilization.
///
/// Tourism sources:
/// - Built wonders registered in `GameState::wonder_tourism`
/// - Building yields with tourism > 0 (future: great works, national parks)
pub fn compute_tourism(state: &GameState, civ_id: CivId) -> u32 {
    let mut tourism = 0u32;

    // Tourism from built wonders.
    for wt in &state.wonder_tourism {
        if wt.owner == civ_id {
            tourism += wt.tourism_per_turn;
        }
    }

    // Tourism from building defs that yield tourism (if any city owns them).
    for city in state.cities.iter().filter(|c| c.owner == civ_id) {
        for &bid in &city.buildings {
            if let Some(bdef) = state.building_defs.iter().find(|b| b.id == bid)
                && bdef.yields.tourism > 0
            {
                tourism += bdef.yields.tourism as u32;
            }
        }
    }

    tourism
}

/// Compute domestic tourists for a civilization.
///
/// Domestic tourists represent the "cultural defense" of a civ. A civ with
/// more lifetime culture is harder to achieve a culture victory against.
///
/// Formula: `lifetime_culture / 100` (integer division).
pub fn domestic_tourists(civ: &Civilization) -> u32 {
    civ.lifetime_culture / 100
}

/// Check whether `civ_id` has achieved cultural dominance over all other civs.
///
/// Cultural dominance means: for every other civilization B,
/// `civ.tourism_accumulated[B] > B.domestic_tourists()`.
///
/// A civ with zero lifetime culture has 0 domestic tourists, so any positive
/// tourism accumulated against them counts as dominance.
pub fn has_cultural_dominance(state: &GameState, civ_id: CivId) -> bool {
    let civ = match state.civ(civ_id) {
        Some(c) => c,
        None => return false,
    };

    let other_civs: Vec<&Civilization> = state.civilizations.iter()
        .filter(|c| c.id != civ_id)
        .collect();

    // Need at least one other civ to dominate.
    if other_civs.is_empty() {
        return false;
    }

    for other in &other_civs {
        let tourists_sent = civ.tourism_accumulated
            .get(&other.id)
            .copied()
            .unwrap_or(0);
        let their_domestic = domestic_tourists(other);
        // Must strictly exceed domestic tourists.
        if tourists_sent <= their_domestic {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::civilization::{Leader, Civilization};
    use crate::civ::Agenda;
    use ulid::Ulid;
    use std::collections::HashMap;

    struct NoOp;
    impl std::fmt::Debug for NoOp {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "NoOp") }
    }
    impl Agenda for NoOp {
        fn name(&self) -> &'static str { "noop" }
        fn description(&self) -> &'static str { "" }
        fn attitude(&self, _: CivId) -> i32 { 0 }
    }

    fn make_civ(n: u64) -> Civilization {
        let id = CivId::from_ulid(Ulid::from_parts(n, 0));
        Civilization::new(id, "Test", "Test",
            Leader { name: "L", civ_id: id, abilities: vec![], agenda: Box::new(NoOp) })
    }

    #[test]
    fn domestic_tourists_scales_with_culture() {
        let mut civ = make_civ(1);
        assert_eq!(domestic_tourists(&civ), 0);
        civ.lifetime_culture = 99;
        assert_eq!(domestic_tourists(&civ), 0);
        civ.lifetime_culture = 100;
        assert_eq!(domestic_tourists(&civ), 1);
        civ.lifetime_culture = 550;
        assert_eq!(domestic_tourists(&civ), 5);
    }

    #[test]
    fn cultural_dominance_requires_exceeding_domestic() {
        let mut state = GameState::new(42, 6, 6);
        let mut civ_a = make_civ(1);
        let mut civ_b = make_civ(2);

        // B has 200 lifetime culture → 2 domestic tourists.
        civ_b.lifetime_culture = 200;

        // A has sent 2 tourism toward B — ties, not enough.
        civ_a.tourism_accumulated.insert(civ_b.id, 2);

        state.civilizations.push(civ_a);
        state.civilizations.push(civ_b);

        let civ_a_id = state.civilizations[0].id;
        assert!(!has_cultural_dominance(&state, civ_a_id));

        // Increase to 3 — now exceeds.
        let target_id = state.civilizations[1].id;
        state.civilizations[0].tourism_accumulated.insert(target_id, 3);
        assert!(has_cultural_dominance(&state, civ_a_id));
    }

    #[test]
    fn cultural_dominance_needs_all_civs() {
        let mut state = GameState::new(42, 6, 6);
        let mut civ_a = make_civ(1);
        let civ_b = make_civ(2);
        let mut civ_c = make_civ(3);

        // B has 0 culture → 0 domestic tourists.
        // C has 500 culture → 5 domestic tourists.
        civ_c.lifetime_culture = 500;

        // A dominates B (1 > 0) but not C (1 <= 5).
        civ_a.tourism_accumulated = HashMap::from([
            (civ_b.id, 1),
            (civ_c.id, 1),
        ]);

        let civ_a_id = civ_a.id;
        state.civilizations.push(civ_a);
        state.civilizations.push(civ_b);
        state.civilizations.push(civ_c);

        assert!(!has_cultural_dominance(&state, civ_a_id));
    }
}
