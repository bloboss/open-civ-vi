/// End-to-end gameplay integration tests.
///
/// Each test exercises one complete gameplay concern (visibility, movement,
/// combat, city founding, etc.) against the public libciv API, using the
/// shared `common::Scenario` setup.  Unit tests for individual rule methods
/// live in `libciv/src/game/rules.rs`; these tests verify full pipelines.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine, TechId, UnitCategory, UnitDomain};
use libciv::civ::{BasicUnit, TechProgress};
use libciv::game::{recalculate_visibility, StateDelta};
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          1,      // barely alive
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        combat_strength: Some(10),
        range:           2,
        vision_range:    2,
        can_found_city:  false,
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
        combat_strength: Some(10),
        promotions:      Vec::new(),
        health:          100,
        range:           2,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
        health:          100,
        range:           0,
        vision_range:    2,
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
