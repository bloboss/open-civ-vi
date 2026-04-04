//! Production gating helpers: tech-gated unit/building production and
//! civ-exclusive unit replacement.
//!
//! These functions determine which units and buildings a civilization can
//! currently produce, enforce exclusivity rules, and apply automatic
//! replacement (e.g., Rome gets Legion instead of Swordsman).

use crate::game::state::{BuildingDef, GameState, UnitTypeDef};
use crate::{BuildingId, CivId, UnitTypeId};

/// Units that are always available for production without any tech prerequisite.
pub const ALWAYS_AVAILABLE_UNITS: &[&str] = &[
    "Warrior",
    "Settler",
    "Builder",
    "Slinger",
    "Trader",
    "Scout",
    "Battering Ram",
];

/// Buildings that are always available for production without any tech prerequisite.
pub const ALWAYS_AVAILABLE_BUILDINGS: &[&str] = &[
    "Monument",
    "Palace",
    "Granary",
];

/// Returns the unit type defs that the given civ can currently produce.
///
/// The list contains **generic/base units** (e.g. "Swordsman"), not unique
/// replacements (e.g. "Legion"). The replacement happens transparently at
/// production completion in `advance_turn` via `resolve_unit_replacement`.
///
/// Filtering rules:
/// 1. Exclude units exclusive to a **different** civ.
/// 2. Exclude unique units that replace a base unit (players queue the base name).
/// 3. The unit must be unlocked (in `civ.unlocked_units`) or be always-available.
pub fn available_unit_defs(state: &GameState, civ_id: CivId) -> Vec<&UnitTypeDef> {
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let civ_identity = civ.civ_identity;

    state.unit_type_defs.iter()
        .filter(|d| {
            // Exclude units exclusive to a different civ.
            if let Some(excl) = d.exclusive_to {
                if Some(excl) != civ_identity {
                    return false;
                }
                // This is a unique unit for THIS civ that replaces a base unit.
                // Players queue the base unit name; the swap happens at completion.
                // So exclude unique replacements from the available list.
                if d.replaces.is_some() {
                    return false;
                }
            }

            // Check if the unit is unlocked or always available.
            civ.unlocked_units.contains(&d.name)
                || ALWAYS_AVAILABLE_UNITS.contains(&d.name)
        })
        .collect()
}

/// Returns the building defs that the given civ can currently produce.
///
/// Filtering rules mirror `available_unit_defs`: exclusivity, tech gating, and
/// replacement for civ-unique buildings.
/// Returns the building defs that the given civ can currently produce.
///
/// Like `available_unit_defs`, this returns **base buildings**. Unique
/// replacements (e.g. Stave Church for Norway instead of Temple) are resolved
/// at production completion via `resolve_building_replacement`.
pub fn available_building_defs(state: &GameState, civ_id: CivId) -> Vec<&BuildingDef> {
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let civ_identity = civ.civ_identity;

    state.building_defs.iter()
        .filter(|d| {
            // Exclude buildings exclusive to a different civ.
            if let Some(excl) = d.exclusive_to {
                if Some(excl) != civ_identity {
                    return false;
                }
                // Unique replacement for this civ — players queue the base name.
                if d.replaces.is_some() {
                    return false;
                }
            }

            civ.unlocked_buildings.contains(&d.name)
                || ALWAYS_AVAILABLE_BUILDINGS.contains(&d.name)
        })
        .collect()
}

/// Resolve unit replacement: if the civ has a unique unit that replaces the
/// given unit type, return the unique unit's `UnitTypeId` and name instead.
///
/// Returns `(resolved_type_id, resolved_name)`.
pub fn resolve_unit_replacement(
    state: &GameState,
    civ_id: CivId,
    unit_type_id: UnitTypeId,
) -> (UnitTypeId, &str) {
    let base_def = match state.unit_type_defs.iter().find(|d| d.id == unit_type_id) {
        Some(d) => d,
        None => return (unit_type_id, "?"),
    };
    let civ_identity = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .and_then(|c| c.civ_identity);

    if let Some(civ_ident) = civ_identity
        && let Some(replacement) = state.unit_type_defs.iter().find(|d| {
            d.exclusive_to == Some(civ_ident) && d.replaces == Some(base_def.name)
        })
    {
        return (replacement.id, replacement.name);
    }

    (base_def.id, base_def.name)
}

/// Resolve building replacement: if the civ has a unique building that replaces
/// the given building, return the unique building's `BuildingId` and name.
pub fn resolve_building_replacement(
    state: &GameState,
    civ_id: CivId,
    building_id: BuildingId,
) -> (BuildingId, &str) {
    let base_def = match state.building_defs.iter().find(|d| d.id == building_id) {
        Some(d) => d,
        None => return (building_id, "?"),
    };
    let civ_identity = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .and_then(|c| c.civ_identity);

    if let Some(civ_ident) = civ_identity
        && let Some(replacement) = state.building_defs.iter().find(|d| {
            d.exclusive_to == Some(civ_ident) && d.replaces == Some(base_def.name)
        })
    {
        return (replacement.id, replacement.name);
    }

    (base_def.id, base_def.name)
}

/// Check whether a civ can produce a specific unit type.
///
/// Returns `true` if the unit is in the available set for this civ.
pub fn can_produce_unit(state: &GameState, civ_id: CivId, unit_type_id: UnitTypeId) -> bool {
    available_unit_defs(state, civ_id).iter().any(|d| d.id == unit_type_id)
}

/// Check whether a civ can produce a specific building.
pub fn can_produce_building(state: &GameState, civ_id: CivId, building_id: BuildingId) -> bool {
    available_building_defs(state, civ_id).iter().any(|d| d.id == building_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::civilization::{Civilization, Leader, BuiltinAgenda};
    use crate::civ::civ_identity::BuiltinCiv;
    use crate::game::state::GameState;

    fn setup_state() -> GameState {
        GameState::new(42, 10, 10)
    }

    fn add_civ(state: &mut GameState, identity: Option<BuiltinCiv>) -> CivId {
        let civ_id = state.id_gen.next_civ_id();
        let mut civ = Civilization::new(
            civ_id, "TestCiv", "Test",
            Leader { name: "Leader", civ_id, agenda: BuiltinAgenda::Default },
        );
        civ.civ_identity = identity;
        state.civilizations.push(civ);
        civ_id
    }

    #[test]
    fn always_available_units_are_producible() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, None);

        let available = available_unit_defs(&state, civ_id);
        let names: Vec<&str> = available.iter().map(|d| d.name).collect();

        for &name in ALWAYS_AVAILABLE_UNITS {
            assert!(names.contains(&name), "{name} should be always available");
        }
    }

    #[test]
    fn tech_gated_units_not_available_without_unlock() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, None);

        let available = available_unit_defs(&state, civ_id);
        let names: Vec<&str> = available.iter().map(|d| d.name).collect();

        // Swordsman requires Iron Working tech unlock.
        assert!(!names.contains(&"Swordsman"), "Swordsman should not be available without tech");
        assert!(!names.contains(&"Archer"), "Archer should not be available without tech");
    }

    #[test]
    fn unlocked_unit_becomes_available() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, None);

        // Manually unlock Swordsman.
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id).unwrap()
            .unlocked_units.push("Swordsman");

        let available = available_unit_defs(&state, civ_id);
        let names: Vec<&str> = available.iter().map(|d| d.name).collect();
        assert!(names.contains(&"Swordsman"), "Swordsman should be available after unlock");
    }

    #[test]
    fn exclusive_unit_not_available_to_wrong_civ() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, Some(BuiltinCiv::Greece));

        // Unlock the base unit that Legion replaces.
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id).unwrap()
            .unlocked_units.push("Swordsman");

        let available = available_unit_defs(&state, civ_id);
        let names: Vec<&str> = available.iter().map(|d| d.name).collect();

        // Legion is exclusive to Rome, so Greece should not see it.
        assert!(!names.contains(&"Legion"), "Legion should not be available to Greece");
        // Greece should still see Swordsman (no replacement).
        assert!(names.contains(&"Swordsman"), "Swordsman should be available to Greece");
    }

    #[test]
    fn rome_queues_swordsman_resolved_to_legion_at_completion() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, Some(BuiltinCiv::Rome));

        // Unlock Swordsman (the base unit that Legion replaces).
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id).unwrap()
            .unlocked_units.push("Swordsman");

        // Available list shows the base unit (Swordsman), not the replacement.
        let available = available_unit_defs(&state, civ_id);
        let names: Vec<&str> = available.iter().map(|d| d.name).collect();
        assert!(names.contains(&"Swordsman"), "Rome should see Swordsman in available list");
        assert!(!names.contains(&"Legion"), "Legion is resolved at completion, not shown in available list");

        // But resolve_unit_replacement swaps it to Legion at production time.
        let swordsman_id = state.unit_type_defs.iter()
            .find(|d| d.name == "Swordsman").unwrap().id;
        let (resolved_id, resolved_name) = resolve_unit_replacement(&state, civ_id, swordsman_id);
        assert_eq!(resolved_name, "Legion");
        assert_ne!(resolved_id, swordsman_id);
    }

    #[test]
    fn resolve_replacement_for_rome_swordsman() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, Some(BuiltinCiv::Rome));

        let swordsman_id = state.unit_type_defs.iter()
            .find(|d| d.name == "Swordsman").unwrap().id;

        let (resolved_id, resolved_name) = resolve_unit_replacement(&state, civ_id, swordsman_id);
        assert_eq!(resolved_name, "Legion");
        assert_ne!(resolved_id, swordsman_id);
    }

    #[test]
    fn resolve_replacement_no_op_for_generic_civ() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, None);

        let swordsman_id = state.unit_type_defs.iter()
            .find(|d| d.name == "Swordsman").unwrap().id;

        let (resolved_id, resolved_name) = resolve_unit_replacement(&state, civ_id, swordsman_id);
        assert_eq!(resolved_name, "Swordsman");
        assert_eq!(resolved_id, swordsman_id);
    }

    #[test]
    fn norway_queues_temple_resolved_to_stave_church_at_completion() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, Some(BuiltinCiv::Norway));

        // Unlock Temple (the base building that Stave Church replaces).
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id).unwrap()
            .unlocked_buildings.push("Temple");

        // Available list shows the base building (Temple), not the replacement.
        let available = available_building_defs(&state, civ_id);
        let names: Vec<&str> = available.iter().map(|d| d.name).collect();
        assert!(names.contains(&"Temple"), "Norway should see Temple in available list");
        assert!(!names.contains(&"Stave Church"), "Stave Church is resolved at completion");

        // But resolve_building_replacement swaps it at production time.
        let temple_id = state.building_defs.iter()
            .find(|d| d.name == "Temple").unwrap().id;
        let (resolved_id, resolved_name) = resolve_building_replacement(&state, civ_id, temple_id);
        assert_eq!(resolved_name, "Stave Church");
        assert_ne!(resolved_id, temple_id);
    }

    #[test]
    fn can_produce_unit_respects_gating() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, None);

        let warrior_id = state.unit_type_defs.iter()
            .find(|d| d.name == "Warrior").unwrap().id;
        let swordsman_id = state.unit_type_defs.iter()
            .find(|d| d.name == "Swordsman").unwrap().id;

        assert!(can_produce_unit(&state, civ_id, warrior_id));
        assert!(!can_produce_unit(&state, civ_id, swordsman_id));
    }

    #[test]
    fn always_available_buildings_are_producible() {
        let mut state = setup_state();
        let civ_id = add_civ(&mut state, None);

        let available = available_building_defs(&state, civ_id);
        let names: Vec<&str> = available.iter().map(|d| d.name).collect();

        for &name in ALWAYS_AVAILABLE_BUILDINGS {
            assert!(names.contains(&name), "{name} should be always available");
        }
    }
}
