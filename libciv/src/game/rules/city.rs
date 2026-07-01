//! City handlers: `found_city`, `claim_tile`, `reassign_tile`, `assign_citizen`, `compute_yields`.

use std::collections::HashSet;
use crate::{BeliefId, BuildingId, CityId, CivId, UnitId, WonderId, YieldBundle};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::MovementCost;

use super::{RulesError, lookup_bundle};
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::rules_helpers::{
    apply_effects, tile_yields_gated, try_claim_tile,
};
use super::super::state::GameState;
use crate::rules::modifier::{ConditionContext, resolve_modifiers};

/// Consume a settler unit and found a new city at its current position.
pub(crate) fn found_city(
    state:   &mut GameState,
    settler: UnitId,
    name:    String,
) -> Result<GameStateDiff, RulesError> {
    let (coord, civ_id, unit_type_id) = {
        let u = state.unit(settler).ok_or(RulesError::UnitNotFound)?;
        (u.coord, u.owner, u.unit_type)
    };

    let is_settler = state.unit_type_defs.iter()
        .any(|d| d.id == unit_type_id && d.can_found_city);
    if !is_settler { return Err(RulesError::NotASettler); }

    let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
    if !tile.terrain.is_land() {
        return Err(RulesError::InvalidFoundingTerrain);
    }
    if tile.terrain.movement_cost() == MovementCost::Impassable {
        return Err(RulesError::InvalidFoundingTerrain);
    }

    if state.cities.iter().any(|c| c.coord == coord) {
        return Err(RulesError::TileOccupied);
    }
    if state.cities.iter().any(|c| c.coord.distance(&coord) <= 3) {
        return Err(RulesError::TooCloseToCity);
    }

    let city_id = state.id_gen.next_city_id();
    let is_capital = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .is_none_or(|c| c.cities.is_empty());
    let mut city = crate::civ::City::new(city_id, name, civ_id, coord);
    city.is_capital = is_capital;

    if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
        civ.cities.push(city_id);
    }
    state.cities.push(city);
    state.units.retain(|u| u.id != settler);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitDestroyed { unit: settler });
    diff.push(StateDelta::CityFounded { city: city_id, coord, owner: civ_id });

    try_claim_tile(state, civ_id, city_id, coord, &mut diff);
    for nb in state.board.neighbors(coord) {
        try_claim_tile(state, civ_id, city_id, nb, &mut diff);
    }

    // ── Civ ability: on_city_founded hooks ──────────────────────────────
    let civ_identity = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .and_then(|c| c.civ_identity);
    if let Some(bundle) = lookup_bundle(civ_identity) {
        use crate::civ::civ_ability::CityFoundedHook;
        for hook in &bundle.on_city_founded {
            match hook {
                CityFoundedHook::FreeBuilding(building_name) => {
                    if let Some(bdef) = state.building_defs.iter()
                        .find(|d| d.name == *building_name)
                        && let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id)
                        && !city.buildings.contains(&bdef.id)
                    {
                        let bid = bdef.id;
                        city.buildings.push(bid);
                        diff.push(StateDelta::BuildingCompleted {
                            city: city_id, building: building_name,
                        });
                    }
                }
                CityFoundedHook::FreeTradingPost => {
                    if let Some(tile) = state.board.tile_mut(coord)
                        && tile.improvement.is_none()
                    {
                        tile.improvement = Some(
                            crate::world::improvement::BuiltinImprovement::TradingPost,
                        );
                        diff.push(StateDelta::ImprovementPlaced {
                            coord,
                            improvement: crate::world::improvement::BuiltinImprovement::TradingPost,
                        });
                    }
                }
                CityFoundedHook::RoadToCapital => {
                    // Stub: road building requires pathfinding infrastructure.
                    // TODO: build road along shortest path to capital.
                }
            }
        }
    }

    Ok(diff)
}

/// Claim `coord` for the civilization that owns `city_id`.
pub(crate) fn claim_tile(
    state: &mut GameState,
    city_id: CityId,
    coord: HexCoord,
    force: bool,
) -> Result<GameStateDiff, RulesError> {
    let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

    let (city_coord, civ_id) = state.cities.iter()
        .find(|c| c.id == city_id)
        .map(|c| (c.coord, c.owner))
        .ok_or(RulesError::CityNotFound)?;

    let dist = city_coord.distance(&coord);
    if !(1..=3).contains(&dist) {
        return Err(RulesError::TileNotInCityRange);
    }

    let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;

    match tile.owner {
        Some(owner) if owner == civ_id => {
            return Ok(GameStateDiff::new());
        }
        Some(_) if !force => return Err(RulesError::TileOwnedByEnemy),
        Some(_) | None => {}
    }

    if let Some(t) = state.board.tile_mut(coord) {
        t.owner = Some(civ_id);
    }
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        city.territory.insert(coord);
    }
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TileClaimed { civ: civ_id, city: city_id, coord });
    Ok(diff)
}

/// Reassign `coord` from one city to another within the same civilization.
pub(crate) fn reassign_tile(
    state: &mut GameState,
    from_city: CityId,
    to_city: CityId,
    coord: HexCoord,
) -> Result<GameStateDiff, RulesError> {
    let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

    let from_civ = state.cities.iter()
        .find(|c| c.id == from_city)
        .map(|c| c.owner)
        .ok_or(RulesError::CityNotFound)?;

    let (to_coord, to_civ) = state.cities.iter()
        .find(|c| c.id == to_city)
        .map(|c| (c.coord, c.owner))
        .ok_or(RulesError::CityNotFound)?;

    if from_civ != to_civ {
        return Err(RulesError::CitiesNotSameCiv);
    }
    let civ_id = from_civ;

    if from_city == to_city {
        return Ok(GameStateDiff::new());
    }

    let owner = state.board.tile(coord)
        .ok_or(RulesError::InvalidCoord)?
        .owner;
    if owner != Some(civ_id) {
        return Err(RulesError::TileNotOwned);
    }

    let to_dist = to_coord.distance(&coord);
    if !(1..=3).contains(&to_dist) {
        return Err(RulesError::TileNotInCityRange);
    }

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TileReassigned { civ: civ_id, from_city, to_city, coord });
    Ok(diff)
}

/// Assign a citizen to work `tile` in `city`.
pub(crate) fn assign_citizen(
    state: &mut GameState,
    city_id: CityId,
    tile: HexCoord,
    lock: bool,
) -> Result<GameStateDiff, RulesError> {
    let city_idx = state.cities.iter().position(|c| c.id == city_id)
        .ok_or(RulesError::CityNotFound)?;

    let tile = state.board.normalize(tile).ok_or(RulesError::InvalidCoord)?;

    if state.board.tile(tile).is_none() {
        return Err(RulesError::InvalidCoord);
    }

    if state.cities[city_idx].coord.distance(&tile) > 3 {
        return Err(RulesError::InvalidCoord);
    }

    let mut diff = GameStateDiff::new();
    let city = &mut state.cities[city_idx];

    if !city.worked_tiles.contains(&tile) {
        city.worked_tiles.push(tile);
        diff.push(StateDelta::CitizenAssigned { city: city_id, tile });
    }
    if lock {
        city.locked_tiles.insert(tile);
    }

    Ok(diff)
}

// ── Re-entrancy guard ─────────────────────────────────────────────────────────
//
// `compute_yields` is a single-pass accumulation: it snapshots a fixed base of
// yields, then resolves modifiers over that frozen base. Its invariant is that
// it MUST NOT be re-entered on the same thread — neither directly nor indirectly
// through a `Condition` that tries to trigger another yield computation. Any
// such cross-entity condition (per-wonder counts, city-state suzerain, etc.)
// must read COUNTS/state, never recurse into yield calculation.
//
// The thread-local flag makes that invariant explicit and *enforced*: a nested
// entry trips the debug assertion (catching the bug in tests) and, in release
// builds, is prevented from recursing by returning an empty bundle instead of
// diverging.
thread_local! {
    static COMPUTING_YIELDS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// RAII guard that marks a `compute_yields` computation as in-flight for the
/// current thread and clears the marker on drop (even on panic/early return).
struct YieldReentrancyGuard {
    /// `true` when this guard detected an existing in-flight computation, i.e. a
    /// re-entrant call. The caller uses this to bail out without recursing.
    reentered: bool,
}

impl YieldReentrancyGuard {
    fn enter() -> Self {
        let already = COMPUTING_YIELDS.with(|f| f.replace(true));
        debug_assert!(
            !already,
            "compute_yields must not be re-entered (recursion / yield feedback detected)"
        );
        Self { reentered: already }
    }
}

impl Drop for YieldReentrancyGuard {
    fn drop(&mut self) {
        // Only the outermost (non-re-entrant) guard clears the flag, so a
        // detected re-entrant call cannot prematurely release it.
        if !self.reentered {
            COMPUTING_YIELDS.with(|f| f.set(false));
        }
    }
}

/// Compute all yields for a civilization this turn.
///
/// # Safety properties
///
/// This function is hardened against three classes of failure:
///
/// * **Re-entrancy / feedback** — a thread-local guard enforces that
///   `compute_yields` never calls itself (directly or via a `Condition`).
///   Modifiers are resolved over a *frozen* base captured before any modifier is
///   applied, so a modifier can never feed back into its own inputs and the
///   resolve pass cannot diverge.
/// * **Unbounded iteration** — every contributor is enumerated from finite game
///   state exactly once; there are no growing worklists or per-application
///   scalers.
/// * **Double-counting** — each contributing element (worked tile, building,
///   wonder, belief) is entered into a de-duplicated worklist keyed by a stable
///   identity (tile coord, `(city, building)`, wonder id, `(city, belief)`) and
///   therefore contributes EXACTLY once.
pub(crate) fn compute_yields(state: &GameState, civ_id: CivId) -> YieldBundle {
    // Enforce the no-recursion invariant. If we are somehow re-entered, bail out
    // with an empty bundle rather than recurse (release-build safety net; the
    // debug assertion fires in tests).
    let guard = YieldReentrancyGuard::enter();
    if guard.reentered {
        return YieldBundle::default();
    }

    let known_techs: HashSet<&str> = state.civ(civ_id)
        .map(|civ| {
            state.tech_tree.nodes.values()
                .filter(|n| civ.researched_techs.contains(&n.id))
                .map(|n| n.name)
                .collect()
        })
        .unwrap_or_default();

    // ── Phase 1: accumulate the FIXED base over a de-duplicated worklist ──
    //
    // De-dup keys guarantee single-counting:
    //   * worked tiles  → by coord, civ-wide (two cities working the same tile
    //                     contribute it once);
    //   * buildings     → by (city, building) (a building listed twice in one
    //                     city's vec contributes once; the same building type in
    //                     two cities still counts in each, which is correct).
    let mut total = YieldBundle::default();
    let mut seen_tiles:     HashSet<HexCoord>            = HashSet::new();
    let mut seen_buildings: HashSet<(CityId, BuildingId)> = HashSet::new();

    for city in state.cities.iter().filter(|c| c.owner == civ_id) {
        for &coord in &city.worked_tiles {
            if seen_tiles.insert(coord)
                && let Some(tile) = state.board.tile(coord)
            {
                total += tile_yields_gated(tile, &known_techs);
            }
        }

        // Building yields (static per-building yields).
        for &bid in &city.buildings {
            if seen_buildings.insert((city.id, bid))
                && let Some(bdef) = state.building_defs.iter().find(|d| d.id == bid)
            {
                total += bdef.yields.clone();
            }
        }
    }

    let city_count = state.cities.iter().filter(|c| c.owner == civ_id).count();
    total.science += city_count as i32;
    total.culture += city_count as i32;

    // ── Trade route yields ────────────────────────────────────────────────
    // Each route is a distinct entity; origin and destination payoffs are
    // disjoint (a route pays its owner OR the foreign destination, never both).
    for route in &state.trade_routes {
        if route.owner == civ_id {
            total += route.origin_yields.clone();
        }
    }
    for route in &state.trade_routes {
        let dest_owner = state.cities.iter()
            .find(|c| c.id == route.destination)
            .map(|c| c.owner);
        if dest_owner == Some(civ_id) && route.owner != civ_id {
            total += route.destination_yields.clone();
        }
    }

    // ── Phase 2: collect the FIXED modifier set (also de-duplicated) ──────
    let modifiers = {
        let mut mods = state.civ(civ_id)
            .map(|civ| {
                let mut m = civ.get_modifiers(
                    &state.policies,
                    &state.governments,
                    &state.diplomatic_relations,
                );
                m.extend(civ.get_tree_modifiers(&state.tech_tree, &state.civic_tree));
                if let Some(bundle) = lookup_bundle(civ.civ_identity) {
                    m.extend(bundle.civ_modifiers);
                    m.extend(bundle.leader_modifiers);
                }
                m
            })
            .unwrap_or_default();
        for gov in &state.governors {
            if gov.owner == civ_id && gov.is_established() && gov.assigned_city.is_some() {
                mods.extend(crate::civ::governor::get_governor_modifiers(gov));
            }
        }

        // ── Religion belief yields ────────────────────────────────────────
        // Route each city's majority-religion belief modifiers into the
        // resolved set. A belief applies once per city that follows the
        // religion (belief bonuses intentionally scale with city count), but a
        // belief listed twice for the SAME city contributes only once.
        let mut seen_beliefs: HashSet<(CityId, BeliefId)> = HashSet::new();
        for city in state.cities.iter().filter(|c| c.owner == civ_id) {
            if let Some(rid) = city.majority_religion()
                && let Some(religion) = state.religions.iter().find(|r| r.id == rid)
            {
                for belief_id in &religion.beliefs {
                    if seen_beliefs.insert((city.id, *belief_id))
                        && let Some(bdef) = state.belief_defs.iter().find(|b| b.id == *belief_id)
                    {
                        mods.extend(bdef.modifiers.iter().cloned());
                    }
                }
            }
        }

        // ── Completed wonder effects ──────────────────────────────────────
        // Fold each completed wonder's persistent modifiers into the set,
        // de-duplicated by wonder id (wonders are globally unique, so a given
        // wonder's effects are folded exactly once even if it appears more than
        // once across the civ's city wonder lists).
        let mut seen_wonders: HashSet<WonderId> = HashSet::new();
        for city in state.cities.iter().filter(|c| c.owner == civ_id) {
            for &wid in &city.wonders {
                if seen_wonders.insert(wid)
                    && let Some(wdef) = state.wonder_defs.iter().find(|w| w.id == wid)
                {
                    mods.extend(wdef.effects.iter().cloned());
                }
            }
        }

        // ── Era-age consequences ──────────────────────────────────────────
        // A Golden/Heroic age grants bonus yields; a Dark age imposes
        // penalties. Read the civ's current era age set by the era phase.
        if let Some(civ) = state.civ(civ_id) {
            mods.extend(crate::civ::era::era_age_modifiers(civ.era_age));
        }

        // ── City-state suzerain & envoy payoffs ───────────────────────────
        // A civ that is suzerain of a city-state — or has envoys invested in
        // it — receives that city-state's defined modifiers. City-state city
        // names match their `CityStateDef` entries, so look up the def by name
        // and fold the earned tiers (cumulative at 1/3/6 envoys) plus the
        // suzerain bonus into the resolved set.
        {
            let cs_defs = crate::civ::builtin_city_state_defs();
            for city in &state.cities {
                if let crate::civ::city::CityKind::CityState(cs_data) = &city.kind
                    && let Some(def) = cs_defs.iter().find(|d| d.name == city.name)
                {
                    let envoys = cs_data.get_influence(civ_id);
                    if envoys >= 1 {
                        mods.extend(def.envoy_1_modifiers.iter().cloned());
                    }
                    if envoys >= 3 {
                        mods.extend(def.envoy_3_modifiers.iter().cloned());
                    }
                    if envoys >= 6 {
                        mods.extend(def.envoy_6_modifiers.iter().cloned());
                    }
                    if cs_data.suzerain == Some(civ_id) {
                        mods.extend(def.suzerain_modifiers.iter().cloned());
                    }
                }
            }
        }

        mods
    };

    // ── Phase 3: single resolve pass over the frozen base ─────────────────
    //
    // `ConditionContext` exposes only game state (civ, cities, board, counts) —
    // it has no handle on the `total` accumulator being built, so a `Condition`
    // physically cannot read the yield it is contributing to. Combined with
    // `apply_effects` applying flats-then-percents once over the fixed `total`,
    // this makes yield resolution non-recursive and convergent by construction.
    let ctx = ConditionContext::for_civ(civ_id, state);
    let effects = resolve_modifiers(&modifiers, Some(&ctx));
    apply_effects(&effects, total)
}

#[cfg(test)]
mod reentrancy_tests {
    use super::{COMPUTING_YIELDS, YieldReentrancyGuard};

    #[test]
    fn guard_sets_and_clears_the_in_flight_flag() {
        COMPUTING_YIELDS.with(|f| assert!(!f.get(), "flag must start clear"));
        {
            let g = YieldReentrancyGuard::enter();
            assert!(!g.reentered, "first entry is not a re-entry");
            COMPUTING_YIELDS.with(|f| assert!(f.get(), "flag set while computation in flight"));
        }
        COMPUTING_YIELDS.with(|f| assert!(!f.get(), "flag cleared on guard drop"));
    }

    #[test]
    fn nested_guard_reports_reentry_and_preserves_flag() {
        let _outer = YieldReentrancyGuard::enter();
        // A nested guard is what a Condition that (wrongly) recursed into
        // compute_yields would create. In release builds `enter()` reports the
        // re-entry (so the caller returns early instead of diverging) without
        // clearing the outer guard's flag.
        #[cfg(not(debug_assertions))]
        {
            let inner = YieldReentrancyGuard::enter();
            assert!(inner.reentered, "nested entry must be reported as a re-entry");
            drop(inner);
            COMPUTING_YIELDS.with(|f| assert!(f.get(), "inner drop must not clear outer flag"));
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "must not be re-entered")]
    fn nested_guard_trips_debug_assertion() {
        let _outer = YieldReentrancyGuard::enter();
        // Simulated recursion: the debug assertion fires, catching the bug.
        let _inner = YieldReentrancyGuard::enter();
    }
}
