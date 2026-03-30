//! Religion handlers: `found_pantheon`, `found_religion`, `spread_religion`, `purchase_with_faith`.

use crate::{CityId, CivId, UnitId};
use crate::civ::BasicUnit;

use super::{FaithPurchaseItem, RulesError};
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;

/// Found a pantheon for `civ`.
pub(crate) fn found_pantheon(
    state: &mut GameState,
    civ_id: CivId,
    belief_id: crate::BeliefId,
) -> Result<GameStateDiff, RulesError> {
    let civ = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    if civ.pantheon_belief.is_some() {
        return Err(RulesError::PantheonAlreadyFounded);
    }
    if civ.faith < 25 {
        return Err(RulesError::InsufficientFaith);
    }
    let valid = state.belief_defs.iter().any(|b| b.id == belief_id
        && b.category == crate::civ::religion::BeliefCategory::Follower);
    if !valid {
        return Err(RulesError::InvalidBelief);
    }
    let taken = state.civilizations.iter()
        .any(|c| c.pantheon_belief == Some(belief_id));
    if taken {
        return Err(RulesError::InvalidBelief);
    }

    let civ = state.civilizations.iter_mut()
        .find(|c| c.id == civ_id).unwrap();
    civ.pantheon_belief = Some(belief_id);
    civ.faith -= 25;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::FaithChanged { civ: civ_id, delta: -25 });
    diff.push(StateDelta::PantheonFounded { civ: civ_id, belief: belief_id });
    Ok(diff)
}

/// Found a religion using a Great Prophet at a Holy Site.
pub(crate) fn found_religion(
    state: &mut GameState,
    prophet_unit: UnitId,
    name: String,
    belief_ids: Vec<crate::BeliefId>,
) -> Result<GameStateDiff, RulesError> {
    use crate::civ::religion::{BeliefCategory, Religion};

    let unit = state.units.iter()
        .find(|u| u.id == prophet_unit)
        .ok_or(RulesError::UnitNotFound)?;
    if unit.category != crate::UnitCategory::GreatPerson {
        return Err(RulesError::NotAGreatProphet);
    }
    let civ_id = unit.owner;
    let unit_coord = unit.coord;

    let civ = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    if civ.founded_religion.is_some() {
        return Err(RulesError::ReligionAlreadyFounded);
    }

    let holy_site_city = state.placed_districts.iter()
        .find(|pd| pd.coord == unit_coord
            && pd.district_type == crate::civ::district::BuiltinDistrict::HolySite);
    let holy_site = holy_site_city.ok_or(RulesError::NoHolySite)?;
    let holy_city_id = holy_site.city_id;

    if state.religions.iter().any(|r| r.name == name) {
        return Err(RulesError::ReligionNameTaken);
    }

    if belief_ids.len() != 2 {
        return Err(RulesError::InvalidBelief);
    }
    let has_founder = belief_ids.iter().any(|bid|
        state.belief_defs.iter().any(|b| b.id == *bid && b.category == BeliefCategory::Founder));
    let has_follower = belief_ids.iter().any(|bid|
        state.belief_defs.iter().any(|b| b.id == *bid && b.category == BeliefCategory::Follower));
    if !has_founder || !has_follower {
        return Err(RulesError::InvalidBelief);
    }
    for bid in &belief_ids {
        let taken = state.religions.iter().any(|r| r.beliefs.contains(bid));
        if taken {
            return Err(RulesError::InvalidBelief);
        }
    }

    let religion_id = crate::ReligionId::from_ulid(state.id_gen.next_ulid());
    let mut religion = Religion::new(religion_id, name.clone(), civ_id, holy_city_id);
    religion.beliefs = belief_ids.clone();
    state.religions.push(religion);

    let civ = state.civilizations.iter_mut()
        .find(|c| c.id == civ_id).unwrap();
    civ.founded_religion = Some(religion_id);

    if let Some(city) = state.cities.iter_mut().find(|c| c.id == holy_city_id) {
        city.religious_followers.insert(religion_id, city.population);
    }

    state.units.retain(|u| u.id != prophet_unit);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::ReligionFounded { civ: civ_id, religion: religion_id, name });
    for bid in &belief_ids {
        diff.push(StateDelta::BeliefSelected { civ: civ_id, religion: religion_id, belief: *bid });
    }
    diff.push(StateDelta::UnitDestroyed { unit: prophet_unit });
    diff.push(StateDelta::HistoricMomentEarned { civ: civ_id, moment: "religion_founded", era_score: 3 });
    Ok(diff)
}

/// Spread religion from a Missionary or Apostle to the city at the unit's current location.
pub(crate) fn spread_religion(
    state: &mut GameState,
    unit_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    let unit = state.units.iter()
        .find(|u| u.id == unit_id)
        .ok_or(RulesError::UnitNotFound)?;
    if unit.category != crate::UnitCategory::Religious {
        return Err(RulesError::NotAReligiousUnit);
    }
    let religion_id = unit.religion_id.ok_or(RulesError::NotAReligiousUnit)?;
    let charges = unit.spread_charges.ok_or(RulesError::NoSpreadCharges)?;
    if charges == 0 {
        return Err(RulesError::NoSpreadCharges);
    }
    let unit_coord = unit.coord;

    let city = state.cities.iter()
        .find(|c| c.coord == unit_coord)
        .ok_or(RulesError::CityNotFound)?;
    let city_id = city.id;

    let followers_added = 200u32.min(city.population);

    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        *city.religious_followers.entry(religion_id).or_insert(0) += followers_added;
        let total: u32 = city.religious_followers.values().sum();
        if total > city.population {
            let scale = city.population as f64 / total as f64;
            for v in city.religious_followers.values_mut() {
                *v = (*v as f64 * scale).floor() as u32;
            }
        }
    }

    let unit = state.units.iter_mut()
        .find(|u| u.id == unit_id).unwrap();
    let new_charges = charges - 1;
    unit.spread_charges = Some(new_charges);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::ReligionSpread { city: city_id, religion: religion_id, followers_added });

    if let Some(city) = state.cities.iter().find(|c| c.id == city_id) {
        let new_majority = city.majority_religion();
        if new_majority == Some(religion_id) {
            diff.push(StateDelta::CityConvertedReligion {
                city: city_id,
                old_religion: None,
                new_religion: religion_id,
            });
        }
    }

    if new_charges == 0 {
        state.units.retain(|u| u.id != unit_id);
        diff.push(StateDelta::UnitDestroyed { unit: unit_id });
    }

    if let Some(u) = state.units.iter_mut().find(|u| u.id == unit_id) {
        u.movement_left = 0;
    }

    Ok(diff)
}

/// Purchase a unit or building with faith.
pub(crate) fn purchase_with_faith(
    state: &mut GameState,
    civ_id: CivId,
    city_id: CityId,
    item: FaithPurchaseItem,
) -> Result<GameStateDiff, RulesError> {
    let civ = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    let city = state.cities.iter()
        .find(|c| c.id == city_id)
        .ok_or(RulesError::CityNotFound)?;

    match item {
        FaithPurchaseItem::Unit(name) => {
            let utd = state.unit_type_defs.iter()
                .find(|u| u.name == name)
                .ok_or(RulesError::UnitNotFound)?;

            let cost = match name {
                "Missionary" => 100u32,
                "Apostle" => 200u32,
                _ => 150u32,
            };

            if civ.faith < cost {
                return Err(RulesError::InsufficientFaith);
            }

            if !city.districts.contains(&crate::civ::district::BuiltinDistrict::HolySite) {
                return Err(RulesError::MissingPrerequisite);
            }

            let religion_id = civ.founded_religion
                .or(civ.pantheon_belief.and_then(|_| {
                    city.majority_religion()
                }));

            let base_charges: u8 = match name {
                "Missionary" => 3,
                "Apostle" => 3,
                _ => 0,
            };
            let religious_strength = match name {
                "Apostle" => Some(110u32),
                _ => None,
            };

            let unit_id = state.id_gen.next_unit_id();
            let coord = city.coord;
            let new_unit = BasicUnit {
                id: unit_id,
                unit_type: utd.id,
                owner: civ_id,
                coord,
                domain: utd.domain,
                category: utd.category,
                movement_left: utd.max_movement,
                max_movement: utd.max_movement,
                combat_strength: utd.combat_strength,
                promotions: Vec::new(),
                health: 100,
                range: utd.range,
                vision_range: utd.vision_range,
                charges: None,
                trade_origin: None,
                trade_destination: None,
                religion_id,
                spread_charges: if base_charges > 0 { Some(base_charges) } else { None },
                religious_strength,
            };
            state.units.push(new_unit);

            let civ = state.civilizations.iter_mut()
                .find(|c| c.id == civ_id).unwrap();
            civ.faith -= cost;

            let mut diff = GameStateDiff::new();
            diff.push(StateDelta::FaithChanged { civ: civ_id, delta: -(cost as i32) });
            diff.push(StateDelta::UnitCreated { unit: unit_id, coord, owner: civ_id });
            Ok(diff)
        }
        FaithPurchaseItem::WorshipBuilding(belief_name) => {
            let religion = civ.founded_religion.and_then(|rid|
                state.religions.iter().find(|r| r.id == rid));
            let religion = religion.ok_or(RulesError::MissingPrerequisite)?;
            let has_worship = religion.beliefs.iter().any(|bid|
                state.belief_defs.iter().any(|b| b.id == *bid
                    && b.category == crate::civ::religion::BeliefCategory::Worship
                    && b.name == belief_name));
            if !has_worship {
                return Err(RulesError::MissingPrerequisite);
            }

            if !city.districts.contains(&crate::civ::district::BuiltinDistrict::HolySite) {
                return Err(RulesError::MissingPrerequisite);
            }
            let has_temple = city.buildings.iter().any(|bid|
                state.building_defs.iter().any(|b| b.id == *bid && b.name == "Temple"));
            if !has_temple {
                return Err(RulesError::MissingPrerequisite);
            }

            let cost = 200u32;
            if civ.faith < cost {
                return Err(RulesError::InsufficientFaith);
            }

            let building_id = state.building_defs.iter()
                .find(|b| b.name == belief_name)
                .map(|b| b.id)
                .unwrap_or_else(|| state.id_gen.next_building_id());

            if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
                city.buildings.push(building_id);
            }

            let civ = state.civilizations.iter_mut()
                .find(|c| c.id == civ_id).unwrap();
            civ.faith -= cost;

            let mut diff = GameStateDiff::new();
            diff.push(StateDelta::FaithChanged { civ: civ_id, delta: -(cost as i32) });
            diff.push(StateDelta::BuildingCompleted { city: city_id, building: belief_name });
            Ok(diff)
        }
    }
}
