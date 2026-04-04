use crate::{CityId, CivicId, TechId};
use crate::civ::civilization::Civilization;
use crate::rules::modifier::Modifier;
use crate::world::resource::BuiltinResource;

/// Whether an effect can produce further effects when applied.
///
/// The two-phase `advance_turn` design already prevents cascades structurally
/// (`apply_effect` returns `()`), but this enum lets each variant advertise its
/// own safety class so the scheduler can document and enforce the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CascadeClass {
    /// Applying this effect never schedules additional effects.
    /// The scheduler applies it unconditionally (still subject to `guard()`).
    NonCascading,
    /// This effect is safe only once per (civ, key) pair. `guard()` is the
    /// idempotency check — returning `false` means "already applied, skip".
    Idempotent,
}

/// A discrete, irreversible mutation to a single civilization's state.
///
/// Triggered by completion events (tech, civic, wonder) and processed in a
/// dedicated drain phase of `advance_turn`. Each variant carries its own
/// cascade class and idempotency guard so no external registry is needed.
///
/// # Cascade safety
/// `apply_effect` has return type `()` — it structurally cannot enqueue more
/// effects. The drain loop terminates because the completable set is monotone
/// decreasing (each tech/civic completes at most once).
#[derive(Debug, Clone)]
pub enum OneShotEffect {
    // ── Resource visibility ──────────────────────────────────────────────────
    /// Make a resource visible and usable to this civ.
    RevealResource(BuiltinResource),

    // ── Production unlocks ───────────────────────────────────────────────────
    /// Unlock a unit type for production.
    UnlockUnit(&'static str),
    /// Unlock a building type for production.
    UnlockBuilding(&'static str),
    /// Unlock an improvement type for builders.
    UnlockImprovement(&'static str),

    // ── Research boosts ──────────────────────────────────────────────────────
    /// Trigger a Eureka boost, halving the remaining cost of the given tech.
    TriggerEureka { tech: TechId },
    /// Trigger an Inspiration boost, halving the remaining cost of the given civic.
    TriggerInspiration { civic: CivicId },

    // ── Free grants ─────────────────────────────────────────────────────────
    /// Spawn a free unit. `city` hints at placement; if `None` or invalid the
    /// unit spawns at the nearest city. Combat units displace enemies; civilian
    /// units in occupied tiles are captured. No capital required.
    ///
    /// Note: full resolution requires a unit-type registry (Phase 4). For now
    /// `apply_effect` emits the `FreeUnitGranted` delta only.
    FreeUnit { unit_type: &'static str, city: Option<CityId> },
    /// Place a free building in the given city, or the capital if `None`.
    ///
    /// Note: full resolution requires a building registry (Phase 4). For now
    /// `apply_effect` emits the `FreeBuildingGranted` delta only.
    FreeBuilding { building: &'static str, city: Option<CityId> },

    // ── Government ───────────────────────────────────────────────────────────
    /// Unlock a government type for adoption (typically from a civic).
    UnlockGovernment(&'static str),
    /// Adopt a government. Removes the previous government's policy slots and
    /// inherent modifier; applies those of the new one. Player-triggered.
    AdoptGovernment(&'static str),

    // ── Policy unlock ─────────────────────────────────────────────────────────
    /// Unlock a policy card for equipping in government slots.
    UnlockPolicy(&'static str),

    // ── Modifier grant from tech/civic ───────────────────────────────────────
    /// Grant a permanent modifier to this civilization, sourced from a completed
    /// tech or civic. The modifier is collected at query time via
    /// `Civilization::get_tree_modifiers(tech_tree, civic_tree)`.
    GrantModifier(Modifier),
}

impl OneShotEffect {
    /// The cascade classification for this variant.
    pub fn cascade_class(&self) -> CascadeClass {
        match self {
            OneShotEffect::RevealResource(_)        => CascadeClass::Idempotent,
            OneShotEffect::TriggerEureka { .. }     => CascadeClass::Idempotent,
            OneShotEffect::TriggerInspiration { .. }=> CascadeClass::Idempotent,
            OneShotEffect::UnlockGovernment(_)      => CascadeClass::Idempotent,
            OneShotEffect::AdoptGovernment(_)       => CascadeClass::Idempotent,
            OneShotEffect::UnlockPolicy(_)          => CascadeClass::Idempotent,
            OneShotEffect::GrantModifier(_)         => CascadeClass::NonCascading,
            _                                       => CascadeClass::NonCascading,
        }
    }

    /// Called by the scheduler before `apply_effect`. Returns `false` if this
    /// effect has already been applied and should be skipped. The guard logic
    /// lives here on the variant, not in a global registry.
    pub fn guard(&self, civ: &Civilization) -> bool {
        match self {
            OneShotEffect::RevealResource(r) =>
                !civ.revealed_resources.contains(r),
            OneShotEffect::TriggerEureka { tech } =>
                !civ.eureka_triggered.contains(tech),
            OneShotEffect::TriggerInspiration { civic } =>
                !civ.inspiration_triggered.contains(civic),
            OneShotEffect::UnlockGovernment(g) =>
                !civ.unlocked_governments.contains(g),
            OneShotEffect::AdoptGovernment(g) =>
                civ.current_government_name != Some(*g),
            OneShotEffect::UnlockPolicy(p) =>
                !civ.unlocked_policies.contains(p),
            OneShotEffect::GrantModifier(_) => true,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::resource::BuiltinResource;

    fn empty_civ() -> Civilization {
        use crate::CivId;
        use crate::civ::civilization::{Leader, BuiltinAgenda};
        use ulid::Ulid;

        let civ_id = CivId::from_ulid(Ulid::nil());
        Civilization::new(
            civ_id, "Test", "Test",
            Leader { name: "L", civ_id, agenda: BuiltinAgenda::Default },
        )
    }

    #[test]
    fn reveal_resource_guard_fires_once() {
        let mut civ = empty_civ();
        let effect = OneShotEffect::RevealResource(BuiltinResource::Wheat);
        assert!(effect.guard(&civ), "first application should pass guard");
        civ.revealed_resources.insert(BuiltinResource::Wheat);
        assert!(!effect.guard(&civ), "second application should be blocked");
    }

    #[test]
    fn eureka_guard_fires_once() {
        use ulid::Ulid;
        let mut civ = empty_civ();
        let pottery_id = TechId::from_ulid(Ulid::nil());
        let effect = OneShotEffect::TriggerEureka { tech: pottery_id };
        assert!(effect.guard(&civ));
        civ.eureka_triggered.insert(pottery_id);
        assert!(!effect.guard(&civ));
    }

    #[test]
    fn non_cascading_guard_always_true() {
        let civ = empty_civ();
        let effect = OneShotEffect::UnlockUnit("Warrior");
        assert_eq!(effect.cascade_class(), CascadeClass::NonCascading);
        assert!(effect.guard(&civ));
    }

    #[test]
    fn adopt_government_guard() {
        let mut civ = empty_civ();
        let effect = OneShotEffect::AdoptGovernment("Autocracy");
        assert!(effect.guard(&civ));
        civ.current_government_name = Some("Autocracy");
        assert!(!effect.guard(&civ), "already active government should be skipped");
    }

    #[test]
    fn unlock_policy_guard_fires_once() {
        let mut civ = empty_civ();
        let effect = OneShotEffect::UnlockPolicy("Strategos");
        assert!(effect.guard(&civ), "first application should pass");
        civ.unlocked_policies.push("Strategos");
        assert!(!effect.guard(&civ), "second application should be blocked");
        assert_eq!(effect.cascade_class(), CascadeClass::Idempotent);
    }

    #[test]
    fn grant_modifier_guard_always_true() {
        use crate::rules::modifier::{EffectType, Modifier, ModifierSource, StackingRule, TargetSelector};
        use crate::YieldType;

        let civ = empty_civ();
        let modifier = Modifier::new(
            ModifierSource::Tech("Pottery"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Culture, 1),
            StackingRule::Additive,
        );
        let effect = OneShotEffect::GrantModifier(modifier);
        assert!(effect.guard(&civ), "GrantModifier guard is always true");
        assert_eq!(effect.cascade_class(), CascadeClass::NonCascading);
    }
}
