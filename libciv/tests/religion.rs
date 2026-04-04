/// Integration tests for the religion system:
///   - Pantheon founding
///   - Religion founding via Great Prophet
///   - Spread religion via Missionary/Apostle
///   - Theological combat
///   - Faith purchasing
///   - Passive religious pressure (advance_turn phase 3d)
///   - Faith accumulation (advance_turn phase 3e)

mod common;

use common::build_scenario;
use libciv::civ::district::{BuiltinDistrict, PlacedDistrict};
use libciv::civ::BasicUnit;
use libciv::game::diff::StateDelta;
use libciv::game::rules::FaithPurchaseItem;
use libciv::{DefaultRulesEngine, RulesEngine, UnitCategory, UnitDomain};
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn add_district(
    state: &mut libciv::GameState,
    city_id: libciv::CityId,
    district: BuiltinDistrict,
    coord: HexCoord,
) {
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        city.districts.push(district);
    }
    state.placed_districts.push(PlacedDistrict {
        district_type: district,
        city_id,
        coord,
        buildings: Vec::new(),
        is_pillaged: false,
        unique_variant: None,
    });
}

fn grant_faith(state: &mut libciv::GameState, civ_id: libciv::CivId, amount: u32) {
    let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
    civ.faith += amount;
}

/// Add a named building to a city (registers a building def if needed).
fn add_building(state: &mut libciv::GameState, city_id: libciv::CityId, name: &'static str) {
    let bid = state.building_defs.iter()
        .find(|b| b.name == name)
        .map(|b| b.id)
        .unwrap_or_else(|| {
            let id = state.id_gen.next_building_id();
            state.building_defs.push(libciv::game::state::BuildingDef {
                id,
                name,
                cost: 50,
                maintenance: 0,
                yields: libciv::YieldBundle::default(),
                requires_district: None,
                great_work_slots: Vec::new(),
                exclusive_to: None,
                replaces: None,
            });
            id
        });
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        city.buildings.push(bid);
    }
}

fn spawn_great_prophet(state: &mut libciv::GameState, warrior_type: libciv::UnitTypeId, civ_id: libciv::CivId, coord: HexCoord) -> libciv::UnitId {
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: unit_id,
        unit_type: warrior_type, // reuse type id; category override is what matters
        owner: civ_id,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::GreatPerson,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None,
        trade_origin: None,
        trade_destination: None,
        religion_id: None,
        spread_charges: None,
        religious_strength: None,
    });
    unit_id
}

fn spawn_missionary(
    state: &mut libciv::GameState,
    warrior_type: libciv::UnitTypeId,
    civ_id: libciv::CivId,
    coord: HexCoord,
    religion_id: libciv::ReligionId,
) -> libciv::UnitId {
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: unit_id,
        unit_type: warrior_type,
        owner: civ_id,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Religious,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None,
        trade_origin: None,
        trade_destination: None,
        religion_id: Some(religion_id),
        spread_charges: Some(3),
        religious_strength: None,
    });
    unit_id
}

fn spawn_apostle(
    state: &mut libciv::GameState,
    warrior_type: libciv::UnitTypeId,
    civ_id: libciv::CivId,
    coord: HexCoord,
    religion_id: libciv::ReligionId,
) -> libciv::UnitId {
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: unit_id,
        unit_type: warrior_type,
        owner: civ_id,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Religious,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None,
        trade_origin: None,
        trade_destination: None,
        religion_id: Some(religion_id),
        spread_charges: Some(3),
        religious_strength: Some(110),
    });
    unit_id
}

/// Set up Rome with faith, a Holy Site, a Great Prophet, and found a religion.
/// Returns the religion ID.
fn found_rome_religion(s: &mut common::Scenario) -> libciv::ReligionId {
    let rules = DefaultRulesEngine;

    // Give Rome faith and a Holy Site district adjacent to capital.
    grant_faith(&mut s.state, s.rome_id, 100);
    let holy_site_coord = HexCoord::from_qr(4, 3);
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::HolySite, holy_site_coord);

    // Spawn Great Prophet at the Holy Site.
    let prophet = spawn_great_prophet(&mut s.state, s.warrior_type, s.rome_id, holy_site_coord);

    // Select beliefs: one Founder (tithe) + one Follower (divine_inspiration).
    let founder_belief = s.state.belief_refs.tithe;
    let follower_belief = s.state.belief_refs.divine_inspiration;

    let diff = rules
        .found_religion(
            &mut s.state,
            prophet,
            "Civ Faith".to_string(),
            vec![founder_belief, follower_belief],
        )
        .expect("found_religion should succeed");

    // Extract religion ID from diff.
    let religion_id = diff.deltas.iter().find_map(|d| {
        if let StateDelta::ReligionFounded { religion, .. } = d {
            Some(*religion)
        } else {
            None
        }
    }).expect("ReligionFounded delta expected");

    religion_id
}

// ===========================================================================
// Pantheon tests
// ===========================================================================

#[test]
fn found_pantheon_success() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    grant_faith(&mut s.state, s.rome_id, 30);

    let belief = s.state.belief_refs.stone_circles;
    let diff = rules.found_pantheon(&mut s.state, s.rome_id, belief)
        .expect("should succeed");

    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::PantheonFounded { .. })));
    let civ = s.state.civ(s.rome_id).unwrap();
    assert_eq!(civ.pantheon_belief, Some(belief));
    assert_eq!(civ.faith, 5); // 30 - 25
}

#[test]
fn found_pantheon_insufficient_faith() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    grant_faith(&mut s.state, s.rome_id, 20);

    let belief = s.state.belief_refs.stone_circles;
    let result = rules.found_pantheon(&mut s.state, s.rome_id, belief);
    assert!(matches!(result, Err(libciv::game::RulesError::InsufficientFaith)));
}

#[test]
fn found_pantheon_already_founded() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    grant_faith(&mut s.state, s.rome_id, 60);

    let belief = s.state.belief_refs.stone_circles;
    rules.found_pantheon(&mut s.state, s.rome_id, belief).unwrap();

    let belief2 = s.state.belief_refs.desert_folklore;
    let result = rules.found_pantheon(&mut s.state, s.rome_id, belief2);
    assert!(matches!(result, Err(libciv::game::RulesError::PantheonAlreadyFounded)));
}

#[test]
fn found_pantheon_belief_already_taken() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    grant_faith(&mut s.state, s.rome_id, 30);
    grant_faith(&mut s.state, s.babylon_id, 30);

    let belief = s.state.belief_refs.stone_circles;
    rules.found_pantheon(&mut s.state, s.rome_id, belief).unwrap();

    // Babylon tries the same belief.
    let result = rules.found_pantheon(&mut s.state, s.babylon_id, belief);
    assert!(matches!(result, Err(libciv::game::RulesError::InvalidBelief)));
}

// ===========================================================================
// Religion founding tests
// ===========================================================================

#[test]
fn found_religion_success() {
    let mut s = build_scenario();
    let religion_id = found_rome_religion(&mut s);

    // Religion exists in state.
    let religion = s.state.religions.iter().find(|r| r.id == religion_id).unwrap();
    assert_eq!(religion.name, "Civ Faith");
    assert_eq!(religion.founded_by, s.rome_id);
    assert_eq!(religion.beliefs.len(), 2);

    // Civ has founded_religion set.
    let civ = s.state.civ(s.rome_id).unwrap();
    assert_eq!(civ.founded_religion, Some(religion_id));

    // Holy city has followers.
    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert!(city.religious_followers.get(&religion_id).copied().unwrap_or(0) > 0);
}

#[test]
fn found_religion_no_holy_site() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    grant_faith(&mut s.state, s.rome_id, 100);

    // Spawn prophet at capital (no Holy Site there).
    let prophet = spawn_great_prophet(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(3, 3));

    let founder_belief = s.state.belief_refs.tithe;
    let follower_belief = s.state.belief_refs.divine_inspiration;
    let result = rules.found_religion(
        &mut s.state,
        prophet,
        "Test".to_string(),
        vec![founder_belief, follower_belief],
    );
    assert!(matches!(result, Err(libciv::game::RulesError::NoHolySite)));
}

#[test]
fn found_religion_already_founded() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let _rid = found_rome_religion(&mut s);

    // Try founding again.
    let holy_site_coord = HexCoord::from_qr(4, 3);
    let prophet2 = spawn_great_prophet(&mut s.state, s.warrior_type, s.rome_id, holy_site_coord);
    let b1 = s.state.belief_refs.church_property;
    let b2 = s.state.belief_refs.choral_music;
    let result = rules.found_religion(
        &mut s.state,
        prophet2,
        "Second Faith".to_string(),
        vec![b1, b2],
    );
    assert!(matches!(result, Err(libciv::game::RulesError::ReligionAlreadyFounded)));
}

#[test]
fn found_religion_duplicate_name() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let _rid = found_rome_religion(&mut s);

    // Babylon tries founding with the same name.
    grant_faith(&mut s.state, s.babylon_id, 100);
    let holy_site_coord = HexCoord::from_qr(11, 5);
    add_district(&mut s.state, s.babylon_city, BuiltinDistrict::HolySite, holy_site_coord);
    let prophet = spawn_great_prophet(&mut s.state, s.warrior_type, s.babylon_id, holy_site_coord);
    let b1 = s.state.belief_refs.church_property;
    let b2 = s.state.belief_refs.choral_music;

    let result = rules.found_religion(
        &mut s.state,
        prophet,
        "Civ Faith".to_string(),
        vec![b1, b2],
    );
    assert!(matches!(result, Err(libciv::game::RulesError::ReligionNameTaken)));
}

#[test]
fn found_religion_prophet_consumed() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    grant_faith(&mut s.state, s.rome_id, 100);
    let holy_site_coord = HexCoord::from_qr(4, 3);
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::HolySite, holy_site_coord);
    let prophet = spawn_great_prophet(&mut s.state, s.warrior_type, s.rome_id, holy_site_coord);
    let b1 = s.state.belief_refs.tithe;
    let b2 = s.state.belief_refs.divine_inspiration;

    rules.found_religion(
        &mut s.state,
        prophet,
        "Test".to_string(),
        vec![b1, b2],
    ).unwrap();

    // Prophet should be consumed.
    assert!(s.state.unit(prophet).is_none(), "Great Prophet should be consumed after founding");
}

// ===========================================================================
// Spread religion tests
// ===========================================================================

#[test]
fn spread_religion_success() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let religion_id = found_rome_religion(&mut s);

    // Spawn missionary at Babylon's city.
    let missionary = spawn_missionary(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(10, 5), religion_id);

    let diff = rules.spread_religion(&mut s.state, missionary).unwrap();
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::ReligionSpread { .. })));

    // Babylon city should have some followers.
    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert!(city.religious_followers.get(&religion_id).copied().unwrap_or(0) > 0);
}

#[test]
fn spread_religion_decrements_charges() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let religion_id = found_rome_religion(&mut s);

    let missionary = spawn_missionary(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(10, 5), religion_id);

    rules.spread_religion(&mut s.state, missionary).unwrap();
    // Reset movement for the next spread.
    if let Some(u) = s.state.units.iter_mut().find(|u| u.id == missionary) {
        u.movement_left = 200;
    }

    let unit = s.state.unit(missionary).unwrap();
    assert_eq!(unit.spread_charges, Some(2));
}

#[test]
fn spread_religion_unit_destroyed_at_zero_charges() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let religion_id = found_rome_religion(&mut s);

    // Spawn with 1 charge only.
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: unit_id,
        unit_type: s.warrior_type,
        owner: s.rome_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land,
        category: UnitCategory::Religious,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None,
        trade_origin: None,
        trade_destination: None,
        religion_id: Some(religion_id),
        spread_charges: Some(1),
        religious_strength: None,
    });

    let diff = rules.spread_religion(&mut s.state, unit_id).unwrap();
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitDestroyed { .. })));
    assert!(s.state.unit(unit_id).is_none(), "unit should be destroyed at zero charges");
}

#[test]
fn spread_religion_not_on_city() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let religion_id = found_rome_religion(&mut s);

    // Missionary in the middle of nowhere.
    let missionary = spawn_missionary(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(7, 4), religion_id);
    let result = rules.spread_religion(&mut s.state, missionary);
    assert!(matches!(result, Err(libciv::game::RulesError::CityNotFound)));
}

// ===========================================================================
// Theological combat tests
// ===========================================================================

#[test]
fn theological_combat_success() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let rome_religion = found_rome_religion(&mut s);

    // Found Babylon's religion too.
    grant_faith(&mut s.state, s.babylon_id, 100);
    let bab_hs_coord = HexCoord::from_qr(11, 5);
    add_district(&mut s.state, s.babylon_city, BuiltinDistrict::HolySite, bab_hs_coord);
    let bab_prophet = spawn_great_prophet(&mut s.state, s.warrior_type, s.babylon_id, bab_hs_coord);
    let b1 = s.state.belief_refs.church_property;
    let b2 = s.state.belief_refs.choral_music;
    let bab_diff = rules.found_religion(
        &mut s.state,
        bab_prophet,
        "Bab Faith".to_string(),
        vec![b1, b2],
    ).unwrap();
    let bab_religion = bab_diff.deltas.iter().find_map(|d| {
        if let StateDelta::ReligionFounded { religion, .. } = d { Some(*religion) } else { None }
    }).unwrap();

    // Place apostles adjacent to each other.
    let rome_apostle = spawn_apostle(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(6, 3), rome_religion);
    let bab_apostle = spawn_apostle(&mut s.state, s.warrior_type, s.babylon_id, HexCoord::from_qr(7, 3), bab_religion);

    let diff = rules.theological_combat(&mut s.state, rome_apostle, bab_apostle).unwrap();
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::TheologicalCombat { .. })));
}

#[test]
fn theological_combat_not_adjacent() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let rome_religion = found_rome_religion(&mut s);

    // Two apostles far apart.
    let a1 = spawn_apostle(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(3, 3), rome_religion);
    let a2 = spawn_apostle(&mut s.state, s.warrior_type, s.babylon_id, HexCoord::from_qr(10, 5), rome_religion);

    let result = rules.theological_combat(&mut s.state, a1, a2);
    assert!(matches!(result, Err(libciv::game::RulesError::NotInRange)));
}

#[test]
fn theological_combat_same_civ() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let rome_religion = found_rome_religion(&mut s);

    let a1 = spawn_apostle(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(6, 3), rome_religion);
    let a2 = spawn_apostle(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(7, 3), rome_religion);

    let result = rules.theological_combat(&mut s.state, a1, a2);
    assert!(matches!(result, Err(libciv::game::RulesError::SameCivilization)));
}

#[test]
fn theological_combat_non_religious_unit() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let rome_religion = found_rome_religion(&mut s);

    let apostle = spawn_apostle(&mut s.state, s.warrior_type, s.rome_id, HexCoord::from_qr(6, 3), rome_religion);

    // Try combat with a warrior (non-religious).
    // Move Rome's warrior adjacent.
    s.state.units.iter_mut()
        .find(|u| u.id == s.babylon_warrior).unwrap()
        .coord = HexCoord::from_qr(7, 3);

    let result = rules.theological_combat(&mut s.state, apostle, s.babylon_warrior);
    assert!(matches!(result, Err(libciv::game::RulesError::NotAReligiousUnit)));
}

// ===========================================================================
// Faith purchase tests
// ===========================================================================

#[test]
fn purchase_missionary_with_faith() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let _religion_id = found_rome_religion(&mut s);

    // Need a Missionary unit type in the registry.
    let missionary_type_id = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(libciv::game::state::UnitTypeDef {
        id: missionary_type_id,
        name: "Missionary",
        production_cost: 0,
        max_movement: 200,
        combat_strength: None,
        domain: UnitDomain::Land,
        category: UnitCategory::Religious,
        range: 0,
        vision_range: 2,
        can_found_city: false,
        resource_cost: None,
        siege_bonus: 0,
        max_charges: 0,
        exclusive_to: None,
        replaces: None, era: None, promotion_class: None,
    });

    // Missionary requires a Shrine building.
    add_building(&mut s.state, s.rome_city, "Shrine");

    // Give Rome faith (250 base cost for Missionary).
    grant_faith(&mut s.state, s.rome_id, 300);

    let faith_before = s.state.civ(s.rome_id).unwrap().faith;

    let diff = rules.purchase_with_faith(
        &mut s.state,
        s.rome_id,
        s.rome_city,
        FaithPurchaseItem::Unit("Missionary"),
    ).unwrap();

    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitCreated { .. })));
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::FaithChanged { .. })));

    let civ = s.state.civ(s.rome_id).unwrap();
    assert!(civ.faith < faith_before, "faith should have been spent");
}

#[test]
fn purchase_with_insufficient_faith() {
    let mut s = build_scenario();
    let rules = DefaultRulesEngine;
    let _religion_id = found_rome_religion(&mut s);

    // Register Missionary type.
    let missionary_type_id = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(libciv::game::state::UnitTypeDef {
        id: missionary_type_id,
        name: "Missionary",
        production_cost: 0,
        max_movement: 200,
        combat_strength: None,
        domain: UnitDomain::Land,
        category: UnitCategory::Religious,
        range: 0,
        vision_range: 2,
        can_found_city: false,
        resource_cost: None,
        siege_bonus: 0,
        max_charges: 0,
        exclusive_to: None,
        replaces: None, era: None, promotion_class: None,
    });

    // No extra faith (founding spent the 100 we gave, leaving whatever is left).
    // Set faith to 0 explicitly.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().faith = 0;

    let result = rules.purchase_with_faith(
        &mut s.state,
        s.rome_id,
        s.rome_city,
        FaithPurchaseItem::Unit("Missionary"),
    );
    assert!(matches!(result, Err(libciv::game::RulesError::InsufficientFaith)));
}

// ===========================================================================
// Religion data model tests
// ===========================================================================

#[test]
fn religion_total_followers() {
    let mut s = build_scenario();
    let religion_id = found_rome_religion(&mut s);

    let religion = s.state.religions.iter().find(|r| r.id == religion_id).unwrap();
    let total = religion.total_followers(&s.state.cities);
    assert!(total > 0, "holy city should have followers");
}

#[test]
fn religion_majority_cities() {
    let mut s = build_scenario();
    let religion_id = found_rome_religion(&mut s);

    let religion = s.state.religions.iter().find(|r| r.id == religion_id).unwrap();
    let majority = religion.majority_cities(&s.state.cities);
    // Holy city should be majority.
    assert!(majority.contains(&s.rome_city), "holy city should have majority");
}

#[test]
fn city_majority_religion() {
    let mut s = build_scenario();
    let religion_id = found_rome_religion(&mut s);

    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert_eq!(city.majority_religion(), Some(religion_id));
}

#[test]
fn religion_is_enhanced() {
    let mut s = build_scenario();
    let religion_id = found_rome_religion(&mut s);

    let religion = s.state.religions.iter().find(|r| r.id == religion_id).unwrap();
    // Initially has Founder + Follower, not enhanced.
    assert!(!religion.is_enhanced(&s.state.belief_defs));

    // Add an Enhancer belief.
    let enhancer = s.state.belief_refs.missionary_zeal;
    let religion = s.state.religions.iter_mut().find(|r| r.id == religion_id).unwrap();
    religion.beliefs.push(enhancer);
    assert!(religion.is_enhanced(&s.state.belief_defs));
}

#[test]
fn belief_defs_loaded() {
    let s = build_scenario();
    // Verify that beliefs were loaded into state.
    assert!(!s.state.belief_defs.is_empty(), "belief_defs should be populated");
    assert!(s.state.belief_defs.len() >= 38, "at least 38 built-in beliefs (18 original + 20 pantheon)");
}

// ===========================================================================
// Pantheon adjacency bonus tests
// ===========================================================================

use libciv::rules::modifier::{
    Condition, ConditionContext, ConditionResult, evaluate_condition,
};
use libciv::world::terrain::BuiltinTerrain;
use libciv::world::feature::BuiltinFeature;
use libhexgrid::board::HexBoard;

/// Helper: set the terrain of a tile on the board.
fn set_terrain(state: &mut libciv::GameState, coord: HexCoord, terrain: BuiltinTerrain) {
    if let Some(tile) = state.board.tile_mut(coord) {
        tile.terrain = terrain;
    }
}

/// Helper: set the feature of a tile on the board.
fn set_feature(state: &mut libciv::GameState, coord: HexCoord, feature: BuiltinFeature) {
    if let Some(tile) = state.board.tile_mut(coord) {
        tile.feature = Some(feature);
    }
}

#[test]
fn per_adjacent_terrain_counts_neighbors_not_self() {
    let mut s = build_scenario();

    // Place the Holy Site at (4,3).
    let holy_site_coord = HexCoord::from_qr(4, 3);

    // Set the Holy Site tile itself to Tundra — this should NOT count.
    set_terrain(&mut s.state, holy_site_coord, BuiltinTerrain::Tundra);

    // Set 3 of the 6 neighbors to Tundra.
    let neighbors = s.state.board.neighbors(holy_site_coord);
    assert!(neighbors.len() >= 3, "need at least 3 neighbors");
    for &nb in &neighbors[..3] {
        set_terrain(&mut s.state, nb, BuiltinTerrain::Tundra);
    }
    // Remaining neighbors stay as default (Grassland).

    // Evaluate the condition.
    let ctx = ConditionContext {
        civ_id: s.rome_id,
        state: &s.state,
        tile: Some(holy_site_coord),
        unit_id: None,
        city_id: None,
    };
    let result = evaluate_condition(
        &Condition::PerAdjacentTerrain(BuiltinTerrain::Tundra),
        &ctx,
    );
    // Should count exactly 3 neighbors, NOT the tile itself.
    assert_eq!(result, ConditionResult::Scale(3));
}

#[test]
fn per_adjacent_terrain_zero_when_no_match() {
    let s = build_scenario();

    // Default terrain is Grassland; check for Tundra adjacency at (4,3).
    let coord = HexCoord::from_qr(4, 3);
    let ctx = ConditionContext {
        civ_id: s.rome_id,
        state: &s.state,
        tile: Some(coord),
        unit_id: None,
        city_id: None,
    };
    let result = evaluate_condition(
        &Condition::PerAdjacentTerrain(BuiltinTerrain::Tundra),
        &ctx,
    );
    assert_eq!(result, ConditionResult::Scale(0));
}

#[test]
fn per_adjacent_feature_counts_rainforest() {
    let mut s = build_scenario();

    let holy_site_coord = HexCoord::from_qr(4, 3);
    let neighbors = s.state.board.neighbors(holy_site_coord);

    // Set 2 neighbors to have Rainforest feature.
    for &nb in &neighbors[..2] {
        set_feature(&mut s.state, nb, BuiltinFeature::Rainforest);
    }

    // Set the Holy Site tile itself to Rainforest — should NOT count.
    set_feature(&mut s.state, holy_site_coord, BuiltinFeature::Rainforest);

    let ctx = ConditionContext {
        civ_id: s.rome_id,
        state: &s.state,
        tile: Some(holy_site_coord),
        unit_id: None,
        city_id: None,
    };
    let result = evaluate_condition(
        &Condition::PerAdjacentFeature(BuiltinFeature::Rainforest),
        &ctx,
    );
    // Should count exactly 2, not including the tile itself.
    assert_eq!(result, ConditionResult::Scale(2));
}

#[test]
fn dance_of_the_aurora_belief_has_tundra_condition() {
    let s = build_scenario();

    // Find the "Dance of the Aurora" belief and verify it has the right condition.
    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "Dance of the Aurora")
        .expect("Dance of the Aurora should exist");

    assert_eq!(belief.modifiers.len(), 1, "should have exactly one modifier");
    let modifier = &belief.modifiers[0];
    assert_eq!(
        modifier.condition,
        Some(Condition::PerAdjacentTerrain(BuiltinTerrain::Tundra)),
        "modifier should scale by adjacent Tundra tiles"
    );
}

#[test]
fn desert_folklore_belief_has_desert_condition() {
    let s = build_scenario();

    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "Desert Folklore")
        .expect("Desert Folklore should exist");

    assert_eq!(belief.modifiers.len(), 1);
    let modifier = &belief.modifiers[0];
    assert_eq!(
        modifier.condition,
        Some(Condition::PerAdjacentTerrain(BuiltinTerrain::Desert)),
        "modifier should scale by adjacent Desert tiles"
    );
}

#[test]
fn sacred_path_belief_has_rainforest_condition() {
    let s = build_scenario();

    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "Sacred Path")
        .expect("Sacred Path should exist");

    assert_eq!(belief.modifiers.len(), 1);
    let modifier = &belief.modifiers[0];
    assert_eq!(
        modifier.condition,
        Some(Condition::PerAdjacentFeature(BuiltinFeature::Rainforest)),
        "modifier should scale by adjacent Rainforest tiles"
    );
}

#[test]
fn per_adjacent_terrain_all_six_neighbors() {
    let mut s = build_scenario();

    // Use a central tile with all 6 neighbors on the board.
    let coord = HexCoord::from_qr(6, 4);
    let neighbors = s.state.board.neighbors(coord);
    assert_eq!(neighbors.len(), 6, "interior tile should have 6 neighbors");

    // Set all 6 neighbors to Desert.
    for &nb in &neighbors {
        set_terrain(&mut s.state, nb, BuiltinTerrain::Desert);
    }

    let ctx = ConditionContext {
        civ_id: s.rome_id,
        state: &s.state,
        tile: Some(coord),
        unit_id: None,
        city_id: None,
    };
    let result = evaluate_condition(
        &Condition::PerAdjacentTerrain(BuiltinTerrain::Desert),
        &ctx,
    );
    assert_eq!(result, ConditionResult::Scale(6));
}

// ===========================================================================
// Tile-level condition tests
// ===========================================================================

use libciv::world::improvement::BuiltinImprovement;
use libciv::world::resource::BuiltinResource;
use libciv::ResourceCategory;

/// Helper: set improvement on a tile.
fn set_improvement(state: &mut libciv::GameState, coord: HexCoord, imp: BuiltinImprovement) {
    if let Some(tile) = state.board.tile_mut(coord) {
        tile.improvement = Some(imp);
    }
}

/// Helper: set resource on a tile.
fn set_resource(state: &mut libciv::GameState, coord: HexCoord, res: BuiltinResource) {
    if let Some(tile) = state.board.tile_mut(coord) {
        tile.resource = Some(res);
    }
}

#[test]
fn tile_has_improvement_passes_when_present() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);
    set_improvement(&mut s.state, coord, BuiltinImprovement::Mine);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    assert_eq!(
        evaluate_condition(&Condition::TileHasImprovement(BuiltinImprovement::Mine), &ctx),
        ConditionResult::Pass,
    );
}

#[test]
fn tile_has_improvement_fails_wrong_type() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);
    set_improvement(&mut s.state, coord, BuiltinImprovement::Farm);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    assert_eq!(
        evaluate_condition(&Condition::TileHasImprovement(BuiltinImprovement::Mine), &ctx),
        ConditionResult::Fail,
    );
}

#[test]
fn tile_has_any_improvement_passes() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);
    set_improvement(&mut s.state, coord, BuiltinImprovement::Pasture);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    assert_eq!(
        evaluate_condition(&Condition::TileHasAnyImprovement, &ctx),
        ConditionResult::Pass,
    );
}

#[test]
fn tile_has_any_improvement_fails_when_empty() {
    let s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    assert_eq!(
        evaluate_condition(&Condition::TileHasAnyImprovement, &ctx),
        ConditionResult::Fail,
    );
}

#[test]
fn tile_has_resource_of_category_passes() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);
    set_resource(&mut s.state, coord, BuiltinResource::Iron); // Strategic

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    assert_eq!(
        evaluate_condition(&Condition::TileHasResourceOfCategory(ResourceCategory::Strategic), &ctx),
        ConditionResult::Pass,
    );
}

#[test]
fn tile_has_resource_of_category_fails_wrong_category() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);
    set_resource(&mut s.state, coord, BuiltinResource::Wheat); // Bonus, not Strategic

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    assert_eq!(
        evaluate_condition(&Condition::TileHasResourceOfCategory(ResourceCategory::Strategic), &ctx),
        ConditionResult::Fail,
    );
}

#[test]
fn tile_has_feature_passes() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);
    set_feature(&mut s.state, coord, BuiltinFeature::Marsh);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    assert_eq!(
        evaluate_condition(&Condition::TileHasFeature(BuiltinFeature::Marsh), &ctx),
        ConditionResult::Pass,
    );
}

#[test]
fn and_condition_both_required() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    // Set up: Mine over Iron (strategic resource).
    set_improvement(&mut s.state, coord, BuiltinImprovement::Mine);
    set_resource(&mut s.state, coord, BuiltinResource::Iron);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };

    // And(Mine, Strategic) should pass.
    let cond = Condition::And(
        Box::new(Condition::TileHasImprovement(BuiltinImprovement::Mine)),
        Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Strategic)),
    );
    assert_eq!(evaluate_condition(&cond, &ctx), ConditionResult::Pass);
}

#[test]
fn and_condition_fails_when_one_missing() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    // Only mine, no resource — And(Mine, Strategic) should fail.
    set_improvement(&mut s.state, coord, BuiltinImprovement::Mine);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    let cond = Condition::And(
        Box::new(Condition::TileHasImprovement(BuiltinImprovement::Mine)),
        Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Strategic)),
    );
    assert_eq!(evaluate_condition(&cond, &ctx), ConditionResult::Fail);
}

#[test]
fn or_condition_passes_on_either() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    // Set luxury resource, no bonus resource.
    set_resource(&mut s.state, coord, BuiltinResource::Wine); // Luxury

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };

    // Or(Luxury, Bonus) should pass because Luxury is present.
    let cond = Condition::Or(
        Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Luxury)),
        Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Bonus)),
    );
    assert_eq!(evaluate_condition(&cond, &ctx), ConditionResult::Pass);
}

#[test]
fn or_condition_fails_when_neither() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    // Set strategic resource — neither Luxury nor Bonus.
    set_resource(&mut s.state, coord, BuiltinResource::Iron);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };
    let cond = Condition::Or(
        Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Luxury)),
        Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Bonus)),
    );
    assert_eq!(evaluate_condition(&cond, &ctx), ConditionResult::Fail);
}

#[test]
fn religious_idols_compound_condition() {
    // Religious Idols: And(Mine, Or(Luxury, Bonus))
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    // Mine over Wine (Luxury) → should pass.
    set_improvement(&mut s.state, coord, BuiltinImprovement::Mine);
    set_resource(&mut s.state, coord, BuiltinResource::Wine);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };

    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "Religious Idols")
        .expect("Religious Idols should exist");
    assert_eq!(belief.modifiers.len(), 1, "should have one compound modifier");

    let cond = belief.modifiers[0].condition.as_ref().expect("should have condition");
    assert_eq!(evaluate_condition(cond, &ctx), ConditionResult::Pass);
}

#[test]
fn religious_idols_fails_mine_over_strategic() {
    // Mine over Iron (Strategic) → should fail.
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);
    set_improvement(&mut s.state, coord, BuiltinImprovement::Mine);
    set_resource(&mut s.state, coord, BuiltinResource::Iron);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };

    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "Religious Idols")
        .expect("Religious Idols should exist");
    let cond = belief.modifiers[0].condition.as_ref().expect("should have condition");
    assert_eq!(evaluate_condition(cond, &ctx), ConditionResult::Fail);
}

#[test]
fn god_of_craftsmen_requires_improved_strategic() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    // Iron + Mine → should pass.
    set_resource(&mut s.state, coord, BuiltinResource::Iron);
    set_improvement(&mut s.state, coord, BuiltinImprovement::Mine);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };

    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "God of Craftsmen")
        .expect("God of Craftsmen should exist");
    let cond = belief.modifiers[0].condition.as_ref().expect("should have condition");
    assert_eq!(evaluate_condition(cond, &ctx), ConditionResult::Pass);
}

#[test]
fn god_of_craftsmen_fails_unimproved_strategic() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(4, 3);

    // Iron but no improvement → should fail.
    set_resource(&mut s.state, coord, BuiltinResource::Iron);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };

    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "God of Craftsmen")
        .expect("God of Craftsmen should exist");
    let cond = belief.modifiers[0].condition.as_ref().expect("should have condition");
    assert_eq!(evaluate_condition(cond, &ctx), ConditionResult::Fail);
}

// ===========================================================================
// Appeal, era, and production queue condition tests
// ===========================================================================

use libciv::rules::modifier::compute_tile_appeal;
use libciv::AgeType;

#[test]
fn appeal_increases_from_adjacent_mountains() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(6, 4);
    let neighbors = s.state.board.neighbors(coord);

    // Set 2 neighbors to Mountain.
    for &nb in &neighbors[..2] {
        set_terrain(&mut s.state, nb, BuiltinTerrain::Mountain);
    }

    let appeal = compute_tile_appeal(coord, &s.state);
    assert_eq!(appeal, 2, "2 adjacent mountains = +2 appeal");
}

#[test]
fn appeal_decreases_from_adjacent_marsh() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(6, 4);
    let neighbors = s.state.board.neighbors(coord);

    // Set 3 neighbors to have Marsh feature.
    for &nb in &neighbors[..3] {
        set_feature(&mut s.state, nb, BuiltinFeature::Marsh);
    }

    let appeal = compute_tile_appeal(coord, &s.state);
    assert_eq!(appeal, -3, "3 adjacent marshes = -3 appeal");
}

#[test]
fn appeal_mixed_positive_and_negative() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(6, 4);
    let neighbors = s.state.board.neighbors(coord);

    // 1 Mountain (+1), 1 Forest (+1), 1 Rainforest (-1) = net +1
    set_terrain(&mut s.state, neighbors[0], BuiltinTerrain::Mountain);
    set_feature(&mut s.state, neighbors[1], BuiltinFeature::Forest);
    set_feature(&mut s.state, neighbors[2], BuiltinFeature::Rainforest);

    let appeal = compute_tile_appeal(coord, &s.state);
    assert_eq!(appeal, 1, "Mountain + Forest - Rainforest = +1");
}

#[test]
fn tile_min_appeal_condition_charming() {
    let mut s = build_scenario();
    let coord = HexCoord::from_qr(6, 4);
    let neighbors = s.state.board.neighbors(coord);

    // Set 2 Mountains → appeal 2 (Charming threshold).
    set_terrain(&mut s.state, neighbors[0], BuiltinTerrain::Mountain);
    set_terrain(&mut s.state, neighbors[1], BuiltinTerrain::Mountain);

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: Some(coord), unit_id: None, city_id: None,
    };

    // Charming (2+) should pass.
    assert_eq!(
        evaluate_condition(&Condition::TileMinAppeal(2), &ctx),
        ConditionResult::Pass,
    );
    // Breathtaking (4+) should fail.
    assert_eq!(
        evaluate_condition(&Condition::TileMinAppeal(4), &ctx),
        ConditionResult::Fail,
    );
}

#[test]
fn earth_goddess_belief_has_appeal_condition() {
    let s = build_scenario();
    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "Earth Goddess")
        .expect("Earth Goddess should exist");
    assert_eq!(belief.modifiers.len(), 1);
    assert_eq!(
        belief.modifiers[0].condition,
        Some(Condition::TileMinAppeal(2)),
    );
}

#[test]
fn god_of_the_forge_belief_has_era_condition() {
    let s = build_scenario();
    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "God of the Forge")
        .expect("God of the Forge should exist");
    assert_eq!(belief.modifiers.len(), 1);
    let cond = belief.modifiers[0].condition.as_ref().expect("should have condition");
    assert!(matches!(cond, Condition::Or(_, _)));
}

#[test]
fn monument_to_the_gods_belief_has_wonder_era_condition() {
    let s = build_scenario();
    let belief = s.state.belief_defs.iter()
        .find(|b| b.name == "Monument to the Gods")
        .expect("Monument to the Gods should exist");
    assert_eq!(belief.modifiers.len(), 1);
    let cond = belief.modifiers[0].condition.as_ref().expect("should have condition");
    assert!(matches!(cond, Condition::Or(_, _)));
}

#[test]
fn producing_military_unit_of_era_passes() {
    let mut s = build_scenario();

    // Register a warrior unit type with era = Ancient.
    let warrior_type_id = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(libciv::game::state::UnitTypeDef {
        id: warrior_type_id,
        name: "TestWarrior",
        production_cost: 40,
        max_movement: 200,
        combat_strength: Some(20),
        domain: UnitDomain::Land,
        category: UnitCategory::Combat,
        range: 0,
        vision_range: 2,
        can_found_city: false,
        resource_cost: None,
        siege_bonus: 0,
        max_charges: 0,
        exclusive_to: None,
        replaces: None,
        era: Some(AgeType::Ancient),
        promotion_class: None,
    });

    // Set Rome's production queue to produce this warrior.
    if let Some(city) = s.state.cities.iter_mut().find(|c| c.id == s.rome_city) {
        city.production_queue.push_back(libciv::civ::ProductionItem::Unit(warrior_type_id));
    }

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: None, unit_id: None, city_id: Some(s.rome_city),
    };

    assert_eq!(
        evaluate_condition(&Condition::ProducingMilitaryUnitOfEra(AgeType::Ancient), &ctx),
        ConditionResult::Pass,
    );
    // Wrong era should fail.
    assert_eq!(
        evaluate_condition(&Condition::ProducingMilitaryUnitOfEra(AgeType::Medieval), &ctx),
        ConditionResult::Fail,
    );
}

#[test]
fn producing_military_unit_fails_for_civilian() {
    let mut s = build_scenario();

    // Register a settler (Civilian) with era = Ancient.
    let settler_type_id = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(libciv::game::state::UnitTypeDef {
        id: settler_type_id,
        name: "TestSettler",
        production_cost: 80,
        max_movement: 200,
        combat_strength: None,
        domain: UnitDomain::Land,
        category: UnitCategory::Civilian,
        range: 0,
        vision_range: 2,
        can_found_city: true,
        resource_cost: None,
        siege_bonus: 0,
        max_charges: 0,
        exclusive_to: None,
        replaces: None,
        era: Some(AgeType::Ancient),
        promotion_class: None,
    });

    if let Some(city) = s.state.cities.iter_mut().find(|c| c.id == s.rome_city) {
        city.production_queue.push_back(libciv::civ::ProductionItem::Unit(settler_type_id));
    }

    let ctx = ConditionContext {
        civ_id: s.rome_id, state: &s.state,
        tile: None, unit_id: None, city_id: Some(s.rome_city),
    };

    // Settler is Civilian, not Combat/Support — should fail.
    assert_eq!(
        evaluate_condition(&Condition::ProducingMilitaryUnitOfEra(AgeType::Ancient), &ctx),
        ConditionResult::Fail,
    );
}
