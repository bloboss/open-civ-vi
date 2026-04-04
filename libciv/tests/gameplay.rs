/// End-to-end gameplay integration tests.
///
/// Each test exercises one complete gameplay concern (visibility, movement,
/// combat, city founding, etc.) against the public libciv API, using the
/// shared `common::Scenario` setup.  Unit tests for individual rule methods
/// live in `libciv/src/game/rules.rs`; these tests verify full pipelines.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine, TechId, UnitCategory, UnitDomain};
use libciv::civ::{BasicUnit, TechProgress};
use libciv::civ::city::WallLevel;
use libciv::game::{recalculate_visibility, AttackType, RulesError, StateDelta};
use libciv::rules::TechNode;
use libciv::world::resource::BuiltinResource;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Infrastructure smoke tests — validate the test helpers themselves
// ---------------------------------------------------------------------------

/// Confirm `build_scenario` produces a coherent initial game state.
#[test]
fn scenario_initial_state_is_coherent() {
    let s = common::build_scenario();

    // Two civilisations, two cities, two units.
    assert_eq!(s.state.civilizations.len(), 2, "expected 2 civs");
    assert_eq!(s.state.cities.len(), 2,        "expected 2 cities");
    assert_eq!(s.state.units.len(), 2,         "expected 2 units");

    // Turn counter starts at zero.
    assert_eq!(s.state.turn, 0, "turn should be 0 before any advance");

    // Each city is registered in its owner's city list.
    let rome_civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(rome_civ.cities.contains(&s.rome_city), "roma not in Rome's city list");

    let babylon_civ = s.state.civilizations.iter().find(|c| c.id == s.babylon_id).unwrap();
    assert!(babylon_civ.cities.contains(&s.babylon_city), "babylon not in Babylon's city list");

    // Units are on their expected starting coordinates.
    let rome_unit = s.state.unit(s.rome_warrior).expect("rome warrior missing");
    assert_eq!(rome_unit.coord, HexCoord::from_qr(5, 3), "rome warrior at wrong coord");

    let babylon_unit = s.state.unit(s.babylon_warrior).expect("babylon warrior missing");
    assert_eq!(babylon_unit.coord, HexCoord::from_qr(8, 5), "babylon warrior at wrong coord");
}

/// `advance_turn` increments the turn counter and resets movement.
#[test]
fn advance_turn_increments_counter_and_resets_movement() {
    let mut s = common::build_scenario();

    // Spend some movement on Rome's warrior first.
    let rules = DefaultRulesEngine;
    let dest = HexCoord::from_qr(6, 3);
    let diff = rules.move_unit(&s.state, s.rome_warrior, dest).unwrap();
    common::apply_move(&mut s.state, &diff);

    let unit_before = s.state.unit(s.rome_warrior).unwrap();
    assert!(unit_before.movement_left < unit_before.max_movement, "movement should have been spent");

    common::advance_turn(&mut s);

    assert_eq!(s.state.turn, 1, "turn should be 1 after first advance");

    // Movement must have been reset.
    let unit_after = s.state.unit(s.rome_warrior).unwrap();
    assert_eq!(
        unit_after.movement_left, unit_after.max_movement,
        "movement should be fully reset after end-of-turn"
    );
}

/// Initial visibility: Rome's starting tiles are revealed by its city + warrior.
#[test]
fn initial_visibility_covers_starting_tiles() {
    let s = common::build_scenario();

    let rome_civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();

    // City at (3,3) with radius 2 and warrior at (5,3) with radius 2
    // must produce non-empty visible and explored sets.
    assert!(!rome_civ.visible_tiles.is_empty(),  "Rome should have visible tiles");
    assert!(!rome_civ.explored_tiles.is_empty(), "Rome should have explored tiles");

    // The city's own tile should be visible.
    assert!(
        rome_civ.visible_tiles.contains(&HexCoord::from_qr(3, 3)),
        "Rome's capital tile should be visible"
    );

    // Babylon's tiles should NOT be visible to Rome at game start
    // (capitals are >9 hexes apart).
    assert!(
        !rome_civ.visible_tiles.contains(&HexCoord::from_qr(10, 5)),
        "Babylon's capital should not be visible to Rome at start"
    );
}

// ---------------------------------------------------------------------------
// Visibility — fog of war updates on movement
// ---------------------------------------------------------------------------

/// Moving a unit extends its owner's visible and explored tile sets.
///
/// Rome's warrior starts at (5, 3).  After moving two tiles east to (7, 3),
/// tile (9, 3) — which is 4 tiles from the original position but only 2 from
/// the new position — should enter the visible set.
#[test]
fn fog_of_war_expands_after_unit_move() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Tile (9, 3) is beyond the warrior's initial vision radius.
    let far_tile = HexCoord::from_qr(9, 3);
    let rome_civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(
        !rome_civ.visible_tiles.contains(&far_tile),
        "(9,3) should not be visible from the starting position"
    );

    // Move the warrior from (5,3) to (7,3) — two Grassland tiles, cost 200.
    let dest = HexCoord::from_qr(7, 3);
    let diff = rules.move_unit(&s.state, s.rome_warrior, dest).unwrap();
    common::apply_move(&mut s.state, &diff);
    recalculate_visibility(&mut s.state, s.rome_id);

    let warrior = s.state.unit(s.rome_warrior).unwrap();
    assert_eq!(warrior.coord, dest, "warrior should have reached (7,3)");

    let rome_civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();

    // Tile (9, 3) is now exactly 2 tiles from the warrior — within vision radius.
    assert!(
        rome_civ.visible_tiles.contains(&far_tile),
        "(9,3) should now be visible after moving to (7,3)"
    );
    assert!(
        rome_civ.explored_tiles.contains(&far_tile),
        "(9,3) should be permanently explored after being seen"
    );

    // Previously-visible tiles near the city are still explored (not visible,
    // since the warrior moved away, but the city still covers (3,3)).
    assert!(
        rome_civ.explored_tiles.contains(&HexCoord::from_qr(3, 3)),
        "Roma's tile remains explored after the warrior moves away"
    );
}

// ---------------------------------------------------------------------------
// Combat — melee attack
// ---------------------------------------------------------------------------

/// A melee attack on an adjacent enemy emits `UnitAttacked` with non-zero
/// damage, reduces both units' health, and costs the attacker all movement.
#[test]
fn melee_attack_emits_damage_and_reduces_health() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a second Babylon warrior adjacent to Rome's warrior at (5,3).
    let enemy_coord = HexCoord::from_qr(6, 3);
    let enemy_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              enemy_id,
        unit_type:       s.warrior_type,
        owner:           s.babylon_id,
        coord:           enemy_coord,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, s.rome_warrior, enemy_id)
        .expect("attack should succeed");

    // A UnitAttacked delta must be present.
    let attacked = diff.deltas.iter().find_map(|d| {
        if let StateDelta::UnitAttacked { attacker, defender, defender_damage, attacker_damage, .. }
            = d
        {
            Some((*attacker, *defender, *attacker_damage, *defender_damage))
        } else {
            None
        }
    });
    let (atk, def, atk_dmg, def_dmg) = attacked.expect("UnitAttacked delta expected");
    assert_eq!(atk, s.rome_warrior, "attacker id mismatch");
    assert_eq!(def, enemy_id,       "defender id mismatch");
    assert!(def_dmg > 0, "defender should take damage");
    assert!(atk_dmg > 0, "attacker should take counter-damage in melee");

    // Health should reflect the damage.
    let attacker_health = s.state.unit(s.rome_warrior).unwrap().health;
    let defender_health = s.state.unit(enemy_id).map(|u| u.health);
    assert!(attacker_health < 100, "attacker health reduced");
    // Defender may or may not be dead depending on RNG, but if alive it took damage.
    if let Some(hp) = defender_health {
        assert!(hp < 100, "defender health reduced");
    }

    // Attacker must have expended all remaining movement after attacking.
    let mv_left = s.state.unit(s.rome_warrior).map(|u| u.movement_left).unwrap_or(0);
    assert_eq!(mv_left, 0, "attacker should have 0 movement left after attacking");
}

/// When a defender's health reaches zero the unit is removed from the state
/// and `UnitDestroyed` is emitted.
#[test]
fn attacking_unit_at_one_hp_destroys_it() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place an adjacent enemy with only 1 HP — any hit will kill it.
    let enemy_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              enemy_id,
        unit_type:       s.warrior_type,
        owner:           s.babylon_id,
        coord:           HexCoord::from_qr(6, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          1,      // barely alive
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, s.rome_warrior, enemy_id)
        .expect("attack should succeed");

    // UnitDestroyed must be in the diff.
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitDestroyed { unit } if *unit == enemy_id)),
        "UnitDestroyed delta expected for the killed unit"
    );

    // The unit should no longer exist in state.
    assert!(
        s.state.unit(enemy_id).is_none(),
        "destroyed unit should be removed from state"
    );
}

// ---------------------------------------------------------------------------
// City founding
// ---------------------------------------------------------------------------

/// A settler unit at a valid coordinate founds a new city, is removed from
/// the unit list, and its owner's city list grows by one.
#[test]
fn settler_founds_city_and_is_consumed() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // (7, 3) is 4 tiles from Roma (3,3) and 5 tiles from Babylon (10,5) — valid.
    let settle_coord = HexCoord::from_qr(7, 3);

    let settler_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              settler_id,
        unit_type:       s.settler_type,
        owner:           s.rome_id,
        coord:           settle_coord,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Civilian,
        movement_left:   200,
        max_movement:    200,
        combat_strength: None,
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let unit_count_before = s.state.units.len();
    let city_count_before = s.state.cities.len();

    let diff = rules.found_city(&mut s.state, settler_id, "Nova Roma".to_string())
        .expect("found_city should succeed");

    // A CityFounded delta must be present.
    assert!(
        diff.deltas.iter().any(|d| matches!(d,
            StateDelta::CityFounded { coord, owner, .. }
            if *coord == settle_coord && *owner == s.rome_id
        )),
        "CityFounded delta expected"
    );

    // Settler consumed: unit list shrinks by 1.
    assert_eq!(
        s.state.units.len(), unit_count_before - 1,
        "settler should be removed after founding"
    );

    // New city added.
    assert_eq!(
        s.state.cities.len(), city_count_before + 1,
        "city count should increase by 1"
    );

    // New city is at the settle coordinate.
    let new_city = s.state.cities.iter().find(|c| c.coord == settle_coord)
        .expect("new city should be at settle_coord");
    assert_eq!(new_city.owner, s.rome_id, "new city should be owned by Rome");
    assert_eq!(new_city.name, "Nova Roma", "city name should match");

    // Rome's civ city list is updated.
    let rome_civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(
        rome_civ.cities.contains(&new_city.id),
        "new city should appear in Rome's city list"
    );
}

/// `found_city` rejects a site that is too close to an existing city.
#[test]
fn founder_too_close_to_existing_city_is_rejected() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // (4, 3) is only 1 tile from Roma at (3, 3) — within the 3-tile exclusion zone.
    let settler_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              settler_id,
        unit_type:       s.settler_type,
        owner:           s.rome_id,
        coord:           HexCoord::from_qr(4, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Civilian,
        movement_left:   200,
        max_movement:    200,
        combat_strength: None,
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let result = rules.found_city(&mut s.state, settler_id, "Too Close".to_string());
    assert!(
        matches!(result, Err(libciv::game::RulesError::TooCloseToCity)),
        "expected TooCloseToCity error, got {:?}", result
    );
}

// ---------------------------------------------------------------------------
// Tech research
// ---------------------------------------------------------------------------

/// After queuing a tech with near-complete progress and providing 1 science/turn
/// (via an Aluminum resource on a worked tile), one call to `advance_turn`
/// should complete the tech and emit `TechResearched`.
#[test]
fn tech_completes_when_science_fills_progress() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Find the canonical Pottery TechId from the built-in tree.
    let pottery_id: TechId = s.state.tech_tree.nodes.values()
        .find(|n| n.name == "Pottery")
        .expect("Pottery should be in the built-in tech tree")
        .id;

    // Aluminum gives 1 science/turn but requires the "Refining" tech to be
    // ungated.  Add a stub Refining node and mark it as already researched.
    let refining_id = TechId::from_ulid(s.state.id_gen.next_ulid());
    s.state.tech_tree.add_node(TechNode {
        id:                 refining_id,
        name:               "Refining",
        cost:               9999,
        prerequisites:      vec![],
        effects:            vec![],
        eureka_description: "",
        eureka_effects:     vec![],
    });
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(refining_id);

    // Place Aluminum on Roma's center tile, which is always in worked_tiles.
    let city_tile = HexCoord::from_qr(3, 3);
    s.state.board.tile_mut(city_tile).unwrap().resource =
        Some(BuiltinResource::Aluminum);

    // Queue Pottery with 24/25 progress — needs exactly 1 more science to complete.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .research_queue.push_back(TechProgress {
            tech_id: pottery_id,
            progress: 24,
            boosted: false,
        });

    let diff = rules.advance_turn(&mut s.state);

    let rome_civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(
        rome_civ.researched_techs.contains(&pottery_id),
        "Pottery should be in researched_techs after completion"
    );
    assert!(
        rome_civ.research_queue.is_empty(),
        "research queue should be empty after the only queued tech completes"
    );

    assert!(
        diff.deltas.iter().any(|d| matches!(
            d, StateDelta::TechResearched { tech: "Pottery", .. }
        )),
        "TechResearched(\"Pottery\") delta expected"
    );
}

/// A second tech queued after the completing tech automatically becomes the
/// active research on the following turn.
#[test]
fn second_queued_tech_advances_after_first_completes() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let pottery_id: TechId = s.state.tech_tree.nodes.values()
        .find(|n| n.name == "Pottery").unwrap().id;
    let mining_id: TechId = s.state.tech_tree.nodes.values()
        .find(|n| n.name == "Mining").unwrap().id;

    // Ungate Aluminum with a fake Refining tech.
    let refining_id = TechId::from_ulid(s.state.id_gen.next_ulid());
    s.state.tech_tree.add_node(TechNode {
        id: refining_id, name: "Refining", cost: 9999,
        prerequisites: vec![], effects: vec![],
        eureka_description: "", eureka_effects: vec![],
    });
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(refining_id);
    s.state.board.tile_mut(HexCoord::from_qr(3, 3)).unwrap().resource =
        Some(BuiltinResource::Aluminum);

    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.research_queue.push_back(TechProgress { tech_id: pottery_id, progress: 24, boosted: false });
    civ.research_queue.push_back(TechProgress { tech_id: mining_id,  progress: 0,  boosted: false });

    rules.advance_turn(&mut s.state);

    let rome_civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(rome_civ.researched_techs.contains(&pottery_id), "Pottery should be completed");
    assert_eq!(rome_civ.research_queue.len(), 1, "Mining still in queue");
    assert_eq!(
        rome_civ.research_queue.front().unwrap().tech_id,
        mining_id,
        "Mining should be the new front of the queue"
    );
}

// ---------------------------------------------------------------------------
// Turn mechanics
// ---------------------------------------------------------------------------

/// Multiple consecutive `advance_turn` calls all increment the turn counter
/// and reset unit movement each time.
#[test]
fn multiple_turns_advance_correctly() {
    let mut s = common::build_scenario();

    for expected_turn in 1..=3 {
        common::advance_turn(&mut s);
        assert_eq!(s.state.turn, expected_turn, "turn counter after advance {}", expected_turn);

        // Every unit should have full movement after the reset.
        for unit in &s.state.units {
            assert_eq!(
                unit.movement_left, unit.max_movement,
                "unit {} should have full movement after turn {}", unit.id, expected_turn
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Ranged attacks
// ---------------------------------------------------------------------------

/// Helper: spawn a Rome slinger (range 2) and return its UnitId.
fn spawn_slinger(s: &mut common::Scenario, coord: HexCoord) -> libciv::UnitId {
    use libciv::game::state::UnitTypeDef;
    // Register the slinger type if not already present.
    let slinger_type_id = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id:              slinger_type_id,
        name:            "slinger",
        production_cost: 35,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        max_movement:    200,
        combat_strength: Some(15),
        range:           2,
        vision_range:    2,
        can_found_city:  false,
        resource_cost:   None,
        siege_bonus:     0,
        max_charges:     0,
        exclusive_to:    None,
        replaces:        None,
            era:             None,
            promotion_class: None,
    });
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              unit_id,
        unit_type:       slinger_type_id,
        owner:           s.rome_id,
        coord,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(15),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           2,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });
    unit_id
}

/// A ranged unit (range 2) can attack an enemy exactly 2 tiles away without
/// being adjacent.  Only the defender takes damage; attacker_damage is 0.
#[test]
fn ranged_unit_attacks_from_two_tiles_away() {
    use libciv::game::diff::AttackType;

    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Slinger at (5, 3); target 2 tiles east at (7, 3).
    let slinger_id  = spawn_slinger(&mut s, HexCoord::from_qr(5, 3));
    let target_coord = HexCoord::from_qr(7, 3);

    let target_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              target_id,
        unit_type:       s.warrior_type,
        owner:           s.babylon_id,
        coord:           target_coord,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Distance is exactly 2 — within range, but not adjacent.
    assert_eq!(HexCoord::from_qr(5, 3).distance(&target_coord), 2);

    let diff = rules.attack(&mut s.state, slinger_id, target_id)
        .expect("ranged attack should succeed at range 2");

    let attacked = diff.deltas.iter().find_map(|d| {
        if let StateDelta::UnitAttacked { attacker, defender, attack_type,
                                          attacker_damage, defender_damage } = d
        {
            Some((*attacker, *defender, *attack_type, *attacker_damage, *defender_damage))
        } else { None }
    }).expect("UnitAttacked delta expected");

    assert_eq!(attacked.0, slinger_id,         "attacker id");
    assert_eq!(attacked.1, target_id,           "defender id");
    assert_eq!(attacked.2, AttackType::Ranged,  "should be a ranged attack");
    assert_eq!(attacked.3, 0,                   "ranged attacker takes no counter-damage");
    assert!(attacked.4 > 0,                     "defender should take damage");
}

/// A ranged attack beyond the unit's range is rejected with `NotInRange`.
#[test]
fn ranged_attack_beyond_range_is_rejected() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Slinger at (5, 3); target 3 tiles east at (8, 3) — beyond range 2.
    let slinger_id  = spawn_slinger(&mut s, HexCoord::from_qr(5, 3));
    let target_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              target_id,
        unit_type:       s.warrior_type,
        owner:           s.babylon_id,
        coord:           HexCoord::from_qr(8, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    assert_eq!(HexCoord::from_qr(5, 3).distance(&HexCoord::from_qr(8, 3)), 3);

    let result = rules.attack(&mut s.state, slinger_id, target_id);
    assert!(
        matches!(result, Err(libciv::game::RulesError::NotInRange)),
        "expected NotInRange, got {:?}", result
    );
}

/// A ranged unit does not need to be adjacent (distance 1) to attack.
/// Confirms that attack succeeds at distance 2 even though a melee unit would
/// require distance 1.
#[test]
fn ranged_attack_succeeds_without_adjacency() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Slinger at (5, 3); target at (5, 5) — distance 2, not adjacent.
    let slinger_id = spawn_slinger(&mut s, HexCoord::from_qr(5, 3));
    let target_id  = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              target_id,
        unit_type:       s.warrior_type,
        owner:           s.babylon_id,
        coord:           HexCoord::from_qr(5, 5),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Verify the distance is exactly 2 (not adjacent).
    let dist = HexCoord::from_qr(5, 3).distance(&HexCoord::from_qr(5, 5));
    assert_eq!(dist, 2, "test requires distance == 2");

    // Would fail for a melee unit (range 0 requires dist == 1); succeeds for slinger.
    assert!(
        rules.attack(&mut s.state, slinger_id, target_id).is_ok(),
        "ranged unit should succeed at distance 2"
    );
}

// ---------------------------------------------------------------------------
// Stacking prevention
// ---------------------------------------------------------------------------

/// Two friendly units cannot occupy the same tile.
#[test]
fn two_friendly_units_cannot_stack() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a second Rome warrior at (6, 3) — one step ahead of the first.
    let blocker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              blocker_id,
        unit_type:       s.warrior_type,
        owner:           s.rome_id,       // same civ as rome_warrior
        coord:           HexCoord::from_qr(6, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Rome's warrior (at (5,3)) tries to move onto (6,3) — occupied by a friendly.
    let result = rules.move_unit(&s.state, s.rome_warrior, HexCoord::from_qr(6, 3));
    assert!(
        matches!(result, Err(libciv::game::RulesError::TileOccupiedByUnit)),
        "expected TileOccupiedByUnit when stacking friendlies, got {:?}", result
    );
}

// ---------------------------------------------------------------------------
// Enemy-city proximity
// ---------------------------------------------------------------------------

/// A settler cannot found a city within 3 tiles of an *enemy* city.
#[test]
fn cannot_found_city_near_enemy_capital() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Babylon's capital is at (10, 5).  (9, 5) is exactly 1 tile away — within the
    // 3-tile exclusion zone regardless of city ownership.
    let too_close = HexCoord::from_qr(9, 5);
    assert!(too_close.distance(&HexCoord::from_qr(10, 5)) <= 3);

    let settler_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              settler_id,
        unit_type:       s.settler_type,
        owner:           s.rome_id,
        coord:           too_close,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Civilian,
        movement_left:   200,
        max_movement:    200,
        combat_strength: None,
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let result = rules.found_city(&mut s.state, settler_id, "Too Close".to_string());
    assert!(
        matches!(result, Err(libciv::game::RulesError::TooCloseToCity)),
        "expected TooCloseToCity near enemy capital, got {:?}", result
    );
}

// ---------------------------------------------------------------------------
// Civilian movement restrictions
// ---------------------------------------------------------------------------

/// A civilian (settler) trying to move onto a tile occupied by an enemy
/// combat unit gets `UnitCannotAttack` — it has no combat capability.
#[test]
fn civilian_blocked_from_moving_onto_enemy_unit() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a Rome settler at (7, 3) and a Babylon warrior at (8, 3).
    let settler_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              settler_id,
        unit_type:       s.settler_type,
        owner:           s.rome_id,
        coord:           HexCoord::from_qr(7, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Civilian,
        movement_left:   200,
        max_movement:    200,
        combat_strength: None,
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });
    let enemy_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              enemy_id,
        unit_type:       s.warrior_type,
        owner:           s.babylon_id,
        coord:           HexCoord::from_qr(8, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Settler tries to walk onto the enemy tile.
    let result = rules.move_unit(&s.state, settler_id, HexCoord::from_qr(8, 3));
    assert!(
        matches!(result, Err(libciv::game::RulesError::UnitCannotAttack)),
        "expected UnitCannotAttack for civilian moving onto enemy, got {:?}", result
    );
}

/// A civilian cannot stack with a friendly unit either.
#[test]
fn civilian_blocked_from_stacking_with_friendly_unit() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a settler at (4, 3); Rome's warrior is already at (5, 3).
    let settler_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              settler_id,
        unit_type:       s.settler_type,
        owner:           s.rome_id,
        coord:           HexCoord::from_qr(4, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Civilian,
        movement_left:   200,
        max_movement:    200,
        combat_strength: None,
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Try to move settler onto (5, 3) where Rome's warrior stands.
    let result = rules.move_unit(&s.state, settler_id, HexCoord::from_qr(5, 3));
    assert!(
        matches!(result, Err(libciv::game::RulesError::TileOccupiedByUnit)),
        "expected TileOccupiedByUnit when civilian stacks with friendly, got {:?}", result
    );
}

/// A combat unit that tries to *move* onto an enemy tile gets
/// `TileOccupiedByUnit`.  The player must use `attack()` explicitly.
#[test]
fn combat_unit_cannot_walk_into_enemy_tile() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a second Babylon warrior adjacent to Rome's warrior.
    let enemy_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id:              enemy_id,
        unit_type:       s.warrior_type,
        owner:           s.babylon_id,
        coord:           HexCoord::from_qr(6, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        experience:      0,
        health:          100,
        range:           0,
        vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Rome's warrior (at (5,3)) tries to move onto (6,3) — an enemy tile.
    let result = rules.move_unit(&s.state, s.rome_warrior, HexCoord::from_qr(6, 3));
    assert!(
        matches!(result, Err(libciv::game::RulesError::TileOccupiedByUnit)),
        "expected TileOccupiedByUnit when walking into enemy, got {:?}", result
    );

    // The warrior should still be able to attack that tile via attack().
    assert!(
        rules.attack(&mut s.state, s.rome_warrior, enemy_id).is_ok(),
        "explicit attack() should still succeed"
    );
}

// ---------------------------------------------------------------------------
// Strategic resource consumption (8.7)
// ---------------------------------------------------------------------------

use libciv::civ::city::ProductionItem;
use libciv::game::state::UnitTypeDef;
use libciv::world::improvement::BuiltinImprovement;

/// Unit with no resource cost completes production normally.
#[test]
fn unit_production_no_resource_cost_completes() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Register a unit type with no resource cost.
    let warrior_tid = s.warrior_type;

    // Queue a warrior in Rome (production_cost = 40).
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .production_queue.push_back(ProductionItem::Unit(warrior_tid));

    // Put a Plains tile adjacent to Rome in worked_tiles to get 1 production/turn.
    let plains_tile = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(plains_tile) {
        t.terrain = libciv::world::terrain::BuiltinTerrain::Plains; // 1 production
        t.owner   = Some(s.rome_id);
    }
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .worked_tiles.push(plains_tile);

    let rome_units_before = s.state.units.iter().filter(|u| u.owner == s.rome_id).count();

    // Set production one turn away from completion.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .production_stored = 39;

    let diff = rules.advance_turn(&mut s.state);

    // After advance_turn adds 1 Plains production, 40 >= 40, unit should complete.
    let rome_units_after = s.state.units.iter().filter(|u| u.owner == s.rome_id).count();
    assert_eq!(rome_units_after, rome_units_before + 1, "warrior should have been produced");
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitCreated { .. })),
        "UnitCreated delta expected"
    );
    // Queue should be empty.
    assert!(
        s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap()
            .production_queue.is_empty(),
        "production queue should be empty after completion"
    );
}

/// Unit with a resource cost is blocked when the civ lacks the resource.
#[test]
fn unit_production_blocked_without_resource() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Register a unit type that costs 1 Iron.
    let swordsman_tid = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id:              swordsman_tid,
        name:            "swordsman",
        production_cost: 1,           // trivially met
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        max_movement:    200,
        combat_strength: Some(35),
        range:           0,
        vision_range:    2,
        can_found_city:  false,
        resource_cost:   Some((BuiltinResource::Iron, 1)),
        siege_bonus:     0,
        max_charges:     0,
        exclusive_to:    None,
        replaces:        None,
            era:             None,
            promotion_class: None,
    });

    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .production_queue.push_back(ProductionItem::Unit(swordsman_tid));

    // Ensure production_stored already covers the cost.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .production_stored = 10;

    let rome_units_before = s.state.units.iter().filter(|u| u.owner == s.rome_id).count();

    // Rome has no Iron — production should be deferred.
    let diff = rules.advance_turn(&mut s.state);

    let rome_units_after = s.state.units.iter().filter(|u| u.owner == s.rome_id).count();
    assert_eq!(rome_units_after, rome_units_before, "no unit should be produced without Iron");
    assert!(
        !diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitCreated { owner, .. } if *owner == s.rome_id)),
        "no UnitCreated delta expected for Rome when blocked by resource"
    );
    // Queue still has the item.
    assert!(
        !s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap()
            .production_queue.is_empty(),
        "production queue should remain non-empty when blocked"
    );
}

/// Unit with a resource cost completes when the civ has enough of the resource.
#[test]
fn unit_production_consumes_strategic_resource() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Register swordsman (costs 1 Iron).
    let swordsman_tid = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id:              swordsman_tid,
        name:            "swordsman",
        production_cost: 1,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        max_movement:    200,
        combat_strength: Some(35),
        range:           0,
        vision_range:    2,
        can_found_city:  false,
        resource_cost:   Some((BuiltinResource::Iron, 1)),
        siege_bonus:     0,
        max_charges:     0,
        exclusive_to:    None,
        replaces:        None,
            era:             None,
            promotion_class: None,
    });

    // Grant Rome 3 Iron.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .strategic_resources.insert(BuiltinResource::Iron, 3);

    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .production_queue.push_back(ProductionItem::Unit(swordsman_tid));
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .production_stored = 10;

    let rome_units_before = s.state.units.iter().filter(|u| u.owner == s.rome_id).count();

    let diff = rules.advance_turn(&mut s.state);

    let rome_units_after = s.state.units.iter().filter(|u| u.owner == s.rome_id).count();
    assert_eq!(rome_units_after, rome_units_before + 1, "swordsman should be produced");
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitCreated { owner, .. } if *owner == s.rome_id)),
        "UnitCreated expected for Rome"
    );
    // Iron should have been decremented by 1 (from 3 to 2).
    let iron_left = *s.state.civilizations.iter()
        .find(|c| c.id == s.rome_id).unwrap()
        .strategic_resources.get(&BuiltinResource::Iron).unwrap_or(&0);
    assert_eq!(iron_left, 2, "1 Iron should be consumed");
    // StrategicResourceChanged delta with delta = -1 should be emitted.
    assert!(
        diff.deltas.iter().any(|d| matches!(d,
            StateDelta::StrategicResourceChanged { resource, delta, .. }
            if *resource == BuiltinResource::Iron && *delta == -1
        )),
        "StrategicResourceChanged(-1 Iron) expected"
    );
}

/// Strategic resources are accumulated from worked tiles with improvements each turn.
#[test]
fn strategic_resource_accumulated_from_improvement() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place Iron on a tile adjacent to Rome and add an improvement (Mine).
    let iron_tile = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(iron_tile) {
        t.resource   = Some(BuiltinResource::Iron);
        t.improvement = Some(BuiltinImprovement::Mine);
        t.owner      = Some(s.rome_id);
    }
    // Add the tile to Rome's worked tiles.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .worked_tiles.push(iron_tile);

    let iron_before = *s.state.civilizations.iter()
        .find(|c| c.id == s.rome_id).unwrap()
        .strategic_resources.get(&BuiltinResource::Iron).unwrap_or(&0);

    let diff = rules.advance_turn(&mut s.state);

    let iron_after = *s.state.civilizations.iter()
        .find(|c| c.id == s.rome_id).unwrap()
        .strategic_resources.get(&BuiltinResource::Iron).unwrap_or(&0);

    assert_eq!(iron_after, iron_before + 1, "1 Iron should be gained per turn from the Mine");
    assert!(
        diff.deltas.iter().any(|d| matches!(d,
            StateDelta::StrategicResourceChanged { resource, delta, civ }
            if *resource == BuiltinResource::Iron && *delta == 1 && *civ == s.rome_id
        )),
        "StrategicResourceChanged(+1 Iron) expected in diff"
    );
}

/// Strategic resources are NOT accumulated from tiles without an improvement.
#[test]
fn strategic_resource_not_accumulated_without_improvement() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place Iron but no improvement.
    let iron_tile = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(iron_tile) {
        t.resource = Some(BuiltinResource::Iron);
        t.owner    = Some(s.rome_id);
        // No improvement.
    }
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .worked_tiles.push(iron_tile);

    rules.advance_turn(&mut s.state);

    let iron = *s.state.civilizations.iter()
        .find(|c| c.id == s.rome_id).unwrap()
        .strategic_resources.get(&BuiltinResource::Iron).unwrap_or(&0);
    assert_eq!(iron, 0, "Iron without improvement should not accumulate");
}

// ---------------------------------------------------------------------------
// City Defenses -- wall defense bonus
// ---------------------------------------------------------------------------

/// When a defender stands on a city tile with walls, the wall defense bonus
/// increases effective combat strength, reducing damage taken.
#[test]
fn wall_defense_bonus_reduces_damage_to_defender() {
    // Run two attacks with the same seed setup: one without walls, one with.
    // The defender on the walled city should take less damage.

    fn run_attack_on_city(wall_level: WallLevel) -> u32 {
        let mut s = common::build_scenario();
        let rules = DefaultRulesEngine;

        // Give Babylon's city the specified wall level.
        let city = s.state.cities.iter_mut()
            .find(|c| c.id == s.babylon_city).unwrap();
        city.walls = wall_level;
        city.wall_hp = wall_level.max_hp();

        // Place an enemy unit on Babylon's city tile to defend it.
        let defender_id = s.state.id_gen.next_unit_id();
        let city_coord = HexCoord::from_qr(10, 5);
        s.state.units.push(BasicUnit {
            id:              defender_id,
            unit_type:       s.warrior_type,
            owner:           s.babylon_id,
            coord:           city_coord,
            domain:          UnitDomain::Land,
            category:        UnitCategory::Combat,
            movement_left:   200,
            max_movement:    200,
            combat_strength: Some(20),
            promotions:      Vec::new(),
            experience:      0,
            health:          100,
            range:           0,
            vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        // Place an attacker adjacent to the city.
        let attacker_id = s.state.id_gen.next_unit_id();
        let atk_coord = HexCoord::from_qr(11, 5);
        s.state.units.push(BasicUnit {
            id:              attacker_id,
            unit_type:       s.warrior_type,
            owner:           s.rome_id,
            coord:           atk_coord,
            domain:          UnitDomain::Land,
            category:        UnitCategory::Combat,
            movement_left:   200,
            max_movement:    200,
            combat_strength: Some(20),
            promotions:      Vec::new(),
            experience:      0,
            health:          100,
            range:           0,
            vision_range:    2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        let diff = rules.attack(&mut s.state, attacker_id, defender_id)
            .expect("attack should succeed");

        // Extract defender damage from the diff.
        diff.deltas.iter().find_map(|d| {
            if let StateDelta::UnitAttacked { defender_damage, .. } = d {
                Some(*defender_damage)
            } else {
                None
            }
        }).expect("UnitAttacked delta expected")
    }

    let damage_no_walls   = run_attack_on_city(WallLevel::None);
    let damage_with_walls = run_attack_on_city(WallLevel::Renaissance);

    // Renaissance walls add +8 defense; defender should take strictly less damage.
    assert!(
        damage_with_walls < damage_no_walls,
        "Renaissance walls should reduce defender damage: without={damage_no_walls}, with={damage_with_walls}"
    );
}

// ---------------------------------------------------------------------------
// City Defenses -- wall HP damage on melee attacks
// ---------------------------------------------------------------------------

/// A melee attack on a unit standing on a walled city deals splash damage
/// to the city's wall HP and emits a `WallDamaged` delta.
#[test]
fn melee_attack_damages_city_walls() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Babylon's city Ancient walls (50 HP).
    let city = s.state.cities.iter_mut()
        .find(|c| c.id == s.babylon_city).unwrap();
    city.walls = WallLevel::Ancient;
    city.wall_hp = WallLevel::Ancient.max_hp();
    let initial_wall_hp = city.wall_hp;

    // Place a defender on the city tile.
    let defender_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: defender_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Place a melee attacker adjacent.
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(11, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, attacker_id, defender_id)
        .expect("attack should succeed");

    // WallDamaged delta must be present.
    let wall_damaged = diff.deltas.iter().find_map(|d| {
        if let StateDelta::WallDamaged { city, damage, hp_remaining } = d {
            Some((*city, *damage, *hp_remaining))
        } else {
            None
        }
    });
    assert!(wall_damaged.is_some(), "WallDamaged delta expected after melee attack on walled city");
    let (city_id, damage, hp_remaining) = wall_damaged.unwrap();
    assert_eq!(city_id, s.babylon_city);
    assert!(damage > 0, "wall should take non-zero damage");
    assert!(hp_remaining < initial_wall_hp, "wall HP should decrease");

    // Verify state was mutated.
    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.wall_hp, hp_remaining, "city.wall_hp should match delta");
}

/// When wall HP reaches zero from a melee attack, WallDestroyed is emitted
/// and the wall level is set to None.
#[test]
fn wall_destruction_when_hp_reaches_zero() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Babylon's city Ancient walls with just 1 HP remaining.
    let city = s.state.cities.iter_mut()
        .find(|c| c.id == s.babylon_city).unwrap();
    city.walls = WallLevel::Ancient;
    city.wall_hp = 1;

    // Defender on city tile.
    let defender_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: defender_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Strong attacker to ensure enough damage for wall_damage = def_damage/2 >= 1.
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(11, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(40), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, attacker_id, defender_id)
        .expect("attack should succeed");

    // WallDestroyed delta must be present.
    let wall_destroyed = diff.deltas.iter().find_map(|d| {
        if let StateDelta::WallDestroyed { city, previous_level } = d {
            Some((*city, *previous_level))
        } else {
            None
        }
    });
    assert!(wall_destroyed.is_some(), "WallDestroyed delta expected when wall HP reaches 0");
    let (city_id, prev) = wall_destroyed.unwrap();
    assert_eq!(city_id, s.babylon_city);
    assert_eq!(prev, WallLevel::Ancient);

    // City should now have no walls.
    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.walls, WallLevel::None);
    assert_eq!(city.wall_hp, 0);
}

/// Ranged attacks do NOT damage city walls (only melee does).
#[test]
fn ranged_attack_does_not_damage_walls() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Babylon's city Ancient walls.
    let city = s.state.cities.iter_mut()
        .find(|c| c.id == s.babylon_city).unwrap();
    city.walls = WallLevel::Ancient;
    city.wall_hp = WallLevel::Ancient.max_hp();
    let initial_wall_hp = city.wall_hp;

    // Defender on city tile.
    let defender_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: defender_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Ranged attacker within range 2.
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(12, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(25), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 2, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, attacker_id, defender_id)
        .expect("attack should succeed");

    // No WallDamaged delta for ranged attacks.
    let has_wall_damaged = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::WallDamaged { .. })
    });
    assert!(!has_wall_damaged, "ranged attacks should not damage city walls");

    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.wall_hp, initial_wall_hp, "wall HP should be unchanged after ranged attack");
}

// ---------------------------------------------------------------------------
// City Defenses -- city bombardment
// ---------------------------------------------------------------------------

/// A city with walls can bombard an adjacent enemy unit, dealing damage
/// with no counter-damage, using AttackType::CityBombard.
#[test]
fn city_bombard_deals_damage_no_counter() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Rome's city Ancient walls.
    let city = s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap();
    city.walls = WallLevel::Ancient;
    city.wall_hp = WallLevel::Ancient.max_hp();

    // Place an enemy unit adjacent to Rome's city (3,3) -> (4,3).
    let target_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: target_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(4, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.city_bombard(&mut s.state, s.rome_city, target_id)
        .expect("city_bombard should succeed");

    // Must have UnitAttacked with CityBombard type.
    let attacked = diff.deltas.iter().find_map(|d| {
        if let StateDelta::UnitAttacked { attack_type, attacker_damage, defender_damage, .. } = d {
            Some((*attack_type, *attacker_damage, *defender_damage))
        } else {
            None
        }
    }).expect("UnitAttacked delta expected");
    assert_eq!(attacked.0, AttackType::CityBombard);
    assert_eq!(attacked.1, 0, "city should take no counter-damage");
    assert!(attacked.2 > 0, "target should take damage from bombardment");

    // Target health should be reduced.
    let target_hp = s.state.unit(target_id).map(|u| u.health).unwrap_or(0);
    assert!(target_hp < 100, "target health should be reduced");
}

/// A city without walls cannot bombard -- returns CityCannotAttack.
#[test]
fn city_bombard_requires_walls() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Rome's city has WallLevel::None by default.
    let target_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: target_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(4, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let result = rules.city_bombard(&mut s.state, s.rome_city, target_id);
    assert!(matches!(result, Err(RulesError::CityCannotAttack)),
        "city without walls should return CityCannotAttack, got: {result:?}");
}

/// City bombardment has range 2; targets at distance 3 are out of range.
#[test]
fn city_bombard_range_check() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let city = s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap();
    city.walls = WallLevel::Ancient;
    city.wall_hp = WallLevel::Ancient.max_hp();

    // Place target at distance 3 from Rome's city (3,3) -> (6,3).
    let target_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: target_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(6, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let result = rules.city_bombard(&mut s.state, s.rome_city, target_id);
    assert!(matches!(result, Err(RulesError::NotInRange)),
        "target at distance 3 should be out of range, got: {result:?}");
}

/// A city can only bombard once per turn; second attempt returns
/// CityAlreadyAttacked, and the flag resets after advance_turn.
#[test]
fn city_bombard_once_per_turn_resets_after_advance() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let city = s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap();
    city.walls = WallLevel::Ancient;
    city.wall_hp = WallLevel::Ancient.max_hp();

    // Place two enemy units adjacent to Rome's city.
    let target1 = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: target1, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(4, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });
    let target2 = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: target2, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(2, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // First bombardment succeeds.
    rules.city_bombard(&mut s.state, s.rome_city, target1)
        .expect("first bombardment should succeed");

    // Second bombardment same turn fails.
    let result = rules.city_bombard(&mut s.state, s.rome_city, target2);
    assert!(matches!(result, Err(RulesError::CityAlreadyAttacked)),
        "second bombardment same turn should fail, got: {result:?}");

    // After advance_turn, the flag resets and bombardment works again.
    common::advance_turn(&mut s);
    let result2 = rules.city_bombard(&mut s.state, s.rome_city, target2);
    assert!(result2.is_ok(), "bombardment should succeed after advance_turn resets the flag");
}

// ---------------------------------------------------------------------------
// City Defenses -- siege unit bonus
// ---------------------------------------------------------------------------

/// A siege unit gets bonus attack strength when attacking a unit on a city tile.
#[test]
fn siege_unit_bonus_applies_on_city_tile() {
    use libciv::game::state::UnitTypeDef;

    // Run two attacks: one with siege_bonus, one without. Same base strength.
    fn run_attack(siege_bonus: u32) -> u32 {
        let mut s = common::build_scenario();
        let rules = DefaultRulesEngine;

        // Register a ranged unit type with the specified siege_bonus.
        let catapult_type = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
        s.state.unit_type_defs.push(UnitTypeDef {
            id:              catapult_type,
            name:            "catapult",
            production_cost: 120,
            domain:          UnitDomain::Land,
            category:        UnitCategory::Combat,
            max_movement:    200,
            combat_strength: Some(20),
            range:           2,
            vision_range:    2,
            can_found_city:  false,
            resource_cost:   None,
            siege_bonus, max_charges: 0,
            exclusive_to:    None,
            replaces:        None,
            era:             None,
            promotion_class: None,
        });

        // Defender on Babylon's city tile (10, 5).
        let defender_id = s.state.id_gen.next_unit_id();
        s.state.units.push(BasicUnit {
            id: defender_id, unit_type: s.warrior_type, owner: s.babylon_id,
            coord: HexCoord::from_qr(10, 5),
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200,
            combat_strength: Some(20), promotions: Vec::new(),
            experience: 0,
            health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        // Attacker within range 2 of the city.
        let attacker_id = s.state.id_gen.next_unit_id();
        s.state.units.push(BasicUnit {
            id: attacker_id, unit_type: catapult_type, owner: s.rome_id,
            coord: HexCoord::from_qr(12, 5),
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200,
            combat_strength: Some(20), promotions: Vec::new(),
            experience: 0,
            health: 100, range: 2, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        let diff = rules.attack(&mut s.state, attacker_id, defender_id)
            .expect("attack should succeed");

        diff.deltas.iter().find_map(|d| {
            if let StateDelta::UnitAttacked { defender_damage, .. } = d {
                Some(*defender_damage)
            } else {
                None
            }
        }).expect("UnitAttacked delta expected")
    }

    let damage_no_siege   = run_attack(0);
    let damage_with_siege = run_attack(10);

    assert!(
        damage_with_siege > damage_no_siege,
        "siege bonus should increase damage on city tile: without={damage_no_siege}, with={damage_with_siege}"
    );
}

/// Siege bonus does NOT apply when the defender is not on a city tile.
#[test]
fn siege_bonus_not_applied_in_open_field() {
    use libciv::game::state::UnitTypeDef;

    fn run_attack_open_field(siege_bonus: u32) -> u32 {
        let mut s = common::build_scenario();
        let rules = DefaultRulesEngine;

        let catapult_type = libciv::UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
        s.state.unit_type_defs.push(UnitTypeDef {
            id:              catapult_type,
            name:            "catapult",
            production_cost: 120,
            domain:          UnitDomain::Land,
            category:        UnitCategory::Combat,
            max_movement:    200,
            combat_strength: Some(20),
            range:           2,
            vision_range:    2,
            can_found_city:  false,
            resource_cost:   None,
            siege_bonus, max_charges: 0,
            exclusive_to:    None,
            replaces:        None,
            era:             None,
            promotion_class: None,
        });

        // Defender in open field (not on a city tile).
        let defender_id = s.state.id_gen.next_unit_id();
        s.state.units.push(BasicUnit {
            id: defender_id, unit_type: s.warrior_type, owner: s.babylon_id,
            coord: HexCoord::from_qr(7, 4),
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200,
            combat_strength: Some(20), promotions: Vec::new(),
            experience: 0,
            health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        // Ranged attacker within range 2.
        let attacker_id = s.state.id_gen.next_unit_id();
        s.state.units.push(BasicUnit {
            id: attacker_id, unit_type: catapult_type, owner: s.rome_id,
            coord: HexCoord::from_qr(9, 4),
            domain: UnitDomain::Land, category: UnitCategory::Combat,
            movement_left: 200, max_movement: 200,
            combat_strength: Some(20), promotions: Vec::new(),
            experience: 0,
            health: 100, range: 2, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });

        let diff = rules.attack(&mut s.state, attacker_id, defender_id)
            .expect("attack should succeed");

        diff.deltas.iter().find_map(|d| {
            if let StateDelta::UnitAttacked { defender_damage, .. } = d {
                Some(*defender_damage)
            } else {
                None
            }
        }).expect("UnitAttacked delta expected")
    }

    let damage_no_siege   = run_attack_open_field(0);
    let damage_with_siege = run_attack_open_field(10);

    // In open field, siege bonus should NOT apply -- damage should be equal.
    assert_eq!(
        damage_no_siege, damage_with_siege,
        "siege bonus should not apply in open field: without={damage_no_siege}, with={damage_with_siege}"
    );
}

// ---------------------------------------------------------------------------
// City capture
// ---------------------------------------------------------------------------

/// When a melee attacker kills the last defender on an enemy city tile, the
/// city is captured: ownership transfers, CityCaptured is emitted, and the
/// city's ownership flag is set to Occupied.
#[test]
fn city_capture_transfers_ownership_on_last_defender_killed() {
    use libciv::civ::CityOwnership;

    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a single Babylon defender on Babylon's city tile (10, 5).
    let defender_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: defender_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Rome attacker adjacent; strong enough to one-shot the defender.
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(11, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(60), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, attacker_id, defender_id)
        .expect("attack should succeed");

    // CityCaptured delta must be present.
    let captured = diff.deltas.iter().find_map(|d| {
        if let StateDelta::CityCaptured { city, new_owner, old_owner } = d {
            Some((*city, *new_owner, *old_owner))
        } else {
            None
        }
    });
    assert!(captured.is_some(), "CityCaptured delta expected after killing last defender");
    let (city_id, new_owner, old_owner) = captured.unwrap();
    assert_eq!(city_id, s.babylon_city);
    assert_eq!(new_owner, s.rome_id);
    assert_eq!(old_owner, s.babylon_id);

    // City state must reflect the new owner and Occupied status.
    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.owner, s.rome_id, "city.owner should be Rome");
    assert_eq!(city.ownership, CityOwnership::Occupied, "captured city should be Occupied");

    // Civilization city lists must be updated.
    let rome_cities = &s.state.civilizations.iter()
        .find(|c| c.id == s.rome_id).unwrap().cities;
    let babylon_cities = &s.state.civilizations.iter()
        .find(|c| c.id == s.babylon_id).unwrap().cities;
    assert!(rome_cities.contains(&s.babylon_city), "Rome's city list should include captured city");
    assert!(!babylon_cities.contains(&s.babylon_city), "Babylon's city list should no longer include the city");
}

/// City is only captured when the LAST defender on the tile is killed.
/// With two defenders, killing only one should NOT trigger a capture.
/// After both are dead, the city should be captured.
#[test]
fn city_capture_destroys_garrisoned_units_on_tile() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Two Babylon units on the city tile (both weak so we can kill them in sequence).
    let defender1_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: defender1_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 1,   // one HP so it dies immediately
        range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });
    let garrison_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: garrison_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 1,   // also one HP
        range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Rome attacker adjacent.
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(11, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Kill defender1.  Garrison still alive → no capture yet.
    let diff1 = rules.attack(&mut s.state, attacker_id, defender1_id)
        .expect("attack should succeed");
    let captured1 = diff1.deltas.iter().any(|d| matches!(d, StateDelta::CityCaptured { .. }));
    assert!(!captured1, "should not capture city while garrison remains");

    // Reset movement for the attacker so it can attack again.
    if let Some(u) = s.state.unit_mut(attacker_id) { u.movement_left = 200; }

    // Kill garrison (the last defender) → city should now be captured.
    let diff2 = rules.attack(&mut s.state, attacker_id, garrison_id)
        .expect("attack should succeed");
    let captured2 = diff2.deltas.iter().any(|d| matches!(d, StateDelta::CityCaptured { .. }));
    assert!(captured2, "city should be captured when last defender falls");

    // The garrison should not exist in the live unit list.
    assert!(
        s.state.unit(garrison_id).is_none(),
        "garrison unit should be removed from state after capture"
    );

}

/// A non-melee (ranged) kill on a city tile does NOT capture the city.
#[test]
fn ranged_kill_on_city_tile_does_not_capture() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Weak defender (1 HP) on Babylon's city tile.
    let defender_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: defender_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 1, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Ranged attacker within range 2.
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(12, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 2, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, attacker_id, defender_id)
        .expect("attack should succeed");

    // No CityCaptured delta for ranged kills.
    let captured = diff.deltas.iter().any(|d| matches!(d, StateDelta::CityCaptured { .. }));
    assert!(!captured, "ranged kills should not capture a city");

    // City still belongs to Babylon.
    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.owner, s.babylon_id, "city owner should not change after a ranged kill");
}

/// If any old-owner units are still alive on the city tile after the attack,
/// no capture occurs yet.
#[test]
fn no_capture_while_defenders_remain() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Two Babylon warriors on the city tile; first one has lots of HP.
    let defender1_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: defender1_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });
    let _defender2_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: _defender2_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(10, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Weak attacker that deals very little damage (won't kill the defender).
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(11, 5),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(1), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.attack(&mut s.state, attacker_id, defender1_id)
        .expect("attack should succeed");

    // No capture while a second Babylon unit remains on the tile.
    let captured = diff.deltas.iter().any(|d| matches!(d, StateDelta::CityCaptured { .. }));
    assert!(!captured, "city should not be captured while defenders still remain on the tile");
    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.owner, s.babylon_id, "city should still belong to Babylon");
}

// ---------------------------------------------------------------------------
// City bombardment without walls
// ---------------------------------------------------------------------------

/// A city whose walls have been breached (WallLevel::None, wall_hp == 0)
/// can no longer perform a bombardment.  This exercises the post-destruction
/// path: walls existed, were zeroed out, and bombardment must now fail.
#[test]
fn city_bombard_fails_after_walls_are_destroyed() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Start with Ancient walls, then simulate a breach by clearing them.
    {
        let city = s.state.cities.iter_mut()
            .find(|c| c.id == s.rome_city).unwrap();
        city.walls    = WallLevel::None; // walls were destroyed (breach)
        city.wall_hp  = 0;
    }

    // Enemy unit within range 2 of Rome's city (3,3) -> (4,3) is adjacent.
    let enemy_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: enemy_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(4, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let result = rules.city_bombard(&mut s.state, s.rome_city, enemy_id);
    assert!(
        matches!(result, Err(RulesError::CityCannotAttack)),
        "city with no walls should not be able to bombard, got: {result:?}"
    );
}

/// Wall breach via combat (wall_hp driven to 0) causes subsequent
/// city bombardment to fail.
#[test]
fn city_bombard_fails_after_walls_breached_by_combat() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Rome's city gets Ancient walls at 1 HP.
    {
        let city = s.state.cities.iter_mut()
            .find(|c| c.id == s.rome_city).unwrap();
        city.walls   = WallLevel::Ancient;
        city.wall_hp = 1;
    }

    // Place a Babylon defender ON Rome's city tile (3,3) so the melee attack
    // triggers wall damage (def_coord == city.coord).
    let def_on_city = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: def_on_city, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(3, 3),   // Rome's city tile
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Strong Rome attacker at (4,3) -- adjacent to (3,3) -- attacks the
    // Babylon unit that has occupied Rome's city tile.
    let attacker_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: attacker_id, unit_type: s.warrior_type, owner: s.rome_id,
        coord: HexCoord::from_qr(4, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(60), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // Attack the unit on Rome's city tile; wall_damage = def_damage/2 >= 1,
    // so the 1-HP wall is breached and a WallDestroyed delta is emitted.
    let diff = rules.attack(&mut s.state, attacker_id, def_on_city)
        .expect("attack should succeed");

    let wall_destroyed = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::WallDestroyed { city, .. } if *city == s.rome_city)
    });
    assert!(wall_destroyed, "WallDestroyed expected after combat reduced wall_hp to 0");

    // City walls should now be None.
    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert_eq!(city.walls, WallLevel::None, "walls should be None after breach");

    // Bombardment should now fail.
    let new_enemy_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: new_enemy_id, unit_type: s.warrior_type, owner: s.babylon_id,
        coord: HexCoord::from_qr(4, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let result = rules.city_bombard(&mut s.state, s.rome_city, new_enemy_id);
    assert!(
        matches!(result, Err(RulesError::CityCannotAttack)),
        "city with breached walls should not be able to bombard, got: {result:?}"
    );
}
