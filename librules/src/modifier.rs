use libcommon::{CivId, YieldType};

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
    UnitDomain(libcommon::UnitDomain),
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

/// Resolve a list of modifiers with the same effect type and stacking rule.
pub fn resolve_modifiers(modifiers: &[Modifier]) -> Vec<EffectType> {
    if modifiers.is_empty() {
        return vec![];
    }

    // Group by (effect discriminant, stacking rule) — simplified: apply stacking per modifier
    // For Phase 1, just return all effects; Phase 2 will implement proper resolution.
    modifiers.iter().map(|m| m.effect).collect()
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
    #[ignore = "Phase 2: implement modifier resolution"]
    fn test_modifier_stacking_additive() {
        let mods = vec![
            make_yield_modifier(2, StackingRule::Additive),
            make_yield_modifier(3, StackingRule::Additive),
        ];
        let effects = resolve_modifiers(&mods);
        // Additive stacking: sum = 5
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
    #[ignore = "Phase 2: implement modifier resolution"]
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
    #[ignore = "Phase 2: implement modifier resolution"]
    fn test_modifier_stacking_replace() {
        let mods = vec![
            make_yield_modifier(2, StackingRule::Replace),
            make_yield_modifier(7, StackingRule::Replace),
        ];
        let effects = resolve_modifiers(&mods);
        // Replace: only last value
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
