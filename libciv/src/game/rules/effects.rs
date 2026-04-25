//! Effect application: `apply_effect`.

use crate::{CivId, PolicyType, UnitTypeId};
use crate::civ::BasicUnit;
use crate::civ::civ_ability::RuleOverride;
use libhexgrid::coord::HexCoord;

use super::{lookup_bundle};
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;
use crate::rules::effect::OneShotEffect;

/// Apply a single `OneShotEffect` to state, recording the resulting `StateDelta`.
pub(crate) fn apply_effect(
    state: &mut GameState,
    civ_id: CivId,
    effect: &OneShotEffect,
    diff: &mut GameStateDiff,
) {
    let civ_idx = match state.civilizations.iter().position(|c| c.id == civ_id) {
        Some(i) => i,
        None => return,
    };

    match effect {
        OneShotEffect::RevealResource(r) => {
            state.civilizations[civ_idx].revealed_resources.insert(*r);
            diff.push(StateDelta::ResourceRevealed { civ: civ_id, resource: *r });
        }

        OneShotEffect::UnlockUnit(u) => {
            state.civilizations[civ_idx].unlocked_units.push(u);
            diff.push(StateDelta::UnitUnlocked { civ: civ_id, unit_type: u });
        }

        OneShotEffect::UnlockBuilding(b) => {
            state.civilizations[civ_idx].unlocked_buildings.push(b);
            diff.push(StateDelta::BuildingUnlocked { civ: civ_id, building: b });
        }

        OneShotEffect::UnlockImprovement(i) => {
            state.civilizations[civ_idx].unlocked_improvements.push(i);
            diff.push(StateDelta::ImprovementUnlocked { civ: civ_id, improvement: i });
        }

        OneShotEffect::TriggerEureka { tech } => {
            let civ_identity = state.civilizations[civ_idx].civ_identity;
            let babylon_full_tech = lookup_bundle(civ_identity)
                .is_some_and(|b| b.rule_overrides.iter()
                    .any(|o| matches!(o, RuleOverride::EurekaGivesFullTech)));

            if babylon_full_tech {
                if !state.civilizations[civ_idx].researched_techs.contains(tech) {
                    state.civilizations[civ_idx].researched_techs.push(*tech);
                    state.civilizations[civ_idx].research_queue
                        .retain(|tp| tp.tech_id != *tech);
                    let tech_name = state.tech_tree.get(*tech).map(|n| n.name).unwrap_or("?");
                    diff.push(StateDelta::TechResearched { civ: civ_id, tech: tech_name });
                    if let Some(effects) = state.tech_tree.get(*tech).map(|n| n.effects.clone()) {
                        for effect in effects {
                            state.effect_queue.push_back((civ_id, effect));
                        }
                    }
                }
            } else {
                let in_progress_id = state.civilizations[civ_idx]
                    .research_queue.front().map(|tp| tp.tech_id);
                let matches_current = in_progress_id.map(|id| id == *tech).unwrap_or(false);
                if matches_current
                    && let Some(tp) = state.civilizations[civ_idx].research_queue.front_mut()
                {
                    tp.boosted = true;
                }
            }
            state.civilizations[civ_idx].eureka_triggered.insert(*tech);
            let tech_name = state.tech_tree.get(*tech).map(|n| n.name).unwrap_or("?");
            diff.push(StateDelta::EurekaTriggered { civ: civ_id, tech: tech_name });
        }

        OneShotEffect::TriggerInspiration { civic } => {
            let in_progress_id = state.civilizations[civ_idx]
                .civic_in_progress.as_ref().map(|cp| cp.civic_id);
            let matches_current = in_progress_id.map(|id| id == *civic).unwrap_or(false);

            state.civilizations[civ_idx].inspiration_triggered.insert(*civic);
            if matches_current
                && let Some(cp) = state.civilizations[civ_idx].civic_in_progress.as_mut()
            {
                cp.inspired = true;
            }
            let civic_name = state.civic_tree.get(*civic).map(|n| n.name).unwrap_or("?");
            diff.push(StateDelta::InspirationTriggered { civ: civ_id, civic: civic_name });
        }

        OneShotEffect::FreeUnit { unit_type, city: hint_city } => {
            let coord = hint_city
                .and_then(|cid| state.cities.iter().find(|c| c.id == cid))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id && c.is_capital))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id))
                .map(|c| c.coord)
                .unwrap_or(HexCoord::from_qr(0, 0));

            if let Some(def) = state.unit_type_defs.iter().find(|d| d.name == *unit_type).cloned() {
                let unit_id   = state.id_gen.next_unit_id();
                let type_id   = UnitTypeId::from_ulid(state.id_gen.next_ulid());
                state.units.push(BasicUnit {
                    id:              unit_id,
                    unit_type:       type_id,
                    owner:           civ_id,
                    coord,
                    domain:          def.domain,
                    category:        def.category,
                    movement_left:   def.max_movement,
                    max_movement:    def.max_movement,
                    combat_strength: def.combat_strength,
                    promotions:      Vec::new(),
                    experience:      0,
                    health:          100,
                    range:           0,
                    vision_range:    2,
                    charges:         if def.max_charges > 0 { Some(def.max_charges) } else { None },
                    trade_origin: None,
                    trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None, is_embarked: false,
                });
                diff.push(StateDelta::UnitCreated { unit: unit_id, coord, owner: civ_id });
            } else {
                diff.push(StateDelta::FreeUnitGranted { civ: civ_id, unit_type, coord });
            }
        }

        OneShotEffect::FreeBuilding { building, city } => {
            let target_city_id = city
                .and_then(|cid| state.cities.iter().find(|c| c.id == cid))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id && c.is_capital))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id))
                .map(|c| c.id);

            if let Some(target_cid) = target_city_id {
                if let Some(def) = state.building_defs.iter().find(|d| d.name == *building).cloned() {
                    let building_instance_id = state.id_gen.next_building_id();
                    if let Some(city_mut) = state.cities.iter_mut().find(|c| c.id == target_cid) {
                        city_mut.buildings.push(building_instance_id);
                    }
                    let _ = def;
                    diff.push(StateDelta::FreeBuildingGranted { civ: civ_id, building, city: target_cid });
                } else {
                    diff.push(StateDelta::FreeBuildingGranted { civ: civ_id, building, city: target_cid });
                }
            }
        }

        OneShotEffect::UnlockGovernment(g) => {
            state.civilizations[civ_idx].unlocked_governments.push(g);
            diff.push(StateDelta::GovernmentUnlocked { civ: civ_id, government: g });
        }

        OneShotEffect::AdoptGovernment(g) => {
            let new_gov = state.governments.iter().find(|gov| gov.name == *g).cloned();

            if let Some(new_gov) = new_gov {
                let mut mil_free  = new_gov.slots.military as i32;
                let mut eco_free  = new_gov.slots.economic as i32;
                let mut dip_free  = new_gov.slots.diplomatic as i32;
                let mut wc_free   = new_gov.slots.wildcard as i32;

                let active: Vec<crate::PolicyId> = state.civilizations[civ_idx].active_policies.clone();
                let mut kept    = Vec::new();
                let mut removed = Vec::new();

                for pid in active {
                    let policy_type = state.policies.iter()
                        .find(|p| p.id == pid)
                        .map(|p| p.policy_type);
                    let fits = match policy_type {
                        Some(PolicyType::Military)   if mil_free > 0 => { mil_free -= 1; true }
                        Some(PolicyType::Economic)   if eco_free > 0 => { eco_free -= 1; true }
                        Some(PolicyType::Diplomatic) if dip_free > 0 => { dip_free -= 1; true }
                        Some(PolicyType::Wildcard)   if wc_free  > 0 => { wc_free  -= 1; true }
                        _ => false,
                    };
                    if fits { kept.push(pid); } else { removed.push(pid); }
                }

                state.civilizations[civ_idx].active_policies = kept;
                state.civilizations[civ_idx].current_government = Some(new_gov.id);
                state.civilizations[civ_idx].current_government_name = Some(g);

                for pid in removed {
                    diff.push(StateDelta::PolicyUnslotted { civ: civ_id, policy: pid });
                }
            } else {
                state.civilizations[civ_idx].current_government_name = Some(g);
            }

            diff.push(StateDelta::GovernmentAdopted { civ: civ_id, government: g });
        }

        OneShotEffect::UnlockPolicy(p) => {
            state.civilizations[civ_idx].unlocked_policies.push(p);
            diff.push(StateDelta::PolicyUnlocked { civ: civ_id, policy: p });
        }

        OneShotEffect::GrantModifier(_) => {
            // No stored mutation: modifier is collected at query time via
            // `Civilization::get_tree_modifiers`. Nothing to do here.
        }

        OneShotEffect::EnableEmbarkCoast => {
            state.civilizations[civ_idx].can_embark_coast = true;
            diff.push(StateDelta::EmbarkCoastUnlocked { civ: civ_id });
        }

        OneShotEffect::EnableEmbarkOcean => {
            state.civilizations[civ_idx].can_embark_ocean = true;
            diff.push(StateDelta::EmbarkOceanUnlocked { civ: civ_id });
        }
    }
}
