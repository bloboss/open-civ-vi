//! Combat handlers: `attack`, `city_bombard`, `theological_combat`.

use crate::{UnitId, UnitDomain, AgeType};
use crate::civ::unit::Unit;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use super::{RulesError, lookup_bundle};
use super::super::diff::{AttackType, GameStateDiff, StateDelta};
use super::super::state::GameState;
use crate::rules::modifier::{
    ConditionContext, EffectType, Modifier, resolve_modifiers,
};
use crate::rules::unique::UniqueUnitAbility;

/// XP thresholds: each threshold unlocks one promotion slot.
const XP_THRESHOLDS: &[u32] = &[15, 45, 90, 150, 225, 315];

/// Returns the number of promotions a unit is eligible for given its total XP.
#[allow(dead_code)]
pub(crate) fn promotion_slots_for_xp(xp: u32) -> usize {
    XP_THRESHOLDS.iter().filter(|&&t| xp >= t).count()
}

/// Returns the era index (0=Ancient..8=Future) for an `Option<AgeType>`.
fn era_index(era: Option<AgeType>) -> i32 {
    match era {
        None => 0,
        Some(AgeType::Ancient) => 0,
        Some(AgeType::Classical) => 1,
        Some(AgeType::Medieval) => 2,
        Some(AgeType::Renaissance) => 3,
        Some(AgeType::Industrial) => 4,
        Some(AgeType::Modern) => 5,
        Some(AgeType::Atomic) => 6,
        Some(AgeType::Information) => 7,
        Some(AgeType::Future) => 8,
    }
}

/// Collect modifiers from all combat-relevant sources for a unit and return
/// (flat_bonus, percent_bonus).
fn resolve_combat_modifiers(state: &GameState, unit_id: UnitId) -> (i32, i32) {
    let unit = match state.unit(unit_id) {
        Some(u) => u,
        None => return (0, 0),
    };
    let civ_id = unit.owner;
    let unit_coord = unit.coord;
    let unit_domain = unit.domain;

    let mut all_modifiers: Vec<Modifier> = Vec::new();

    // 1. Promotions: collect modifiers from each PromotionId the unit has.
    for promo_id in &unit.promotions {
        if let Some(rp) = state.promotion_defs.iter().find(|rp| rp.id == *promo_id) {
            all_modifiers.extend(rp.def.modifiers.iter().cloned());
        }
    }

    // 2. Government inherent modifiers.
    if let Some(civ) = state.civ(civ_id)
        && let Some(gov_name) = civ.current_government_name
        && let Some(gov) = state.governments.iter().find(|g| g.name == gov_name)
    {
        all_modifiers.extend(gov.inherent_modifiers.iter().cloned());
    }

    // 3. Active policies.
    if let Some(civ) = state.civ(civ_id) {
        for policy_id in &civ.active_policies {
            if let Some(policy) = state.policies.iter().find(|p| p.id == *policy_id) {
                all_modifiers.extend(policy.modifiers.iter().cloned());
            }
        }
    }

    // 4. Great person modifiers (permanent CS bonuses from retired GPs).
    if let Some(civ) = state.civ(civ_id) {
        all_modifiers.extend(civ.great_person_modifiers.iter().cloned());
    }

    // 5. Nearby Great Person aura: scan for GreatPerson units within 2 tiles
    //    that share the same domain (General=Land, Admiral=Naval).
    for other in &state.units {
        if other.id == unit_id || other.owner != civ_id {
            continue;
        }
        if other.category != crate::UnitCategory::GreatPerson {
            continue;
        }
        if other.coord.distance(&unit_coord) > 2 {
            continue;
        }
        // Check if GP domain matches unit domain.
        let gp_domain_matches = match other.domain {
            UnitDomain::Land => unit_domain == UnitDomain::Land,
            UnitDomain::Sea => unit_domain == UnitDomain::Sea,
            _ => false,
        };
        if gp_domain_matches {
            // Look up the GP's def to get aura modifiers.
            // Great Generals/Admirals grant +5 CS to nearby units.
            all_modifiers.push(Modifier::new(
                crate::rules::modifier::ModifierSource::Custom("GreatPersonAura"),
                crate::rules::modifier::TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(5),
                crate::rules::modifier::StackingRule::Additive,
            ));
        }
    }

    // 6. Religion: check for Crusade belief (+10 CS near foreign holy city).
    if let Some(religion) = state.religions.iter().find(|r| r.founded_by == civ_id) {
        for belief_id in &religion.beliefs {
            if let Some(belief_def) = state.belief_defs.iter().find(|b| b.id == *belief_id) {
                if belief_def.name == "Crusade" {
                    // Check if unit is within 10 tiles of a foreign holy city.
                    let near_foreign_holy = state.religions.iter()
                        .filter(|r| r.founded_by != civ_id)
                        .any(|r| {
                            state.cities.iter()
                                .find(|c| c.id == r.holy_city)
                                .is_some_and(|c| c.coord.distance(&unit_coord) <= 10)
                        });
                    if near_foreign_holy {
                        all_modifiers.push(Modifier::new(
                            crate::rules::modifier::ModifierSource::Religion("Crusade"),
                            crate::rules::modifier::TargetSelector::AllUnits,
                            EffectType::CombatStrengthFlat(10),
                            crate::rules::modifier::StackingRule::Additive,
                        ));
                    }
                }
                // Also pass belief modifiers through the standard pipeline.
                all_modifiers.extend(belief_def.modifiers.iter().cloned());
            }
        }
    }

    // Build condition context and resolve.
    let ctx = ConditionContext {
        civ_id,
        state,
        tile: Some(unit_coord),
        unit_id: Some(unit_id),
        city_id: None,
    };
    let effects = resolve_modifiers(&all_modifiers, Some(&ctx));

    let mut flat = 0i32;
    let mut pct = 0i32;
    for eff in &effects {
        match eff {
            EffectType::CombatStrengthFlat(v) => flat += v,
            EffectType::CombatStrengthPercent(v) => pct += v,
            _ => {}
        }
    }
    (flat, pct)
}

/// Resolve combat between `attacker` and `defender`.
pub(crate) fn attack(
    state:       &mut GameState,
    attacker_id: UnitId,
    defender_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    // --- validation -------------------------------------------------------
    let (atk_coord, atk_range, atk_cs, atk_owner, atk_unit_type, _atk_domain) = {
        let u = state.unit(attacker_id).ok_or(RulesError::UnitNotFound)?;
        (u.coord, u.range, u.combat_strength, u.owner, u.unit_type, u.domain)
    };
    let atk_cs = atk_cs.ok_or(RulesError::UnitCannotAttack)?;

    let (def_coord, def_cs, _def_owner, _def_domain, def_unit_type) = {
        let u = state.unit(defender_id).ok_or(RulesError::UnitNotFound)?;
        (u.coord, u.combat_strength.unwrap_or(0), u.owner, u.domain, u.unit_type)
    };

    if atk_owner == state.unit(defender_id).unwrap().owner {
        return Err(RulesError::SameCivilization);
    }

    if state.unit(attacker_id).unwrap().movement_left == 0 {
        return Err(RulesError::InsufficientMovement(GameStateDiff::new()));
    }

    let dist = atk_coord.distance(&def_coord);
    if atk_range == 0 {
        if dist != 1 { return Err(RulesError::NotInRange); }
    } else if dist > atk_range as u32 {
        return Err(RulesError::NotInRange);
    }

    // --- unique unit ability adjustments -----------------------------------
    let mut atk_cs_bonus: i32 = 0;
    let mut def_cs_bonus: i32 = 0;

    // Look up unique unit abilities for the attacker.
    if let Some(atk_unit) = state.unit(attacker_id) {
        let atk_civ_identity = state.civilizations.iter()
            .find(|c| c.id == atk_owner)
            .and_then(|c| c.civ_identity);
        if let Some(bundle) = lookup_bundle(atk_civ_identity)
            && let Some(uu) = &bundle.unique_unit
        {
            let atk_type_name = state.unit_type_defs.iter()
                .find(|d| d.id == atk_unit.unit_type)
                .map(|d| d.name);
            if atk_type_name == Some(uu.name) {
                for ability in &uu.abilities {
                    match ability {
                        UniqueUnitAbility::BonusAdjacentSameType(bonus) => {
                            // Hoplite: +10 CS when adjacent to another Hoplite.
                            let adj_same = state.units.iter()
                                .filter(|u| u.id != atk_unit.id
                                    && u.owner == atk_unit.owner
                                    && u.unit_type == atk_unit.unit_type
                                    && u.coord.distance(&atk_unit.coord) == 1)
                                .count();
                            if adj_same > 0 {
                                atk_cs_bonus += bonus;
                            }
                        }
                        UniqueUnitAbility::DebuffAdjacentEnemies(debuff) => {
                            // Varu: -5 CS to adjacent enemies.
                            if atk_unit.coord.distance(&def_coord) == 1 {
                                def_cs_bonus -= debuff;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Look up unique unit abilities for the defender (e.g., Varu debuffs adjacent attackers).
    if let Some(def_unit) = state.unit(defender_id) {
        let def_civ_identity = state.civilizations.iter()
            .find(|c| c.id == def_unit.owner)
            .and_then(|c| c.civ_identity);
        if let Some(bundle) = lookup_bundle(def_civ_identity)
            && let Some(uu) = &bundle.unique_unit
        {
            let def_type_name = state.unit_type_defs.iter()
                .find(|d| d.id == def_unit.unit_type)
                .map(|d| d.name);
            if def_type_name == Some(uu.name) {
                for ability in &uu.abilities {
                    if let UniqueUnitAbility::DebuffAdjacentEnemies(debuff) = ability
                        && def_unit.coord.distance(&atk_coord) == 1
                    {
                        atk_cs_bonus -= debuff;
                    }
                }
            }
        }
    }

    // --- damage calculation -----------------------------------------------
    // Terrain defense bonus applies to the defender's tile.
    let terrain_def_bonus = state.board
        .tile(def_coord)
        .map(|t| t.terrain_defense_bonus())
        .unwrap_or(0);
    // Wall defense bonus: if the defender is on a city tile with walls,
    // the wall's defense_bonus is added to effective combat strength.
    let wall_def_bonus = state.cities.iter()
        .find(|c| c.coord == def_coord)
        .map(|c| c.walls.defense_bonus())
        .unwrap_or(0);
    // Siege bonus: extra attack strength when attacking a unit on a city tile.
    let is_city_tile = state.cities.iter().any(|c| c.coord == def_coord);
    let siege_bonus = if is_city_tile {
        state.unit_type_defs.iter()
            .find(|d| d.id == atk_unit_type)
            .map(|d| d.siege_bonus)
            .unwrap_or(0)
    } else {
        0
    };

    // Modifier pipeline: collect CS bonuses from promotions, government,
    // policies, great persons, nearby GP auras, and religion.
    let (atk_mod_flat, atk_mod_pct) = resolve_combat_modifiers(state, attacker_id);
    let (def_mod_flat, def_mod_pct) = resolve_combat_modifiers(state, defender_id);

    let effective_atk_cs = ((atk_cs as i32 + atk_mod_flat + atk_cs_bonus + siege_bonus as i32)
        * (100 + atk_mod_pct) / 100).max(1) as u32;
    let effective_def_cs = ((def_cs as i32 + terrain_def_bonus + wall_def_bonus + def_cs_bonus + def_mod_flat)
        * (100 + def_mod_pct) / 100).max(1) as u32;

    // Formula: 30 * exp((cs_atk - cs_def_effective) / 25) * rng[0.75, 1.25]
    let rng_a = 0.75 + state.id_gen.next_f32() * 0.5;
    let def_damage = (30.0_f32
        * f32::exp((effective_atk_cs as f32 - effective_def_cs as f32) / 25.0)
        * rng_a) as u32;

    let (attack_type, atk_damage) = if atk_range == 0 {
        let rng_b = 0.75 + state.id_gen.next_f32() * 0.5;
        let d = (30.0_f32
            * f32::exp((def_cs as f32 - atk_cs as f32) / 25.0)
            * rng_b) as u32;
        (AttackType::Melee, d)
    } else {
        (AttackType::Ranged, 0u32)
    };

    // --- mutate state and build diff --------------------------------------
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitAttacked {
        attacker:        attacker_id,
        defender:        defender_id,
        attack_type,
        attacker_damage: atk_damage,
        defender_damage: def_damage,
    });

    if let Some(u) = state.unit_mut(defender_id) {
        u.health = u.health.saturating_sub(def_damage);
        if u.health == 0 {
            diff.push(StateDelta::UnitDestroyed { unit: defender_id });
        }
    }
    if atk_damage > 0
        && let Some(u) = state.unit_mut(attacker_id)
    {
        u.health = u.health.saturating_sub(atk_damage);
        if u.health == 0 {
            diff.push(StateDelta::UnitDestroyed { unit: attacker_id });
        }
    }
    state.units.retain(|u| u.health > 0);

    // --- wall damage on melee attacks against city tiles ─────────────────
    if attack_type == AttackType::Melee
        && let Some(city_idx) = state.cities.iter().position(|c| c.coord == def_coord)
    {
        let city = &state.cities[city_idx];
        if city.walls != crate::civ::city::WallLevel::None && city.wall_hp > 0 {
            let wall_damage = def_damage / 2;
            if wall_damage > 0 {
                let city = &mut state.cities[city_idx];
                city.wall_hp = city.wall_hp.saturating_sub(wall_damage);
                let hp_remaining = city.wall_hp;
                diff.push(StateDelta::WallDamaged {
                    city: city.id,
                    damage: wall_damage,
                    hp_remaining,
                });
                if hp_remaining == 0 {
                    let previous_level = city.walls;
                    city.walls = crate::civ::city::WallLevel::None;
                    diff.push(StateDelta::WallDestroyed {
                        city: city.id,
                        previous_level,
                    });
                }
            }
        }
    }

    // --- city capture on melee kill ───────────────────────────────────────
    let defender_was_killed = !state.units.iter().any(|u| u.id() == defender_id);
    let attacker_alive = state.unit(attacker_id).is_some();
    if attack_type == AttackType::Melee
        && defender_was_killed
        && attacker_alive
        && let Some(city_idx) = state.cities.iter().position(|c| c.coord == def_coord)
    {
        let old_owner = state.cities[city_idx].owner;
        // Only capture if no remaining defenders on the tile.
        let defenders_left = state.units.iter()
            .any(|u| u.coord == def_coord && u.owner == old_owner);
        if old_owner != atk_owner && !defenders_left {

                // Transfer city ownership.
                let city = &mut state.cities[city_idx];
                city.owner     = atk_owner;
                city.ownership = crate::civ::city::CityOwnership::Occupied;
                let city_id    = city.id;
                diff.push(StateDelta::CityCaptured {
                    city:      city_id,
                    new_owner: atk_owner,
                    old_owner,
                });

                // Update civilization city lists.
                if let Some(old_civ) = state.civilizations.iter_mut()
                    .find(|c| c.id == old_owner)
                {
                    old_civ.cities.retain(|&id| id != city_id);
                }
                if let Some(new_civ) = state.civilizations.iter_mut()
                    .find(|c| c.id == atk_owner)
                {
                    new_civ.cities.push(city_id);
                }

                // Transfer tile ownership for the city's territory.
                let territory: Vec<HexCoord> = state.cities.iter()
                    .find(|c| c.id == city_id)
                    .map(|c| c.territory.iter().copied().collect())
                    .unwrap_or_default();
                for coord in &territory {
                    if let Some(tile) = state.board.tile_mut(*coord) {
                        tile.owner = Some(atk_owner);
                    }
                }

                // Move the attacker onto the city tile.
                if let Some(u) = state.unit_mut(attacker_id) {
                    u.coord = def_coord;
                }
            }
    }

    if let Some(u) = state.unit_mut(attacker_id) {
        u.movement_left = 0;
    }

    // --- XP awards ─────────────────────────────────────────────────────────
    // Look up eras for scaling.
    let atk_era = state.unit_type_defs.iter()
        .find(|d| d.id == atk_unit_type)
        .and_then(|d| d.era);
    let def_era = state.unit_type_defs.iter()
        .find(|d| d.id == def_unit_type)
        .and_then(|d| d.era);
    let era_diff = era_index(def_era) - era_index(atk_era);
    let scale: f32 = match era_diff {
        d if d >= 2 => 2.0,
        1 => 1.5,
        0 => 1.0,
        -1 => 0.5,
        _ => 0.25,
    };

    // Award XP to attacker (if alive).
    if attacker_alive {
        let base_xp: u32 = if defender_was_killed { 5 } else { 3 };
        let xp = ((base_xp as f32 * scale).round() as u32).max(1);
        if let Some(u) = state.unit_mut(attacker_id) {
            u.experience += xp;
            let new_total = u.experience;
            diff.push(StateDelta::ExperienceGained {
                unit: attacker_id,
                amount: xp,
                new_total,
            });
        }
    }

    // Award XP to defender (if alive, melee only — defender took damage).
    if !defender_was_killed && attack_type == AttackType::Melee {
        let def_base_xp: u32 = 2; // defender survives melee
        let def_era_diff = era_index(atk_era) - era_index(def_era);
        let def_scale: f32 = match def_era_diff {
            d if d >= 2 => 2.0,
            1 => 1.5,
            0 => 1.0,
            -1 => 0.5,
            _ => 0.25,
        };
        let def_xp = ((def_base_xp as f32 * def_scale).round() as u32).max(1);
        if let Some(u) = state.unit_mut(defender_id) {
            u.experience += def_xp;
            let new_total = u.experience;
            diff.push(StateDelta::ExperienceGained {
                unit: defender_id,
                amount: def_xp,
                new_total,
            });
        }
    }

    // --- Cascading events: historic moments on kill ────────────────────────
    // Only for non-barbarian attackers.
    let is_barbarian_attacker = state.barbarian_civ.is_some_and(|bc| bc == atk_owner);
    if defender_was_killed && attacker_alive && !is_barbarian_attacker {
        // BattleWon: +1 era score.
        if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == atk_owner) {
            civ.era_score += 1;
            civ.historic_moments.push(crate::civ::era::HistoricMoment {
                civ: atk_owner,
                moment_name: "BattleWon",
                era_score: 1,
                turn: state.turn,
                era: atk_era.unwrap_or(AgeType::Ancient),
            });
        }
        diff.push(StateDelta::HistoricMomentEarned {
            civ: atk_owner,
            moment: "BattleWon",
            era_score: 1,
        });

        // HigherEraUnitKilled: +3 era score (first time only).
        if era_index(def_era) > era_index(atk_era) {
            let already_earned = state.civilizations.iter()
                .find(|c| c.id == atk_owner)
                .is_some_and(|c| c.earned_moments.contains("HigherEraUnitKilled"));
            if !already_earned {
                if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == atk_owner) {
                    civ.era_score += 3;
                    civ.earned_moments.insert("HigherEraUnitKilled");
                    civ.historic_moments.push(crate::civ::era::HistoricMoment {
                        civ: atk_owner,
                        moment_name: "HigherEraUnitKilled",
                        era_score: 3,
                        turn: state.turn,
                        era: atk_era.unwrap_or(AgeType::Ancient),
                    });
                }
                diff.push(StateDelta::HistoricMomentEarned {
                    civ: atk_owner,
                    moment: "HigherEraUnitKilled",
                    era_score: 3,
                });
            }
        }
    }

    Ok(diff)
}

/// City with walls fires a ranged bombardment at an enemy unit within range 2.
pub(crate) fn city_bombard(
    state: &mut GameState,
    city_id: crate::CityId,
    target:  UnitId,
) -> Result<GameStateDiff, RulesError> {
    // 1. Validate city exists and has walls.
    let city_idx = state.cities.iter().position(|c| c.id == city_id)
        .ok_or(RulesError::CityNotFound)?;
    let city = &state.cities[city_idx];
    if city.walls == crate::civ::city::WallLevel::None {
        return Err(RulesError::CityCannotAttack);
    }
    if city.has_attacked_this_turn {
        return Err(RulesError::CityAlreadyAttacked);
    }
    let city_coord = city.coord;
    let city_owner = city.owner;

    // City ranged strength = 15 + wall defense bonus.
    let city_cs = 15_u32 + city.walls.defense_bonus() as u32;

    // 2. Validate target unit exists, is an enemy, and is within range 2.
    let (def_coord, def_cs) = {
        let u = state.unit(target).ok_or(RulesError::UnitNotFound)?;
        if u.owner == city_owner {
            return Err(RulesError::SameCivilization);
        }
        (u.coord, u.combat_strength.unwrap_or(0))
    };
    let dist = city_coord.distance(&def_coord);
    if dist > 2 || dist == 0 {
        return Err(RulesError::NotInRange);
    }

    // 3. Damage formula (same exponential; no terrain bonus for city offense).
    let rng = 0.75 + state.id_gen.next_f32() * 0.5;
    let damage = (30.0_f32
        * f32::exp((city_cs as f32 - def_cs as f32) / 25.0)
        * rng) as u32;

    // 4. Apply damage to target; no counter-damage to city.
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitAttacked {
        attacker:        UnitId::nil(),
        defender:        target,
        attack_type:     AttackType::CityBombard,
        attacker_damage: 0,
        defender_damage: damage,
    });
    if let Some(u) = state.unit_mut(target) {
        u.health = u.health.saturating_sub(damage);
        if u.health == 0 {
            diff.push(StateDelta::UnitDestroyed { unit: target });
        }
    }
    state.units.retain(|u| u.health > 0);

    // 5. Mark city as having attacked this turn.
    state.cities[city_idx].has_attacked_this_turn = true;

    Ok(diff)
}

/// Check if a religion has the Scripture enhancer belief (+25% combat strength).
fn religion_has_scripture(state: &GameState, religion_id: Option<crate::ReligionId>) -> bool {
    let Some(rid) = religion_id else { return false };
    let Some(religion) = state.religions.iter().find(|r| r.id == rid) else { return false };
    religion.beliefs.iter().any(|bid|
        state.belief_defs.iter().any(|b| b.id == *bid && b.name == "Scripture"))
}

/// Apply +250 followers of a winning religion to all cities within 10 tiles of combat.
fn apply_theological_victory_pressure(
    state: &mut GameState,
    religion_id: crate::ReligionId,
    combat_coord: libhexgrid::coord::HexCoord,
) {
    let nearby_cities: Vec<crate::CityId> = state.cities.iter()
        .filter(|c| c.coord.distance(&combat_coord) <= 10)
        .map(|c| c.id)
        .collect();
    for cid in nearby_cities {
        if let Some(city) = state.cities.iter_mut().find(|c| c.id == cid) {
            let added = 250u32.min(city.population);
            *city.religious_followers.entry(religion_id).or_insert(0) += added;
        }
    }
}

/// Theological combat between two religious units.
pub(crate) fn theological_combat(
    state: &mut GameState,
    attacker_id: UnitId,
    defender_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    // Validate attacker.
    let attacker = state.units.iter()
        .find(|u| u.id == attacker_id)
        .ok_or(RulesError::UnitNotFound)?;
    if attacker.category != crate::UnitCategory::Religious {
        return Err(RulesError::NotAReligiousUnit);
    }
    let mut atk_str = attacker.religious_strength.ok_or(RulesError::NoReligiousStrength)?;
    let atk_religion = attacker.religion_id;
    let atk_coord = attacker.coord;

    // Validate defender.
    let defender = state.units.iter()
        .find(|u| u.id == defender_id)
        .ok_or(RulesError::UnitNotFound)?;
    if defender.category != crate::UnitCategory::Religious {
        return Err(RulesError::NotAReligiousUnit);
    }
    let mut def_str = defender.religious_strength.unwrap_or(100);
    let def_religion = defender.religion_id;
    let def_coord = defender.coord;

    // Must be adjacent.
    if atk_coord.distance(&def_coord) != 1 {
        return Err(RulesError::NotInRange);
    }

    // Can't fight own units.
    if attacker.owner == defender.owner {
        return Err(RulesError::SameCivilization);
    }

    // Attacker must have movement.
    if attacker.movement_left == 0 {
        return Err(RulesError::InsufficientMovement(GameStateDiff::new()));
    }

    // Apply Scripture enhancer: +25% religious combat strength.
    if religion_has_scripture(state, atk_religion) {
        atk_str = (atk_str as f64 * 1.25).round() as u32;
    }
    if religion_has_scripture(state, def_religion) {
        def_str = (def_str as f64 * 1.25).round() as u32;
    }

    // Apply exponential damage formula (same as normal combat).
    let cs_diff = atk_str as f64 - def_str as f64;
    let rng_factor = 0.75 + (state.id_gen.next_f32() as f64) * 0.50;
    let base_damage = 30.0 * (cs_diff / 25.0).exp() * rng_factor;
    let defender_damage = base_damage.max(1.0) as u32;

    let rng_factor2 = 0.75 + (state.id_gen.next_f32() as f64) * 0.50;
    let counter_damage = 30.0 * (-cs_diff / 25.0).exp() * rng_factor2;
    let attacker_damage = counter_damage.max(1.0) as u32;

    // Apply damage to religious strength (use original, un-buffed values for storage).
    let atk_orig = state.units.iter().find(|u| u.id == attacker_id)
        .and_then(|u| u.religious_strength).unwrap_or(110);
    let def_orig = state.units.iter().find(|u| u.id == defender_id)
        .and_then(|u| u.religious_strength).unwrap_or(100);
    let def_new_str = def_orig.saturating_sub(defender_damage);
    let atk_new_str = atk_orig.saturating_sub(attacker_damage);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TheologicalCombat {
        attacker: attacker_id,
        defender: defender_id,
        attacker_damage,
        defender_damage,
    });

    // Determine winner — loser is destroyed first.
    let defender_killed = def_new_str == 0;
    let attacker_killed = atk_new_str == 0;

    // Update strengths.
    if let Some(u) = state.units.iter_mut().find(|u| u.id == defender_id) {
        u.religious_strength = Some(def_new_str);
    }
    if let Some(u) = state.units.iter_mut().find(|u| u.id == attacker_id) {
        u.religious_strength = Some(atk_new_str);
        u.movement_left = 0;
    }

    // Destroy killed units and apply pressure waves.
    if defender_killed {
        state.units.retain(|u| u.id != defender_id);
        diff.push(StateDelta::UnitDestroyed { unit: defender_id });

        // Winner (attacker) gets +250 pressure in nearby cities.
        if let Some(rid) = atk_religion {
            apply_theological_victory_pressure(state, rid, def_coord);
        }
    }
    if attacker_killed {
        state.units.retain(|u| u.id != attacker_id);
        diff.push(StateDelta::UnitDestroyed { unit: attacker_id });

        // Winner (defender) gets +250 pressure in nearby cities.
        if let Some(rid) = def_religion {
            apply_theological_victory_pressure(state, rid, atk_coord);
        }
    }

    Ok(diff)
}

/// Promote a unit with a named promotion.
///
/// Validates: XP eligibility (thresholds [15,45,90,150,225,315]), class match,
/// prerequisites. On success: push PromotionId, heal +50 HP (capped at 100),
/// emit UnitPromoted + UnitHealed.
pub(crate) fn promote_unit(
    state: &mut GameState,
    unit_id: UnitId,
    promotion_name: &str,
) -> Result<GameStateDiff, RulesError> {
    // 1. Find the registered promotion by name.
    let rp = state.promotion_defs.iter()
        .find(|rp| rp.def.name == promotion_name)
        .ok_or(RulesError::UnitPromotionNotFound)?;
    let promo_id = rp.id;
    let promo_class = rp.def.class;
    let promo_prereqs: Vec<&'static str> = rp.def.prerequisites.to_vec();
    let promo_name: &'static str = rp.def.name;

    // 2. Find the unit.
    let unit = state.unit(unit_id).ok_or(RulesError::UnitNotFound)?;

    // 3. Check promotion class matches the unit's promotion class.
    let unit_promo_class = state.unit_type_defs.iter()
        .find(|d| d.id == unit.unit_type)
        .and_then(|d| d.promotion_class);
    if unit_promo_class != Some(promo_class) {
        return Err(RulesError::PromotionNotEligible);
    }

    // 4. Check XP eligibility: count current promotions, check against threshold.
    let current_promo_count = unit.promotions.len();
    if current_promo_count >= XP_THRESHOLDS.len() {
        return Err(RulesError::PromotionNotEligible); // max promotions reached
    }
    let needed_xp = XP_THRESHOLDS[current_promo_count];
    if unit.experience < needed_xp {
        return Err(RulesError::PromotionNotEligible);
    }

    // 5. Check unit doesn't already have this promotion.
    if unit.promotions.contains(&promo_id) {
        return Err(RulesError::PromotionNotEligible);
    }

    // 6. Check prerequisites: all prerequisite promotions must be present.
    for prereq_name in &promo_prereqs {
        let prereq_id = state.promotion_defs.iter()
            .find(|rp2| rp2.def.name == *prereq_name)
            .map(|rp2| rp2.id);
        if let Some(pid) = prereq_id
            && !unit.promotions.contains(&pid)
        {
            return Err(RulesError::PromotionPrerequisiteNotMet);
        }
    }

    // 7. Apply: push promotion, heal +50.
    let mut diff = GameStateDiff::new();

    let unit = state.unit_mut(unit_id).unwrap();
    unit.promotions.push(promo_id);
    let old_health = unit.health;
    unit.health = (unit.health + 50).min(100);
    let new_health = unit.health;

    diff.push(StateDelta::UnitPromoted {
        unit: unit_id,
        promotion: promo_id,
        promotion_name: promo_name,
    });
    if new_health != old_health {
        diff.push(StateDelta::UnitHealed {
            unit: unit_id,
            old_health,
            new_health,
        });
    }

    Ok(diff)
}

/// Raid a barbarian camp: grants 25 gold and reduces conversion_progress by 5.
pub(crate) fn raid_barbarian_camp(
    state: &mut GameState,
    unit_id: UnitId,
    camp_id: crate::BarbarianCampId,
) -> Result<GameStateDiff, RulesError> {
    let unit = state.unit(unit_id).ok_or(RulesError::UnitNotFound)?;
    let unit_owner = unit.owner;
    let unit_coord = unit.coord;

    let camp = state.barbarian_camp(camp_id)
        .ok_or(RulesError::BarbarianCampNotFound)?;
    let camp_coord = camp.coord;

    // Unit must be on or adjacent to the camp tile.
    if unit_coord.distance(&camp_coord) > 1 {
        return Err(RulesError::NotInRange);
    }

    let mut diff = GameStateDiff::new();

    // Grant 25 gold.
    if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == unit_owner) {
        civ.gold += 25;
    }
    diff.push(StateDelta::GoldChanged { civ: unit_owner, delta: 25 });

    // Reduce conversion progress by 5.
    if let Some(camp) = state.barbarian_camp_mut(camp_id) {
        camp.conversion_progress = camp.conversion_progress.saturating_sub(5);
    }

    Ok(diff)
}

// ── Rock Band cultural combat (GS-16) ���───────────────────────────────────────

/// Perform a Rock Band concert at a foreign city tile.
///
/// Tourism = base 500 + 250 per district in the target city.
/// 30% chance the band disbands after performing; otherwise charges are
/// decremented and the unit is destroyed when charges reach 0.
pub(crate) fn rock_band_perform(
    state: &mut GameState,
    unit_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    // 1. Validate unit exists.
    let unit = state.unit(unit_id).ok_or(RulesError::UnitNotFound)?;
    let unit_coord = unit.coord;
    let unit_owner = unit.owner;
    let unit_type_id = unit.unit_type;

    // 2. Validate unit is a Rock Band.
    let is_rock_band = state.unit_type_defs.iter()
        .find(|d| d.id == unit_type_id)
        .is_some_and(|d| d.name == "Rock Band");
    if !is_rock_band {
        return Err(RulesError::NotARockBand);
    }

    // 3. Validate unit is on a foreign city tile.
    let target_city = state.cities.iter()
        .find(|c| c.coord == unit_coord && c.owner != unit_owner);
    let target_city_id = match target_city {
        Some(c) => c.id,
        None => return Err(RulesError::NotOnForeignCity),
    };

    // 4. Calculate tourism: base 500 + 250 per district in target city.
    let district_count = state.cities.iter()
        .find(|c| c.id == target_city_id)
        .map(|c| c.districts.len() as u32)
        .unwrap_or(0);
    let tourism_gained = 500 + 250 * district_count;

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::RockBandPerformed {
        unit: unit_id,
        city: target_city_id,
        tourism_gained,
    });

    // 5. Roll for disband: 30% base chance.
    let roll = state.id_gen.next_f32();
    let disband = roll < 0.30;

    if disband {
        // Band disbands immediately.
        state.units.retain(|u| u.id != unit_id);
        diff.push(StateDelta::UnitDestroyed { unit: unit_id });
    } else {
        // Decrement charges; destroy if 0.
        let unit = state.unit_mut(unit_id).unwrap();
        let new_charges = unit.charges.unwrap_or(1).saturating_sub(1);
        if new_charges == 0 {
            unit.charges = None;
            let uid = unit.id;
            state.units.retain(|u| u.id != uid);
            diff.push(StateDelta::ChargesChanged { unit: unit_id, remaining: 0 });
            diff.push(StateDelta::UnitDestroyed { unit: unit_id });
        } else {
            unit.charges = Some(new_charges);
            diff.push(StateDelta::ChargesChanged { unit: unit_id, remaining: new_charges });
        }
    }

    Ok(diff)
}
