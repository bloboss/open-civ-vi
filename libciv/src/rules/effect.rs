use crate::CityId;
use crate::civ::civilization::Civilization;
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
    /// Trigger a Eureka boost, halving the remaining cost of the named tech.
    TriggerEureka { tech: &'static str },
    /// Trigger an Inspiration boost, halving the remaining cost of the named civic.
    TriggerInspiration { civic: &'static str },

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
                !civ.eureka_triggered.contains(*tech),
            OneShotEffect::TriggerInspiration { civic } =>
                !civ.inspiration_triggered.contains(*civic),
            OneShotEffect::UnlockGovernment(g) =>
                !civ.unlocked_governments.contains(g),
            OneShotEffect::AdoptGovernment(g) =>
                civ.current_government_name != Some(*g),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::resource::BuiltinResource;
    use crate::world::resource::Wheat;

    fn empty_civ() -> Civilization {
        use crate::CivId;
        use crate::civ::civilization::{Leader, Agenda};
        use ulid::Ulid;

        struct NoOp;
        impl std::fmt::Debug for NoOp {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "NoOp")
            }
        }
        impl Agenda for NoOp {
            fn name(&self) -> &'static str { "noop" }
            fn description(&self) -> &'static str { "" }
            fn attitude(&self, _: CivId) -> i32 { 0 }
        }

        let civ_id = CivId::from_ulid(Ulid::nil());
        Civilization::new(
            civ_id, "Test", "Test",
            Leader { name: "L", civ_id, abilities: vec![], agenda: Box::new(NoOp) },
        )
    }

    #[test]
    fn reveal_resource_guard_fires_once() {
        let mut civ = empty_civ();
        let effect = OneShotEffect::RevealResource(BuiltinResource::Wheat(Wheat));
        assert!(effect.guard(&civ), "first application should pass guard");
        civ.revealed_resources.insert(BuiltinResource::Wheat(Wheat));
        assert!(!effect.guard(&civ), "second application should be blocked");
    }

    #[test]
    fn eureka_guard_fires_once() {
        let mut civ = empty_civ();
        let effect = OneShotEffect::TriggerEureka { tech: "Pottery" };
        assert!(effect.guard(&civ));
        civ.eureka_triggered.insert("Pottery");
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
}
