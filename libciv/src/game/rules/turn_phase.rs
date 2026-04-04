//! Turn phase: advance_turn implementation.

use crate::{CityId, CivId, TechId, UnitId, UnitTypeId, YieldBundle};
use crate::civ::DiplomaticStatus;
use crate::civ::civ_ability::RuleOverride;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use super::{RulesError, lookup_bundle, has_rule_override};
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::rules_helpers::{
    auto_assign_citizen, city_culture_output, compute_city_loyalty_delta,
    compute_diplomatic_status, highest_pressure_civ, tile_border_cost, try_claim_tile,
};
use super::super::state::GameState;
use crate::rules::unique::UniqueUnitAbility;

const RELIGIOUS_PRESSURE_RADIUS: u32 = 10;

/// Advance the game state by one turn. Returns diff.
pub(crate) fn advance_turn(_engine: &super::DefaultRulesEngine, state: &mut GameState) -> GameStateDiff {
    let mut diff = GameStateDiff::new();

    // ── Reset per-turn city bombardment flag ─────────────────────────────
    for city in &mut state.cities {
        city.has_attacked_this_turn = false;
    }

    // ── Per-city food accumulation and population growth ──────────────────
    // Collect food from worked_tiles (immutable board borrow), then mutate cities.
    let city_food: Vec<(usize, i32)> = {
        let board = &state.board;
        state.cities.iter().enumerate().map(|(i, city)| {
            let food: i32 = city.worked_tiles.iter()
                .filter_map(|&coord| board.tile(coord))
                .map(|t| t.total_yields().food)
                .sum();
            (i, food)
        }).collect()
    };

    // Track cities that grew so we can auto-assign a new citizen after the loop.
    let mut grew_cities: Vec<usize> = Vec::new();

    for (i, food) in city_food {
        let city = &mut state.cities[i];
        if food > 0 {
            city.food_stored += food as u32;
        }
        if city.food_stored >= city.food_to_grow {
            city.food_stored  = 0;
            city.population  += 1;
            city.food_to_grow = 15 + 6 * (city.population - 1);
            diff.push(StateDelta::PopulationGrew {
                city: city.id,
                new_population: city.population,
            });
            grew_cities.push(i);
        }
    }

    // Auto-assign a new worked tile for each city that just grew.
    for i in grew_cities {
        let city_id = state.cities[i].id;
        if let Some(coord) = auto_assign_citizen(&state.board, &mut state.cities[i]) {
            diff.push(StateDelta::CitizenAssigned { city: city_id, tile: coord });
        }
    }

    // ── Per-city production accumulation + strategic resource yield ────────
    // Collect production yield and strategic resource tiles from worked tiles
    // (immutable board borrow), then apply mutations in separate passes.
    use crate::world::resource::BuiltinResource;
    use crate::enums::ResourceCategory;
    use std::collections::HashMap as StdHashMap;

    struct CityTurnData {
        city_idx: usize,
        civ_id:   CivId,
        coord:    HexCoord,
        prod:     u32,
        /// Strategic resources yielded this turn by worked tiles with an improvement.
        resource_yields: Vec<BuiltinResource>,
    }

    let city_turn_data: Vec<CityTurnData> = {
        let board = &state.board;
        state.cities.iter().enumerate().map(|(i, city)| {
            let prod: i32 = city.worked_tiles.iter()
                .filter_map(|&coord| board.tile(coord))
                .map(|t| t.total_yields().production)
                .sum();
            let resource_yields: Vec<BuiltinResource> = city.worked_tiles.iter()
                .filter_map(|&coord| board.tile(coord))
                .filter_map(|t| {
                    let res = t.resource?;
                    if res.category() == ResourceCategory::Strategic && t.improvement.is_some() {
                        Some(res)
                    } else {
                        None
                    }
                })
                .collect();
            CityTurnData {
                city_idx: i,
                civ_id:   city.owner,
                coord:    city.coord,
                prod:     prod.max(0) as u32,
                resource_yields,
            }
        }).collect()
    };

    // Apply production accumulation.
    for d in &city_turn_data {
        state.cities[d.city_idx].production_stored += d.prod;
    }

    // Aggregate strategic resource gains per (civ, resource) and apply.
    let mut resource_gains: StdHashMap<(CivId, BuiltinResource), u32> = StdHashMap::new();
    for d in &city_turn_data {
        for &res in &d.resource_yields {
            *resource_gains.entry((d.civ_id, res)).or_insert(0) += 1;
        }
    }
    for ((civ_id, resource), amount) in resource_gains {
        if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
            *civ.strategic_resources.entry(resource).or_insert(0) += amount;
            diff.push(StateDelta::StrategicResourceChanged { civ: civ_id, resource, delta: amount as i32 });
        }
    }

    // Complete unit production for cities whose stored production meets the cost.
    // Units with a strategic resource cost are blocked if the civ lacks the resource.
    struct UnitCompletion {
        city_idx:        usize,
        civ_id:          CivId,
        coord:           HexCoord,
        type_id:         UnitTypeId,
        production_cost: u32,
        resource_cost:   Option<(BuiltinResource, u32)>,
        domain:          crate::UnitDomain,
        category:        crate::UnitCategory,
        max_movement:    u32,
        combat_strength: Option<u32>,
        range:           u8,
        vision_range:    u8,
        max_charges:     u8,
    }

    let unit_completions: Vec<UnitCompletion> = city_turn_data.iter().filter_map(|d| {
        use crate::civ::city::ProductionItem;
        use crate::game::production_helpers::resolve_unit_replacement;
        let city = &state.cities[d.city_idx];
        if let Some(ProductionItem::Unit(tid)) = city.production_queue.front() {
            // Resolve civ-exclusive replacement: the queue stores the generic
            // unit (e.g. "Swordsman") and we swap in the civ's unique variant
            // (e.g. "Legion" for Rome) at completion time.
            let (resolved_tid, _) = resolve_unit_replacement(state, d.civ_id, *tid);
            let def = state.unit_type_defs.iter().find(|def| def.id == resolved_tid)?;
            if city.production_stored >= def.production_cost {
                Some(UnitCompletion {
                    city_idx:        d.city_idx,
                    civ_id:          d.civ_id,
                    coord:           d.coord,
                    type_id:         def.id,
                    production_cost: def.production_cost,
                    resource_cost:   def.resource_cost,
                    domain:          def.domain,
                    category:        def.category,
                    max_movement:    def.max_movement,
                    combat_strength: def.combat_strength,
                    range:           def.range,
                    vision_range:    def.vision_range,
                    max_charges:     def.max_charges,
                })
            } else {
                None
            }
        } else {
            None
        }
    }).collect();

    for uc in unit_completions {
        // Check and deduct strategic resource cost.
        if let Some((resource, required)) = uc.resource_cost {
            let available = state.civilizations.iter()
                .find(|c| c.id == uc.civ_id)
                .map(|c| *c.strategic_resources.get(&resource).unwrap_or(&0))
                .unwrap_or(0);
            if available < required {
                continue; // Insufficient — defer until resources are available.
            }
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == uc.civ_id) {
                *civ.strategic_resources.entry(resource).or_insert(0) -= required;
                diff.push(StateDelta::StrategicResourceChanged {
                    civ: uc.civ_id, resource, delta: -(required as i32),
                });
            }
        }

        // Deduct production cost and complete the item.
        state.cities[uc.city_idx].production_stored -= uc.production_cost;
        state.cities[uc.city_idx].production_queue.pop_front();

        let unit_id = state.id_gen.next_unit_id();
        let charges = if uc.max_charges > 0 { Some(uc.max_charges) } else { None };
        state.units.push(crate::civ::BasicUnit {
            id:              unit_id,
            unit_type:       uc.type_id,
            owner:           uc.civ_id,
            coord:           uc.coord,
            domain:          uc.domain,
            category:        uc.category,
            movement_left:   uc.max_movement,
            max_movement:    uc.max_movement,
            combat_strength: uc.combat_strength,
            promotions:      Vec::new(),
            experience:      0,
            health:          100,
            range:           uc.range,
            vision_range:    uc.vision_range,
            charges,
            trade_origin: None,
            trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });
        diff.push(StateDelta::UnitCreated { unit: unit_id, coord: uc.coord, owner: uc.civ_id });
        // TODO(PHASE3-4.3): Building, District, Wonder completion.
    }

    // ── Phase 2a-2: Project completion ──────────────────────────────────
    // Complete projects when stored production meets the cost.
    {
        use crate::civ::city::ProductionItem;
        use crate::ProjectId;

        struct ProjectCompletion {
            city_idx: usize,
            civ_id:   CivId,
            project_id: ProjectId,
            project_name: &'static str,
            production_cost: u32,
        }

        let completions: Vec<ProjectCompletion> = city_turn_data.iter().filter_map(|d| {
            let city = &state.cities[d.city_idx];
            if let Some(ProductionItem::Project(pid)) = city.production_queue.front() {
                let def = state.project_defs.iter().find(|def| def.id == *pid)?;
                if city.production_stored >= def.production_cost {
                    // Validate district requirement.
                    if let Some(req_district) = def.requires_district {
                        let has_district = city.districts.iter().any(|d| d.name() == req_district);
                        if !has_district {
                            return None;
                        }
                    }
                    Some(ProjectCompletion {
                        city_idx: d.city_idx,
                        civ_id: d.civ_id,
                        project_id: def.id,
                        project_name: def.name,
                        production_cost: def.production_cost,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }).collect();

        let science_milestone_names: &[&str] = &[
            "Launch Earth Satellite",
            "Launch Moon Landing",
            "Launch Mars Colony",
            "Exoplanet Expedition",
        ];

        for pc in completions {
            state.cities[pc.city_idx].production_stored -= pc.production_cost;
            state.cities[pc.city_idx].production_queue.pop_front();

            diff.push(StateDelta::ProjectCompleted {
                city: state.cities[pc.city_idx].id,
                project: pc.project_name,
            });

            // Science milestone projects increment the civ's milestone counter.
            if science_milestone_names.contains(&pc.project_name) {
                if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == pc.civ_id) {
                    civ.science_milestones_completed += 1;
                }
                diff.push(StateDelta::ScienceMilestoneCompleted {
                    civ: pc.civ_id,
                    milestone: pc.project_name,
                });
            }

            // Carbon Recapture reduces global CO2.
            if pc.project_name == "Carbon Recapture" {
                state.global_co2 = state.global_co2.saturating_sub(50);
            }

            // Non-repeatable projects: prevent re-queuing by removing from defs.
            // (Repeatable projects stay in the registry.)
            let is_repeatable = state.project_defs.iter()
                .find(|d| d.id == pc.project_id)
                .map(|d| d.repeatable)
                .unwrap_or(false);
            if !is_repeatable {
                state.project_defs.retain(|d| d.id != pc.project_id);
            }
        }
    }

    // ── Phase 2b: Trade route countdown ──────────────────────────────────
    // Expire routes that reached turns_remaining == 0 last turn (they already
    // delivered on all their turns), then decrement remaining routes.
    {
        use crate::TradeRouteId;
        let expired: Vec<TradeRouteId> = state.trade_routes.iter()
            .filter(|r| r.turns_remaining == Some(0))
            .map(|r| r.id)
            .collect();
        for id in &expired {
            state.trade_routes.retain(|r| r.id != *id);
            diff.push(StateDelta::TradeRouteExpired { route: *id });
        }
        for route in state.trade_routes.iter_mut() {
            if let Some(ref mut t) = route.turns_remaining {
                *t = t.saturating_sub(1);
            }
        }
    }

    // ── Phase 2b-2: Autonomous trader movement ─────────────────────────────
    // Traders with an assigned destination move toward the destination city
    // each turn. When a trader arrives at the destination city tile, the trade
    // route is automatically established and the trader is consumed.
    {
        // Collect traders with assigned destinations (sorted for determinism).
        let mut trader_ids: Vec<UnitId> = state.units.iter()
            .filter(|u| u.category == crate::UnitCategory::Trader && u.trade_destination.is_some())
            .map(|u| u.id)
            .collect();
        trader_ids.sort();

        for trader_id in trader_ids {
            // Re-read each iteration since state is mutated.
            let info = state.units.iter()
                .find(|u| u.id == trader_id)
                .map(|u| (u.trade_origin, u.trade_destination, u.max_movement));
            let Some((Some(_origin), Some(dest_city_id), max_movement)) = info else {
                continue;
            };

            // Reset movement for this turn.
            if let Some(u) = state.units.iter_mut().find(|u| u.id == trader_id) {
                u.movement_left = max_movement;
            }

            // Find the destination city's coord.
            let dest_coord = state.cities.iter()
                .find(|c| c.id == dest_city_id)
                .map(|c| c.coord);
            let Some(dest_coord) = dest_coord else { continue };

            // Move the trader toward the destination using Dijkstra pathfinding.
            let move_deltas = match super::movement::move_unit(state, trader_id, dest_coord) {
                Ok(move_diff) => move_diff.deltas,
                Err(RulesError::InsufficientMovement(partial)) => partial.deltas,
                Err(_) => Vec::new(),
            };
            for delta in &move_deltas {
                if let StateDelta::UnitMoved { unit, to, cost, .. } = delta
                    && let Some(u) = state.units.iter_mut().find(|u| u.id == *unit)
                {
                    u.coord = *to;
                    u.movement_left = u.movement_left.saturating_sub(*cost);
                }
            }
            diff.deltas.extend(move_deltas);

            // Check if the trader has arrived at the destination city tile.
            let arrived = state.units.iter()
                .find(|u| u.id == trader_id)
                .map(|u| u.coord == dest_coord)
                .unwrap_or(false);

            if arrived {
                // Auto-establish the trade route.
                match super::trade::establish_trade_route(state, trader_id, dest_city_id) {
                    Ok(route_diff) => {
                        diff.deltas.extend(route_diff.deltas);
                    }
                    Err(_) => {
                        // Establishment failed; clear assignment so it doesn't loop.
                        if let Some(u) = state.units.iter_mut().find(|u| u.id == trader_id) {
                            u.trade_origin = None;
                            u.trade_destination = None;
                        }
                        diff.push(StateDelta::TradeRouteCleared { unit: trader_id });
                    }
                }
            }
        }
    }

    // ── Phase 2c: Road maintenance gold deduction ──────────────────────────
    // For each civilization, sum maintenance costs of all road tiles they own
    // and deduct from the civ's gold.
    {
        use std::collections::HashMap as RoadMap;
        let mut road_costs: RoadMap<CivId, i32> = RoadMap::new();
        for coord in state.board.all_coords() {
            if let Some(tile) = state.board.tile(coord)
                && let (Some(owner), Some(road)) = (tile.owner, &tile.road)
            {
                *road_costs.entry(owner).or_insert(0) += road.as_def().maintenance() as i32;
            }
        }
        for (civ_id, cost) in road_costs {
            if cost > 0
                && let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id)
            {
                civ.gold -= cost;
                diff.push(StateDelta::GoldChanged { civ: civ_id, delta: -cost });
            }
        }
    }

    // ── Phase 2c-2: Power & CO2 accumulation ──────────────────────────────
    // For each city, recompute power balance and accumulate CO2 from fossil fuel plants.
    {
        // Collect per-city power data (immutable borrow of both cities + building_defs).
        let city_power: Vec<(usize, u32, u32, u32)> = state.cities.iter().enumerate()
            .map(|(i, city)| {
                let mut consumed: u32 = 0;
                let mut generated: u32 = 0;
                let mut co2: u32 = 0;
                for &bid in &city.buildings {
                    if let Some(bdef) = state.building_defs.iter().find(|d| d.id == bid) {
                        consumed += bdef.power_cost;
                        generated += bdef.power_generated;
                        co2 += bdef.co2_per_turn;
                    }
                }
                (i, consumed, generated, co2)
            })
            .collect();
        let mut co2_this_turn: u32 = 0;
        for (i, consumed, generated, co2) in city_power {
            state.cities[i].power_consumed = consumed;
            state.cities[i].power_generated = generated;
            co2_this_turn += co2;
        }
        if co2_this_turn > 0 {
            state.global_co2 += co2_this_turn;
            diff.push(StateDelta::CO2Accumulated { total: state.global_co2 });
        }
    }

    // ── Phase 2c-3: Climate & Disasters ──────────────────────────────────
    {
        use crate::world::climate::climate_level_for_co2;
        use crate::world::disaster::DisasterKind;
        use crate::world::feature::BuiltinFeature;

        // (a) Check if global_co2 crossed a threshold → increment climate_level.
        let new_level = climate_level_for_co2(state.global_co2);
        if new_level > state.climate_level {
            state.climate_level = new_level;
            diff.push(StateDelta::SeaLevelRose { new_level });
        }

        // (b) Submerge coastal lowland tiles whose elevation <= climate_level.
        let coords_to_submerge: Vec<HexCoord> = state.board.all_coords().into_iter()
            .filter(|&coord| {
                if let Some(tile) = state.board.tile(coord) {
                    if let Some(elev) = tile.coastal_lowland {
                        !tile.submerged && elev <= state.climate_level
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .collect();

        for coord in coords_to_submerge {
            if let Some(tile) = state.board.tile_mut(coord) {
                tile.submerged = true;
                tile.improvement = None;
                tile.improvement_pillaged = false;
            }
            diff.push(StateDelta::TileSubmerged { coord });
        }

        // (c) Roll for random disaster (simplified: at most one per turn).
        // Base probability 5% + 2% per climate_level.
        let disaster_chance = 0.05 + 0.02 * state.climate_level as f32;
        let roll = state.id_gen.next_f32();
        if roll < disaster_chance {
            // Pick a random disaster type.
            let type_roll = state.id_gen.next_f32();
            let type_idx = (type_roll * DisasterKind::ALL.len() as f32) as usize;
            let kind = DisasterKind::ALL[type_idx.min(DisasterKind::ALL.len() - 1)];

            // Pick a random tile.
            let all_coords = state.board.all_coords();
            if !all_coords.is_empty() {
                let coord_roll = state.id_gen.next_f32();
                let coord_idx = (coord_roll * all_coords.len() as f32) as usize;
                let coord = all_coords[coord_idx.min(all_coords.len() - 1)];

                // Severity 1-3, biased by climate level.
                let sev_roll = state.id_gen.next_f32();
                let severity = ((sev_roll * 3.0) as u8 + 1).min(3);

                // Apply damage: destroy improvement.
                if let Some(tile) = state.board.tile_mut(coord) {
                    tile.improvement = None;
                    tile.improvement_pillaged = false;
                    // Volcanic eruptions add VolcanicSoil.
                    if kind == DisasterKind::VolcanicEruption {
                        tile.feature = Some(BuiltinFeature::VolcanicSoil);
                    }
                }

                diff.push(StateDelta::DisasterOccurred { kind, coord, severity });
            }
        }
    }

    // ── Per-civ yields: gold, science, culture ────────────────────────────
    let civ_ids: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();

    // Collect yields while state is immutably borrowed.
    // Apply civ-specific yield multipliers (e.g., Babylon's -50% science).
    let civ_yields: Vec<(CivId, YieldBundle)> = civ_ids.iter()
        .map(|&id| {
            let mut y = super::city::compute_yields(state, id);
            // Babylon: -50% science per turn.
            if has_rule_override(state, id, &|o| matches!(o, RuleOverride::SciencePerTurnMultiplier(_))) {
                let civ_identity = state.civilizations.iter()
                    .find(|c| c.id == id).and_then(|c| c.civ_identity);
                if let Some(bundle) = lookup_bundle(civ_identity) {
                    for ovr in &bundle.rule_overrides {
                        if let RuleOverride::SciencePerTurnMultiplier(pct) = ovr {
                            // pct is -50, meaning 50% of normal.
                            let multiplier = (100 + pct).max(0) as f32 / 100.0;
                            y.science = (y.science as f32 * multiplier) as i32;
                        }
                    }
                }
            }
            (id, y)
        })
        .collect();

    // Apply gold.
    for (civ_id, yields) in &civ_yields {
        if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id)
            && yields.gold != 0
        {
            civ.gold += yields.gold;
            diff.push(StateDelta::GoldChanged { civ: *civ_id, delta: yields.gold });
        }
    }

    // Apply science -> tech progress and check completion.
    // Two-pass: first update progress (mutates civilizations), then check
    // tech_tree (different field, disjoint borrow).
    struct TechCheck { civ_idx: usize, civ_id: CivId, tech_id: TechId, progress: u32 }
    let mut tech_checks: Vec<TechCheck> = Vec::new();

    for (civ_id, yields) in &civ_yields {
        if yields.science <= 0 { continue; }
        if let Some((idx, civ)) = state.civilizations.iter_mut()
            .enumerate().find(|(_, c)| c.id == *civ_id)
            && let Some(tp) = civ.research_queue.front_mut()
        {
            tp.progress += yields.science as u32;
            tech_checks.push(TechCheck {
                civ_idx: idx,
                civ_id:  *civ_id,
                tech_id: tp.tech_id,
                progress: tp.progress,
            });
        }
    }

    for tc in tech_checks {
        let node_info = state.tech_tree.get(tc.tech_id)
            .map(|n| (n.cost, n.name, n.effects.clone()));
        if let Some((cost, name, effects)) = node_info {
            let boosted = state.civilizations[tc.civ_idx]
                .research_queue.front()
                .map(|tp| tp.boosted)
                .unwrap_or(false);
            let effective_cost = if boosted { cost / 2 } else { cost };
            if tc.progress >= effective_cost {
                state.civilizations[tc.civ_idx].researched_techs.push(tc.tech_id);
                state.civilizations[tc.civ_idx].research_queue.pop_front();
                diff.push(StateDelta::TechResearched { civ: tc.civ_id, tech: name });
                for effect in effects {
                    state.effect_queue.push_back((tc.civ_id, effect));
                }
            }
        }
    }

    // Apply culture -> civic progress (same pattern as science).
    struct CivicCheck { civ_idx: usize, civ_id: CivId, civic_id: crate::CivicId, progress: u32 }
    let mut civic_checks: Vec<CivicCheck> = Vec::new();

    for (civ_id, yields) in &civ_yields {
        if yields.culture <= 0 { continue; }
        if let Some((idx, civ)) = state.civilizations.iter_mut()
            .enumerate().find(|(_, c)| c.id == *civ_id)
            && let Some(cp) = civ.civic_in_progress.as_mut()
        {
            cp.progress += yields.culture as u32;
            civic_checks.push(CivicCheck {
                civ_idx: idx,
                civ_id:  *civ_id,
                civic_id: cp.civic_id,
                progress: cp.progress,
            });
        }
    }

    for cc in civic_checks {
        let node_info = state.civic_tree.get(cc.civic_id)
            .map(|n| (n.cost, n.name, n.effects.clone()));
        if let Some((cost, name, effects)) = node_info {
            let inspired = state.civilizations[cc.civ_idx]
                .civic_in_progress.as_ref()
                .map(|cp| cp.inspired)
                .unwrap_or(false);
            let effective_cost = if inspired { cost / 2 } else { cost };
            if cc.progress >= effective_cost {
                state.civilizations[cc.civ_idx].completed_civics.push(cc.civic_id);
                state.civilizations[cc.civ_idx].civic_in_progress = None;
                diff.push(StateDelta::CivicCompleted { civ: cc.civ_id, civic: name });
                for effect in effects {
                    state.effect_queue.push_back((cc.civ_id, effect));
                }
                // Governor title grant from certain civics.
                if super::governors::civic_grants_governor_title(name) {
                    state.civilizations[cc.civ_idx].governor_titles += 1;
                    diff.push(StateDelta::GovernorTitleEarned { civ: cc.civ_id });
                }
            }
        }
    }

    // ── Phase 3a: Tourism accumulation ────────────────────────────────────
    // Accumulate lifetime culture from this turn's culture output, compute
    // tourism, and distribute tourism pressure to other civilizations.
    {
        use crate::civ::tourism::compute_tourism;

        let civ_ids_for_tourism: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();

        // Accumulate lifetime culture from this turn's culture yield.
        for (civ_id, yields) in &civ_yields {
            if yields.culture > 0
                && let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id)
            {
                civ.lifetime_culture += yields.culture as u32;
            }
        }

        // Compute tourism and distribute to all other civs.
        // Must collect tourism values first (immutable borrow) then mutate.
        let tourism_outputs: Vec<(CivId, u32)> = civ_ids_for_tourism.iter()
            .map(|&id| (id, compute_tourism(state, id)))
            .collect();

        for (civ_id, tourism) in &tourism_outputs {
            if *tourism == 0 { continue; }

            // Find the civ's lifetime culture for the delta.
            let lifetime_culture = state.civ(*civ_id)
                .map(|c| c.lifetime_culture)
                .unwrap_or(0);

            diff.push(StateDelta::TourismGenerated {
                civ: *civ_id,
                tourism: *tourism,
                lifetime_culture,
            });

            // Distribute tourism equally to all other civs.
            let other_count = civ_ids_for_tourism.len().saturating_sub(1);
            if other_count == 0 { continue; }

            let per_civ = *tourism; // Each other civ gets full tourism pressure.
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id) {
                for &target in &civ_ids_for_tourism {
                    if target != *civ_id {
                        *civ.tourism_accumulated.entry(target).or_insert(0) += per_civ;
                    }
                }
            }
        }
    }

    // ── Phase 3b: Cultural border expansion ───────────────────────────────
    // For each regular city, accumulate shadow culture and claim the cheapest
    // unclaimed tile within radius 2–5 while the budget allows.
    // City-states use different expansion rules (TBD); they are skipped here.
    // TODO(PHASE3-BORDERS-CITYSTATE): implement city-state territory expansion.
    let city_indices: Vec<usize> = (0..state.cities.len()).collect();
    for city_idx in city_indices {
        {
            let city = &state.cities[city_idx];
            // Skip city-states — different expansion rules TBD.
            if matches!(city.kind, crate::civ::city::CityKind::CityState(_)) {
                continue;
            }
        }
        let (city_coord, civ_id, culture) = {
            let city = &state.cities[city_idx];
            (city.coord, city.owner, city_culture_output(&state.board, city))
        };
        state.cities[city_idx].culture_border += culture;

        loop {
            // Collect unclaimed candidates at radius 2–5 (re-evaluated each
            // iteration so that tiles claimed in this same turn are not re-selected).
            let candidates: Vec<(u32, HexCoord)> = state.board.all_coords()
                .into_iter()
                .filter(|&coord| {
                    let dist = city_coord.distance(&coord);
                    (2..=5).contains(&dist)
                        && state.board.tile(coord)
                            .map(|t| t.owner != Some(civ_id))
                            .unwrap_or(false)
                })
                .map(|coord| (tile_border_cost(city_coord.distance(&coord)), coord))
                .collect();

            let Some(&(min_cost, _)) = candidates.iter().min_by_key(|(c, _)| c) else {
                break;
            };
            if state.cities[city_idx].culture_border < min_cost {
                break;
            }

            let cheapest: Vec<HexCoord> = candidates.iter()
                .filter(|(c, _)| *c == min_cost)
                .map(|(_, coord)| *coord)
                .collect();

            let chosen = if cheapest.len() == 1 {
                cheapest[0]
            } else {
                let idx = (state.id_gen.next_f32() * cheapest.len() as f32) as usize;
                cheapest[idx.min(cheapest.len() - 1)]
            };

            state.cities[city_idx].culture_border -= min_cost;
            let city_id = state.cities[city_idx].id;
            try_claim_tile(state, civ_id, city_id, chosen, &mut diff);
        }
    }

    // ── Phase 3c: Loyalty pressure ──────────────────────────────────────────
    // For each non-city-state city, compute the loyalty delta from nearby
    // friendly/foreign cities, governor bonuses, occupation penalties, etc.
    // When loyalty reaches 0 the city revolts: it flips to the civ exerting
    // the highest foreign pressure, or becomes independent if none.
    {
        let num_cities = state.cities.len();
        // Compute deltas first (immutable borrow of cities slice).
        let loyalty_deltas: Vec<(usize, i32)> = (0..num_cities)
            .map(|i| (i, compute_city_loyalty_delta(i, &state.cities, &state.governors)))
            .filter(|(_, d)| *d != 0)
            .collect();

        // Apply loyalty changes.
        for &(city_idx, delta) in &loyalty_deltas {
            let city = &mut state.cities[city_idx];
            let old = city.loyalty;
            city.loyalty = (old + delta).clamp(0, 100);
            if city.loyalty != old {
                diff.push(StateDelta::LoyaltyChanged {
                    city: city.id,
                    delta: city.loyalty - old,
                    new_value: city.loyalty,
                });
            }
        }

        // Handle revolts (loyalty == 0). Collect revolt info first, then mutate.
        let revolts: Vec<(usize, CityId, CivId)> = (0..state.cities.len())
            .filter_map(|i| {
                let c = &state.cities[i];
                if c.loyalty == 0
                    && !matches!(c.kind, crate::civ::city::CityKind::CityState(_))
                {
                    Some((i, c.id, c.owner))
                } else {
                    None
                }
            })
            .collect();

        // Determine new owner for each revolting city.
        let revolt_targets: Vec<(usize, CityId, CivId, Option<CivId>)> = revolts.iter()
            .map(|&(idx, cid, old_owner)| {
                let new_owner = highest_pressure_civ(idx, &state.cities);
                (idx, cid, old_owner, new_owner)
            })
            .collect();

        for (city_idx, city_id, old_owner, new_owner) in revolt_targets {
            let city = &mut state.cities[city_idx];
            if let Some(new_civ) = new_owner {
                // Flip to the pressuring civilization.
                city.owner = new_civ;
                city.ownership = crate::civ::city::CityOwnership::Occupied;
                city.loyalty = 50; // starts at reduced loyalty under new owner

                // Update civ city lists.
                if let Some(old_civ) = state.civilizations.iter_mut().find(|c| c.id == old_owner) {
                    old_civ.cities.retain(|&id| id != city_id);
                }
                if let Some(new_civ_obj) = state.civilizations.iter_mut().find(|c| c.id == new_civ) {
                    new_civ_obj.cities.push(city_id);
                }

                // Update tile ownership for city's territory.
                let territory: Vec<HexCoord> = city.territory.iter().copied().collect();
                for coord in territory {
                    if let Some(tile) = state.board.tile_mut(coord) {
                        tile.owner = Some(new_civ);
                    }
                }

                diff.push(StateDelta::LoyaltyChanged {
                    city: city_id,
                    delta: 50,
                    new_value: 50,
                });
            } else {
                // No foreign pressure — city becomes independent (Free City).
                // Remove from old owner's city list; owner stays but ownership
                // is set to Occupied to signal the city is in revolt.
                // In a full implementation this would create a new "Free City"
                // civilization; for now we leave the owner and mark Occupied.
                city.ownership = crate::civ::city::CityOwnership::Occupied;
                city.loyalty = 25; // low loyalty as an independent

                diff.push(StateDelta::LoyaltyChanged {
                    city: city_id,
                    delta: 25,
                    new_value: 25,
                });
            }

            diff.push(StateDelta::CityRevolted {
                city: city_id,
                new_owner,
                old_owner,
            });
        }
    }

    // ── Phase 3d: Religious pressure ──────────────────────────────────────
    // Canonical Civ VI pressure: per source city with majority religion,
    // base +1, +2 if has Holy Site, +4 if is Holy City. Divided by distance.
    // Itinerant Preachers enhancer extends radius by 3 tiles.
    // Trade route pressure: +0.5 per active trade route between cities.
    {
        let num_cities = state.cities.len();
        let mut pressure_deltas: Vec<(usize, crate::ReligionId, i32)> = Vec::new();

        // Pre-compute: which religions have Itinerant Preachers.
        let itinerant_religions: std::collections::HashSet<crate::ReligionId> = state.religions.iter()
            .filter(|r| r.beliefs.iter().any(|bid|
                state.belief_defs.iter().any(|b| b.id == *bid && b.name == "Itinerant Preachers")))
            .map(|r| r.id)
            .collect();

        for target_idx in 0..num_cities {
            let target = &state.cities[target_idx];
            if matches!(target.kind, crate::civ::city::CityKind::CityState(_)) {
                continue;
            }
            let target_coord = target.coord;
            let target_id = target.id;
            let target_pop = target.population;

            let mut religion_pressure: std::collections::HashMap<crate::ReligionId, i32> =
                std::collections::HashMap::new();

            for source_idx in 0..num_cities {
                if source_idx == target_idx { continue; }
                let source = &state.cities[source_idx];
                let dist = source.coord.distance(&target_coord);
                if dist == 0 { continue; }

                // Determine majority religion of source city.
                let source_majority = source.majority_religion();
                let Some(majority_rid) = source_majority else { continue };

                // Itinerant Preachers extends radius by 3.
                let radius = if itinerant_religions.contains(&majority_rid) {
                    RELIGIOUS_PRESSURE_RADIUS + 3
                } else {
                    RELIGIOUS_PRESSURE_RADIUS
                };
                if dist > radius { continue; }

                // Canonical pressure formula:
                let mut pressure: i32 = 1; // Base: city has majority religion.

                // +2 if source city has Holy Site district.
                let has_holy_site = state.placed_districts.iter()
                    .any(|pd| pd.city_id == source.id
                        && pd.district_type == crate::civ::district::BuiltinDistrict::HolySite);
                if has_holy_site {
                    pressure += 2;
                }

                // +4 if source city is a Holy City.
                if state.religions.iter().any(|r| r.id == majority_rid && r.holy_city == source.id) {
                    pressure += 4;
                }

                // Scale by inverse distance.
                let scaled = (pressure * 10) / dist as i32;
                *religion_pressure.entry(majority_rid).or_insert(0) += scaled;
            }

            // Trade route pressure: +0.5 per active trade route to/from this city.
            for route in &state.trade_routes {
                let other_city_id = if route.origin == target_id {
                    Some(route.destination)
                } else if route.destination == target_id {
                    Some(route.origin)
                } else {
                    None
                };
                if let Some(other_id) = other_city_id
                    && let Some(other) = state.cities.iter().find(|c| c.id == other_id)
                    && let Some(rid) = other.majority_religion()
                {
                    // +0.5 per route, represented as +5 in our i32 pressure scale.
                    *religion_pressure.entry(rid).or_insert(0) += 5;
                }
            }

            // Convert pressure to follower change (scaled down).
            for (rid, pressure) in religion_pressure {
                if pressure <= 0 { continue; }
                let delta = (pressure / 50).max(1).min(target_pop as i32);
                pressure_deltas.push((target_idx, rid, delta));
            }
        }

        // Apply follower changes.
        for &(city_idx, religion_id, delta) in &pressure_deltas {
            let city = &mut state.cities[city_idx];
            let old_majority = city.majority_religion();
            *city.religious_followers.entry(religion_id).or_insert(0) += delta as u32;

            // Cap total followers at population.
            let total: u32 = city.religious_followers.values().sum();
            if total > city.population {
                let scale = city.population as f64 / total as f64;
                for v in city.religious_followers.values_mut() {
                    *v = (*v as f64 * scale).floor() as u32;
                }
            }

            diff.push(StateDelta::ReligiousPressureApplied {
                city: city.id,
                religion: religion_id,
                delta,
            });

            let new_majority = city.majority_religion();
            if new_majority != old_majority
                && let Some(new_rid) = new_majority
            {
                diff.push(StateDelta::CityConvertedReligion {
                    city: city.id,
                    old_religion: old_majority,
                    new_religion: new_rid,
                });
            }
        }
    }

    // ── Phase 3e: Faith yield accumulation ───────────────────────────────
    // Add faith from civ yields (building yields etc.) to civ's faith pool.
    for (civ_id, yields) in &civ_yields {
        if yields.faith > 0
            && let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id)
        {
            civ.faith += yields.faith as u32;
            diff.push(StateDelta::FaithChanged { civ: *civ_id, delta: yields.faith });
        }
    }

    // ── Phase 3c: Tourism computation + domestic culture accumulation ─────
    for (civ_id, yields) in &civ_yields {
        // Domestic culture defense: lifetime accumulation of culture output.
        if yields.culture > 0
            && let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id)
        {
            civ.domestic_culture += yields.culture as u32;
        }

        // Tourism output: sum of great work tourism + building base tourism.
        let mut tourism: u32 = 0;

        // Great works slotted in owned cities.
        for city in state.cities.iter().filter(|c| c.owner == *civ_id) {
            for slot in &city.great_work_slots {
                if let Some(work_id) = slot.work
                    && let Some(work) = state.great_works.iter().find(|w| w.id == work_id)
                {
                    tourism += work.tourism;
                }
            }
        }

        // Base tourism yields from buildings (e.g. Broadcast Center).
        tourism += yields.tourism.max(0) as u32;

        if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id) {
            civ.tourism_output = tourism;
        }
    }

    // ── Phase 4: Drain effect queue ───────────────────────────────────────
    // Take the queue out of state so apply_effect can borrow state mutably.
    // apply_effect returns () and never re-enqueues, so the loop terminates.
    let pending = std::mem::take(&mut state.effect_queue);
    for (civ_id, effect) in &pending {
        let should_apply = state.civilizations.iter()
            .find(|c| c.id == *civ_id)
            .map(|civ| effect.guard(civ))
            .unwrap_or(false);
        if should_apply {
            super::effects::apply_effect(state, *civ_id, effect, &mut diff);
        }
    }
    // Any effects pushed during apply_effect (none expected) would stay in
    // state.effect_queue for the next turn. pending is dropped here.

    // ── Phase 4b: Unique unit abilities — end-of-turn healing (Mamluk) ────
    for unit in &mut state.units {
        let civ_identity = state.civilizations.iter()
            .find(|c| c.id == unit.owner)
            .and_then(|c| c.civ_identity);
        if let Some(bundle) = lookup_bundle(civ_identity)
            && let Some(uu) = &bundle.unique_unit
        {
            let unit_type_name = state.unit_type_defs.iter()
                .find(|d| d.id == unit.unit_type)
                .map(|d| d.name);
            if unit_type_name == Some(uu.name)
                && uu.abilities.contains(&UniqueUnitAbility::HealEveryTurn)
                && unit.health < 100
            {
                let old_health = unit.health;
                unit.health = (unit.health + 10).min(100);
                diff.push(StateDelta::UnitHealed {
                    unit: unit.id,
                    old_health,
                    new_health: unit.health,
                });
            }
        }
    }

    // ── Phase 5: Diplomacy — grievance decay and status recomputation ─────
    // Decay each grievance by 1 per turn; drop records that reach zero.
    // Increment turns_at_war for warring pairs.
    // Recompute status from the combined opinion score and emit a delta when
    // it changes.
    const GRIEVANCE_DECAY: i32 = 1;
    let mut status_changes: Vec<(CivId, CivId, DiplomaticStatus)> = Vec::new();
    for rel in state.diplomatic_relations.iter_mut() {
        for rec in rel.grievances_a_against_b.iter_mut() {
            rec.amount -= GRIEVANCE_DECAY;
        }
        rel.grievances_a_against_b.retain(|r| r.amount > 0);
        for rec in rel.grievances_b_against_a.iter_mut() {
            rec.amount -= GRIEVANCE_DECAY;
        }
        rel.grievances_b_against_a.retain(|r| r.amount > 0);

        if rel.status == DiplomaticStatus::War {
            rel.turns_at_war += 1;
        }

        let new_status = compute_diplomatic_status(rel);
        if new_status != rel.status {
            status_changes.push((rel.civ_a, rel.civ_b, new_status));
        }
    }
    for (civ_a, civ_b, new_status) in status_changes {
        if let Some(rel) = state.diplomatic_relations.iter_mut()
            .find(|r| r.civ_a == civ_a && r.civ_b == civ_b)
        {
            rel.status = new_status;
        }
        diff.push(StateDelta::DiplomacyChanged { civ_a, civ_b, new_status });
    }

    // TODO(PHASE3-8.3): Deliver trade route yields; decrement turns_remaining; expire routes.
    // TODO(PHASE3-8.5): Compute religion spread per city pair; update Religion.followers.

    // ── Phase 5a-2: Governor establishment countdown ─────────────────────
    for gov in &mut state.governors {
        if let Some(city) = gov.assigned_city
            && gov.turns_to_establish > 0
        {
            gov.turns_to_establish -= 1;
            if gov.turns_to_establish == 0 {
                diff.push(StateDelta::GovernorEstablished {
                    governor: gov.id,
                    city,
                });
            }
        }
    }

    // ── Phase 5a: Great person point accumulation ────────────────────────────
    // Each district generates GP points per turn for its associated type(s).
    // When accumulated points reach the recruitment threshold, the next
    // available candidate is automatically recruited.
    {
        use crate::civ::great_people::{
            district_great_person_types, recruitment_threshold, next_candidate_name,
            spawn_great_person, GP_BASE_POINTS_PER_DISTRICT,
        };

        // Collect per-civ GP point increments (immutable pass over cities/districts).
        let mut civ_gp_increments: Vec<(CivId, std::collections::HashMap<crate::GreatPersonType, u32>)> = Vec::new();
        for civ in &state.civilizations {
            let mut increments: std::collections::HashMap<crate::GreatPersonType, u32> = std::collections::HashMap::new();
            for city in state.cities.iter().filter(|c| c.owner == civ.id) {
                for district in &city.districts {
                    for &gp_type in district_great_person_types(*district) {
                        *increments.entry(gp_type).or_insert(0) += GP_BASE_POINTS_PER_DISTRICT;
                    }
                }
            }
            if !increments.is_empty() {
                civ_gp_increments.push((civ.id, increments));
            }
        }

        // Add modifier-derived GP point bonus (from policies, buildings, wonders)
        // to each GP type the civ is actively generating points for.
        for (civ_id, increments) in &mut civ_gp_increments {
            let bonus = civ_yields.iter()
                .find(|(id, _)| id == civ_id)
                .map(|(_, y)| y.great_person_points.max(0) as u32)
                .unwrap_or(0);
            if bonus > 0 {
                for pts in increments.values_mut() {
                    *pts += bonus;
                }
            }
        }

        // Apply increments (mutable pass).
        for (civ_id, increments) in &civ_gp_increments {
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id) {
                for (&gp_type, &points) in increments {
                    let total = civ.great_person_points.entry(gp_type).or_insert(0);
                    *total += points;
                    diff.push(StateDelta::GreatPersonPointsAccumulated {
                        civ: *civ_id,
                        person_type: gp_type,
                        points,
                        total: *total,
                    });
                }
            }
        }

        // Check thresholds and auto-recruit.
        // Collect recruitment actions first (need immutable state for lookups).
        struct GpRecruit { civ_id: CivId, gp_type: crate::GreatPersonType, def_name: &'static str, threshold: u32 }
        let mut recruits: Vec<GpRecruit> = Vec::new();

        for (civ_id, increments) in &civ_gp_increments {
            for &gp_type in increments.keys() {
                let current_points = state.civilizations.iter()
                    .find(|c| c.id == *civ_id)
                    .and_then(|c| c.great_person_points.get(&gp_type).copied())
                    .unwrap_or(0);
                let threshold = recruitment_threshold(gp_type, state);
                if current_points >= threshold
                    && let Some(name) = next_candidate_name(gp_type, state)
                {
                    recruits.push(GpRecruit { civ_id: *civ_id, gp_type, def_name: name, threshold });
                }
            }
        }

        // Execute recruitments (mutable state).
        for recruit in recruits {
            // Find the civ's capital coord for spawning.
            let spawn_coord = state.cities.iter()
                .find(|c| c.owner == recruit.civ_id && c.is_capital)
                .map(|c| c.coord)
                .unwrap_or_else(|| {
                    // Fallback: first owned city.
                    state.cities.iter()
                        .find(|c| c.owner == recruit.civ_id)
                        .map(|c| c.coord)
                        .unwrap_or(HexCoord::from_qr(0, 0))
                });

            let gp_id = spawn_great_person(state, recruit.civ_id, recruit.def_name, spawn_coord);

            // Subtract threshold from accumulated points.
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == recruit.civ_id)
                && let Some(pts) = civ.great_person_points.get_mut(&recruit.gp_type)
            {
                *pts = pts.saturating_sub(recruit.threshold);
            }

            diff.push(StateDelta::GreatPersonRecruited {
                great_person: gp_id,
                civ: recruit.civ_id,
                person_type: recruit.gp_type,
            });
        }
    }

    // ── Phase 5b-0: Visibility refresh ──────────────────────────────────────
    // Recalculate visibility for all civs so that any newly explored natural
    // wonders emit NaturalWonderDiscovered deltas before the era score observer.
    {
        let civ_ids: Vec<_> = state.civilizations.iter().map(|c| c.id).collect();
        for cid in civ_ids {
            let vis_diff = crate::game::recalculate_visibility(state, cid);
            for delta in vis_diff.deltas {
                diff.push(delta);
            }
        }
    }

    // ── Phase 5b-1: Era score observer ──────────────────────────────────────
    // Scan deltas produced so far and award era score for matching historic moments.
    {
        use crate::civ::historic_moments::observe_deltas;
        use crate::civ::era::{HistoricMoment, compute_era_age, should_advance_era};

        let moments = observe_deltas(&diff.deltas, state);
        for (civ_id, moment_def) in moments {
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
                civ.era_score += moment_def.era_score;
                civ.historic_moments.push(HistoricMoment {
                    civ: civ_id,
                    moment_name: moment_def.name,
                    era_score: moment_def.era_score,
                    turn: state.turn,
                    era: civ.current_era,
                });
                if moment_def.unique {
                    civ.earned_moments.insert(moment_def.name);
                }
                diff.push(StateDelta::HistoricMomentEarned {
                    civ: civ_id,
                    moment: moment_def.name,
                    era_score: moment_def.era_score,
                });
            }
        }

        // ── Phase 5b-2: Era advancement check ───────────────────────────────
        if !state.eras.is_empty() && state.current_era_index < state.eras.len() {
            let should_advance = should_advance_era(
                &state.eras[state.current_era_index],
                &state.civilizations,
            );
            if should_advance && state.current_era_index + 1 < state.eras.len() {
                state.current_era_index += 1;
                let new_era = &state.eras[state.current_era_index];
                let new_age_type = new_era.age;

                for civ in &mut state.civilizations {
                    let was_dark = civ.era_age == crate::civ::era::EraAge::Dark;
                    let new_era_age = compute_era_age(civ.era_score, was_dark);
                    civ.era_age = new_era_age;
                    civ.current_era = new_age_type;
                    civ.era_score = 0;
                    civ.earned_moments.clear();

                    diff.push(StateDelta::EraAdvanced {
                        civ: civ.id,
                        new_era: new_age_type,
                        era_age: new_era_age,
                    });
                }
            }
        }
    }

    // ── Phase 5b-3: Diplomatic favor accumulation ─────────────────────────
    // Each civ gains: +1 base, +1 per suzerained city-state, +1 if not at war.
    {
        use crate::civ::city::CityKind;

        let civ_ids_for_favor: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();
        for &civ_id in &civ_ids_for_favor {
            let mut favor: i32 = 1; // base

            // +1 per suzerained city-state
            for city in &state.cities {
                if let CityKind::CityState(ref cs_data) = city.kind
                    && cs_data.suzerain == Some(civ_id)
                {
                    favor += 1;
                }
            }

            // +1 if not at war with anyone
            let at_war = state.diplomatic_relations.iter().any(|rel| {
                (rel.civ_a == civ_id || rel.civ_b == civ_id) && rel.status == DiplomaticStatus::War
            });
            if !at_war {
                favor += 1;
            }

            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
                civ.diplomatic_favor = civ.diplomatic_favor.saturating_add(favor as u32);
            }
            diff.push(StateDelta::DiplomaticFavorChanged { civ: civ_id, delta: favor });
        }
    }

    // ── Phase 5c-0: World Congress session ─────────────────────────────────
    // If the current turn reaches the next scheduled session, hold a session.
    // The civ with the most diplomatic_favor wins and earns +1 Diplomatic VP.
    if state.turn >= state.world_congress.next_session_turn
        && !state.civilizations.is_empty()
    {
        // Schedule next session.
        state.world_congress.next_session_turn += state.world_congress.session_interval;

        // Find the civ with the most diplomatic favor (skip barbarian civ).
        let winner = state.civilizations.iter()
            .filter(|c| Some(c.id) != state.barbarian_civ)
            .max_by_key(|c| c.diplomatic_favor)
            .map(|c| c.id);

        if let Some(winner_id) = winner {
            *state.world_congress.diplomatic_victory_points
                .entry(winner_id).or_insert(0) += 1;
            diff.push(StateDelta::CongressSessionHeld { winner: winner_id });
            diff.push(StateDelta::DiplomaticVPEarned { civ: winner_id, points: 1 });
        }
    }

    // ── Phase 5c: Victory condition evaluation ─────────────────────────────
    if state.game_over.is_none() {
        use super::super::victory::{GameOver, VictoryKind};
        use super::super::score::all_scores;

        let civ_ids: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();

        // Check ImmediateWin conditions every turn; first match wins.
        'immediate: for vc_idx in 0..state.victory_conditions.len() {
            if matches!(state.victory_conditions[vc_idx].kind(), VictoryKind::ImmediateWin) {
                for &civ_id in &civ_ids {
                    let progress = state.victory_conditions[vc_idx].check_progress(civ_id, state);
                    if progress.is_won() {
                        let name = state.victory_conditions[vc_idx].name();
                        state.game_over = Some(GameOver { winner: civ_id, condition: name, turn: state.turn });
                        diff.push(StateDelta::VictoryAchieved { civ: civ_id, condition: name });
                        break 'immediate;
                    }
                }
            }
        }

        // Check TurnLimit conditions when the turn limit is reached.
        // Compare `state.turn + 1` because Phase 5c runs before the turn counter
        // is incremented; `turn + 1` is the turn that will be completed.
        if state.game_over.is_none() {
            for vc_idx in 0..state.victory_conditions.len() {
                if let VictoryKind::TurnLimit { turn_limit } = state.victory_conditions[vc_idx].kind()
                    && state.turn + 1 >= turn_limit
                {
                    let name = state.victory_conditions[vc_idx].name();
                    let completed_turn = state.turn + 1;
                    if let Some((winner, _)) = all_scores(state).into_iter().next() {
                        state.game_over = Some(GameOver { winner, condition: name, turn: completed_turn });
                        diff.push(StateDelta::VictoryAchieved { civ: winner, condition: name });
                    }
                    break;
                }
            }
        }
    }

    // ── Barbarian phase ────────────────────────────────────────────────────
    super::barbarians::process_barbarian_turn(state, &mut diff);

    // ── Advance turn counter ──────────────────────────────────────────────
    let prev = state.turn;
    state.turn += 1;
    diff.push(StateDelta::TurnAdvanced { from: prev, to: state.turn });

    diff
}
