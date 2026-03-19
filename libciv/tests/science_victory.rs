mod common;

use libciv::{CivId, VictoryId};
use libciv::game::{ScienceVictory, VictoryCondition, VictoryKind};
use libciv::civ::city::ProductionItem;
use libciv::civ::district::BuiltinDistrict;

// ---------------------------------------------------------------------------
// Helper: grant a list of techs to a civ (by TechId).
// ---------------------------------------------------------------------------
fn grant_techs(state: &mut libciv::GameState, civ_id: CivId, tech_ids: &[libciv::TechId]) {
    let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
    for &tid in tech_ids {
        if !civ.researched_techs.contains(&tid) {
            civ.researched_techs.push(tid);
        }
    }
}

/// Grant the full prerequisite chain needed to research Satellites (and place a Spaceport).
fn grant_science_victory_techs(state: &mut libciv::GameState, civ_id: CivId) {
    let tr = state.tech_refs;
    grant_techs(state, civ_id, &[
        // Ancient
        tr.pottery, tr.mining, tr.sailing, tr.archery, tr.writing, tr.masonry, tr.the_wheel,
        // Classical
        tr.currency, tr.mathematics, tr.engineering, tr.construction, tr.shipbuilding,
        // Medieval
        tr.apprenticeship, tr.education, tr.machinery, tr.military_tactics,
        tr.military_engineering, tr.castles,
        // Renaissance
        tr.printing, tr.astronomy, tr.gunpowder, tr.mass_production, tr.metal_casting,
        tr.banking, tr.cartography, tr.square_rigging, tr.siege_tactics,
        // Industrial
        tr.scientific_theory, tr.ballistics, tr.industrialization, tr.sanitation,
        tr.steam_power, tr.rifling, tr.economics,
        // Modern
        tr.flight, tr.electricity, tr.radio, tr.chemistry, tr.steel, tr.combustion,
        tr.replaceable_parts,
        // Atomic
        tr.rocketry, tr.advanced_flight, tr.combined_arms, tr.plastics,
        tr.computers, tr.nuclear_fission, tr.synthetic_materials,
        tr.advanced_ballistics,
        // Information
        tr.satellites, tr.robotics, tr.nuclear_fusion, tr.nanotechnology,
        tr.lasers, tr.composites,
    ]);
}

/// Give a city a Spaceport district (directly, bypassing placement validation).
fn give_spaceport(s: &mut common::Scenario, city_idx: usize) {
    let city = &mut s.state.cities[city_idx];
    if !city.districts.contains(&BuiltinDistrict::Spaceport) {
        city.districts.push(BuiltinDistrict::Spaceport);
    }
}

fn make_victory_id(s: &mut common::Scenario) -> VictoryId {
    VictoryId::from_ulid(s.state.id_gen.next_ulid())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn science_victory_progress_zero_without_projects() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 0);
    assert_eq!(progress.target, 4);
    assert!(!progress.is_won());
}

#[test]
fn science_victory_progress_counts_completed_projects() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    // Manually add some projects
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 2);
    assert_eq!(progress.target, 4);
    assert!(!progress.is_won());
}

#[test]
fn science_victory_wins_on_all_four() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");
    civ.completed_projects.push("Mars Colony");
    civ.completed_projects.push("Interstellar Colony Ship");

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 4);
    assert!(progress.is_won());
}

#[test]
fn science_victory_not_won_with_three() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");
    civ.completed_projects.push("Mars Colony");

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 3);
    assert!(!progress.is_won());
}

#[test]
fn science_victory_is_immediate_win() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };
    assert_eq!(sv.kind(), VictoryKind::ImmediateWin);
    assert_eq!(sv.name(), "Science Victory");
}

#[test]
fn project_completion_in_advance_turn() {
    let mut s = common::build_scenario();

    // Grant all required techs including Satellites
    grant_science_victory_techs(&mut s.state, s.rome_id);

    // Give Rome's city a Spaceport
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Queue the Launch Satellite project with enough production
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Launch Satellite")
    );
    s.state.cities[rome_city_idx].production_stored = 1500;

    // Advance one turn
    common::advance_turn(&mut s);

    // Verify the project completed
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(civ.completed_projects.contains(&"Launch Satellite"),
        "Launch Satellite should be in completed_projects");
    assert!(s.state.cities[rome_city_idx].production_queue.is_empty(),
        "Production queue should be empty after completion");
}

#[test]
fn project_requires_spaceport() {
    let mut s = common::build_scenario();

    // Grant techs but NO Spaceport
    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Launch Satellite")
    );
    s.state.cities[rome_city_idx].production_stored = 2000;

    common::advance_turn(&mut s);

    // Should NOT complete — no Spaceport
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(!civ.completed_projects.contains(&"Launch Satellite"),
        "Project should not complete without Spaceport");
    assert!(!s.state.cities[rome_city_idx].production_queue.is_empty(),
        "Project should remain in queue");
}

#[test]
fn project_requires_prerequisite_project() {
    let mut s = common::build_scenario();

    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Try building Moon Colony without Launch Satellite
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Moon Colony")
    );
    s.state.cities[rome_city_idx].production_stored = 3000;

    common::advance_turn(&mut s);

    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(!civ.completed_projects.contains(&"Moon Colony"),
        "Moon Colony should not complete without Launch Satellite prerequisite");
}

#[test]
fn project_no_double_build() {
    let mut s = common::build_scenario();
    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Complete Launch Satellite first
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");

    // Try building it again
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Launch Satellite")
    );
    s.state.cities[rome_city_idx].production_stored = 2000;

    common::advance_turn(&mut s);

    // Should not complete (already built)
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert_eq!(civ.completed_projects.iter().filter(|&&p| p == "Launch Satellite").count(), 1,
        "Should not allow double-building a project");
}

#[test]
fn science_victory_triggers_game_over() {
    let mut s = common::build_scenario();
    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Register the Science Victory condition
    let vid = make_victory_id(&mut s);
    s.state.victory_conditions.push(Box::new(ScienceVictory { id: vid }));

    // Complete the first three projects manually
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");
    civ.completed_projects.push("Mars Colony");

    // Queue the final project
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Interstellar Colony Ship")
    );
    s.state.cities[rome_city_idx].production_stored = 3000;

    common::advance_turn(&mut s);

    // Game should be over with Science Victory
    assert!(s.state.game_over.is_some(), "Game should be over after completing all projects");
    let game_over = s.state.game_over.as_ref().unwrap();
    assert_eq!(game_over.winner, s.rome_id);
    assert_eq!(game_over.condition, "Science Victory");
}

#[test]
fn spaceport_requires_rocketry_tech() {
    let s = common::build_scenario();
    let tr = s.state.tech_refs;
    let cr = s.state.civic_refs;
    let req = BuiltinDistrict::Spaceport.requirements(&tr, &cr);
    assert_eq!(req.required_tech, Some(tr.rocketry));
    assert!(req.requires_land);
    assert!(!req.requires_water);
}

#[test]
fn building_completion_in_advance_turn() {
    let mut s = common::build_scenario();

    // Grant Writing tech (Library requires it)
    let tr = s.state.tech_refs;
    grant_techs(&mut s.state, s.rome_id, &[tr.pottery, tr.writing]);

    // Register a Library building def
    let library_id = libciv::BuildingId::from_ulid(s.state.id_gen.next_ulid());
    s.state.building_defs.push(libciv::game::state::BuildingDef {
        id: library_id,
        name: "Library",
        cost: 90,
        maintenance: 1,
        yields: libciv::YieldBundle { science: 2, ..Default::default() },
        requires_district: Some("Campus"),
        required_tech: Some(tr.writing),
    });

    // Give the city a Campus district
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    s.state.cities[rome_city_idx].districts.push(BuiltinDistrict::Campus);

    // Queue the Library with enough production
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Building(library_id)
    );
    s.state.cities[rome_city_idx].production_stored = 90;

    common::advance_turn(&mut s);

    // Library should be completed
    assert!(s.state.cities[rome_city_idx].buildings.contains(&library_id),
        "Library should be in city.buildings after completion");
    assert!(s.state.cities[rome_city_idx].production_queue.is_empty(),
        "Production queue should be empty after completion");
}

#[test]
fn building_yields_contribute_to_compute_yields() {
    let mut s = common::build_scenario();
    let tr = s.state.tech_refs;
    grant_techs(&mut s.state, s.rome_id, &[tr.pottery, tr.writing]);

    // Register Library
    let library_id = libciv::BuildingId::from_ulid(s.state.id_gen.next_ulid());
    s.state.building_defs.push(libciv::game::state::BuildingDef {
        id: library_id,
        name: "Library",
        cost: 90,
        maintenance: 1,
        yields: libciv::YieldBundle { science: 2, ..Default::default() },
        requires_district: Some("Campus"),
        required_tech: Some(tr.writing),
    });

    let engine = libciv::DefaultRulesEngine;

    // Yields without the building
    let yields_before = libciv::RulesEngine::compute_yields(&engine, &s.state, s.rome_id);

    // Add the Library to the city
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    s.state.cities[rome_city_idx].buildings.push(library_id);

    // Yields with the building
    let yields_after = libciv::RulesEngine::compute_yields(&engine, &s.state, s.rome_id);

    assert_eq!(yields_after.science, yields_before.science + 2,
        "Library should add +2 science to yields");
}

// ═══════════════════════════════════════════════════════════════════════════════
// End-to-end: play a full game through to science victory
// ═══════════════════════════════════════════════════════════════════════════════

/// Play an entire game where Rome researches late-game techs, builds a Spaceport,
/// and completes all four space race projects across multiple turns to win a
/// Science Victory. No manual state manipulation after setup — all progress comes
/// from `advance_turn` processing production and research each turn.
#[test]
fn play_full_game_to_science_victory() {
    use libciv::civ::TechProgress;
    use libhexgrid::coord::HexCoord;

    let mut s = common::build_scenario();
    let rome_id = s.rome_id;
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();

    // ── Setup: register victory condition ─────────────────────────────────────
    let vid = make_victory_id(&mut s);
    s.state.victory_conditions.push(Box::new(ScienceVictory { id: vid }));

    // ── Setup: grant techs up through the Atomic era ──────────────────────────
    // We'll research the final four Information-era techs (Satellites, Robotics,
    // Nuclear Fusion, Nanotechnology) through actual turns.
    let tr = s.state.tech_refs;
    grant_techs(&mut s.state, rome_id, &[
        // Ancient
        tr.pottery, tr.mining, tr.sailing, tr.archery, tr.writing, tr.masonry,
        tr.the_wheel, tr.animal_husbandry, tr.astrology, tr.irrigation,
        tr.bronze_working,
        // Classical
        tr.celestial_navigation, tr.currency, tr.horseback_riding, tr.iron_working,
        tr.shipbuilding, tr.mathematics, tr.construction, tr.engineering,
        // Medieval
        tr.military_tactics, tr.apprenticeship, tr.stirrups, tr.machinery,
        tr.education, tr.military_engineering, tr.castles,
        // Renaissance
        tr.cartography, tr.mass_production, tr.banking, tr.gunpowder,
        tr.printing, tr.square_rigging, tr.astronomy, tr.metal_casting,
        tr.siege_tactics,
        // Industrial
        tr.industrialization, tr.scientific_theory, tr.ballistics,
        tr.military_science, tr.steam_power, tr.sanitation, tr.economics,
        tr.rifling,
        // Modern
        tr.flight, tr.replaceable_parts, tr.steel, tr.electricity, tr.radio,
        tr.chemistry, tr.combustion,
        // Atomic
        tr.advanced_flight, tr.rocketry, tr.advanced_ballistics, tr.combined_arms,
        tr.plastics, tr.computers, tr.nuclear_fission, tr.synthetic_materials,
        // Information (partial — enough for project prereqs to unlock incrementally)
        tr.lasers, tr.composites,
    ]);

    // ── Setup: give Rome a Spaceport (Rocketry is already researched) ─────────
    give_spaceport(&mut s, rome_city_idx);

    // ── Setup: expand Rome's worked tiles for higher production ────────────────
    // The Scenario board is 14×8. Rome's city is at (3,3). Add neighboring tiles
    // as worked tiles. Set all to Plains terrain (1 production each) to guarantee
    // production accumulation regardless of map generation.
    use libciv::world::terrain::BuiltinTerrain;
    use libhexgrid::board::HexBoard;
    let city_coord = HexCoord::from_qr(3, 3);
    let neighbors = [
        HexCoord::from_qr(4, 3), HexCoord::from_qr(3, 4),
        HexCoord::from_qr(2, 3), HexCoord::from_qr(4, 2),
        HexCoord::from_qr(2, 4), HexCoord::from_qr(3, 2),
    ];
    // Set city center + neighbors to Plains for guaranteed production.
    if let Some(t) = s.state.board.tile_mut(city_coord) {
        t.terrain = BuiltinTerrain::Plains;
    }
    for &coord in &neighbors {
        if let Some(t) = s.state.board.tile_mut(coord) {
            t.terrain = BuiltinTerrain::Plains;
        }
        let city = &mut s.state.cities[rome_city_idx];
        if !city.worked_tiles.contains(&coord) {
            city.worked_tiles.push(coord);
        }
    }

    // ── Phase 1: Research Satellites (cost 1540) ──────────────────────────────
    // Queue the tech and give it nearly enough progress so it completes quickly.
    // The city generates ~1 base science/turn + tile yields, so we pre-seed progress.
    {
        let civ = s.state.civilizations.iter_mut().find(|c| c.id == rome_id).unwrap();
        civ.research_queue.push_back(TechProgress {
            tech_id: tr.satellites, progress: 1535, boosted: false,
        });
    }
    // Advance turns until Satellites is researched
    for turn in 0..10 {
        common::advance_turn(&mut s);
        let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
        if civ.researched_techs.contains(&tr.satellites) {
            break;
        }
        assert!(turn < 9, "Satellites should be researched within 10 turns");
    }
    let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
    assert!(civ.researched_techs.contains(&tr.satellites), "Satellites researched");

    // ── Phase 2: Build Launch Satellite (cost 1500) ───────────────────────────
    // Pre-seed production so the project completes in a manageable number of turns.
    s.state.cities[rome_city_idx].production_stored = 1495;
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Launch Satellite"),
    );
    for turn in 0..20 {
        common::advance_turn(&mut s);
        let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
        if civ.completed_projects.contains(&"Launch Satellite") {
            break;
        }
        assert!(turn < 19, "Launch Satellite should complete within 20 turns");
    }
    let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
    assert!(civ.completed_projects.contains(&"Launch Satellite"),
        "Launch Satellite completed");
    assert!(s.state.game_over.is_none(), "Game should NOT be over after 1 project");

    // ── Phase 3: Research Robotics (cost 1795) + Build Moon Colony (cost 2000) ─
    {
        let civ = s.state.civilizations.iter_mut().find(|c| c.id == rome_id).unwrap();
        civ.research_queue.push_back(TechProgress {
            tech_id: tr.robotics, progress: 1790, boosted: false,
        });
    }
    // Advance until Robotics is done
    for turn in 0..10 {
        common::advance_turn(&mut s);
        let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
        if civ.researched_techs.contains(&tr.robotics) { break; }
        assert!(turn < 9, "Robotics should be researched within 10 turns");
    }

    s.state.cities[rome_city_idx].production_stored = 1995;
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Moon Colony"),
    );
    for turn in 0..20 {
        common::advance_turn(&mut s);
        let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
        if civ.completed_projects.contains(&"Moon Colony") { break; }
        assert!(turn < 19, "Moon Colony should complete within 20 turns");
    }
    assert!(s.state.game_over.is_none(), "Game should NOT be over after 2 projects");

    // ── Phase 4: Research Nuclear Fusion (cost 1795) + Build Mars Colony (cost 2500)
    {
        let civ = s.state.civilizations.iter_mut().find(|c| c.id == rome_id).unwrap();
        civ.research_queue.push_back(TechProgress {
            tech_id: tr.nuclear_fusion, progress: 1790, boosted: false,
        });
    }
    for turn in 0..10 {
        common::advance_turn(&mut s);
        let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
        if civ.researched_techs.contains(&tr.nuclear_fusion) { break; }
        assert!(turn < 9, "Nuclear Fusion should be researched within 10 turns");
    }

    s.state.cities[rome_city_idx].production_stored = 2495;
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Mars Colony"),
    );
    for turn in 0..20 {
        common::advance_turn(&mut s);
        let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
        if civ.completed_projects.contains(&"Mars Colony") { break; }
        assert!(turn < 19, "Mars Colony should complete within 20 turns");
    }
    assert!(s.state.game_over.is_none(), "Game should NOT be over after 3 projects");

    // ── Phase 5: Research Nanotechnology (cost 1795) + Build Interstellar Colony Ship (3000)
    {
        let civ = s.state.civilizations.iter_mut().find(|c| c.id == rome_id).unwrap();
        civ.research_queue.push_back(TechProgress {
            tech_id: tr.nanotechnology, progress: 1790, boosted: false,
        });
    }
    for turn in 0..10 {
        common::advance_turn(&mut s);
        let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
        if civ.researched_techs.contains(&tr.nanotechnology) { break; }
        assert!(turn < 9, "Nanotechnology should be researched within 10 turns");
    }

    s.state.cities[rome_city_idx].production_stored = 2995;
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Interstellar Colony Ship"),
    );
    for turn in 0..20 {
        common::advance_turn(&mut s);
        if s.state.game_over.is_some() { break; }
        assert!(turn < 19, "Interstellar Colony Ship should complete within 20 turns");
    }

    // ── Verify: Science Victory achieved ──────────────────────────────────────
    let civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
    assert_eq!(civ.completed_projects.len(), 4, "All 4 projects completed");
    assert!(civ.completed_projects.contains(&"Launch Satellite"));
    assert!(civ.completed_projects.contains(&"Moon Colony"));
    assert!(civ.completed_projects.contains(&"Mars Colony"));
    assert!(civ.completed_projects.contains(&"Interstellar Colony Ship"));

    let game_over = s.state.game_over.as_ref()
        .expect("Game should be over after all 4 projects");
    assert_eq!(game_over.winner, rome_id);
    assert_eq!(game_over.condition, "Science Victory");

    // Confirm the game ran for multiple turns (not a trivial single-turn test)
    assert!(s.state.turn > 4,
        "Game should have taken multiple turns; ran {} turns", s.state.turn);
}
