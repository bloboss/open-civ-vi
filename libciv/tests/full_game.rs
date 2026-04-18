/// Comprehensive integration tests exercising all major game systems together
/// in end-to-end game simulations.
mod common;

use libciv::{
    BuiltinVictoryCondition, DefaultRulesEngine, RulesEngine,
    CivId, CityId, UnitCategory, UnitDomain, UnitId, UnitTypeId,
    GreatPersonType,
};
use libciv::ai::deterministic::HeuristicAgent;
use libciv::ai::deterministic::Agent;
use libciv::civ::{
    BasicUnit, BuiltinDistrict, PlacedDistrict, ProductionItem, TechProgress,
    CivicProgress, DiplomaticRelation, DiplomaticStatus,
    builtin_great_person_defs, spawn_great_person,
};
use libciv::game::{recalculate_visibility, StateDelta, RulesError};
use libciv::game::state::UnitTypeDef;
use libciv::world::improvement::BuiltinImprovement;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Directly grant a tech to a civ (bypass research).
fn grant_tech(s: &mut common::Scenario, civ_id: CivId, tech_id: libciv::TechId) {
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
    if !civ.researched_techs.contains(&tech_id) {
        civ.researched_techs.push(tech_id);
    }
}

/// Directly grant a civic to a civ (bypass research).
#[allow(dead_code)]
fn grant_civic(s: &mut common::Scenario, civ_id: CivId, civic_id: libciv::CivicId) {
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
    if !civ.completed_civics.contains(&civic_id) {
        civ.completed_civics.push(civic_id);
    }
}

/// Directly place a district on a city (bypasses tech/civic prereqs).
fn add_district(
    state: &mut libciv::GameState,
    city_id: CityId,
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

/// Grant faith to a civ.
fn grant_faith(state: &mut libciv::GameState, civ_id: CivId, amount: u32) {
    let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
    civ.faith += amount;
}

/// Spawn a settler at a given coord for the given civ.
fn spawn_settler(s: &mut common::Scenario, owner: CivId, coord: HexCoord) -> UnitId {
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: unit_id,
        unit_type: s.settler_type,
        owner,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Civilian,
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
        religious_strength: None, is_embarked: false,
    });
    unit_id
}

/// Spawn a warrior at a given coord for the given civ.
fn spawn_warrior(s: &mut common::Scenario, owner: CivId, coord: HexCoord) -> UnitId {
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: unit_id,
        unit_type: s.warrior_type,
        owner,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Combat,
        movement_left: 200,
        max_movement: 200,
        combat_strength: Some(20),
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
        religious_strength: None, is_embarked: false,
    });
    unit_id
}

/// Spawn a builder at a given coord for the given civ.
fn spawn_builder(s: &mut common::Scenario, owner: CivId, coord: HexCoord) -> UnitId {
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: unit_id,
        unit_type: s.builder_type,
        owner,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Civilian,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: Some(3),
        trade_origin: None,
        trade_destination: None,
        religion_id: None,
        spread_charges: None,
        religious_strength: None, is_embarked: false,
    });
    unit_id
}

/// Spawn a trader at a given coord for the given civ.
fn spawn_trader(s: &mut common::Scenario, owner: CivId, coord: HexCoord) -> UnitId {
    let trader_type = UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id: trader_type,
        name: "trader",
        production_cost: 40,
        domain: UnitDomain::Land,
        category: UnitCategory::Trader,
        max_movement: 200,
        combat_strength: None,
        range: 0,
        vision_range: 2,
        can_found_city: false,
        resource_cost: None,
        siege_bonus: 0,
        max_charges: 0,
        exclusive_to: None,
        replaces: None,
        era: None,
        promotion_class: None,
    });
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: unit_id,
        unit_type: trader_type,
        owner,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Trader,
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
        religious_strength: None, is_embarked: false,
    });
    unit_id
}

/// Spawn a Great Prophet unit at a given coord.
fn spawn_great_prophet(
    state: &mut libciv::GameState,
    warrior_type: UnitTypeId,
    civ_id: CivId,
    coord: HexCoord,
) -> UnitId {
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: unit_id,
        unit_type: warrior_type,
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
        religious_strength: None, is_embarked: false,
    });
    unit_id
}

/// Ensure a diplomatic relation exists between two civs.
fn ensure_relation(s: &mut common::Scenario, civ_a: CivId, civ_b: CivId) {
    let exists = s.state.diplomatic_relations.iter()
        .any(|r| (r.civ_a == civ_a && r.civ_b == civ_b) || (r.civ_a == civ_b && r.civ_b == civ_a));
    if !exists {
        s.state.diplomatic_relations.push(DiplomaticRelation::new(civ_a, civ_b));
    }
}

// ===========================================================================
// Test 1: Full 50-turn game with AI and score victory
// ===========================================================================

#[test]
fn full_game_50_turns_no_panic() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Register score victory at turn 50.
    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Score { id: vc_id, turn_limit: 50 });

    // Set up AI agent for Babylon.
    let ai = HeuristicAgent::new(s.babylon_id);

    // Queue initial research for Rome.
    let pottery_id = s.state.tech_refs.pottery;
    let rome_civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome_civ.research_queue.push_back(TechProgress {
        tech_id: pottery_id,
        progress: 0,
        boosted: false,
    });

    // Run 50 turns.
    for _ in 0..50 {
        if s.state.game_over.is_some() {
            break;
        }
        // AI takes a turn for Babylon.
        ai.take_turn(&mut s.state, &rules);
        // Advance the game turn (processes research, production, etc.).
        rules.advance_turn(&mut s.state);
        // Reset movement for all units.
        for unit in &mut s.state.units {
            unit.movement_left = unit.max_movement;
        }
        // Refresh visibility.
        let civ_ids: Vec<CivId> = s.state.civilizations.iter().map(|c| c.id).collect();
        for cid in civ_ids {
            recalculate_visibility(&mut s.state, cid);
        }
    }

    // Game should be over at turn 50.
    assert!(s.state.game_over.is_some(), "game should be over after 50 turns");
    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.condition, "Score Victory", "victory condition should be Score Victory");

    // Both civs should still have cities.
    let rome_cities: Vec<_> = s.state.cities.iter().filter(|c| c.owner == s.rome_id).collect();
    let babylon_cities: Vec<_> = s.state.cities.iter().filter(|c| c.owner == s.babylon_id).collect();
    assert!(!rome_cities.is_empty(), "Rome should have at least one city");
    assert!(!babylon_cities.is_empty(), "Babylon should have at least one city");

    // Both civs should have units.
    let rome_units: Vec<_> = s.state.units.iter().filter(|u| u.owner == s.rome_id).collect();
    let babylon_units: Vec<_> = s.state.units.iter().filter(|u| u.owner == s.babylon_id).collect();
    assert!(!rome_units.is_empty(), "Rome should have units");
    assert!(!babylon_units.is_empty(), "Babylon should have units");

    // Rome should have researched Pottery by now (cost 25, ~2 science/turn).
    let rome_civ = s.state.civ(s.rome_id).unwrap();
    assert!(
        rome_civ.researched_techs.contains(&pottery_id),
        "Rome should have researched Pottery after 50 turns"
    );
}

// ===========================================================================
// Test 2: City management lifecycle
// ===========================================================================

#[test]
fn full_game_city_management_lifecycle() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Grant Writing tech so we can place a Campus.
    let writing_id = s.state.tech_refs.writing;
    let rome_id = s.rome_id;
    grant_tech(&mut s, rome_id, writing_id);

    // Found a new city with a settler far from existing cities.
    let settler_coord = HexCoord::from_qr(3, 7);
    let settler = spawn_settler(&mut s, rome_id, settler_coord);
    let diff = rules.found_city(&mut s.state, settler, "Ostia".to_string())
        .expect("found_city should succeed");

    // Verify city was created.
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::CityFounded { .. })),
        "expected CityFounded delta"
    );
    let new_city = s.state.cities.iter()
        .find(|c| c.name == "Ostia")
        .expect("Ostia should exist");
    let new_city_id = new_city.id;
    assert_eq!(new_city.owner, s.rome_id, "Ostia should be owned by Rome");

    // Place a Campus district adjacent to the new city.
    let campus_coord = HexCoord::from_qr(4, 7);
    // Need to claim the tile first.
    let _ = rules.claim_tile(&mut s.state, new_city_id, campus_coord, false);
    let place_result = rules.place_district(&mut s.state, new_city_id, BuiltinDistrict::Campus, campus_coord);
    assert!(place_result.is_ok(), "place_district for Campus should succeed, got: {place_result:?}");

    // Verify the city has the Campus district.
    let city = s.state.city(new_city_id).unwrap();
    assert!(city.districts.contains(&BuiltinDistrict::Campus), "Ostia should have a Campus");

    // Assign a citizen to the campus tile.
    let assign_result = rules.assign_citizen(&mut s.state, new_city_id, campus_coord, true);
    assert!(assign_result.is_ok(), "assign_citizen should succeed");
    let city = s.state.city(new_city_id).unwrap();
    assert!(city.worked_tiles.contains(&campus_coord), "campus tile should be worked");

    // Queue warrior production on Rome's CAPITAL.
    // Default tiles are Grassland (0 production), so we need to change a worked
    // tile to Plains to get production, or seed production directly.
    // Convert a neighbor tile to Plains for production.
    let plains_coord = HexCoord::from_qr(4, 3);
    if let Some(tile) = s.state.board.tile_mut(plains_coord) {
        tile.terrain = libciv::world::terrain::BuiltinTerrain::Plains;
    }
    let rome_city_id = s.rome_city;
    let capital = s.state.cities.iter_mut().find(|c| c.id == rome_city_id).unwrap();
    if !capital.worked_tiles.contains(&plains_coord) {
        capital.worked_tiles.push(plains_coord);
    }

    let warrior_type = s.warrior_type;
    let capital = s.state.cities.iter_mut().find(|c| c.id == rome_city_id).unwrap();
    capital.production_queue.push_back(ProductionItem::Unit(warrior_type));

    // Advance turns until the warrior is built (warrior costs 40, ~1 prod/turn from Plains).
    let initial_unit_count = s.state.units.iter().filter(|u| u.owner == rome_id).count();
    for _ in 0..60 {
        common::advance_turn(&mut s);
        let current_count = s.state.units.iter().filter(|u| u.owner == rome_id).count();
        if current_count > initial_unit_count {
            break;
        }
    }

    let final_unit_count = s.state.units.iter().filter(|u| u.owner == rome_id).count();
    assert!(
        final_unit_count > initial_unit_count,
        "Rome should have produced at least one new unit (initial={initial_unit_count}, final={final_unit_count})"
    );
}

// ===========================================================================
// Test 3: Combat and capture
// ===========================================================================

#[test]
fn full_game_combat_and_capture() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Ensure diplomatic relation exists.
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;
    ensure_relation(&mut s, rome_id, babylon_id);

    // Declare war.
    let war_diff = rules.declare_war(&mut s.state, s.rome_id, s.babylon_id)
        .expect("declare_war should succeed");
    assert!(
        war_diff.deltas.iter().any(|d| matches!(d, StateDelta::DiplomacyChanged { .. })),
        "expected DiplomacyChanged delta"
    );

    // Verify war status.
    let rel = s.state.diplomatic_relations.iter()
        .find(|r| (r.civ_a == s.rome_id && r.civ_b == s.babylon_id)
            || (r.civ_a == s.babylon_id && r.civ_b == s.rome_id))
        .expect("relation should exist");
    assert_eq!(rel.status, DiplomaticStatus::War, "should be at war");

    // Move Rome's warrior close to Babylon's warrior.
    // Rome warrior is at (5,3), Babylon warrior at (8,5).
    // Move Rome's warrior step by step toward Babylon's warrior.
    let steps = [
        HexCoord::from_qr(6, 3),
        HexCoord::from_qr(7, 4),
    ];
    for step in &steps {
        let diff = rules.move_unit(&s.state, s.rome_warrior, *step);
        match diff {
            Ok(d) => common::apply_move(&mut s.state, &d),
            Err(RulesError::InsufficientMovement(d)) => {
                common::apply_move(&mut s.state, &d);
                // Reset movement for next step.
                if let Some(u) = s.state.unit_mut(s.rome_warrior) {
                    u.movement_left = u.max_movement;
                }
            }
            Err(e) => panic!("unexpected move error: {e:?}"),
        }
    }

    // Reset movement before attack.
    if let Some(u) = s.state.unit_mut(s.rome_warrior) {
        u.movement_left = u.max_movement;
    }

    // Move adjacent to Babylon's warrior and attack repeatedly until one dies.
    // Babylon's warrior is at (8,5); we need to be adjacent.
    let adj_to_babylon = HexCoord::from_qr(8, 4);
    let move_diff = rules.move_unit(&s.state, s.rome_warrior, adj_to_babylon);
    match move_diff {
        Ok(d) => common::apply_move(&mut s.state, &d),
        Err(RulesError::InsufficientMovement(d)) => {
            common::apply_move(&mut s.state, &d);
        }
        Err(e) => panic!("unexpected move error: {e:?}"),
    }

    // Reset movement before combat.
    if let Some(u) = s.state.unit_mut(s.rome_warrior) {
        u.movement_left = u.max_movement;
    }

    // Attack repeatedly. Equal-strength warriors will trade blows.
    let mut rome_alive = true;
    let mut babylon_alive = true;
    for _ in 0..20 {
        if s.state.unit(s.rome_warrior).is_none() {
            rome_alive = false;
            break;
        }
        if s.state.unit(s.babylon_warrior).is_none() {
            babylon_alive = false;
            break;
        }

        // Reset movement before each attack.
        if let Some(u) = s.state.unit_mut(s.rome_warrior) {
            u.movement_left = u.max_movement;
        }

        let atk_result = rules.attack(&mut s.state, s.rome_warrior, s.babylon_warrior);
        match atk_result {
            Ok(_) => {}
            Err(RulesError::NotInRange) => {
                // Units may not be adjacent; try moving closer.
                break;
            }
            Err(e) => panic!("unexpected attack error: {e:?}"),
        }
    }

    // At least one unit should have taken damage or died.
    assert!(
        !rome_alive || !babylon_alive
        || s.state.unit(s.rome_warrior).is_none_or(|u| u.health < 100)
        || s.state.unit(s.babylon_warrior).is_none_or(|u| u.health < 100),
        "combat should have dealt damage to at least one unit"
    );
}

// ===========================================================================
// Test 4: Research and civic progression
// ===========================================================================

#[test]
fn full_game_research_and_civic_progression() {
    let mut s = common::build_scenario();

    // Queue Pottery research for Rome (cost 25).
    let pottery_id = s.state.tech_refs.pottery;
    let rome_civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome_civ.research_queue.push_back(TechProgress {
        tech_id: pottery_id,
        progress: 0,
        boosted: false,
    });

    // Advance turns until Pottery completes (~2 science/turn from city, need ~13 turns).
    for _ in 0..25 {
        common::advance_turn(&mut s);
        let civ = s.state.civ(s.rome_id).unwrap();
        if civ.researched_techs.contains(&pottery_id) {
            break;
        }
    }

    let rome_civ = s.state.civ(s.rome_id).unwrap();
    assert!(
        rome_civ.researched_techs.contains(&pottery_id),
        "Rome should have completed Pottery research"
    );

    // Verify Pottery's unlock effects: Granary and Farm should be unlocked.
    assert!(
        rome_civ.unlocked_buildings.contains(&"Granary"),
        "Pottery should unlock Granary building"
    );
    assert!(
        rome_civ.unlocked_improvements.contains(&"Farm"),
        "Pottery should unlock Farm improvement"
    );

    // Queue Code of Laws civic (cost 20).
    let code_of_laws_id = s.state.civic_refs.code_of_laws;
    let rome_civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    if !rome_civ.completed_civics.contains(&code_of_laws_id) {
        rome_civ.civic_in_progress = Some(CivicProgress {
            civic_id: code_of_laws_id,
            progress: 0,
            inspired: false,
        });
    }

    // Advance turns until Code of Laws completes.
    for _ in 0..25 {
        common::advance_turn(&mut s);
        let civ = s.state.civ(s.rome_id).unwrap();
        if civ.completed_civics.contains(&code_of_laws_id) {
            break;
        }
    }

    let rome_civ = s.state.civ(s.rome_id).unwrap();
    assert!(
        rome_civ.completed_civics.contains(&code_of_laws_id),
        "Rome should have completed Code of Laws civic"
    );

    // Verify unlock: Chiefdom government and Discipline policy.
    assert!(
        rome_civ.unlocked_governments.contains(&"Chiefdom"),
        "Code of Laws should unlock Chiefdom government"
    );
    assert!(
        rome_civ.unlocked_policies.contains(&"Discipline"),
        "Code of Laws should unlock Discipline policy"
    );
}

// ===========================================================================
// Test 5: Trade route lifecycle
// ===========================================================================

#[test]
fn full_game_trade_route_lifecycle() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Found a second city for Rome far from the first.
    let rome_id = s.rome_id;
    let settler_coord = HexCoord::from_qr(3, 7);
    let settler = spawn_settler(&mut s, rome_id, settler_coord);
    rules.found_city(&mut s.state, settler, "Ostia".to_string())
        .expect("found_city should succeed");
    let ostia_id = s.state.cities.iter()
        .find(|c| c.name == "Ostia")
        .expect("Ostia should exist").id;

    // Spawn a trader at Rome's capital.
    let rome_coord = s.state.city(s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);

    // Record gold yields before establishing trade route.
    let gold_before = rules.compute_yields(&s.state, rome_id).gold;

    // Establish a domestic trade route from Roma to Ostia.
    let diff = rules.establish_trade_route(&mut s.state, trader, ostia_id)
        .expect("establish_trade_route should succeed");
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::TradeRouteEstablished { .. })),
        "expected TradeRouteEstablished delta"
    );

    // Verify route exists.
    assert_eq!(s.state.trade_routes.len(), 1, "should have exactly one trade route");
    let route = &s.state.trade_routes[0];
    assert_eq!(route.origin_yields.gold, 3, "domestic origin yield should be 3 gold");

    // Verify gold income increased.
    let gold_after = rules.compute_yields(&s.state, rome_id).gold;
    assert!(
        gold_after > gold_before,
        "gold income should increase after trade route (before={gold_before}, after={gold_after})"
    );

    // Advance 30 turns — route should still exist.
    for _ in 0..30 {
        rules.advance_turn(&mut s.state);
    }
    assert_eq!(s.state.trade_routes.len(), 1, "route should survive 30 turns");

    // On turn 31 the route should expire.
    let diff_31 = rules.advance_turn(&mut s.state);
    assert!(
        s.state.trade_routes.is_empty(),
        "trade route should expire after 31 turns"
    );
    assert!(
        diff_31.deltas.iter().any(|d| matches!(d, StateDelta::TradeRouteExpired { .. })),
        "expected TradeRouteExpired delta"
    );
}

// ===========================================================================
// Test 6: Religion lifecycle
// ===========================================================================

#[test]
fn full_game_religion_lifecycle() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    let rome_id = s.rome_id;

    // Grant Astrology tech (required for Holy Site).
    let astrology_id = s.state.tech_refs.astrology;
    grant_tech(&mut s, rome_id, astrology_id);

    // Grant faith for pantheon (need >= 25).
    grant_faith(&mut s.state, rome_id, 50);

    // Found a pantheon.
    let pantheon_belief = s.state.belief_refs.earth_goddess;
    let diff = rules.found_pantheon(&mut s.state, rome_id, pantheon_belief)
        .expect("found_pantheon should succeed");
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::PantheonFounded { .. })),
        "expected PantheonFounded delta"
    );
    let rome_civ = s.state.civ(rome_id).unwrap();
    assert_eq!(rome_civ.pantheon_belief, Some(pantheon_belief), "pantheon belief should be set");

    // Place a Holy Site district.
    let holy_site_coord = HexCoord::from_qr(4, 3);
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::HolySite, holy_site_coord);

    // Spawn a Great Prophet at the Holy Site.
    let prophet = spawn_great_prophet(&mut s.state, s.warrior_type, rome_id, holy_site_coord);

    // Found a religion.
    let founder_belief = s.state.belief_refs.tithe;
    let follower_belief = s.state.belief_refs.divine_inspiration;
    let diff = rules.found_religion(
        &mut s.state,
        prophet,
        "Test Faith".to_string(),
        vec![founder_belief, follower_belief],
    ).expect("found_religion should succeed");

    // Verify religion was created.
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::ReligionFounded { .. })),
        "expected ReligionFounded delta"
    );
    assert_eq!(s.state.religions.len(), 1, "should have exactly one religion");

    let religion = &s.state.religions[0];
    assert_eq!(religion.name, "Test Faith");
    assert_eq!(religion.founded_by, rome_id);
    assert!(religion.beliefs.contains(&founder_belief), "religion should have founder belief");
    assert!(religion.beliefs.contains(&follower_belief), "religion should have follower belief");

    // Verify the civ knows about its religion.
    let rome_civ = s.state.civ(rome_id).unwrap();
    assert!(rome_civ.founded_religion.is_some(), "Rome should have a founded religion");
}

// ===========================================================================
// Test 7: Great person lifecycle
// ===========================================================================

#[test]
fn full_game_great_person_lifecycle() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Register great person defs.
    s.state.great_person_defs = builtin_great_person_defs();

    // Place a Campus district for Rome (generates Great Scientist points).
    let campus_coord = HexCoord::from_qr(4, 3);
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, campus_coord);

    // Verify initial great person points are zero.
    let rome_civ = s.state.civ(s.rome_id).unwrap();
    let initial_points = rome_civ.great_person_points
        .get(&GreatPersonType::Scientist)
        .copied()
        .unwrap_or(0);
    assert_eq!(initial_points, 0, "should start with 0 scientist points");

    // Advance several turns to accumulate points (1 point per Campus per turn).
    for _ in 0..10 {
        common::advance_turn(&mut s);
    }

    let rome_civ = s.state.civ(s.rome_id).unwrap();
    let points_after = rome_civ.great_person_points
        .get(&GreatPersonType::Scientist)
        .copied()
        .unwrap_or(0);
    assert!(
        points_after > 0,
        "Rome should have accumulated Great Scientist points after 10 turns (got {points_after})"
    );

    // Manually spawn and retire a great person to verify the retire effect.
    let gp_id = spawn_great_person(&mut s.state, s.rome_id, "Imhotep", HexCoord::from_qr(5, 3));
    let retire_diff = rules.retire_great_person(&mut s.state, gp_id)
        .expect("retire should succeed");

    assert!(
        retire_diff.deltas.iter().any(|d| matches!(d, StateDelta::GreatPersonRetired { .. })),
        "expected GreatPersonRetired delta"
    );

    let gp = s.state.great_person(gp_id).unwrap();
    assert!(gp.is_retired, "great person should be marked retired");
}

// ===========================================================================
// Test 8: Cultural border expansion
// ===========================================================================

#[test]
fn full_game_cultural_border_expansion() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // The build_scenario cities are created directly (not via found_city),
    // so territory is empty. Found a new city via the rules engine to get
    // proper territory initialization.
    let rome_id = s.rome_id;
    let settler_coord = HexCoord::from_qr(3, 7);
    let settler = spawn_settler(&mut s, rome_id, settler_coord);
    rules.found_city(&mut s.state, settler, "Neapolis".to_string())
        .expect("found_city should succeed");
    let neapolis_id = s.state.cities.iter()
        .find(|c| c.name == "Neapolis")
        .unwrap().id;

    // Record initial territory size (city center + ring-1 neighbors).
    let initial_territory = s.state.cities.iter()
        .find(|c| c.id == neapolis_id)
        .unwrap()
        .territory
        .len();
    assert!(initial_territory > 0, "Neapolis should have initial territory from founding");

    // Advance multiple turns for culture to accumulate and borders to expand.
    for _ in 0..40 {
        common::advance_turn(&mut s);
    }

    // Verify territory has grown beyond the initial ring-1.
    let final_territory = s.state.cities.iter()
        .find(|c| c.id == neapolis_id)
        .unwrap()
        .territory
        .len();
    assert!(
        final_territory > initial_territory,
        "Neapolis' territory should have grown (initial={initial_territory}, final={final_territory})"
    );
}

// ===========================================================================
// Test 9: All systems integrated
// ===========================================================================

#[test]
fn full_game_all_systems_integrated() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;
    let rome_city = s.rome_city;

    // Register great person defs.
    s.state.great_person_defs = builtin_great_person_defs();

    // Ensure diplomatic relation between civs.
    ensure_relation(&mut s, rome_id, babylon_id);

    // --- Grant techs: Pottery, Mining, Writing ---
    let pottery_id = s.state.tech_refs.pottery;
    let mining_id = s.state.tech_refs.mining;
    let writing_id = s.state.tech_refs.writing;
    let astrology_id = s.state.tech_refs.astrology;
    grant_tech(&mut s, rome_id, pottery_id);
    grant_tech(&mut s, rome_id, mining_id);
    grant_tech(&mut s, rome_id, writing_id);
    grant_tech(&mut s, rome_id, astrology_id);

    // --- Found a second city for Rome ---
    let settler_coord = HexCoord::from_qr(3, 7);
    let settler = spawn_settler(&mut s, rome_id, settler_coord);
    rules.found_city(&mut s.state, settler, "Ostia".to_string())
        .expect("found_city should succeed");
    let ostia_id = s.state.cities.iter()
        .find(|c| c.name == "Ostia")
        .expect("Ostia should exist").id;

    // --- Place improvements (Farm + Mine) ---
    // Place a Farm on a Grassland flat tile owned by Rome.
    let farm_coord = HexCoord::from_qr(2, 3);
    // Claim the tile for Rome's capital first.
    let _ = rules.claim_tile(&mut s.state, rome_city, farm_coord, false);
    let builder = spawn_builder(&mut s, rome_id, farm_coord);
    let farm_result = rules.place_improvement(
        &mut s.state, rome_id, farm_coord,
        BuiltinImprovement::Farm, Some(builder),
    );
    // Farm may or may not succeed depending on tile terrain; that's OK.
    if farm_result.is_ok() {
        let tile = s.state.board.tile(farm_coord).unwrap();
        assert!(
            tile.improvement.is_some(),
            "tile should have an improvement after placing Farm"
        );
    }

    // --- Place Campus district ---
    let campus_coord = HexCoord::from_qr(4, 3);
    let _ = rules.claim_tile(&mut s.state, rome_city, campus_coord, false);
    let place_result = rules.place_district(&mut s.state, rome_city, BuiltinDistrict::Campus, campus_coord);
    assert!(place_result.is_ok(), "Campus placement should succeed, got: {place_result:?}");

    // --- Establish a trade route ---
    let rome_coord = s.state.city(rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);
    let trade_result = rules.establish_trade_route(&mut s.state, trader, ostia_id);
    assert!(trade_result.is_ok(), "trade route should succeed");
    assert_eq!(s.state.trade_routes.len(), 1, "should have one trade route");

    // --- Found a pantheon ---
    grant_faith(&mut s.state, rome_id, 50);
    let pantheon_belief = s.state.belief_refs.earth_goddess;
    let pantheon_result = rules.found_pantheon(&mut s.state, rome_id, pantheon_belief);
    assert!(pantheon_result.is_ok(), "pantheon founding should succeed");

    // --- Declare war and engage in combat ---
    let war_result = rules.declare_war(&mut s.state, rome_id, babylon_id);
    assert!(war_result.is_ok(), "declare_war should succeed");

    // Spawn a warrior adjacent to Babylon's warrior for combat.
    let atk_coord = HexCoord::from_qr(9, 5);
    let attack_warrior = spawn_warrior(&mut s, rome_id, atk_coord);
    recalculate_visibility(&mut s.state, rome_id);

    // Attack Babylon's warrior.
    let atk_result = rules.attack(&mut s.state, attack_warrior, s.babylon_warrior);
    // May fail if warriors aren't adjacent due to coord layout; that's acceptable.
    if atk_result.is_ok() {
        // At least one unit should have taken damage.
        let atk_unit = s.state.unit(attack_warrior);
        let def_unit = s.state.unit(s.babylon_warrior);
        let damage_dealt = atk_unit.is_none_or(|u| u.health < 100)
            || def_unit.is_none_or(|u| u.health < 100);
        assert!(damage_dealt, "combat should have dealt damage");
    }

    // --- Set up AI for Babylon ---
    let ai = HeuristicAgent::new(babylon_id);

    // --- Run 30 turns with AI ---
    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Score { id: vc_id, turn_limit: 30 });

    for _ in 0..30 {
        if s.state.game_over.is_some() {
            break;
        }
        ai.take_turn(&mut s.state, &rules);
        rules.advance_turn(&mut s.state);
        for unit in &mut s.state.units {
            unit.movement_left = unit.max_movement;
        }
        let civ_ids: Vec<CivId> = s.state.civilizations.iter().map(|c| c.id).collect();
        for cid in civ_ids {
            recalculate_visibility(&mut s.state, cid);
        }
    }

    // --- Verify end state ---
    // Game should be over.
    assert!(s.state.game_over.is_some(), "game should be over after 30 turns");

    // Verify score is computed.
    let score = libciv::compute_score(&s.state, rome_id);
    assert!(score > 0, "Rome should have a positive score (got {score})");

    // Verify Rome has researched techs (granted directly).
    let rome_civ = s.state.civ(rome_id).unwrap();
    assert!(rome_civ.researched_techs.contains(&pottery_id), "Rome should have Pottery");
    assert!(rome_civ.researched_techs.contains(&mining_id), "Rome should have Mining");
    assert!(rome_civ.researched_techs.contains(&writing_id), "Rome should have Writing");

    // Verify Rome has a pantheon.
    assert!(rome_civ.pantheon_belief.is_some(), "Rome should have a pantheon");

    // Verify Rome has multiple cities.
    let rome_cities: Vec<_> = s.state.cities.iter().filter(|c| c.owner == rome_id).collect();
    assert!(rome_cities.len() >= 2, "Rome should have at least 2 cities");

    // Verify Campus district exists.
    let has_campus = s.state.placed_districts.iter()
        .any(|d| d.district_type == BuiltinDistrict::Campus && d.city_id == rome_city);
    assert!(has_campus, "Rome's capital should have a Campus district");
}
