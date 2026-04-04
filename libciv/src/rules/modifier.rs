use crate::{CivId, PolicyType, ResourceCategory, UnitDomain, YieldType};
use crate::civ::district::BuiltinDistrict;
use crate::world::feature::BuiltinFeature;
use crate::world::improvement::BuiltinImprovement;
use crate::world::terrain::BuiltinTerrain;

// ── Effect types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    /// Flat bonus to a yield.
    YieldFlat(YieldType, i32),
    /// Percentage bonus to a yield (scaled by 100, e.g. 50 = +50%).
    YieldPercent(YieldType, i32),
    /// Combat strength modifier (flat).
    CombatStrengthFlat(i32),
    /// Combat strength modifier (percent).
    CombatStrengthPercent(i32),
    /// Movement bonus (additional movement points).
    MovementBonus(u32),
    /// Flat housing bonus.
    HousingFlat(i32),
    /// Flat amenity bonus.
    AmenityFlat(i32),
    /// Production speed modifier (percent, e.g. 15 = +15% faster).
    ProductionPercent(i32),
    /// Grant an extra policy slot of the given type.
    ExtraPolicySlot(PolicyType),
    /// Grant extra district slots beyond population limit.
    ExtraDistrictSlot(i32),
    /// Build time modifier for specific districts (percent, -50 = half time).
    BuildTimePercent(i32),
    /// Unit heals at end of every turn regardless of action.
    HealEveryTurn,
    /// Modify trade route yields (flat).
    TradeRouteYieldFlat(YieldType, i32),
    /// Worship building purchase cost modifier (percent, -90 = 90% cheaper).
    WorshipBuildingCostPercent(i32),
}

// ── Target selectors ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetSelector {
    /// Applies to all tiles.
    AllTiles,
    /// Applies to all units.
    AllUnits,
    /// Applies to units of a specific domain.
    UnitDomain(UnitDomain),
    /// Applies to a specific civilization.
    Civilization(CivId),
    /// Applies globally.
    Global,
    /// Applies to a specific unit type by name.
    UnitType(&'static str),
    /// Applies to adjacent enemy units (debuff).
    AdjacentEnemyUnits,
    /// Applies to trade routes owned by this civ.
    TradeRoutesOwned,
    /// Applies to the production queue.
    ProductionQueue,
    /// Applies to a specific district type's adjacency calculation.
    DistrictAdjacency(BuiltinDistrict),
}

// ── Stacking rules ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StackingRule {
    /// All modifiers of this type are summed.
    Additive,
    /// Only the highest value applies.
    Max,
    /// The most recently applied value replaces all previous.
    Replace,
}

// ── Modifier sources ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModifierSource {
    Tech(&'static str),
    Civic(&'static str),
    Policy(&'static str),
    Building(&'static str),
    Wonder(&'static str),
    Leader(&'static str),
    Religion(&'static str),
    Era(&'static str),
    Custom(&'static str),
    /// Source is a civilization's innate ability.
    CivAbility(&'static str),
    /// Source is a governor promotion or base ability.
    Governor(&'static str),
}

// ── Conditions ───────────────────────────────────────────────────────────────

/// A predicate evaluated at modifier-resolution time. Determines whether a
/// modifier applies and optionally scales its effect by a count.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Condition {
    // ── Tile/placement conditions ────────────────────────────────────────
    /// Tile or city is adjacent to a river.
    AdjacentToRiver,
    /// Tile is on hills terrain.
    OnHills,
    /// Tile is coastal or unit is on a coast tile.
    OnCoast,

    // ── Tile-level conditions (for per-tile yield modifiers) ─────────────
    /// Tile has the given improvement built on it.
    TileHasImprovement(BuiltinImprovement),
    /// Tile has any improvement built on it (resource is "improved").
    TileHasAnyImprovement,
    /// Tile has a resource of the given category (Bonus, Luxury, Strategic).
    TileHasResourceOfCategory(ResourceCategory),
    /// Tile has the given feature (Marsh, Oasis, Floodplain, etc.).
    TileHasFeature(BuiltinFeature),
    /// Tile has appeal >= the given threshold (Charming = 2, Breathtaking = 4).
    TileMinAppeal(i32),

    // ── Production-queue conditions ─────────────────────────────────────
    /// City is producing a military unit (Combat/Support) of the given era.
    ProducingMilitaryUnitOfEra(crate::AgeType),
    /// City is producing a wonder of the given era.
    ProducingWonderOfEra(crate::AgeType),

    // ── Adjacency-counting conditions (scale by count of matching neighbors) ──
    /// Multiply by number of adjacent tiles with the given terrain type.
    /// Does NOT count the tile the district sits on, only the 6 neighbors.
    PerAdjacentTerrain(BuiltinTerrain),
    /// Multiply by number of adjacent tiles with the given feature.
    /// Does NOT count the tile the district sits on, only the 6 neighbors.
    PerAdjacentFeature(BuiltinFeature),

    // ── Scaling conditions (effect multiplied by count) ──────────────────
    /// Multiply by number of city-states where this civ is suzerain.
    PerCityStateSuzerain,
    /// Multiply by number of adjacent districts.
    PerAdjacentDistrict,
    /// Multiply by number of trading posts in the trade route.
    PerTradingPostInRoute,
    /// Multiply by number of foreign cities with this civ's worship building.
    PerForeignCityWithWorshipBuilding,
    /// Multiply by number of met civs that founded a religion and are not at war with us.
    PerCivMetWithReligionNotAtWar,

    // ── Game-state conditions ────────────────────────────────────────────
    /// Civ is currently at war with any other civ.
    AtWar,
    /// Civ is not at war with any other civ.
    NotAtWar,
    /// Unit's health is below max.
    UnitDamaged,
    /// Unit is adjacent to another unit of the same type owned by the same civ.
    AdjacentToSameUnitType,
    /// Unit is adjacent to an enemy unit.
    AdjacentToEnemy,
    /// Target of combat is a city-state.
    TargetIsCityState,
    /// City is currently producing a district or wonder.
    ProducingDistrictOrWonder,
    /// City has a worship building.
    CityHasWorshipBuilding,

    // ── Composite ────────────────────────────────────────────────────────
    /// Both conditions must hold (conjunction).
    And(Box<Condition>, Box<Condition>),
    /// At least one condition must hold (disjunction).
    Or(Box<Condition>, Box<Condition>),
}

/// Result of evaluating a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionResult {
    /// Condition passes — modifier applies with its base value.
    Pass,
    /// Condition fails — modifier does not apply.
    Fail,
    /// Condition passes and scales — multiply effect value by this factor.
    Scale(i32),
}

/// Context for evaluating conditions against game state.
pub struct ConditionContext<'a> {
    pub civ_id: CivId,
    pub state: &'a crate::game::state::GameState,
    // Optional context narrowing:
    pub tile: Option<libhexgrid::coord::HexCoord>,
    pub unit_id: Option<crate::UnitId>,
    pub city_id: Option<crate::CityId>,
}

impl<'a> ConditionContext<'a> {
    pub fn for_civ(civ_id: CivId, state: &'a crate::game::state::GameState) -> Self {
        Self { civ_id, state, tile: None, unit_id: None, city_id: None }
    }
}

/// Compute the appeal of a tile based on adjacent terrain, features, and districts.
/// Charming = 2+, Breathtaking = 4+.
pub fn compute_tile_appeal(coord: libhexgrid::coord::HexCoord, state: &crate::game::state::GameState) -> i32 {
    use libhexgrid::board::HexBoard;
    let mut appeal: i32 = 0;
    for nb in state.board.neighbors(coord) {
        if let Some(tile) = state.board.tile(nb) {
            // Terrain bonuses.
            match tile.terrain {
                BuiltinTerrain::Mountain => appeal += 1,
                BuiltinTerrain::Coast    => appeal += 1,
                _ => {}
            }
            // Feature bonuses/penalties.
            match tile.feature {
                Some(BuiltinFeature::Forest) | Some(BuiltinFeature::Oasis) => appeal += 1,
                Some(BuiltinFeature::Rainforest) | Some(BuiltinFeature::Marsh)
                | Some(BuiltinFeature::Floodplain) => appeal -= 1,
                _ => {}
            }
        }
        // District penalties.
        for pd in &state.placed_districts {
            if pd.coord == nb {
                match pd.district_type {
                    crate::civ::district::BuiltinDistrict::Encampment
                    | crate::civ::district::BuiltinDistrict::IndustrialZone => appeal -= 1,
                    _ => {}
                }
            }
        }
    }
    // Natural wonder adjacency.
    if let Some(tile) = state.board.tile(coord)
        && tile.natural_wonder.is_some()
    {
        appeal += 2;
    }
    appeal
}

/// Evaluate a condition against the given context.
#[allow(clippy::collapsible_if)]
pub fn evaluate_condition(condition: &Condition, ctx: &ConditionContext<'_>) -> ConditionResult {
    use libhexgrid::board::HexBoard;
    match condition {
        Condition::AdjacentToRiver => {
            if let Some(coord) = ctx.tile {
                // Check if tile has any river edges.
                for dir in libhexgrid::coord::HexDir::ALL {
                    if let Some(edge) = ctx.state.board.edge(coord, dir) {
                        if edge.feature.as_ref().is_some_and(|f|
                            matches!(f, crate::world::edge::BuiltinEdgeFeature::River(_)))
                        {
                            return ConditionResult::Pass;
                        }
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::OnHills => {
            if let Some(coord) = ctx.tile {
                if let Some(tile) = ctx.state.board.tile(coord) {
                    if tile.hills {
                        return ConditionResult::Pass;
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::OnCoast => {
            if let Some(coord) = ctx.tile {
                if let Some(tile) = ctx.state.board.tile(coord) {
                    if matches!(tile.terrain, crate::world::terrain::BuiltinTerrain::Coast) {
                        return ConditionResult::Pass;
                    }
                    // Also check if adjacent to coast.
                    for nb in ctx.state.board.neighbors(coord) {
                        if let Some(nb_tile) = ctx.state.board.tile(nb) {
                            if matches!(nb_tile.terrain, crate::world::terrain::BuiltinTerrain::Coast) {
                                return ConditionResult::Pass;
                            }
                        }
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::TileHasImprovement(improvement) => {
            if let Some(coord) = ctx.tile {
                if let Some(tile) = ctx.state.board.tile(coord) {
                    if tile.improvement == Some(*improvement) {
                        return ConditionResult::Pass;
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::TileHasResourceOfCategory(category) => {
            if let Some(coord) = ctx.tile {
                if let Some(tile) = ctx.state.board.tile(coord) {
                    if let Some(res) = tile.resource {
                        if res.category() == *category {
                            return ConditionResult::Pass;
                        }
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::TileHasFeature(feature) => {
            if let Some(coord) = ctx.tile {
                if let Some(tile) = ctx.state.board.tile(coord) {
                    if tile.feature == Some(*feature) {
                        return ConditionResult::Pass;
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::PerAdjacentTerrain(terrain) => {
            if let Some(coord) = ctx.tile {
                let count = ctx.state.board.neighbors(coord).iter()
                    .filter(|nb| {
                        ctx.state.board.tile(**nb)
                            .is_some_and(|t| t.terrain == *terrain)
                    })
                    .count();
                ConditionResult::Scale(count as i32)
            } else {
                ConditionResult::Scale(0)
            }
        }
        Condition::PerAdjacentFeature(feature) => {
            if let Some(coord) = ctx.tile {
                let count = ctx.state.board.neighbors(coord).iter()
                    .filter(|nb| {
                        ctx.state.board.tile(**nb)
                            .is_some_and(|t| t.feature == Some(*feature))
                    })
                    .count();
                ConditionResult::Scale(count as i32)
            } else {
                ConditionResult::Scale(0)
            }
        }
        Condition::PerCityStateSuzerain => {
            // Count city-states where this civ is suzerain.
            // City-states are cities with CityKind::CityState.
            // For now, return Scale(0) — full city-state suzerain tracking is a future system.
            ConditionResult::Scale(0)
        }
        Condition::PerAdjacentDistrict => {
            if let Some(coord) = ctx.tile {
                let count = ctx.state.placed_districts.iter()
                    .filter(|pd| {
                        let d = pd.coord.distance(&coord);
                        d == 1
                    })
                    .count();
                ConditionResult::Scale(count as i32)
            } else {
                ConditionResult::Scale(0)
            }
        }
        Condition::PerTradingPostInRoute => {
            // Simplified: count trade routes owned by this civ.
            let count = ctx.state.trade_routes.iter()
                .filter(|tr| tr.owner == ctx.civ_id)
                .count();
            ConditionResult::Scale(count as i32)
        }
        Condition::PerForeignCityWithWorshipBuilding => {
            // Stub — worship building system not yet implemented.
            ConditionResult::Scale(0)
        }
        Condition::PerCivMetWithReligionNotAtWar => {
            // Count civs that are not at war with us.
            // Simplified: count civs we have Neutral/Friendly/Alliance relations with.
            let count = ctx.state.diplomatic_relations.iter()
                .filter(|r| {
                    (r.civ_a == ctx.civ_id || r.civ_b == ctx.civ_id)
                        && !r.is_at_war()
                })
                .count();
            ConditionResult::Scale(count as i32)
        }
        Condition::AtWar => {
            let at_war = ctx.state.diplomatic_relations.iter()
                .any(|r| (r.civ_a == ctx.civ_id || r.civ_b == ctx.civ_id) && r.is_at_war());
            if at_war { ConditionResult::Pass } else { ConditionResult::Fail }
        }
        Condition::NotAtWar => {
            let at_war = ctx.state.diplomatic_relations.iter()
                .any(|r| (r.civ_a == ctx.civ_id || r.civ_b == ctx.civ_id) && r.is_at_war());
            if at_war { ConditionResult::Fail } else { ConditionResult::Pass }
        }
        Condition::UnitDamaged => {
            if let Some(uid) = ctx.unit_id {
                if let Some(u) = ctx.state.unit(uid) {
                    if u.health < 100 {
                        return ConditionResult::Pass;
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::AdjacentToSameUnitType => {
            if let Some(uid) = ctx.unit_id {
                if let Some(u) = ctx.state.unit(uid) {
                    let count = ctx.state.units.iter()
                        .filter(|other| {
                            other.id != u.id
                                && other.owner == u.owner
                                && other.unit_type == u.unit_type
                                && other.coord.distance(&u.coord) == 1
                        })
                        .count();
                    if count > 0 {
                        return ConditionResult::Scale(count as i32);
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::AdjacentToEnemy => {
            if let Some(uid) = ctx.unit_id {
                if let Some(u) = ctx.state.unit(uid) {
                    let has_adj_enemy = ctx.state.units.iter().any(|other| {
                        other.owner != u.owner && other.coord.distance(&u.coord) == 1
                    });
                    if has_adj_enemy {
                        return ConditionResult::Pass;
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::TargetIsCityState => {
            // Check if the target unit's owner is a city-state.
            // Would need target context — for now, Pass if explicitly set.
            ConditionResult::Fail
        }
        Condition::ProducingDistrictOrWonder => {
            if let Some(city_id) = ctx.city_id {
                if let Some(city) = ctx.state.cities.iter().find(|c| c.id == city_id) {
                    if let Some(front) = city.production_queue.front() {
                        use crate::civ::ProductionItem;
                        if matches!(front, ProductionItem::District(_) | ProductionItem::Wonder(_)) {
                            return ConditionResult::Pass;
                        }
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::CityHasWorshipBuilding => {
            // Stub — worship building system not yet implemented.
            ConditionResult::Fail
        }
        Condition::TileMinAppeal(threshold) => {
            if let Some(coord) = ctx.tile {
                let appeal = compute_tile_appeal(coord, ctx.state);
                if appeal >= *threshold {
                    return ConditionResult::Pass;
                }
            }
            ConditionResult::Fail
        }
        Condition::ProducingMilitaryUnitOfEra(era) => {
            if let Some(city_id) = ctx.city_id {
                if let Some(city) = ctx.state.cities.iter().find(|c| c.id == city_id) {
                    if let Some(crate::civ::ProductionItem::Unit(utid)) = city.production_queue.front() {
                        if let Some(utd) = ctx.state.unit_type_defs.iter().find(|u| u.id == *utid) {
                            if matches!(utd.category, crate::UnitCategory::Combat | crate::UnitCategory::Support)
                                && utd.era == Some(*era)
                            {
                                return ConditionResult::Pass;
                            }
                        }
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::ProducingWonderOfEra(era) => {
            if let Some(city_id) = ctx.city_id {
                if let Some(city) = ctx.state.cities.iter().find(|c| c.id == city_id) {
                    if let Some(crate::civ::ProductionItem::Wonder(wid)) = city.production_queue.front() {
                        // Check wonder era via wonder_defs if available.
                        if let Some(wdef) = ctx.state.wonder_defs.iter().find(|w| w.id == *wid) {
                            if wdef.era == Some(*era) {
                                return ConditionResult::Pass;
                            }
                        }
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::TileHasAnyImprovement => {
            if let Some(coord) = ctx.tile {
                if let Some(tile) = ctx.state.board.tile(coord) {
                    if tile.improvement.is_some() {
                        return ConditionResult::Pass;
                    }
                }
            }
            ConditionResult::Fail
        }
        Condition::And(a, b) => {
            let ra = evaluate_condition(a, ctx);
            if matches!(ra, ConditionResult::Fail) {
                return ConditionResult::Fail;
            }
            let rb = evaluate_condition(b, ctx);
            if matches!(rb, ConditionResult::Fail) {
                return ConditionResult::Fail;
            }
            // If both Scale, multiply. If one Scale and one Pass, use Scale. If both Pass, Pass.
            match (ra, rb) {
                (ConditionResult::Scale(a), ConditionResult::Scale(b)) => ConditionResult::Scale(a * b),
                (ConditionResult::Scale(n), _) | (_, ConditionResult::Scale(n)) => ConditionResult::Scale(n),
                _ => ConditionResult::Pass,
            }
        }
        Condition::Or(a, b) => {
            let ra = evaluate_condition(a, ctx);
            if !matches!(ra, ConditionResult::Fail) {
                return ra;
            }
            evaluate_condition(b, ctx)
        }
    }
}

// ── Modifier struct ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Modifier {
    pub source: ModifierSource,
    pub target: TargetSelector,
    pub effect: EffectType,
    pub stacking: StackingRule,
    /// Optional condition that must be met for this modifier to apply.
    /// `None` means always active (unconditional).
    pub condition: Option<Condition>,
}

impl Modifier {
    pub fn new(
        source: ModifierSource,
        target: TargetSelector,
        effect: EffectType,
        stacking: StackingRule,
    ) -> Self {
        Self { source, target, effect, stacking, condition: None }
    }

    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }
}

// ── resolve_modifiers ────────────────────────────────────────────────────────

/// Resolve a list of modifiers into a deduplicated set of effects by applying
/// stacking rules. If a `ConditionContext` is provided, conditional modifiers
/// are evaluated and filtered/scaled accordingly.
pub fn resolve_modifiers(modifiers: &[Modifier], ctx: Option<&ConditionContext<'_>>) -> Vec<EffectType> {
    if modifiers.is_empty() {
        return vec![];
    }

    use std::collections::HashMap;

    let mut yield_flat:  HashMap<(YieldType, StackingRule), Vec<i32>> = HashMap::new();
    let mut yield_pct:   HashMap<(YieldType, StackingRule), Vec<i32>> = HashMap::new();
    let mut combat_flat: HashMap<StackingRule, Vec<i32>>              = HashMap::new();
    let mut combat_pct:  HashMap<StackingRule, Vec<i32>>              = HashMap::new();
    let mut movement:    HashMap<StackingRule, Vec<u32>>              = HashMap::new();
    // New effect types — collected separately.
    let mut other_effects: Vec<EffectType> = Vec::new();

    for m in modifiers {
        // Evaluate condition if present.
        let scale = if let Some(cond) = &m.condition {
            if let Some(ctx) = ctx {
                match evaluate_condition(cond, ctx) {
                    ConditionResult::Fail => continue, // skip this modifier
                    ConditionResult::Pass => 1,
                    ConditionResult::Scale(n) => {
                        if n == 0 { continue; } // scale by 0 = no effect
                        n
                    }
                }
            } else {
                continue; // conditional modifier but no context provided — skip
            }
        } else {
            1 // unconditional
        };

        match m.effect {
            EffectType::YieldFlat(yt, v) =>
                yield_flat.entry((yt, m.stacking)).or_default().push(v * scale),
            EffectType::YieldPercent(yt, v) =>
                yield_pct.entry((yt, m.stacking)).or_default().push(v * scale),
            EffectType::CombatStrengthFlat(v) =>
                combat_flat.entry(m.stacking).or_default().push(v * scale),
            EffectType::CombatStrengthPercent(v) =>
                combat_pct.entry(m.stacking).or_default().push(v * scale),
            EffectType::MovementBonus(v) =>
                movement.entry(m.stacking).or_default().push(v * scale as u32),
            // New effect types pass through directly (not stacking-resolved).
            other => other_effects.push(other),
        }
    }

    let mut out = Vec::new();

    for ((yt, rule), vals) in &yield_flat {
        out.push(EffectType::YieldFlat(*yt, reduce_i32(vals, *rule)));
    }
    for ((yt, rule), vals) in &yield_pct {
        out.push(EffectType::YieldPercent(*yt, reduce_i32(vals, *rule)));
    }
    for (rule, vals) in &combat_flat {
        out.push(EffectType::CombatStrengthFlat(reduce_i32(vals, *rule)));
    }
    for (rule, vals) in &combat_pct {
        out.push(EffectType::CombatStrengthPercent(reduce_i32(vals, *rule)));
    }
    for (rule, vals) in &movement {
        out.push(EffectType::MovementBonus(reduce_u32(vals, *rule)));
    }

    out.extend(other_effects);
    out
}

fn reduce_i32(vals: &[i32], rule: StackingRule) -> i32 {
    match rule {
        StackingRule::Additive => vals.iter().sum(),
        StackingRule::Max      => *vals.iter().max().unwrap_or(&0),
        StackingRule::Replace  => *vals.last().unwrap_or(&0),
    }
}

fn reduce_u32(vals: &[u32], rule: StackingRule) -> u32 {
    match rule {
        StackingRule::Additive => vals.iter().sum(),
        StackingRule::Max      => *vals.iter().max().unwrap_or(&0),
        StackingRule::Replace  => *vals.last().unwrap_or(&0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_yield_modifier(amount: i32, stacking: StackingRule) -> Modifier {
        Modifier::new(
            ModifierSource::Tech("test"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Production, amount),
            stacking,
        )
    }

    #[test]
    fn test_modifier_stacking_additive() {
        let mods = vec![
            make_yield_modifier(2, StackingRule::Additive),
            make_yield_modifier(3, StackingRule::Additive),
        ];
        let effects = resolve_modifiers(&mods, None);
        let total: i32 = effects
            .iter()
            .filter_map(|e| {
                if let EffectType::YieldFlat(YieldType::Production, v) = e {
                    Some(*v)
                } else {
                    None
                }
            })
            .sum();
        assert_eq!(total, 5);
    }

    #[test]
    fn test_modifier_stacking_max() {
        let mods = vec![
            make_yield_modifier(2, StackingRule::Max),
            make_yield_modifier(5, StackingRule::Max),
            make_yield_modifier(3, StackingRule::Max),
        ];
        let effects = resolve_modifiers(&mods, None);
        let max = effects
            .iter()
            .filter_map(|e| {
                if let EffectType::YieldFlat(YieldType::Production, v) = e {
                    Some(*v)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);
        assert_eq!(max, 5);
    }

    #[test]
    fn test_modifier_stacking_replace() {
        let mods = vec![
            make_yield_modifier(2, StackingRule::Replace),
            make_yield_modifier(7, StackingRule::Replace),
        ];
        let effects = resolve_modifiers(&mods, None);
        let vals: Vec<i32> = effects
            .iter()
            .filter_map(|e| {
                if let EffectType::YieldFlat(YieldType::Production, v) = e {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0], 7);
    }

    #[test]
    fn test_conditional_modifier_skipped_without_context() {
        let m = Modifier::new(
            ModifierSource::CivAbility("test"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Gold, 5),
            StackingRule::Additive,
        ).with_condition(Condition::AdjacentToRiver);
        let effects = resolve_modifiers(&[m], None);
        assert!(effects.is_empty(), "conditional modifier should be skipped without context");
    }
}
