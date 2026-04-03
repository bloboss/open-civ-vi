//! Combat handlers: `attack`, `city_bombard`, `theological_combat`.

use crate::UnitId;
use crate::civ::unit::Unit;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use super::{RulesError, lookup_bundle};
use super::super::diff::{AttackType, GameStateDiff, StateDelta};
use super::super::state::GameState;
use super::great_people::great_person_cs_bonus;
use crate::rules::unique::UniqueUnitAbility;

/// Resolve combat between `attacker` and `defender`.
pub(crate) fn attack(
    state:       &mut GameState,
    attacker_id: UnitId,
    defender_id: UnitId,
) -> Result<GameStateDiff, RulesError> {
    // --- validation -------------------------------------------------------
    let (atk_coord, atk_range, atk_cs, atk_owner, atk_unit_type, atk_domain) = {
        let u = state.unit(attacker_id).ok_or(RulesError::UnitNotFound)?;
        (u.coord, u.range, u.combat_strength, u.owner, u.unit_type, u.domain)
    };
    let atk_cs = atk_cs.ok_or(RulesError::UnitCannotAttack)?;

    let (def_coord, def_cs, _def_owner, def_domain) = {
        let u = state.unit(defender_id).ok_or(RulesError::UnitNotFound)?;
        (u.coord, u.combat_strength.unwrap_or(0), u.owner, u.domain)
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

    // Great person modifiers: retired great persons grant permanent CS bonuses
    // filtered by unit domain.
    let atk_gp_bonus = great_person_cs_bonus(state, atk_owner, atk_domain);
    let def_gp_bonus = great_person_cs_bonus(state, _def_owner, def_domain);

    let effective_atk_cs = (atk_cs as i32 + atk_cs_bonus + siege_bonus as i32 + atk_gp_bonus as i32).max(1) as u32;
    let effective_def_cs = (def_cs as i32 + terrain_def_bonus + wall_def_bonus + def_cs_bonus + def_gp_bonus as i32).max(1) as u32;

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
