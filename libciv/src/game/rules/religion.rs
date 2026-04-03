//! Religion handlers: `found_pantheon`, `found_religion`, `spread_religion`,
//! `purchase_with_faith`, `evangelize_belief`, `launch_inquisition`,
//! `remove_heresy`, `guru_heal`.

use crate::{CityId, CivId, UnitId};
use crate::civ::BasicUnit;

use super::{FaithPurchaseItem, RulesError};
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;

// ── Constants ────────────────────────────────────────────────────────────────

const PANTHEON_FAITH_COST: u32 = 25;
const MISSIONARY_BASE_COST: u32 = 250;
const APOSTLE_BASE_COST: u32 = 400;
const INQUISITOR_BASE_COST: u32 = 200;
const GURU_BASE_COST: u32 = 250;
const WORSHIP_BUILDING_COST: u32 = 200;
/// Cost increase per unit purchased of the same type.
const FAITH_PURCHASE_SCALING: u32 = 50;
/// Max beliefs on a religion (Founder + Follower + Worship + Enhancer).
const MAX_RELIGION_BELIEFS: usize = 4;
/// Missionary spread: reduce other religions by this fraction.
const MISSIONARY_OTHER_REDUCTION: f64 = 0.10;
/// Apostle spread: reduce other religions by this fraction.
const APOSTLE_OTHER_REDUCTION: f64 = 0.25;
/// Inquisitor Remove Heresy: remove this fraction of foreign followers.
const INQUISITOR_HERESY_REDUCTION: f64 = 0.75;
/// Guru heal amount per charge.
const GURU_HEAL_AMOUNT: u32 = 40;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Look up the religion that a civilization founded, checking enhancer beliefs.
fn civ_religion_has_belief(state: &GameState, civ_id: CivId, belief_name: &str) -> bool {
    let Some(civ) = state.civilizations.iter().find(|c| c.id == civ_id) else { return false };
    let Some(rid) = civ.founded_religion else { return false };
    let Some(religion) = state.religions.iter().find(|r| r.id == rid) else { return false };
    religion.beliefs.iter().any(|bid|
        state.belief_defs.iter().any(|b| b.id == *bid && b.name == belief_name))
}

/// Compute the scaled faith cost for a unit type, applying per-purchase scaling
/// and enhancer discounts.
fn scaled_faith_cost(state: &GameState, civ_id: CivId, unit_name: &str, base_cost: u32) -> u32 {
    let count = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .and_then(|c| c.faith_purchase_counts.get(unit_name))
        .copied()
        .unwrap_or(0);
    let mut cost = base_cost + count * FAITH_PURCHASE_SCALING;

    // Holy Order: 30% discount on Missionaries and Apostles.
    if (unit_name == "Missionary" || unit_name == "Apostle")
        && civ_religion_has_belief(state, civ_id, "Holy Order")
    {
        cost = (cost as f64 * 0.70).round() as u32;
    }
    cost
}

// ── Pantheon ─────────────────────────────────────────────────────────────────

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
    if civ.faith < PANTHEON_FAITH_COST {
        return Err(RulesError::InsufficientFaith);
    }
    let valid = state.belief_defs.iter().any(|b| b.id == belief_id
        && b.category == crate::civ::religion::BeliefCategory::Pantheon);
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
    civ.faith -= PANTHEON_FAITH_COST;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::FaithChanged { civ: civ_id, delta: -(PANTHEON_FAITH_COST as i32) });
    diff.push(StateDelta::PantheonFounded { civ: civ_id, belief: belief_id });
    Ok(diff)
}

// ── Religion founding ────────────────────────────────────────────────────────

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

    // Religion limit: (num_major_civs / 2) + 1.
    let num_major_civs = state.civilizations.len();
    let max_religions = (num_major_civs / 2) + 1;
    if state.religions.len() >= max_religions {
        return Err(RulesError::MaxReligionsReached);
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

    // Canonical Civ VI: holy city gets half population + 1 as initial followers.
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == holy_city_id) {
        let initial = (city.population / 2) + 1;
        city.religious_followers.insert(religion_id, initial);
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

// ── Spread religion ──────────────────────────────────────────────────────────

/// Spread religion from a Missionary or Apostle to the city at the unit's current location.
/// Spread power = 2 * unit.health. Apostles also reduce other religions by 25%;
/// Missionaries reduce by 10%.
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
    let unit_health = unit.health;
    // Apostles have religious_strength; missionaries do not.
    let is_apostle = unit.religious_strength.is_some();

    let city = state.cities.iter()
        .find(|c| c.coord == unit_coord)
        .ok_or(RulesError::CityNotFound)?;
    let city_id = city.id;
    let old_majority = city.majority_religion();

    // Spread power: 2 * current HP (max 200 at full health).
    let followers_added = (2 * unit_health).min(city.population);

    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        // Reduce other religions first.
        let reduction_rate = if is_apostle { APOSTLE_OTHER_REDUCTION } else { MISSIONARY_OTHER_REDUCTION };
        for (rid, count) in city.religious_followers.iter_mut() {
            if *rid != religion_id {
                let reduce = (*count as f64 * reduction_rate).floor() as u32;
                *count = count.saturating_sub(reduce);
            }
        }

        // Add followers of the spreading religion.
        *city.religious_followers.entry(religion_id).or_insert(0) += followers_added;

        // Cap total followers at population.
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
        if new_majority != old_majority
            && let Some(new_rid) = new_majority
        {
            diff.push(StateDelta::CityConvertedReligion {
                city: city_id,
                old_religion: old_majority,
                new_religion: new_rid,
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

// ── Faith purchase ───────────────────────────────────────────────────────────

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

            // Canonical costs with per-purchase scaling.
            let base_cost = match name {
                "Missionary"  => MISSIONARY_BASE_COST,
                "Apostle"     => APOSTLE_BASE_COST,
                "Inquisitor"  => INQUISITOR_BASE_COST,
                "Guru"        => GURU_BASE_COST,
                _             => 250u32,
            };
            let cost = scaled_faith_cost(state, civ_id, name, base_cost);

            if civ.faith < cost {
                return Err(RulesError::InsufficientFaith);
            }

            // District/building prerequisites.
            if !city.districts.contains(&crate::civ::district::BuiltinDistrict::HolySite) {
                return Err(RulesError::MissingPrerequisite);
            }
            match name {
                "Missionary" => {
                    // Requires Shrine building.
                    let has_shrine = city.buildings.iter().any(|bid|
                        state.building_defs.iter().any(|b| b.id == *bid && b.name == "Shrine"));
                    if !has_shrine {
                        return Err(RulesError::MissingPrerequisite);
                    }
                }
                "Apostle" | "Guru" => {
                    // Requires Temple building.
                    let has_temple = city.buildings.iter().any(|bid|
                        state.building_defs.iter().any(|b| b.id == *bid && b.name == "Temple"));
                    if !has_temple {
                        return Err(RulesError::MissingPrerequisite);
                    }
                }
                "Inquisitor" => {
                    // Requires Launch Inquisition to have been used.
                    if !civ.inquisition_launched {
                        return Err(RulesError::InquisitionNotLaunched);
                    }
                }
                _ => {}
            }

            let religion_id = civ.founded_religion
                .or(civ.pantheon_belief.and_then(|_| {
                    city.majority_religion()
                }));

            // Spread charges: Missionary Zeal enhancer adds +2 to Missionaries.
            let mut base_charges: u8 = match name {
                "Missionary" => 3,
                "Apostle" => 3,
                _ => 0,
            };
            if name == "Missionary" && civ_religion_has_belief(state, civ_id, "Missionary Zeal") {
                base_charges += 2;
            }

            let religious_strength = match name {
                "Apostle" => Some(110u32),
                _ => None,
            };

            let inquisitor_charges: Option<u8> = if name == "Inquisitor" { Some(3) } else { None };
            let heal_charges: Option<u8> = if name == "Guru" { Some(3) } else { None };

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
                charges: inquisitor_charges.or(heal_charges),
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
            *civ.faith_purchase_counts.entry(name.to_string()).or_insert(0) += 1;

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

            let cost = WORSHIP_BUILDING_COST;
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

// ── Evangelize belief ────────────────────────────────────────────────────────

/// Use an Apostle to add a belief (Worship or Enhancer) to the civ's founded religion.
/// Consumes one spread charge. Religion can have at most 4 beliefs.
pub(crate) fn evangelize_belief(
    state: &mut GameState,
    apostle_id: UnitId,
    belief_id: crate::BeliefId,
) -> Result<GameStateDiff, RulesError> {
    use crate::civ::religion::BeliefCategory;

    let unit = state.units.iter()
        .find(|u| u.id == apostle_id)
        .ok_or(RulesError::UnitNotFound)?;
    if unit.category != crate::UnitCategory::Religious {
        return Err(RulesError::NotAnApostle);
    }
    if unit.religious_strength.is_none() {
        return Err(RulesError::NotAnApostle);
    }
    let charges = unit.spread_charges.ok_or(RulesError::NoSpreadCharges)?;
    if charges == 0 {
        return Err(RulesError::NoSpreadCharges);
    }
    let civ_id = unit.owner;

    let civ = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    let religion_id = civ.founded_religion.ok_or(RulesError::MissingPrerequisite)?;

    let religion = state.religions.iter()
        .find(|r| r.id == religion_id)
        .ok_or(RulesError::MissingPrerequisite)?;
    if religion.beliefs.len() >= MAX_RELIGION_BELIEFS {
        return Err(RulesError::ReligionFullyEnhanced);
    }

    // Belief must be Worship or Enhancer category.
    let belief_def = state.belief_defs.iter()
        .find(|b| b.id == belief_id)
        .ok_or(RulesError::InvalidBelief)?;
    if belief_def.category != BeliefCategory::Worship && belief_def.category != BeliefCategory::Enhancer {
        return Err(RulesError::InvalidBelief);
    }

    // Belief must not already be taken by another religion.
    let taken = state.religions.iter().any(|r| r.beliefs.contains(&belief_id));
    if taken {
        return Err(RulesError::InvalidBelief);
    }

    // Add belief to religion.
    let religion = state.religions.iter_mut()
        .find(|r| r.id == religion_id).unwrap();
    religion.beliefs.push(belief_id);

    // Consume one spread charge.
    let unit = state.units.iter_mut()
        .find(|u| u.id == apostle_id).unwrap();
    let new_charges = charges - 1;
    unit.spread_charges = Some(new_charges);
    unit.movement_left = 0;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::BeliefEvangelized { civ: civ_id, religion: religion_id, belief: belief_id });

    if new_charges == 0 {
        state.units.retain(|u| u.id != apostle_id);
        diff.push(StateDelta::UnitDestroyed { unit: apostle_id });
    }

    Ok(diff)
}

// ── Launch inquisition ───────────────────────────────────────────────────────

/// Use an Apostle to launch an inquisition, enabling Inquisitor purchases.
/// Consumes the Apostle unit.
pub(crate) fn launch_inquisition(
    state: &mut GameState,
    apostle_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    let unit = state.units.iter()
        .find(|u| u.id == apostle_id)
        .ok_or(RulesError::UnitNotFound)?;
    if unit.category != crate::UnitCategory::Religious {
        return Err(RulesError::NotAnApostle);
    }
    if unit.religious_strength.is_none() {
        return Err(RulesError::NotAnApostle);
    }
    let civ_id = unit.owner;

    // Set flag on the civilization.
    let civ = state.civilizations.iter_mut()
        .find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;
    civ.inquisition_launched = true;

    // Consume the Apostle.
    state.units.retain(|u| u.id != apostle_id);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::InquisitionLaunched { civ: civ_id });
    diff.push(StateDelta::UnitDestroyed { unit: apostle_id });
    Ok(diff)
}

// ── Remove heresy ────────────────────────────────────────────────────────────

/// Use an Inquisitor to remove 75% of foreign religion followers from the city
/// at the unit's location. The city must be owned by the same civ.
pub(crate) fn remove_heresy(
    state: &mut GameState,
    inquisitor_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    let unit = state.units.iter()
        .find(|u| u.id == inquisitor_id)
        .ok_or(RulesError::UnitNotFound)?;
    if unit.category != crate::UnitCategory::Religious {
        return Err(RulesError::NotAnInquisitor);
    }
    // Inquisitors have no religious_strength and use charges field.
    let charges = unit.charges.ok_or(RulesError::NoChargesRemaining)?;
    if charges == 0 {
        return Err(RulesError::NoChargesRemaining);
    }
    let civ_id = unit.owner;
    let unit_coord = unit.coord;
    let unit_religion = unit.religion_id;

    let city = state.cities.iter()
        .find(|c| c.coord == unit_coord)
        .ok_or(RulesError::CityNotFound)?;
    // Must be in a city owned by the same civ.
    if city.owner != civ_id {
        return Err(RulesError::TileNotOwned);
    }
    let city_id = city.id;

    // Remove 75% of all foreign religion followers.
    let mut total_removed = 0u32;
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        for (rid, count) in city.religious_followers.iter_mut() {
            if Some(*rid) != unit_religion {
                let remove = (*count as f64 * INQUISITOR_HERESY_REDUCTION).floor() as u32;
                *count = count.saturating_sub(remove);
                total_removed += remove;
            }
        }
    }

    // Consume one charge.
    let unit = state.units.iter_mut()
        .find(|u| u.id == inquisitor_id).unwrap();
    let new_charges = charges - 1;
    unit.charges = Some(new_charges);
    unit.movement_left = 0;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::HeresyRemoved { city: city_id, followers_removed: total_removed });

    if new_charges == 0 {
        state.units.retain(|u| u.id != inquisitor_id);
        diff.push(StateDelta::UnitDestroyed { unit: inquisitor_id });
    }

    Ok(diff)
}

// ── Guru heal ────────────────────────────────────────────────────────────────

/// Use a Guru to heal nearby friendly religious units by up to 40 HP.
/// Heals all friendly religious units within 1 tile.
pub(crate) fn guru_heal(
    state: &mut GameState,
    guru_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    let unit = state.units.iter()
        .find(|u| u.id == guru_id)
        .ok_or(RulesError::UnitNotFound)?;
    if unit.category != crate::UnitCategory::Religious {
        return Err(RulesError::NotAReligiousUnit);
    }
    let charges = unit.charges.ok_or(RulesError::NoHealCharges)?;
    if charges == 0 {
        return Err(RulesError::NoHealCharges);
    }
    let guru_owner = unit.owner;
    let guru_coord = unit.coord;

    // Find all friendly religious units within 1 tile.
    let targets: Vec<UnitId> = state.units.iter()
        .filter(|u| u.id != guru_id
            && u.owner == guru_owner
            && u.category == crate::UnitCategory::Religious
            && u.coord.distance(&guru_coord) <= 1)
        .map(|u| u.id)
        .collect();

    let healed_count = targets.len() as u32;

    // Apply healing.
    for target_id in &targets {
        if let Some(u) = state.units.iter_mut().find(|u| u.id == *target_id) {
            // Heal religious_strength if present (Apostles).
            if let Some(ref mut rs) = u.religious_strength {
                *rs = (*rs + GURU_HEAL_AMOUNT).min(110); // Cap at base strength.
            }
            // Also heal regular health.
            u.health = (u.health + GURU_HEAL_AMOUNT).min(100);
        }
    }

    // Consume one charge.
    let unit = state.units.iter_mut()
        .find(|u| u.id == guru_id).unwrap();
    let new_charges = charges - 1;
    unit.charges = Some(new_charges);
    unit.movement_left = 0;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::ReligiousUnitsHealed { healer: guru_id, healed_count });

    if new_charges == 0 {
        state.units.retain(|u| u.id != guru_id);
        diff.push(StateDelta::UnitDestroyed { unit: guru_id });
    }

    Ok(diff)
}
