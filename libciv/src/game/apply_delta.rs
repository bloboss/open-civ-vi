#![allow(clippy::collapsible_if)]
//! Delta application: reconstruct game state from a sequence of `StateDelta`s.
//!
//! `apply_delta` is designed for **replay from a base state** — applying a
//! recorded diff to reconstruct what happened. Some deltas are informational
//! only (the rules engine already mutated state before emitting the delta) and
//! are no-ops here, documented with `// Informational` comments.

use super::diff::{GameStateDiff, StateDelta};
use super::state::GameState;
use super::victory::GameOver;
use libhexgrid::board::HexBoard;

/// Apply a single atomic state delta to the game state.
pub fn apply_delta(state: &mut GameState, delta: &StateDelta) {
    match delta {
        // ── Turn ────────────────────────────────────────────────────────
        StateDelta::TurnAdvanced { to, .. } => {
            state.turn = *to;
        }

        // ── Units ───────────────────────────────────────────────────────
        StateDelta::UnitMoved { unit, to, cost, .. } => {
            if let Some(u) = state.unit_mut(*unit) {
                u.coord = *to;
                u.movement_left = u.movement_left.saturating_sub(*cost);
            }
        }
        StateDelta::UnitCreated { .. } => {
            // Informational: unit was already added to state.units by the
            // rules engine before this delta was emitted.
        }
        StateDelta::UnitDestroyed { unit } => {
            state.units.retain(|u| u.id != *unit);
        }
        StateDelta::UnitHealed { unit, new_health, .. } => {
            if let Some(u) = state.unit_mut(*unit) {
                u.health = *new_health;
            }
        }
        StateDelta::ChargesChanged { unit, remaining } => {
            if let Some(u) = state.unit_mut(*unit) {
                u.charges = if *remaining == 0 { None } else { Some(*remaining) };
            }
        }

        // ── Cities ──────────────────────────────────────────────────────
        StateDelta::CityFounded { .. } => {
            // Informational: city was already added to state.cities.
        }
        StateDelta::CityCaptured { city, new_owner, old_owner } => {
            // Update the city's owner.
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                c.owner = *new_owner;
            }
            // Update the civ city lists.
            if let Some(old_civ) = state.civilizations.iter_mut().find(|c| c.id == *old_owner) {
                old_civ.cities.retain(|cid| *cid != *city);
            }
            if let Some(new_civ) = state.civilizations.iter_mut().find(|c| c.id == *new_owner)
                && !new_civ.cities.contains(city)
            {
                new_civ.cities.push(*city);
            }
        }
        StateDelta::PopulationGrew { city, new_population } => {
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                c.population = *new_population;
            }
        }
        StateDelta::CitizenAssigned { .. } => {
            // Informational: citizen assignment already applied by auto_assign_citizen.
        }

        // ── Economy ─────────────────────────────────────────────────────
        StateDelta::GoldChanged { civ, delta: d } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.gold += d;
            }
        }
        StateDelta::FaithChanged { civ, delta: d } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.faith = (c.faith as i32 + d).max(0) as u32;
            }
        }
        StateDelta::StrategicResourceChanged { civ, resource, delta: d } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                let entry = c.strategic_resources.entry(*resource).or_insert(0);
                *entry = (*entry as i32 + d).max(0) as u32;
            }
        }

        // ── Tech & Civics ───────────────────────────────────────────────
        StateDelta::TechResearched { civ, tech } => {
            let tid = state.tech_tree.nodes.values()
                .find(|n| n.name == *tech).map(|n| n.id);
            if let Some(tid) = tid {
                if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ)
                    && !c.researched_techs.contains(&tid)
                {
                    c.researched_techs.push(tid);
                }
            }
        }
        StateDelta::CivicCompleted { civ, civic } => {
            let cid = state.civic_tree.nodes.values()
                .find(|n| n.name == *civic).map(|n| n.id);
            if let Some(cid) = cid {
                if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ)
                    && !c.completed_civics.contains(&cid)
                {
                    c.completed_civics.push(cid);
                }
            }
        }

        // ── Diplomacy ───────────────────────────────────────────────────
        StateDelta::DiplomacyChanged { civ_a, civ_b, new_status } => {
            if let Some(rel) = state.diplomatic_relations.iter_mut()
                .find(|r| (r.civ_a == *civ_a && r.civ_b == *civ_b) ||
                          (r.civ_a == *civ_b && r.civ_b == *civ_a))
            {
                rel.status = *new_status;
            }
        }
        StateDelta::DiplomaticFavorChanged { civ, delta: d } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.diplomatic_favor = (c.diplomatic_favor as i32 + d).max(0) as u32;
            }
        }

        // ── One-shot effect outcomes ────────────────────────────────────
        StateDelta::ResourceRevealed { .. } => {
            // Informational: resource reveal is tracked via researched_techs.
        }
        StateDelta::EurekaTriggered { .. } |
        StateDelta::InspirationTriggered { .. } => {
            // Informational: boost applied during tech/civic progress.
        }
        StateDelta::UnitUnlocked { .. } |
        StateDelta::BuildingUnlocked { .. } |
        StateDelta::ImprovementUnlocked { .. } |
        StateDelta::GovernmentUnlocked { .. } |
        StateDelta::PolicyUnlocked { .. } => {
            // Informational: unlock tracking is derived from researched techs/civics.
        }
        StateDelta::GovernmentAdopted { civ, government } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.current_government_name = Some(government);
            }
        }
        StateDelta::PolicyUnslotted { civ, policy } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.active_policies.retain(|p| *p != *policy);
            }
        }
        StateDelta::PolicyAssigned { civ, policy } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ)
                && !c.active_policies.contains(policy)
            {
                c.active_policies.push(*policy);
            }
        }
        StateDelta::FreeUnitGranted { .. } => {
            // Informational: unit creation handled by effect queue processing.
        }
        StateDelta::FreeBuildingGranted { .. } => {
            // Informational: building addition handled by effect queue processing.
        }

        // ── Production ──────────────────────────────────────────────────
        StateDelta::BuildingCompleted { city, building } => {
            let bid = state.building_defs.iter()
                .find(|d| d.name == *building).map(|d| d.id);
            if let Some(bid) = bid {
                if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city)
                    && !c.buildings.contains(&bid)
                {
                    c.buildings.push(bid);
                }
            }
        }
        StateDelta::DistrictBuilt { city: _, district: _, coord: _ } => {
            // Informational: district already placed by rules engine.
        }
        StateDelta::WonderBuilt { .. } => {
            // Informational: wonder tracking already updated.
        }
        StateDelta::ProductionStarted { .. } => {
            // Informational.
        }
        StateDelta::ProductionBurst { city, amount } => {
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                c.production_stored += amount;
            }
        }

        // ── Combat ──────────────────────────────────────────────────────
        StateDelta::UnitAttacked { .. } => {
            // Informational: damage was applied to units directly by the
            // combat handler before this delta was emitted.
        }
        StateDelta::ExperienceGained { unit, new_total, .. } => {
            if let Some(u) = state.unit_mut(*unit) {
                u.experience = *new_total;
            }
        }
        StateDelta::UnitPromoted { unit, promotion, .. } => {
            if let Some(u) = state.unit_mut(*unit) {
                if !u.promotions.contains(promotion) {
                    u.promotions.push(*promotion);
                }
            }
        }
        StateDelta::TheologicalCombat { .. } => {
            // Informational: damage applied by theological combat handler.
        }

        // ── Fog of war ──────────────────────────────────────────────────
        StateDelta::TilesRevealed { civ, coords } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                for coord in coords {
                    c.explored_tiles.insert(*coord);
                }
            }
        }
        StateDelta::NaturalWonderDiscovered { .. } => {
            // Informational: tile was already added to explored_tiles via TilesRevealed.
        }

        // ── Improvements & Roads ────────────────────────────────────────
        StateDelta::ImprovementPlaced { coord, improvement } => {
            if let Some(tile) = state.board.tile_mut(*coord) {
                tile.improvement = Some(*improvement);
            }
        }
        StateDelta::RoadPlaced { coord, road } => {
            if let Some(tile) = state.board.tile_mut(*coord) {
                tile.road = Some(*road);
            }
        }

        // ── Territory ───────────────────────────────────────────────────
        StateDelta::TileClaimed { civ, city, coord } => {
            if let Some(tile) = state.board.tile_mut(*coord) {
                tile.owner = Some(*civ);
            }
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                c.territory.insert(*coord);
            }
        }
        StateDelta::TileReassigned { civ: _, from_city, to_city, coord } => {
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *from_city) {
                c.territory.remove(coord);
            }
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *to_city) {
                c.territory.insert(*coord);
            }
        }

        // ── Trade routes ────────────────────────────────────────────────
        StateDelta::TradeRouteAssigned { .. } => {
            // Informational: trader fields already set by assign_trade_route.
        }
        StateDelta::TradeRouteEstablished { .. } => {
            // Informational: route already added to state.trade_routes.
        }
        StateDelta::TradeRouteExpired { route } => {
            state.trade_routes.retain(|r| r.id != *route);
        }
        StateDelta::TradeRouteCleared { .. } => {
            // Informational: trader fields already cleared.
        }

        // ── City defense ────────────────────────────────────────────────
        StateDelta::WallDamaged { city, hp_remaining, .. } => {
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                c.wall_hp = *hp_remaining;
            }
        }
        StateDelta::WallDestroyed { city, .. } => {
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                c.walls = WallLevel::None;
                c.wall_hp = 0;
            }
        }

        // ── Tourism ─────────────────────────────────────────────────────
        StateDelta::TourismGenerated { civ, tourism: _, lifetime_culture } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.lifetime_culture = *lifetime_culture;
            }
        }

        // ── Loyalty ─────────────────────────────────────────────────────
        StateDelta::LoyaltyChanged { city, new_value, .. } => {
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                c.loyalty = *new_value;
            }
        }
        StateDelta::CityRevolted { city, new_owner, old_owner } => {
            if let Some(c) = state.cities.iter_mut().find(|c| c.id == *city) {
                if let Some(owner) = new_owner {
                    c.owner = *owner;
                }
            }
            if let Some(old_civ) = state.civilizations.iter_mut().find(|c| c.id == *old_owner) {
                old_civ.cities.retain(|cid| *cid != *city);
            }
            if let Some(Some(new_owner)) = Some(new_owner) {
                if let Some(new_civ) = state.civilizations.iter_mut().find(|c| c.id == *new_owner) {
                    if !new_civ.cities.contains(city) {
                        new_civ.cities.push(*city);
                    }
                }
            }
        }

        // ── Era system ──────────────────────────────────────────────────
        StateDelta::HistoricMomentEarned { .. } => {
            // Informational: era score already updated.
        }
        StateDelta::EraAdvanced { civ, new_era: _, era_age } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.era_age = *era_age;
            }
        }

        // ── Great persons ───────────────────────────────────────────────
        StateDelta::GreatPersonRetired { great_person, .. } => {
            if let Some(gp) = state.great_people.iter_mut().find(|g| g.id == *great_person) {
                gp.is_retired = true;
            }
        }
        StateDelta::GreatPersonPointsAccumulated { .. } => {
            // Informational: points already accumulated in advance_turn.
        }
        StateDelta::GreatPersonRecruited { great_person, civ, .. } => {
            if let Some(gp) = state.great_people.iter_mut().find(|g| g.id == *great_person) {
                gp.owner = Some(*civ);
            }
        }
        StateDelta::GreatPersonPatronized { civ, gold_spent, .. } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.gold -= *gold_spent as i32;
            }
        }
        StateDelta::GreatPersonPatronizedWithFaith { civ, faith_spent, .. } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.faith = c.faith.saturating_sub(*faith_spent);
            }
        }
        StateDelta::GreatWorkCreated { .. } => {
            // Informational: great work already added to state.great_works.
        }

        // ── Governors ───────────────────────────────────────────────────
        StateDelta::GovernorAssigned { governor, city, .. } => {
            if let Some(g) = state.governors.iter_mut().find(|g| g.id == *governor) {
                g.assigned_city = Some(*city);
                g.turns_to_establish = 5;
            }
        }
        StateDelta::GovernorEstablished { governor, .. } => {
            if let Some(g) = state.governors.iter_mut().find(|g| g.id == *governor) {
                g.turns_to_establish = 0;
            }
        }
        StateDelta::GovernorPromoted { governor, promotion } => {
            if let Some(g) = state.governors.iter_mut().find(|g| g.id == *governor) {
                if !g.promotions.contains(promotion) {
                    g.promotions.push(promotion);
                }
            }
        }
        StateDelta::GovernorTitleEarned { civ } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.governor_titles += 1;
            }
        }

        // ── Victory ─────────────────────────────────────────────────────
        StateDelta::VictoryAchieved { civ, condition } => {
            state.game_over = Some(GameOver {
                winner: *civ,
                condition,
                turn: state.turn,
            });
        }

        // ── Religion ────────────────────────────────────────────────────
        StateDelta::ReligionFounded { .. } => {
            // Informational: religion already added to state.religions.
        }
        StateDelta::PantheonFounded { civ, belief } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.pantheon_belief = Some(*belief);
            }
        }
        StateDelta::BeliefSelected { .. } => {
            // Informational: belief already added to religion.
        }
        StateDelta::ReligionSpread { .. } => {
            // Informational: followers already updated.
        }
        StateDelta::ReligiousPressureApplied { .. } => {
            // Informational: followers already updated.
        }
        StateDelta::CityConvertedReligion { .. } => {
            // Informational: majority religion derived from followers.
        }
        StateDelta::BeliefEvangelized { .. } => {
            // Informational: belief already added.
        }
        StateDelta::InquisitionLaunched { .. } => {
            // Informational.
        }
        StateDelta::HeresyRemoved { .. } => {
            // Informational: followers already removed.
        }
        StateDelta::ReligiousUnitsHealed { .. } => {
            // Informational: healing applied to units directly.
        }

        // ── Science victory ─────────────────────────────────────────────
        StateDelta::ScienceMilestoneCompleted { civ, .. } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.science_milestones_completed += 1;
            }
        }

        // ── Power & CO2 ──────────────────────────────────────────────────
        StateDelta::CO2Accumulated { total } => {
            state.global_co2 = *total;
        }

        // ── Projects ────────────────────────────────────────────────────
        StateDelta::ProjectCompleted { .. } => {
            // Informational: project effects already applied by advance_turn.
        }

        // ── Barbarians ──────────────────────────────────────────────────
        StateDelta::BarbarianCampSpawned { .. } |
        StateDelta::BarbarianCampDestroyed { .. } |
        StateDelta::BarbarianScoutSpawned { .. } |
        StateDelta::BarbarianScoutDiscovered { .. } |
        StateDelta::BarbarianScoutReturned { .. } |
        StateDelta::BarbarianUnitGenerated { .. } |
        StateDelta::BarbarianClanHired { .. } |
        StateDelta::BarbarianClanBribed { .. } |
        StateDelta::BarbarianClanIncited { .. } |
        StateDelta::BarbarianCampConverted { .. } => {
            // Informational: barbarian state is managed directly by the
            // barbarian system in advance_turn.
        }

        // ── World Congress (GS-3) ────────────────────────────────────────
        StateDelta::CongressSessionHeld { .. } => {
            // Informational: congress session state already updated in advance_turn.
        }
        StateDelta::DiplomaticVPEarned { civ, points } => {
            *state.world_congress.diplomatic_victory_points
                .entry(*civ).or_insert(0) += points;
        }

        // ── Rock Band / Cultural Combat (GS-16) ─────���──────────────────
        StateDelta::RockBandPerformed { .. } => {
            // Informational: tourism already applied by the rules engine.
        }

        // ── Climate & Disasters (GS-2) ─��───────────────────────────────
        StateDelta::SeaLevelRose { new_level } => {
            state.climate_level = *new_level;
        }
        StateDelta::TileSubmerged { coord } => {
            if let Some(tile) = state.board.tile_mut(*coord) {
                tile.submerged = true;
                tile.improvement = None;
                tile.improvement_pillaged = false;
            }
        }
        StateDelta::DisasterOccurred { coord, kind, .. } => {
            if let Some(tile) = state.board.tile_mut(*coord) {
                tile.improvement = None;
                tile.improvement_pillaged = false;
                if *kind == crate::world::disaster::DisasterKind::VolcanicEruption {
                    tile.feature = Some(crate::world::feature::BuiltinFeature::VolcanicSoil);
                }
            }
        }

        // ── Alliances (Rise & Fall) ────────────────────────────────────────
        StateDelta::AllianceFormed { .. } |
        StateDelta::AllianceLevelUp { .. } => {
            // Informational: alliance state is managed directly by form_alliance
            // and advance_turn.
        }
    }
}

/// Apply an entire diff (batch of deltas) to the game state.
pub fn apply_diff(state: &mut GameState, diff: &GameStateDiff) {
    for delta in &diff.deltas {
        apply_delta(state, delta);
    }
}

use crate::civ::city::WallLevel;
