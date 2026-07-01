//! Deterministic combat-preview computation.
//!
//! Mirrors the effective-CS pipeline used by [`super::combat::attack`] but
//! omits the RNG roll (uses 1.0, the midpoint of `[0.75, 1.25]`) so the
//! result is stable across calls. Used by the web `/combat/preview`
//! endpoint and the CLI `status combat-preview` subcommand.
//!
//! Limitations vs. the live `attack()` formula:
//!  - Unique-unit ability bonuses (Hoplite adjacent-bonus, Varu debuff) are
//!    not yet folded in. The standard modifier pipeline (promotions,
//!    government, policies, great-person auras, religion beliefs) IS
//!    included.
//!  - For ranged attacks the attacker-damage prediction is always 0
//!    (matches the live formula).

use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::UnitId;
use crate::game::diff::AttackType;

use super::super::state::GameState;

/// Stable, RNG-free preview of an attack. Returned when the attacker can
/// conceptually engage the defender (has combat strength + the defender
/// is in range); `None` otherwise.
#[derive(Debug, Clone)]
pub struct CombatPreview {
    pub attacker: UnitId,
    pub defender: UnitId,
    pub attack_type: AttackType,
    /// Effective combat strength of the attacker, including modifier
    /// pipeline and siege bonus when attacking a city tile.
    pub attacker_effective_cs: u32,
    /// Effective combat strength of the defender, including terrain +
    /// wall + modifier pipeline.
    pub defender_effective_cs: u32,
    /// Predicted damage dealt to the defender at `rng = 1.0`.
    pub predicted_defender_damage: u32,
    /// Predicted damage dealt to the attacker at `rng = 1.0`. Always 0
    /// for ranged attacks.
    pub predicted_attacker_damage: u32,
}

/// Compute a combat preview for `attacker` engaging the unit at
/// `defender_coord`. Returns `None` when there is no defender, the
/// attacker can't attack (no combat strength), or the defender is out of
/// range. Mirrors `combat::attack` validation but does not mutate state.
pub(crate) fn preview_combat(
    state: &GameState,
    attacker: UnitId,
    defender_coord: HexCoord,
) -> Option<CombatPreview> {
    let atk = state.unit(attacker)?;
    let atk_cs = atk.combat_strength?;
    let atk_owner = atk.owner;
    let atk_coord = atk.coord;
    let atk_range = atk.range;
    let atk_unit_type = atk.unit_type;

    // Range check (same as combat::attack).
    let dist = atk_coord.distance(&defender_coord);
    if atk_range == 0 {
        if dist != 1 {
            return None;
        }
    } else if dist > atk_range as u32 {
        return None;
    }

    // Find any enemy unit on the defender tile (excluding the attacker's owner).
    let defender = state
        .units
        .iter()
        .find(|u| u.coord == defender_coord && u.owner != atk_owner)?;
    let def_cs = defender.combat_strength.unwrap_or(0);

    // Same modifier sources as attack(): terrain, walls, siege, modifier pipeline.
    let terrain_def_bonus = state
        .board
        .tile(defender_coord)
        .map(|t| t.terrain_defense_bonus())
        .unwrap_or(0);
    let wall_def_bonus = state
        .cities
        .iter()
        .find(|c| c.coord == defender_coord)
        .map(|c| c.walls.defense_bonus())
        .unwrap_or(0);
    let is_city_tile = state.cities.iter().any(|c| c.coord == defender_coord);
    let siege_bonus = if is_city_tile {
        state
            .unit_type_defs
            .iter()
            .find(|d| d.id == atk_unit_type)
            .map(|d| d.siege_bonus)
            .unwrap_or(0)
    } else {
        0
    };

    let (atk_mod_flat, atk_mod_pct) = super::combat::resolve_combat_modifiers(state, attacker);
    let (def_mod_flat, def_mod_pct) = super::combat::resolve_combat_modifiers(state, defender.id);

    let effective_atk_cs =
        ((atk_cs as i32 + atk_mod_flat + siege_bonus as i32) * (100 + atk_mod_pct) / 100).max(1)
            as u32;
    let effective_def_cs = ((def_cs as i32 + terrain_def_bonus + wall_def_bonus + def_mod_flat)
        * (100 + def_mod_pct)
        / 100)
        .max(1) as u32;

    // Same exponential formula, rng = 1.0.
    let predicted_defender_damage =
        (30.0_f32 * f32::exp((effective_atk_cs as f32 - effective_def_cs as f32) / 25.0)) as u32;

    let (attack_type, predicted_attacker_damage) = if atk_range == 0 {
        let d = (30.0_f32 * f32::exp((def_cs as f32 - atk_cs as f32) / 25.0)) as u32;
        (AttackType::Melee, d)
    } else {
        (AttackType::Ranged, 0)
    };

    Some(CombatPreview {
        attacker,
        defender: defender.id,
        attack_type,
        attacker_effective_cs: effective_atk_cs,
        defender_effective_cs: effective_def_cs,
        predicted_defender_damage,
        predicted_attacker_damage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::civilization::Civilization;
    use crate::civ::{BasicUnit, BuiltinAgenda, Leader};
    use crate::game::state::UnitTypeDef;
    use crate::{CivId, DefaultRulesEngine, RulesEngine, UnitCategory, UnitDomain, UnitTypeId};

    fn setup_two_civs() -> (GameState, CivId, CivId) {
        let mut state = GameState::new(13, 12, 12);
        let civ_a = state.id_gen.next_civ_id();
        let civ_b = state.id_gen.next_civ_id();
        for (id, name) in [(civ_a, "Aaa"), (civ_b, "Bbb")] {
            let leader = Leader {
                name: "L",
                civ_id: id,
                agenda: BuiltinAgenda::Default,
            };
            state
                .civilizations
                .push(Civilization::new(id, name, name, leader));
        }
        (state, civ_a, civ_b)
    }

    fn push_warrior(
        state: &mut GameState,
        civ: CivId,
        coord: HexCoord,
        strength: u32,
        mp: u32,
    ) -> UnitId {
        let uid = state.id_gen.next_unit_id();
        let tid = UnitTypeId::from_ulid(state.id_gen.next_ulid());
        state.unit_type_defs.push(UnitTypeDef {
            id: tid,
            name: "warrior",
            production_cost: 40,
            domain: UnitDomain::Land,
            category: UnitCategory::Combat,
            max_movement: 200,
            combat_strength: Some(strength),
            range: 0,
            vision_range: 2,
            can_found_city: false,
            resource_cost: None,
            siege_bonus: 0,
            max_charges: 0,
            exclusive_to: None,
            replaces: None,
            era: None,
            promotion_class: None,
        });
        state.units.push(BasicUnit {
            id: uid,
            unit_type: tid,
            owner: civ,
            coord,
            domain: UnitDomain::Land,
            category: UnitCategory::Combat,
            movement_left: mp,
            max_movement: 200,
            combat_strength: Some(strength),
            promotions: Vec::new(),
            experience: 0,
            health: 100,
            range: 0,
            vision_range: 2,
            charges: None,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges: None,
            religious_strength: None,
            is_embarked: false,
        });
        uid
    }

    #[test]
    fn melee_preview_predicts_symmetric_damage_when_strengths_match() {
        let (mut s, a, b) = setup_two_civs();
        let atk = push_warrior(&mut s, a, HexCoord::from_qr(2, 2), 20, 200);
        let def_coord = HexCoord::from_qr(3, 2);
        let _def = push_warrior(&mut s, b, def_coord, 20, 200);

        let p = DefaultRulesEngine
            .preview_combat(&s, atk, def_coord)
            .expect("preview");
        assert_eq!(p.attack_type, AttackType::Melee);
        // Equal CS → both damages ≈ 30 (modulo terrain bonuses).
        assert!(
            p.predicted_defender_damage >= 25 && p.predicted_defender_damage <= 35,
            "got {}",
            p.predicted_defender_damage
        );
        assert!(
            p.predicted_attacker_damage >= 25 && p.predicted_attacker_damage <= 35,
            "got {}",
            p.predicted_attacker_damage
        );
    }

    #[test]
    fn no_defender_returns_none() {
        let (mut s, a, _b) = setup_two_civs();
        let atk = push_warrior(&mut s, a, HexCoord::from_qr(2, 2), 20, 200);
        assert!(
            DefaultRulesEngine
                .preview_combat(&s, atk, HexCoord::from_qr(7, 7))
                .is_none()
        );
    }

    #[test]
    fn out_of_range_returns_none() {
        let (mut s, a, b) = setup_two_civs();
        let atk = push_warrior(&mut s, a, HexCoord::from_qr(2, 2), 20, 200);
        let far = HexCoord::from_qr(8, 8);
        let _ = push_warrior(&mut s, b, far, 20, 200);
        // Melee (range 0) → only adjacent (distance 1) is valid.
        assert!(DefaultRulesEngine.preview_combat(&s, atk, far).is_none());
    }

    #[test]
    fn stronger_attacker_predicts_higher_defender_damage() {
        let (mut s, a, b) = setup_two_civs();
        let atk = push_warrior(&mut s, a, HexCoord::from_qr(2, 2), 40, 200);
        let def_coord = HexCoord::from_qr(3, 2);
        let _def = push_warrior(&mut s, b, def_coord, 20, 200);
        let p = DefaultRulesEngine
            .preview_combat(&s, atk, def_coord)
            .expect("preview");
        assert!(
            p.predicted_defender_damage > p.predicted_attacker_damage,
            "expected stronger attacker to deal more damage; got def={} atk={}",
            p.predicted_defender_damage,
            p.predicted_attacker_damage
        );
    }
}
