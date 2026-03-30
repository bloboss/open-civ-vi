//! Great people handlers: `retire_great_person`, `create_great_work`, `recruit_great_person`,
//! `great_person_cs_bonus`.

use crate::{CivId, GreatPersonId, GreatPersonType};
use crate::rules::modifier::EffectType;

use super::RulesError;
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;

/// Sum combat strength bonuses from retired great persons for a given civ and
/// unit domain. Returns the total flat CS bonus.
pub(crate) fn great_person_cs_bonus(state: &GameState, civ_id: CivId, domain: crate::UnitDomain) -> u32 {
    let civ = match state.civ(civ_id) {
        Some(c) => c,
        None => return 0,
    };
    let mut total = 0i32;
    for m in &civ.great_person_modifiers {
        let domain_matches = match &m.target {
            crate::rules::modifier::TargetSelector::UnitDomain(d) => *d == domain,
            crate::rules::modifier::TargetSelector::AllUnits => true,
            crate::rules::modifier::TargetSelector::Global => true,
            _ => false,
        };
        if domain_matches
            && let EffectType::CombatStrengthFlat(v) = m.effect
        {
            total += v;
        }
    }
    total.max(0) as u32
}

/// Retire (consume) a great person, applying its one-time ability.
pub(crate) fn retire_great_person(
    state: &mut GameState,
    great_person_id: GreatPersonId,
) -> Result<GameStateDiff, RulesError> {
    use crate::civ::great_people::RetireEffect;
    use crate::rules::modifier::{Modifier, ModifierSource, TargetSelector};

    let gp_idx = state.great_people.iter().position(|gp| gp.id == great_person_id)
        .ok_or(RulesError::GreatPersonNotFound)?;

    if state.great_people[gp_idx].is_retired {
        return Err(RulesError::GreatPersonAlreadyRetired);
    }

    let gp_name = state.great_people[gp_idx].name;
    let gp_owner = state.great_people[gp_idx].owner
        .ok_or(RulesError::GreatPersonNotFound)?;
    let gp_coord = state.great_people[gp_idx].coord
        .ok_or(RulesError::GreatPersonNotFound)?;

    let retire_effect = state.great_person_defs.iter()
        .find(|d| d.name == gp_name)
        .ok_or(RulesError::GreatPersonDefNotFound)?
        .retire_effect.clone();

    let mut diff = GameStateDiff::new();

    match retire_effect {
        RetireEffect::CombatStrengthBonus { domain, bonus } => {
            let modifier = Modifier::new(
                ModifierSource::Custom(gp_name),
                TargetSelector::UnitDomain(domain),
                EffectType::CombatStrengthFlat(bonus),
                crate::rules::modifier::StackingRule::Additive,
            );
            let civ = state.civilizations.iter_mut()
                .find(|c| c.id == gp_owner)
                .ok_or(RulesError::CivNotFound)?;
            civ.great_person_modifiers.push(modifier);
        }
        RetireEffect::ProductionBurst { amount } => {
            let nearest_city_idx = state.cities.iter()
                .enumerate()
                .filter(|(_, c)| c.owner == gp_owner)
                .min_by_key(|(_, c)| gp_coord.distance(&c.coord))
                .map(|(i, _)| i)
                .ok_or(RulesError::CityNotFound)?;

            state.cities[nearest_city_idx].production_stored += amount;
            diff.push(StateDelta::ProductionBurst {
                city: state.cities[nearest_city_idx].id,
                amount,
            });
        }
        RetireEffect::GoldGrant { amount } => {
            let civ = state.civilizations.iter_mut()
                .find(|c| c.id == gp_owner)
                .ok_or(RulesError::CivNotFound)?;
            civ.gold += amount as i32;
            diff.push(StateDelta::GoldChanged {
                civ: gp_owner,
                delta: amount as i32,
            });
        }
    }

    state.great_people[gp_idx].is_retired = true;

    state.units.retain(|u| {
        !(u.owner == gp_owner
            && u.category == crate::UnitCategory::GreatPerson
            && u.coord == gp_coord)
    });

    diff.push(StateDelta::GreatPersonRetired {
        great_person: great_person_id,
        owner: gp_owner,
    });

    Ok(diff)
}

/// Create a great work from a great person (Writer/Artist/Musician).
pub(crate) fn create_great_work(
    state: &mut GameState,
    great_person_id: GreatPersonId,
) -> Result<GameStateDiff, RulesError> {
    use crate::civ::great_works::{GreatWork, GreatWorkType};

    let gp = state.great_people.iter()
        .find(|gp| gp.id == great_person_id)
        .ok_or(RulesError::GreatPersonNotFound)?;
    if gp.is_retired || gp.owner.is_none() {
        return Err(RulesError::GreatPersonNotFound);
    }
    let civ_id = gp.owner.unwrap();

    let work_type = match gp.person_type {
        GreatPersonType::Writer  => GreatWorkType::Writing,
        GreatPersonType::Artist  => GreatWorkType::Art,
        GreatPersonType::Musician => GreatWorkType::Music,
        _ => return Err(RulesError::InvalidGreatPersonType),
    };

    let gp_name = gp.name;

    let (tourism, culture) = match work_type {
        GreatWorkType::Writing  => (2, 2),
        GreatWorkType::Art      => (3, 3),
        GreatWorkType::Music    => (4, 4),
        GreatWorkType::Relic    => (8, 4),
        GreatWorkType::Artifact => (3, 0),
    };

    let slot_city = state.cities.iter()
        .filter(|c| c.owner == civ_id)
        .find(|c| c.great_work_slots.iter().any(|s| s.is_empty() && s.slot_type.accepts(work_type)))
        .map(|c| c.id);

    let city_id = slot_city.ok_or(RulesError::NoGreatWorkSlot)?;

    let work_id = state.id_gen.next_great_work_id();
    state.great_works.push(GreatWork {
        id: work_id,
        name: gp_name,
        work_type,
        creator: Some(civ_id),
        tourism,
        culture,
    });

    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id)
        && let Some(slot) = city.great_work_slots.iter_mut()
            .find(|s| s.is_empty() && s.slot_type.accepts(work_type))
    {
        slot.work = Some(work_id);
    }

    if let Some(gp) = state.great_people.iter_mut().find(|gp| gp.id == great_person_id) {
        gp.is_retired = true;
    }

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::GreatWorkCreated { civ: civ_id, work_name: gp_name, city: city_id });
    Ok(diff)
}

/// Patronize (sponsor) a Great Prophet by spending faith.
pub(crate) fn recruit_great_person_with_faith(
    state: &mut GameState,
    civ_id: CivId,
    person_type: GreatPersonType,
) -> Result<GameStateDiff, RulesError> {
    use crate::civ::great_people::{
        recruitment_threshold, next_candidate_name, spawn_great_person,
        GP_PATRONAGE_FAITH_PER_POINT,
    };

    if person_type != GreatPersonType::Prophet {
        return Err(RulesError::InvalidGreatPersonType);
    }

    let civ = state.civ(civ_id).ok_or(RulesError::CivNotFound)?;

    let def_name = next_candidate_name(person_type, state)
        .ok_or(RulesError::NoGreatPersonAvailable)?;

    let current_points = civ.great_person_points.get(&person_type).copied().unwrap_or(0);
    let threshold = recruitment_threshold(person_type, state);
    let points_needed = threshold.saturating_sub(current_points);
    let faith_cost = points_needed * GP_PATRONAGE_FAITH_PER_POINT;

    if civ.faith < faith_cost {
        return Err(RulesError::InsufficientFaith);
    }

    let spawn_coord = state.cities.iter()
        .find(|c| c.owner == civ_id && c.is_capital)
        .or_else(|| state.cities.iter().find(|c| c.owner == civ_id))
        .map(|c| c.coord)
        .ok_or(RulesError::CityNotFound)?;

    let civ = state.civilizations.iter_mut()
        .find(|c| c.id == civ_id).unwrap();
    civ.faith -= faith_cost;
    civ.great_person_points.insert(person_type, 0);

    let gp_id = spawn_great_person(state, civ_id, def_name, spawn_coord);

    let mut diff = GameStateDiff::new();
    if faith_cost > 0 {
        diff.push(StateDelta::FaithChanged { civ: civ_id, delta: -(faith_cost as i32) });
    }
    diff.push(StateDelta::GreatPersonPatronizedWithFaith {
        great_person: gp_id,
        civ: civ_id,
        faith_spent: faith_cost,
    });
    Ok(diff)
}

/// Patronize (sponsor) a great person by spending gold.
pub(crate) fn recruit_great_person(
    state: &mut GameState,
    civ_id: CivId,
    person_type: GreatPersonType,
) -> Result<GameStateDiff, RulesError> {
    use crate::civ::great_people::{
        recruitment_threshold, next_candidate_name, spawn_great_person,
        GP_PATRONAGE_GOLD_PER_POINT,
    };

    let civ = state.civ(civ_id).ok_or(RulesError::CivNotFound)?;

    let def_name = next_candidate_name(person_type, state)
        .ok_or(RulesError::NoGreatPersonAvailable)?;

    let current_points = civ.great_person_points.get(&person_type).copied().unwrap_or(0);
    let threshold = recruitment_threshold(person_type, state);
    let points_needed = threshold.saturating_sub(current_points);
    let gold_cost = points_needed * GP_PATRONAGE_GOLD_PER_POINT;
    let gold_cost_signed = gold_cost as i32;

    if civ.gold < gold_cost_signed {
        return Err(RulesError::InsufficientGold);
    }

    let spawn_coord = state.cities.iter()
        .find(|c| c.owner == civ_id && c.is_capital)
        .or_else(|| state.cities.iter().find(|c| c.owner == civ_id))
        .map(|c| c.coord)
        .ok_or(RulesError::CityNotFound)?;

    let civ = state.civilizations.iter_mut()
        .find(|c| c.id == civ_id).unwrap();
    civ.gold -= gold_cost_signed;
    civ.great_person_points.insert(person_type, 0);

    let gp_id = spawn_great_person(state, civ_id, def_name, spawn_coord);

    let mut diff = GameStateDiff::new();
    if gold_cost > 0 {
        diff.push(StateDelta::GoldChanged { civ: civ_id, delta: -gold_cost_signed });
    }
    diff.push(StateDelta::GreatPersonPatronized {
        great_person: gp_id,
        civ: civ_id,
        gold_spent: gold_cost,
    });
    Ok(diff)
}
