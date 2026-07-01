//! Per-unit action enumeration: what actions a single unit has
//! available this turn, and whether each is currently usable.
//!
//! Exposed via [`super::RulesEngine::available_unit_actions`]. The web
//! projector maps each `UnitActionKind` into a wire-friendly
//! `(id, label, hotkey)` triple; the engine itself only owns the
//! kind + enabled flag.

use crate::UnitId;

use super::super::state::GameState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitAction {
    pub kind: UnitActionKind,
    /// `false` when the action is conceptually applicable but cannot run
    /// right now (e.g. attack without movement remaining). Greyed-out in
    /// the UI; still listed so the player sees the full action set.
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitActionKind {
    Move,
    Attack,
    Fortify,
    Sleep,
    FoundCity,
    /// Builder action — place an improvement / road. Requires
    /// `charges > 0`.
    Build,
    /// Trader action — assign or establish a trade route. Available to
    /// `UnitCategory::Trader`.
    TradeRoute,
    /// Religious unit action — spread religion. Requires
    /// `spread_charges > 0`.
    SpreadReligion,
}

pub(crate) fn available_unit_actions(state: &GameState, unit: UnitId) -> Vec<UnitAction> {
    let Some(u) = state.units.iter().find(|u| u.id == unit) else {
        return Vec::new();
    };
    let def = state.unit_type_defs.iter().find(|d| d.id == u.unit_type);

    let has_movement = u.movement_left > 0;
    let has_combat = u.combat_strength.is_some();

    let mut out = Vec::new();

    // Move — every unit. Enabled iff movement remains.
    out.push(UnitAction {
        kind: UnitActionKind::Move,
        enabled: has_movement,
    });

    // Attack — combat units. Enabled iff movement remains. (Range/target
    // checks happen at attack time; the action is still surfaced so the UI
    // can prompt for a target.)
    if has_combat {
        out.push(UnitAction {
            kind: UnitActionKind::Attack,
            enabled: has_movement,
        });
    }

    // Fortify — combat units. Always enabled (it's a stance, not a
    // movement-consuming action).
    if has_combat {
        out.push(UnitAction {
            kind: UnitActionKind::Fortify,
            enabled: true,
        });
    }

    // Sleep — every unit.
    out.push(UnitAction {
        kind: UnitActionKind::Sleep,
        enabled: true,
    });

    // Found city — settler-class.
    if def.is_some_and(|d| d.can_found_city) {
        out.push(UnitAction {
            kind: UnitActionKind::FoundCity,
            enabled: has_movement,
        });
    }

    // Build — builder with remaining charges.
    if u.charges.is_some_and(|c| c > 0) {
        out.push(UnitAction {
            kind: UnitActionKind::Build,
            enabled: has_movement,
        });
    }

    // Trade route — trader category. Enabled when no route is currently
    // assigned (otherwise the unit is en route to its destination).
    if matches!(u.category, crate::UnitCategory::Trader) {
        out.push(UnitAction {
            kind: UnitActionKind::TradeRoute,
            enabled: has_movement && u.trade_origin.is_none(),
        });
    }

    // Spread religion — religious unit with charges.
    if u.spread_charges.is_some_and(|c| c > 0) {
        out.push(UnitAction {
            kind: UnitActionKind::SpreadReligion,
            enabled: has_movement,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::civilization::Civilization;
    use crate::civ::{BasicUnit, BuiltinAgenda, Leader};
    use crate::game::state::UnitTypeDef;
    use crate::{CivId, DefaultRulesEngine, RulesEngine, UnitCategory, UnitDomain, UnitTypeId};
    use libhexgrid::coord::HexCoord;

    fn make_state() -> (GameState, CivId) {
        let mut state = GameState::new(11, 10, 10);
        let civ_id = state.id_gen.next_civ_id();
        let leader = Leader {
            name: "TestLeader",
            civ_id,
            agenda: BuiltinAgenda::Default,
        };
        state
            .civilizations
            .push(Civilization::new(civ_id, "TestCiv", "Test", leader));
        (state, civ_id)
    }

    fn push_unit(
        state: &mut GameState,
        civ: CivId,
        category: UnitCategory,
        combat_strength: Option<u32>,
        movement: u32,
        can_found_city: bool,
        charges: Option<u8>,
        spread_charges: Option<u8>,
    ) -> UnitId {
        let unit_id = state.id_gen.next_unit_id();
        let type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
        state.unit_type_defs.push(UnitTypeDef {
            id: type_id,
            name: "test",
            production_cost: 10,
            domain: UnitDomain::Land,
            category,
            max_movement: 200,
            combat_strength,
            range: 0,
            vision_range: 2,
            can_found_city,
            resource_cost: None,
            siege_bonus: 0,
            max_charges: charges.unwrap_or(0),
            exclusive_to: None,
            replaces: None,
            era: None,
            promotion_class: None,
        });
        state.units.push(BasicUnit {
            id: unit_id,
            unit_type: type_id,
            owner: civ,
            coord: HexCoord::from_qr(0, 0),
            domain: UnitDomain::Land,
            category,
            movement_left: movement,
            max_movement: 200,
            combat_strength,
            promotions: Vec::new(),
            experience: 0,
            health: 100,
            range: 0,
            vision_range: 2,
            charges,
            trade_origin: None,
            trade_destination: None,
            religion_id: None,
            spread_charges,
            religious_strength: None,
            is_embarked: false,
        });
        unit_id
    }

    #[test]
    fn warrior_has_move_attack_fortify_sleep() {
        let (mut state, civ) = make_state();
        let warrior = push_unit(
            &mut state,
            civ,
            UnitCategory::Combat,
            Some(20),
            200,
            false,
            None,
            None,
        );
        let actions = DefaultRulesEngine.available_unit_actions(&state, warrior);
        let kinds: Vec<_> = actions.iter().map(|a| a.kind).collect();
        assert!(kinds.contains(&UnitActionKind::Move));
        assert!(kinds.contains(&UnitActionKind::Attack));
        assert!(kinds.contains(&UnitActionKind::Fortify));
        assert!(kinds.contains(&UnitActionKind::Sleep));
        // No build / trade / spread / found_city.
        assert!(!kinds.contains(&UnitActionKind::FoundCity));
        assert!(!kinds.contains(&UnitActionKind::Build));
    }

    #[test]
    fn settler_gets_found_city() {
        let (mut state, civ) = make_state();
        let settler = push_unit(
            &mut state,
            civ,
            UnitCategory::Civilian,
            None,
            200,
            true,
            None,
            None,
        );
        let actions = DefaultRulesEngine.available_unit_actions(&state, settler);
        assert!(
            actions
                .iter()
                .any(|a| a.kind == UnitActionKind::FoundCity && a.enabled)
        );
        assert!(!actions.iter().any(|a| a.kind == UnitActionKind::Attack));
    }

    #[test]
    fn builder_with_charges_gets_build() {
        let (mut state, civ) = make_state();
        let builder = push_unit(
            &mut state,
            civ,
            UnitCategory::Civilian,
            None,
            200,
            false,
            Some(3),
            None,
        );
        let actions = DefaultRulesEngine.available_unit_actions(&state, builder);
        assert!(
            actions
                .iter()
                .any(|a| a.kind == UnitActionKind::Build && a.enabled)
        );
    }

    #[test]
    fn move_disabled_when_no_movement_remaining() {
        let (mut state, civ) = make_state();
        let warrior = push_unit(
            &mut state,
            civ,
            UnitCategory::Combat,
            Some(20),
            0,
            false,
            None,
            None,
        );
        let actions = DefaultRulesEngine.available_unit_actions(&state, warrior);
        let move_a = actions
            .iter()
            .find(|a| a.kind == UnitActionKind::Move)
            .expect("move action");
        assert!(!move_a.enabled);
        let fortify = actions
            .iter()
            .find(|a| a.kind == UnitActionKind::Fortify)
            .expect("fortify action");
        assert!(
            fortify.enabled,
            "fortify is a stance and remains usable without movement"
        );
    }

    #[test]
    fn unknown_unit_yields_empty() {
        let (state, _) = make_state();
        let bogus = UnitId::from_ulid(ulid::Ulid::new());
        assert!(
            DefaultRulesEngine
                .available_unit_actions(&state, bogus)
                .is_empty()
        );
    }
}
