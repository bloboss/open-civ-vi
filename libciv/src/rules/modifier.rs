use crate::{CivId, YieldType};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetSelector {
    /// Applies to all tiles.
    AllTiles,
    /// Applies to all units.
    AllUnits,
    /// Applies to units of a specific domain.
    UnitDomain(crate::UnitDomain),
    /// Applies to a specific civilization.
    Civilization(CivId),
    /// Applies globally.
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StackingRule {
    /// All modifiers of this type are summed.
    Additive,
    /// Only the highest value applies.
    Max,
    /// The most recently applied value replaces all previous.
    Replace,
}

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
}

#[derive(Debug, Clone)]
pub struct Modifier {
    pub source: ModifierSource,
    pub target: TargetSelector,
    pub effect: EffectType,
    pub stacking: StackingRule,
}

impl Modifier {
    pub fn new(
        source: ModifierSource,
        target: TargetSelector,
        effect: EffectType,
        stacking: StackingRule,
    ) -> Self {
        Self { source, target, effect, stacking }
    }
}

/// Resolve a list of modifiers into a deduplicated set of effects by applying stacking rules.
///
/// Modifiers are grouped by `(EffectType discriminant, YieldType if applicable, StackingRule)`.
/// Within each group:
/// - `Additive`  → sum all values; emit one effect with the total.
/// - `Max`       → keep the largest value; emit one effect.
/// - `Replace`   → keep the last value in slice order; emit one effect.
pub fn resolve_modifiers(modifiers: &[Modifier]) -> Vec<EffectType> {
    if modifiers.is_empty() {
        return vec![];
    }

    use std::collections::HashMap;

    // Accumulators keyed by (YieldType, StackingRule); order preserved via insertion order vec.
    let mut yield_flat:  HashMap<(YieldType, StackingRule), Vec<i32>> = HashMap::new();
    let mut yield_pct:   HashMap<(YieldType, StackingRule), Vec<i32>> = HashMap::new();
    let mut combat_flat: HashMap<StackingRule, Vec<i32>>              = HashMap::new();
    let mut combat_pct:  HashMap<StackingRule, Vec<i32>>              = HashMap::new();
    let mut movement:    HashMap<StackingRule, Vec<u32>>              = HashMap::new();

    for m in modifiers {
        match m.effect {
            EffectType::YieldFlat(yt, v)          => yield_flat.entry((yt, m.stacking)).or_default().push(v),
            EffectType::YieldPercent(yt, v)       => yield_pct.entry((yt, m.stacking)).or_default().push(v),
            EffectType::CombatStrengthFlat(v)     => combat_flat.entry(m.stacking).or_default().push(v),
            EffectType::CombatStrengthPercent(v)  => combat_pct.entry(m.stacking).or_default().push(v),
            EffectType::MovementBonus(v)          => movement.entry(m.stacking).or_default().push(v),
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
        let effects = resolve_modifiers(&mods);
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
        let effects = resolve_modifiers(&mods);
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
        let effects = resolve_modifiers(&mods);
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
}
