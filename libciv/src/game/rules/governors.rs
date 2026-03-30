//! Governor handlers: `assign_governor`, `promote_governor`, `civic_grants_governor_title`.

use crate::{CityId, GovernorId};

use super::RulesError;
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;

/// Assign (or reassign) a governor to a city.
pub(crate) fn assign_governor(
    state: &mut GameState,
    governor_id: GovernorId,
    city_id: CityId,
) -> Result<GameStateDiff, RulesError> {
    let (owner, current_city) = {
        let gov = state.governors.iter()
            .find(|g| g.id == governor_id)
            .ok_or(RulesError::GovernorNotFound)?;
        (gov.owner, gov.assigned_city)
    };

    let city = state.cities.iter()
        .find(|c| c.id == city_id)
        .ok_or(RulesError::CityNotFound)?;
    if city.owner != owner {
        return Err(RulesError::GovernorNotOwned);
    }

    if current_city == Some(city_id) {
        return Err(RulesError::GovernorAlreadyInCity);
    }

    let gov = state.governors.iter_mut()
        .find(|g| g.id == governor_id)
        .unwrap();
    gov.assigned_city = Some(city_id);
    gov.turns_to_establish = 5;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::GovernorAssigned { governor: governor_id, city: city_id, owner });
    Ok(diff)
}

/// Unlock a promotion for a governor.
pub(crate) fn promote_governor(
    state: &mut GameState,
    governor_id: GovernorId,
    promotion_name: &'static str,
) -> Result<GameStateDiff, RulesError> {
    use crate::civ::governor::promotion_def;

    let (owner, def_name) = {
        let gov = state.governors.iter()
            .find(|g| g.id == governor_id)
            .ok_or(RulesError::GovernorNotFound)?;
        (gov.owner, gov.def_name)
    };

    let promo = promotion_def(promotion_name)
        .ok_or(RulesError::PromotionNotFound)?;
    if promo.governor != def_name {
        return Err(RulesError::PromotionNotFound);
    }

    {
        let gov = state.governors.iter()
            .find(|g| g.id == governor_id).unwrap();
        if gov.has_promotion(promotion_name) {
            return Err(RulesError::PromotionAlreadyUnlocked);
        }
        for &req in promo.requires {
            if !gov.has_promotion(req) {
                return Err(RulesError::PromotionPrerequisiteNotMet);
            }
        }
    }

    let civ = state.civilizations.iter()
        .find(|c| c.id == owner)
        .ok_or(RulesError::CivNotFound)?;
    if civ.governor_titles < 1 {
        return Err(RulesError::InsufficientGovernorTitles);
    }

    state.civilizations.iter_mut()
        .find(|c| c.id == owner).unwrap()
        .governor_titles -= 1;
    state.governors.iter_mut()
        .find(|g| g.id == governor_id).unwrap()
        .promotions.push(promotion_name);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::GovernorPromoted { governor: governor_id, promotion: promotion_name });
    Ok(diff)
}

/// Returns true if completing this civic grants a governor title.
pub(crate) fn civic_grants_governor_title(civic_name: &str) -> bool {
    matches!(
        civic_name,
        "Code of Laws"
            | "State Workforce"
            | "Early Empire"
            | "Diplomatic Service"
            | "Medieval Faires"
            | "Guilds"
            | "Civil Service"
            | "Nationalism"
            | "Mass Media"
            | "Mobilization"
            | "Globalization"
    )
}
