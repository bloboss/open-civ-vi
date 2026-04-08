mod cli;
mod handlers;
mod output;
mod player_view;
mod repl;
mod state_io;

use std::io::{self, BufRead, Write};

use clap::{Parser, Subcommand};
use libciv::{
    all_scores, BarbarianCampId, BeliefId, CityId, CivId, GameState, GameStateDiff,
    BuiltinVictoryCondition, DefaultRulesEngine, GreatPersonType, RulesEngine,
    TurnEngine, UnitCategory, UnitDomain, UnitId, UnitTypeId,
};
use libciv::game::FaithPurchaseItem;
use libciv::ai::{Agent, HeuristicAgent};
use libciv::civ::{BasicUnit, BuiltinAgenda, City, Civilization, Leader, ProductionItem, TechProgress, Unit};
use libciv::civ::district::BuiltinDistrict;
use libciv::civ::great_people::{recruitment_threshold, next_candidate_name, GP_PATRONAGE_GOLD_PER_POINT};
use libciv::game::{AttackType, RulesError, StateDelta, recalculate_visibility};
use libciv::game::state::UnitTypeDef;
use libciv::visualize::Visualizer;
use libciv::world::improvement::BuiltinImprovement;
use libciv::world::mapgen::{MapGenConfig, generate as mapgen_generate};
use libciv::world::road::{AncientRoad, BuiltinRoad};
use libciv::world::tile::WorldTile;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let parsed = cli::Cli::parse();
    match parsed.command {
        // ── New CLI commands ─────────────────────────────────────────────
        cli::Command::NewGame { game_file, seed, width, height, player, ai, victory: _ } => {
            if let Err(e) = handlers::new_game::handle_new_game(
                &game_file, seed, width, height, &player, &ai,
            ) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        cli::Command::Action { game_file, player, action } => {
            if let Err(e) = handlers::action::handle_action(&game_file, &player, &action) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        cli::Command::EndTurn { game_file, player } => {
            if let Err(e) = handlers::end_turn::handle_end_turn(&game_file, &player) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        cli::Command::View { game_file, player } => {
            if let Err(e) = handlers::view::handle_view(&game_file, &player) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        cli::Command::Status { game_file, player, kind } => {
            if let Err(e) = handlers::status::handle_status(&game_file, &player, &kind) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        cli::Command::List { game_file, player, kind } => {
            if let Err(e) = handlers::list::handle_list(&game_file, &player, &kind) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        // ── Interactive REPL ──────────────────────────────────────────────
        cli::Command::Repl { game_file, player } => {
            match repl::ReplSession::new(&game_file, &player) {
                Ok(mut session) => session.run(),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        // ── Legacy interactive commands ──────────────────────────────────
        cli::Command::Play => run_play(),
        cli::Command::Demo => run_demo(),
        cli::Command::AiDemo { turns, seed, board_every } => {
            run_ai_demo(turns, seed, board_every);
        }
    }
}

// ── Session ───────────────────────────────────────────────────────────────────

struct Session {
    state:         GameState,
    civ_id:        CivId,
    city_ids:      Vec<CityId>,
    current_city:  usize,
    selected_unit: Option<UnitId>,
    /// Civilization ID of the AI adversary, if one is present in this session.
    ai_civ_id:     Option<CivId>,
}


fn build_session() -> Session {
    let seed = 42u64;
    const W: u32 = 40;
    const H: u32 = 24;
    let mut state = GameState::new(seed, W, H);

    // Generate terrain via mapgen pipeline; get two habitable starting positions.
    let mapgen_result = mapgen_generate(
        &MapGenConfig { width: W, height: H, seed,
                        land_fraction: None, num_continents: None,
                        num_zone_seeds: None, num_starts: 2 },
        &mut state.board,
    );
    let starts = &mapgen_result.starting_positions;
    let city_coord         = starts.first().copied().unwrap_or(HexCoord::from_qr(10, 8));
    let babylon_city_coord = starts.get(1).copied().unwrap_or(HexCoord::from_qr(30, 16));

    // Civilization: Rome / Caesar
    let civ_id = state.id_gen.next_civ_id();
    let leader = Leader {
        name: "Caesar",
        civ_id,
        agenda: BuiltinAgenda::Default,
    };
    state.civilizations.push(Civilization::new(civ_id, "Rome", "Roman", leader));

    // Capital city.
    let city_id    = state.id_gen.next_city_id();
    let mut city   = City::new(city_id, "Roma".to_string(), civ_id, city_coord);
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations[0].cities.push(city_id);

    // Populate unit type registry. Each def has a stable ID used for production lookups.
    let warrior_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let builder_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let slinger_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let trader_type_id  = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef { id: warrior_type_id, name: "warrior", production_cost: 40,
                      max_movement: 200, combat_strength: Some(20),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: settler_type_id, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: builder_type_id, name: "builder", production_cost: 50,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: slinger_type_id, name: "slinger", production_cost: 35,
                      max_movement: 200, combat_strength: Some(15),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 2, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: trader_type_id, name: "trader", production_cost: 40,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Trader,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
    ]);

    // Starting Warrior (co-located with capital for simplicity).
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id:              unit_id,
        unit_type:       warrior_type_id,
        owner:           civ_id,
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

    // Starting Builder at city coord (3, 3) — for testing improve command.
    let builder_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id:              builder_id,
        unit_type:       builder_type_id,
        owner:           civ_id,
        coord:           city_coord,
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

    // Starting Trader at city coord (3, 3) — for testing trade commands.
    let trader_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id:              trader_id,
        unit_type:       trader_type_id,
        owner:           civ_id,
        coord:           city_coord,
        domain:          UnitDomain::Land,
        category:        UnitCategory::Trader,
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

    // Babylon — AI adversary civilization.
    let babylon_id = state.id_gen.next_civ_id();
    state.civilizations.push(Civilization::new(
        babylon_id, "Babylon", "Babylonian",
        Leader { name: "Hammurabi", civ_id: babylon_id,
                 agenda: BuiltinAgenda::Default },
    ));
    let babylon_city_id = state.id_gen.next_city_id();
    let mut babylon_city = City::new(babylon_city_id, "Babylon".to_string(),
                                     babylon_id, babylon_city_coord);
    babylon_city.is_capital = true;
    state.cities.push(babylon_city);
    state.civilizations.iter_mut()
        .find(|c| c.id == babylon_id).unwrap()
        .cities.push(babylon_city_id);

    // Babylon gets a starting Warrior so the AI has something to move immediately.
    let babylon_warrior_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id:              babylon_warrior_id,
        unit_type:       warrior_type_id,
        owner:           babylon_id,
        coord:           babylon_city_coord,
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

    recalculate_visibility(&mut state, civ_id);
    recalculate_visibility(&mut state, babylon_id);

    Session {
        state, civ_id,
        city_ids: vec![city_id],
        current_city: 0,
        selected_unit: Some(unit_id),
        ai_civ_id: Some(babylon_id),
    }
}

// ── Non-interactive demo ──────────────────────────────────────────────────────

fn run_demo() {
    let rules = DefaultRulesEngine;
    let mut session = build_session();
    let unit_id = session.selected_unit.unwrap();

    let unit_start = session.state.unit(unit_id).unwrap().coord();
    let city_coord = session.state.cities[0].coord;

    banner("Turn 0  |  Rome  --  before any action");
    println!("  Legend:  # = city   @ = unit");
    println!("  Roma {}   Warrior {}   movement {}/{}",
        fmtc(city_coord), fmtc(unit_start),
        session.state.unit(unit_id).unwrap().movement_left(),
        session.state.unit(unit_id).unwrap().max_movement(),
    );
    println!();
    print_board(&session);

    let target = HexCoord::from_qr(8, 3);
    println!("\n  >>> Moving Warrior {} -> {}...", fmtc(unit_start), fmtc(target));

    match rules.move_unit(&session.state, unit_id, target) {
        Ok(diff) => {
            apply_diff(&mut session.state, &diff);
            println!("      Move OK");
        }
        Err(RulesError::InsufficientMovement(diff)) => {
            apply_diff(&mut session.state, &diff);
            println!("      Partial move (insufficient movement)");
        }
        Err(e) => println!("      Move failed: {e}"),
    }

    let diff = rules.advance_turn(&mut session.state);
    reset_movement(&mut session.state);

    println!();
    banner(&format!("Turn {}  |  Rome", session.state.turn));
    print_board(&session);
    print_turn_events(&diff);

    let city = &session.state.cities[0];
    let unit = session.state.unit(unit_id).unwrap();
    println!("\n  Roma pop={}  food {}/{}   Warrior {}  mv {}/{}",
        city.population, city.food_stored, city.food_to_grow,
        fmtc(unit.coord()), unit.movement_left(), unit.max_movement(),
    );
}

// ── AI agent demo ─────────────────────────────────────────────────────────────

/// State bundle for the two-civ AI demo.
struct AiDemo {
    state:          GameState,
    rome_id:        CivId,
    babylon_id:     CivId,
    /// All city IDs that belong to Rome (grows as the AI founds cities).
    rome_cities:    Vec<CityId>,
    /// All city IDs that belong to Babylon.
    babylon_cities: Vec<CityId>,
}

fn build_ai_demo(seed: u64) -> AiDemo {
    // Larger map so exploration is interesting over 50 turns.
    let mut state = GameState::new(seed, 20, 12);

    // Generate terrain via mapgen; get two well-separated starting positions.
    let mapgen_result = mapgen_generate(
        &MapGenConfig { width: 20, height: 12, seed,
                        land_fraction: None, num_continents: None,
                        num_zone_seeds: None, num_starts: 2 },
        &mut state.board,
    );
    let starts = &mapgen_result.starting_positions;
    let rome_city_coord    = starts.first().copied().unwrap_or(HexCoord::from_qr(3, 4));
    let babylon_city_coord = starts.get(1).copied().unwrap_or(HexCoord::from_qr(16, 7));

    // ── Unit-type registry ────────────────────────────────────────────────
    let warrior_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let slinger_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef { id: warrior_type, name: "warrior", production_cost: 40,
                      max_movement: 200, combat_strength: Some(20),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: settler_type, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
        UnitTypeDef { id: slinger_type, name: "slinger", production_cost: 35,
                      max_movement: 200, combat_strength: Some(15),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 2, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None, era: None, promotion_class: None },
    ]);

    // ── Rome (west side) ──────────────────────────────────────────────────
    let rome_id = state.id_gen.next_civ_id();
    state.civilizations.push(Civilization::new(rome_id, "Rome", "Roman",
        Leader { name: "Caesar", civ_id: rome_id,
                 agenda: BuiltinAgenda::Default }));

    let rome_city = state.id_gen.next_city_id();
    let mut rc = City::new(rome_city, "Roma".to_string(), rome_id, rome_city_coord);
    rc.is_capital = true;
    state.cities.push(rc);
    state.civilizations.iter_mut().find(|c| c.id == rome_id).unwrap()
        .cities.push(rome_city);

    // Rome's starting warrior (co-located with capital).
    let rome_warrior = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: rome_warrior, unit_type: warrior_type, owner: rome_id,
        coord: rome_city_coord,
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // ── Babylon (east side) ───────────────────────────────────────────────
    let babylon_id = state.id_gen.next_civ_id();
    state.civilizations.push(Civilization::new(babylon_id, "Babylon", "Babylonian",
        Leader { name: "Hammurabi", civ_id: babylon_id,
                 agenda: BuiltinAgenda::Default }));

    let babylon_city = state.id_gen.next_city_id();
    let mut bc = City::new(babylon_city, "Babylon".to_string(), babylon_id, babylon_city_coord);
    bc.is_capital = true;
    state.cities.push(bc);
    state.civilizations.iter_mut().find(|c| c.id == babylon_id).unwrap()
        .cities.push(babylon_city);

    // Babylon's starting warrior (co-located with capital).
    let babylon_warrior = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: babylon_warrior, unit_type: warrior_type, owner: babylon_id,
        coord: babylon_city_coord,
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        experience: 0,
        health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    // ── Victory conditions ────────────────────────────────────────────────
    // Default: score victory at turn 500.
    let score_vc_id = state.id_gen.next_victory_id();
    state.victory_conditions.push(BuiltinVictoryCondition::Score { id: score_vc_id, turn_limit: 500 });

    // ── Initial visibility ────────────────────────────────────────────────
    recalculate_visibility(&mut state, rome_id);
    recalculate_visibility(&mut state, babylon_id);

    AiDemo {
        state,
        rome_id, babylon_id,
        rome_cities: vec![rome_city],
        babylon_cities: vec![babylon_city],
    }
}

/// Track any city IDs that appeared for `civ_id` since the last turn
/// (founded by AI agents) and push them into `new_city_ids`.
fn track_new_cities(demo: &mut AiDemo, civ_id: CivId, new_city_ids: &mut Vec<CityId>) {
    for city in demo.state.cities.iter() {
        if city.owner == civ_id {
            let tracker = if civ_id == demo.rome_id {
                &mut demo.rome_cities
            } else {
                &mut demo.babylon_cities
            };
            if !tracker.contains(&city.id) {
                tracker.push(city.id);
                new_city_ids.push(city.id);
            }
        }
    }
}

/// Omniscient board view for the AI demo (no fog — both civs shown, labelled).
fn print_ai_board(demo: &AiDemo) {
    let state   = &demo.state;
    const N: usize = 1;
    let mut buf = Visualizer::new(board_grid(state)).render_buffer();

    // Overlay cities: # = Rome  % = Babylon
    for city in &state.cities {
        let ch = if city.owner == demo.rome_id { '#' } else { '%' };
        overlay(&mut buf, city.coord, ch, N);
    }

    // Overlay units: @ = Rome  & = Babylon
    for unit in &state.units {
        let ch = if unit.owner == demo.rome_id { '@' } else { '&' };
        overlay(&mut buf, unit.coord(), ch, N);
    }

    for line in &buf {
        println!("  {line}");
    }
    println!("  Legend: #=Rome city  @=Rome unit  %=Babylon city  &=Babylon unit");
    println!("  Terrain: G)rassland  P)lains  D)esert  M)ountain  O)cean  T)undra");
}

/// One-line summary for a civilization.
fn civ_summary_line(demo: &AiDemo, civ_id: CivId) -> String {
    let state = &demo.state;
    let civ   = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
    let cities = state.cities.iter().filter(|c| c.owner == civ_id).collect::<Vec<_>>();
    let units  = state.units.iter().filter(|u| u.owner == civ_id).count();
    let explored = civ.explored_tiles.len();

    // First city production status.
    let prod_str = cities.first()
        .and_then(|c| c.production_queue.front())
        .and_then(|item| match item {
            ProductionItem::Unit(tid) =>
                state.unit_type_defs.iter().find(|d| d.id == *tid)
                    .map(|d| format!("{}[{}/{}]",
                        capitalize(d.name),
                        cities[0].production_stored,
                        d.production_cost)),
            _ => None,
        })
        .unwrap_or_else(|| "idle".to_string());

    let total_pop: u32 = cities.iter().map(|c| c.population).sum();
    format!("  {:8}  cities={:2}  pop={:2}  units={:2}  explored={:3}  prod={}",
        civ.name, cities.len(), total_pop, units, explored, prod_str)
}

fn run_ai_demo(turns: u32, seed: u64, board_every: u32) {
    let rules  = DefaultRulesEngine;
    let engine = TurnEngine::new();

    let mut demo = build_ai_demo(seed);

    let rome_agent    = HeuristicAgent::new(demo.rome_id);
    let babylon_agent = HeuristicAgent::new(demo.babylon_id);

    println!();
    banner(&format!("AI Demo — {turns} turns  seed={seed}  20×12 map"));
    println!("  Rome starts at (3,4)   Babylon starts at (16,7)");
    println!("  Both civs are controlled by HeuristicAgent (deterministic).");
    println!();
    println!("  Turn 0 — initial state");
    println!("{}", civ_summary_line(&demo, demo.rome_id));
    println!("{}", civ_summary_line(&demo, demo.babylon_id));
    println!();
    print_ai_board(&demo);

    for t in 1..=turns {
        // ── End-of-turn rules processing ─────────────────────────────────
        engine.process_turn(&mut demo.state, &rules);

        // ── Reset movement for all units ──────────────────────────────────
        for unit in &mut demo.state.units {
            unit.movement_left = unit.max_movement;
        }

        // ── Refresh visibility ────────────────────────────────────────────
        let civ_ids = [demo.rome_id, demo.babylon_id];
        for cid in civ_ids {
            recalculate_visibility(&mut demo.state, cid);
        }

        // ── Agent decisions ───────────────────────────────────────────────
        let rome_diff    = rome_agent.take_turn(&mut demo.state, &rules);
        let babylon_diff = babylon_agent.take_turn(&mut demo.state, &rules);

        // ── Track newly founded AI cities ─────────────────────────────────
        // Unit production is handled by advance_turn inside process_turn.
        let mut rome_new_cities:    Vec<CityId> = Vec::new();
        let mut babylon_new_cities: Vec<CityId> = Vec::new();

        let (rid, bid) = (demo.rome_id, demo.babylon_id);
        track_new_cities(&mut demo, rid, &mut rome_new_cities);
        track_new_cities(&mut demo, bid, &mut babylon_new_cities);

        // Refresh visibility again for newly spawned units.
        for cid in civ_ids {
            recalculate_visibility(&mut demo.state, cid);
        }

        // ── Turn log ──────────────────────────────────────────────────────
        // Collect notable events for this turn.
        let mut notes: Vec<String> = Vec::new();

        for delta in rome_diff.deltas.iter().chain(babylon_diff.deltas.iter()) {
            match delta {
                StateDelta::UnitMoved { unit, from, to, .. } => {
                    // Only report if it was a first move (from == start of turn).
                    let _ = (unit, from, to);
                }
                StateDelta::ProductionStarted { city, item } => {
                    let city_name = demo.state.cities.iter().find(|c| c.id == *city)
                        .map(|c| c.name.as_str()).unwrap_or("?");
                    notes.push(format!("{} started producing {}", city_name, capitalize(item)));
                }
                _ => {}
            }
        }
        for cid in &rome_new_cities {
            let name = demo.state.cities.iter().find(|c| c.id == *cid)
                .map(|c| c.name.as_str()).unwrap_or("?");
            notes.push(format!("Rome founded city: {name}"));
        }
        for cid in &babylon_new_cities {
            let name = demo.state.cities.iter().find(|c| c.id == *cid)
                .map(|c| c.name.as_str()).unwrap_or("?");
            notes.push(format!("Babylon founded city: {name}"));
        }

        // Always print: turn header + civ summaries.
        print!("\n  Turn {t:3}  ");
        if notes.is_empty() {
            println!("(scouting / producing)");
        } else {
            println!();
            for note in &notes {
                println!("    [event] {note}");
            }
        }
        println!("{}", civ_summary_line(&demo, demo.rome_id));
        println!("{}", civ_summary_line(&demo, demo.babylon_id));

        // Print board periodically.
        let should_print_board = board_every > 0 && t % board_every == 0;
        let is_last = t == turns;
        if should_print_board || is_last {
            println!();
            println!("  --- Board at turn {t} ---");
            print_ai_board(&demo);
        }
    }

    // ── Final summary ─────────────────────────────────────────────────────
    println!();
    banner(&format!("Final state after {turns} turns"));
    println!("{}", civ_summary_line(&demo, demo.rome_id));
    println!("{}", civ_summary_line(&demo, demo.babylon_id));
    println!();

    // Score leaderboard.
    {
        let scores = all_scores(&demo.state);
        println!("  Score leaderboard:");
        for (rank, (civ_id, score)) in scores.iter().enumerate() {
            let name = demo.state.civilizations.iter()
                .find(|c| c.id == *civ_id)
                .map(|c| c.name)
                .unwrap_or("?");
            println!("    {}. {:20}  {}", rank + 1, name, score);
        }
        if let Some(ref go) = demo.state.game_over {
            let winner_name = demo.state.civilizations.iter()
                .find(|c| c.id == go.winner)
                .map(|c| c.name)
                .unwrap_or("?");
            println!();
            println!("  Winner: {} ({})", winner_name, go.condition);
        }
        println!();
    }

    // Explored percentage.
    let total_tiles = (demo.state.board.width * demo.state.board.height) as usize;
    let rome_exp    = demo.state.civilizations.iter().find(|c| c.id == demo.rome_id)
        .map(|c| c.explored_tiles.len()).unwrap_or(0);
    let babylon_exp = demo.state.civilizations.iter().find(|c| c.id == demo.babylon_id)
        .map(|c| c.explored_tiles.len()).unwrap_or(0);
    println!("  Map coverage:  Rome {}/{} ({:.0}%)   Babylon {}/{} ({:.0}%)",
        rome_exp, total_tiles, 100.0 * rome_exp as f64 / total_tiles as f64,
        babylon_exp, total_tiles, 100.0 * babylon_exp as f64 / total_tiles as f64,
    );
    println!();
    print_ai_board(&demo);
}

// ── Interactive play loop ─────────────────────────────────────────────────────

fn run_play() {
    let rules   = DefaultRulesEngine;
    let mut session = build_session();

    // Create the AI adversary agent if one is present.
    let ai_agent = session.ai_civ_id.map(HeuristicAgent::new);

    print_help();

    loop {
        // ── Turn header ───────────────────────────────────────────────────────
        println!();
        print_turn_header(&session, &rules);
        println!();
        print_board(&session);
        println!();

        // ── Command loop for this turn ────────────────────────────────────────
        loop {
            print!("  > ");
            io::stdout().flush().ok();

            let line = match read_line() {
                Some(l) => l,
                None    => return,
            };

            match parse_cmd(&line) {
                Cmd::Move(q, r) => {
                    let target = HexCoord::from_qr(q, r);
                    match session.selected_unit {
                        None => println!("  [error] No unit selected. Use 'select <q> <r>'."),
                        Some(uid) => {
                            let before = session.state.unit(uid).unwrap().coord();
                            match rules.move_unit(&session.state, uid, target) {
                                Ok(diff) => {
                                    apply_diff(&mut session.state, &diff);
                                    recalculate_visibility(&mut session.state, session.civ_id);
                                    let after = session.state.unit(uid).unwrap().coord();
                                    println!("  Unit moved {} -> {}   mv={}/{}",
                                        fmtc(before), fmtc(after),
                                        session.state.unit(uid).unwrap().movement_left(),
                                        session.state.unit(uid).unwrap().max_movement(),
                                    );
                                    println!();
                                    print_board(&session);
                                }
                                Err(RulesError::InsufficientMovement(diff)) if !diff.is_empty() => {
                                    apply_diff(&mut session.state, &diff);
                                    recalculate_visibility(&mut session.state, session.civ_id);
                                    let after = session.state.unit(uid).unwrap().coord();
                                    println!("  [partial] Unit moved {} -> {}  (ran out of movement)",
                                        fmtc(before), fmtc(after));
                                    println!("  mv={}/{}",
                                        session.state.unit(uid).unwrap().movement_left(),
                                        session.state.unit(uid).unwrap().max_movement(),
                                    );
                                    println!();
                                    print_board(&session);
                                }
                                Err(RulesError::InsufficientMovement(_)) => {
                                    println!("  [error] Not enough movement points.");
                                }
                                Err(RulesError::DestinationImpassable) => {
                                    println!("  [error] {} is impassable or unreachable.", fmtc(target));
                                }
                                Err(RulesError::InvalidCoord) => {
                                    println!("  [error] ({q},{r}) is outside the map.");
                                }
                                Err(e) => println!("  [error] {e}"),
                            }
                        }
                    }
                }

                Cmd::Board => {
                    println!();
                    print_board(&session);
                }

                Cmd::Status => {
                    let city = session.state.cities.iter().find(|c| c.id == session.city_ids[session.current_city]).unwrap();
                    println!("  Turn {}  |  Roma pop={}  food {}/{}",
                        session.state.turn, city.population, city.food_stored, city.food_to_grow);
                    if let Some(uid) = session.selected_unit {
                        if let Some(unit) = session.state.unit(uid) {
                            println!("  {} {}  mv={}/{}  hp={}/{}",
                                unit_type_name(&session, unit.unit_type()),
                                fmtc(unit.coord()),
                                unit.movement_left(), unit.max_movement(),
                                unit.health(), unit.max_health(),
                            );
                        }
                    } else {
                        println!("  (no unit selected)");
                    }
                }

                Cmd::Tile(q, r) => cmd_tile(&session.state, q, r),

                Cmd::City => cmd_city(&session, &rules),

                Cmd::Cities => cmd_cities(&session, &rules),

                Cmd::Assign(q, r) => cmd_assign(&mut session, &rules, q, r, false),

                Cmd::Lock(q, r) => cmd_assign(&mut session, &rules, q, r, true),

                Cmd::Unassign(q, r) => cmd_unassign(&mut session, q, r),

                Cmd::Build(name) => cmd_build(&mut session, &name),

                Cmd::Cancel => cmd_cancel(&mut session),

                Cmd::Queue => cmd_queue(&session),

                Cmd::Yields => cmd_yields(&session, &rules),

                Cmd::Units => cmd_units(&session),

                Cmd::Select(q, r) => cmd_select(&mut session, q, r),

                Cmd::Settle(name) => {
                    let name = if name.trim().is_empty() { "New City".to_string() } else { name };
                    match session.selected_unit {
                        None => println!("  [error] No unit selected."),
                        Some(uid) => {
                            match rules.found_city(&mut session.state, uid, name) {
                                Ok(diff) => {
                                    for delta in &diff.deltas {
                                        if let StateDelta::CityFounded { city, coord, .. } = delta {
                                            session.city_ids.push(*city);
                                            println!("  City founded at {}!", fmtc(*coord));
                                        }
                                    }
                                    session.selected_unit = None;
                                    recalculate_visibility(&mut session.state, session.civ_id);
                                    print_board(&session);
                                }
                                Err(e) => println!("  [error] {e}"),
                            }
                        }
                    }
                }

                Cmd::Attack(q, r) => {
                    let target_coord = HexCoord::from_qr(q, r);
                    match session.selected_unit {
                        None => println!("  [error] No unit selected."),
                        Some(uid) => {
                            let defender = session.state.units.iter()
                                .find(|u| u.coord() == target_coord && u.owner != session.civ_id)
                                .map(|u| u.id);
                            match defender {
                                None => println!("  [error] No enemy unit at {}.", fmtc(target_coord)),
                                Some(def_id) => {
                                    match rules.attack(&mut session.state, uid, def_id) {
                                        Ok(diff) => {
                                            for delta in &diff.deltas {
                                                match delta {
                                                    StateDelta::UnitAttacked {
                                                        attacker_damage, defender_damage, attack_type, ..
                                                    } => {
                                                        let type_str = match attack_type {
                                                            AttackType::Melee       => "Melee",
                                                            AttackType::Ranged      => "Ranged",
                                                            AttackType::CityBombard => "CityBombard",
                                                        };
                                                        println!("  [{type_str}] dealt {defender_damage} dmg, took {attacker_damage} dmg");
                                                    }
                                                    StateDelta::UnitDestroyed { .. } => {
                                                        println!("  Unit destroyed!");
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            // Deselect if attacker was destroyed.
                                            if session.state.unit(uid).is_none() {
                                                session.selected_unit = None;
                                            }
                                            print_board(&session);
                                        }
                                        Err(e) => println!("  [error] {e}"),
                                    }
                                }
                            }
                        }
                    }
                }

                Cmd::Techs => {
                    let civ = session.state.civilizations.iter()
                        .find(|c| c.id == session.civ_id).unwrap();
                    println!("  Researched technologies ({}):", civ.researched_techs.len());
                    for tid in &civ.researched_techs {
                        if let Some(node) = session.state.tech_tree.get(*tid) {
                            println!("    - {}", node.name);
                        }
                    }
                    if let Some(prog) = civ.research_queue.front()
                        && let Some(node) = session.state.tech_tree.get(prog.tech_id)
                    {
                        println!("  In progress: {} ({}/{})",
                            node.name, prog.progress, node.cost);
                    }
                }

                Cmd::Civics => {
                    let civ = session.state.civilizations.iter()
                        .find(|c| c.id == session.civ_id).unwrap();
                    println!("  Completed civics ({}):", civ.completed_civics.len());
                    for cid in &civ.completed_civics {
                        if let Some(node) = session.state.civic_tree.get(*cid) {
                            println!("    - {}", node.name);
                        }
                    }
                }

                Cmd::Improve(name) => cmd_improve(&mut session, &rules, &name),

                Cmd::Research(name) => cmd_research(&mut session, &name),

                Cmd::Switch(n) => {
                    if n == 0 || n > session.city_ids.len() {
                        println!("  [error] City index out of range (1-{}).", session.city_ids.len());
                    } else {
                        session.current_city = n - 1;
                        let city_id = session.city_ids[session.current_city];
                        let name = session.state.cities.iter()
                            .find(|c| c.id == city_id)
                            .map(|c| c.name.as_str())
                            .unwrap_or("?");
                        println!("  Switched to city: {name}");
                    }
                }

                Cmd::Trade(q, r) => {
                    let dest_coord = HexCoord::from_qr(q, r);
                    match session.selected_unit {
                        None => println!("  [error] No unit selected. Select a trader with 'select <q> <r>'."),
                        Some(uid) => {
                            let dest = session.state.cities.iter()
                                .find(|c| c.coord == dest_coord)
                                .map(|c| c.id);
                            match dest {
                                None => println!("  [error] No city at ({q},{r})."),
                                Some(dest_id) => {
                                    match rules.establish_trade_route(&mut session.state, uid, dest_id) {
                                        Ok(diff) => {
                                            for delta in &diff.deltas {
                                                if let StateDelta::TradeRouteEstablished {
                                                    origin, destination, ..
                                                } = delta {
                                                    let orig = session.state.cities.iter()
                                                        .find(|c| c.id == *origin)
                                                        .map(|c| c.name.as_str()).unwrap_or("?");
                                                    let dest_name = session.state.cities.iter()
                                                        .find(|c| c.id == *destination)
                                                        .map(|c| c.name.as_str()).unwrap_or("?");
                                                    println!("  Trade route established: {} -> {}!", orig, dest_name);
                                                }
                                            }
                                            if session.state.unit(uid).is_none() {
                                                session.selected_unit = None;
                                            }
                                        }
                                        Err(e) => println!("  [error] {e}"),
                                    }
                                }
                            }
                        }
                    }
                }

                Cmd::Routes => cmd_routes(&session),

                Cmd::Scores => cmd_scores(&session),

                Cmd::Congress => cmd_congress(&session),

                Cmd::District(name, q, r) => cmd_district(&mut session, &rules, &name, q, r),

                Cmd::Road(q, r) => cmd_road(&mut session, &rules, q, r),

                Cmd::War(name) => cmd_war(&mut session, &rules, &name),

                Cmd::Peace(name) => cmd_peace(&mut session, &rules, &name),

                Cmd::Government(name) => cmd_government(&mut session, &name),

                Cmd::Pantheon(name) => cmd_pantheon(&mut session, &rules, &name),

                Cmd::Religion(name, b1, b2) => cmd_religion(&mut session, &rules, &name, &b1, &b2),

                Cmd::GreatPeople => cmd_great_people(&session),

                Cmd::Recruit(name) => cmd_recruit(&mut session, &rules, &name),

                Cmd::GovernorAssign(name, idx) => cmd_governor(&mut session, &rules, &name, idx),

                Cmd::Diplomacy => cmd_diplomacy(&session),

                Cmd::Policy(name) => cmd_policy(&mut session, &rules, &name),
                Cmd::Spread => cmd_spread(&mut session, &rules),
                Cmd::TheoCombat(q, r) => cmd_theo_combat(&mut session, &rules, q, r),
                Cmd::Bombard(q, r) => cmd_bombard(&mut session, &rules, q, r),
                Cmd::Retire(name) => cmd_retire(&mut session, &rules, &name),
                Cmd::GreatWork(name) => cmd_great_work(&mut session, &rules, &name),
                Cmd::RecruitFaith(name) => cmd_recruit_faith(&mut session, &rules, &name),
                Cmd::Evangelize(name) => cmd_evangelize(&mut session, &rules, &name),
                Cmd::Inquisition => cmd_inquisition(&mut session, &rules),
                Cmd::RemoveHeresy => cmd_remove_heresy(&mut session, &rules),
                Cmd::GuruHeal => cmd_guru_heal(&mut session, &rules),
                Cmd::ClaimTile(q, r) => cmd_claim_tile(&mut session, &rules, q, r),
                Cmd::ReassignTile(q, r, idx) => cmd_reassign_tile(&mut session, &rules, q, r, idx),
                Cmd::SendTrader(q, r) => cmd_send_trader(&mut session, &rules, q, r),
                Cmd::BarbHire(q, r) => cmd_barb_hire(&mut session, &rules, q, r),
                Cmd::BarbBribe(q, r) => cmd_barb_bribe(&mut session, &rules, q, r),
                Cmd::BarbIncite(q, r, target) => cmd_barb_incite(&mut session, &rules, q, r, &target),
                Cmd::FaithBuy(name) => cmd_faith_buy(&mut session, &rules, &name),
                Cmd::GovernorPromote(name, promo) => cmd_governor_promote(&mut session, &rules, &name, &promo),

                Cmd::EndTurn => break,

                Cmd::Quit => {
                    println!("  Goodbye.");
                    return;
                }

                Cmd::Perform => {
                    match session.selected_unit {
                        None => println!("  [error] No unit selected."),
                        Some(uid) => {
                            match rules.rock_band_perform(&mut session.state, uid) {
                                Ok(diff) => {
                                    for delta in &diff.deltas {
                                        match delta {
                                            StateDelta::RockBandPerformed { tourism_gained, .. } => {
                                                println!("  Rock Band performed! Tourism gained: {tourism_gained}");
                                            }
                                            StateDelta::UnitDestroyed { .. } => {
                                                println!("  Rock Band disbanded!");
                                                session.selected_unit = None;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(e) => println!("  [error] {e}"),
                            }
                        }
                    }
                }

                Cmd::Help => print_help(),

                Cmd::Unknown(s) => {
                    println!("  [error] Unknown command {s:?} -- type 'help' for a list.");
                }
            }
        }

        // ── End of turn ───────────────────────────────────────────────────────
        // advance_turn handles production completion, strategic resource yield,
        // science, culture, and gold. UnitCreated events appear in print_turn_events.
        let diff = rules.advance_turn(&mut session.state);
        reset_movement(&mut session.state);
        print_turn_events(&diff);

        // ── Game-over check ───────────────────────────────────────────────────
        if let Some(ref go) = session.state.game_over {
            let winner_name = session.state.civilizations.iter()
                .find(|c| c.id == go.winner)
                .map(|c| c.name)
                .unwrap_or("Unknown");
            println!();
            banner(&format!("GAME OVER — {winner_name} wins ({})!", go.condition));
            println!();
            cmd_scores(&session);
            return;
        }

        // ── AI adversary turn ─────────────────────────────────────────────────
        if let Some(ref agent) = ai_agent {
            let ai_diff = agent.take_turn(&mut session.state, &rules);
            print_ai_turn_events(&session.state, agent.civ_id, &ai_diff);
            recalculate_visibility(&mut session.state, agent.civ_id);
        }
        recalculate_visibility(&mut session.state, session.civ_id);
    }
}

// ── Command parsing ───────────────────────────────────────────────────────────

enum Cmd {
    Move(i32, i32),
    Board,
    Status,
    EndTurn,
    Quit,
    Help,
    Tile(i32, i32),
    City,
    Cities,
    Assign(i32, i32),
    Lock(i32, i32),
    Unassign(i32, i32),
    Build(String),
    Cancel,
    Queue,
    Yields,
    Units,
    Select(i32, i32),
    Settle(String),
    Attack(i32, i32),
    Techs,
    Civics,
    Switch(usize),
    Improve(String),
    Research(String),
    Trade(i32, i32),
    Routes,
    Scores,
    District(String, i32, i32),
    Road(i32, i32),
    War(String),
    Peace(String),
    Government(String),
    Pantheon(String),
    Religion(String, String, String),
    GreatPeople,
    Recruit(String),
    GovernorAssign(String, usize),
    GovernorPromote(String, String),
    Diplomacy,
    Policy(String),
    Spread,
    TheoCombat(i32, i32),
    Bombard(i32, i32),
    Retire(String),
    GreatWork(String),
    RecruitFaith(String),
    Evangelize(String),
    Inquisition,
    RemoveHeresy,
    GuruHeal,
    ClaimTile(i32, i32),
    ReassignTile(i32, i32, usize),
    SendTrader(i32, i32),
    BarbHire(i32, i32),
    BarbBribe(i32, i32),
    BarbIncite(i32, i32, String),
    FaithBuy(String),
    Congress,
    Perform,
    Unknown(String),
}

fn parse_coord(q: &str, r: &str) -> Option<(i32, i32)> {
    q.parse::<i32>().ok().zip(r.parse::<i32>().ok())
}

fn parse_cmd(raw: &str) -> Cmd {
    let s = raw.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();
    match parts.as_slice() {
        ["move" | "m", q, r] | ["go", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Move(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["board"] => Cmd::Board,
        ["status" | "s"] => Cmd::Status,
        ["end" | "e" | "next" | "n"] => Cmd::EndTurn,
        ["quit" | "exit"] => Cmd::Quit,
        ["help" | "h" | "?"] => Cmd::Help,
        ["tile" | "t", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Tile(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["city" | "c"] => Cmd::City,
        ["cities" | "cs"] => Cmd::Cities,
        ["assign" | "a", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Assign(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["lock", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Lock(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["unassign" | "ua", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Unassign(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["build" | "b", rest @ ..] if !rest.is_empty() => Cmd::Build(rest.join(" ")),
        ["cancel"] => Cmd::Cancel,
        ["queue" | "q"] => Cmd::Queue,
        ["yields" | "y"] => Cmd::Yields,
        ["units" | "u"] => Cmd::Units,
        ["select" | "sel", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Select(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["settle" | "found", rest @ ..] => Cmd::Settle(rest.join(" ")),
        ["attack" | "atk", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Attack(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["techs"] => Cmd::Techs,
        ["civics"] => Cmd::Civics,
        ["switch", n] => n.parse::<usize>().ok()
            .map(Cmd::Switch)
            .unwrap_or(Cmd::Unknown(s.to_string())),
        ["improve" | "imp", rest @ ..] if !rest.is_empty() => Cmd::Improve(rest.join(" ")),
        ["research" | "res", rest @ ..] if !rest.is_empty() => Cmd::Research(rest.join(" ")),
        ["trade" | "tr", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Trade(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["routes" | "rt"] => Cmd::Routes,
        ["scores" | "sc"] => Cmd::Scores,
        ["congress" | "wc"] => Cmd::Congress,
        ["perform"] => Cmd::Perform,
        ["district" | "dist", name, q, r] => {
            parse_coord(q, r)
                .map(|(q, r)| Cmd::District(name.to_string(), q, r))
                .unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["road", q, r] => {
            parse_coord(q, r)
                .map(|(q, r)| Cmd::Road(q, r))
                .unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["war", rest @ ..] if !rest.is_empty() => Cmd::War(rest.join(" ")),
        ["peace", rest @ ..] if !rest.is_empty() => Cmd::Peace(rest.join(" ")),
        ["government" | "gov", rest @ ..] if !rest.is_empty() => Cmd::Government(rest.join(" ")),
        ["pantheon", rest @ ..] if !rest.is_empty() => Cmd::Pantheon(rest.join(" ")),
        ["religion" | "rel", name, b1, b2] => Cmd::Religion(name.to_string(), b1.to_string(), b2.to_string()),
        ["great-people" | "gp"] => Cmd::GreatPeople,
        ["recruit", rest @ ..] if !rest.is_empty() => Cmd::Recruit(rest.join(" ")),
        ["governor", name, idx] => {
            idx.parse::<usize>().ok()
                .map(|i| Cmd::GovernorAssign(name.to_string(), i))
                .unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["diplomacy" | "diplo"] => Cmd::Diplomacy,
        ["policy", rest @ ..] if !rest.is_empty() => Cmd::Policy(rest.join(" ")),
        ["spread"] => Cmd::Spread,
        ["theo-combat" | "tc", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::TheoCombat(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["bombard", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::Bombard(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["retire", rest @ ..] if !rest.is_empty() => Cmd::Retire(rest.join(" ")),
        ["great-work" | "gw", rest @ ..] if !rest.is_empty() => Cmd::GreatWork(rest.join(" ")),
        ["recruit-faith" | "rf", rest @ ..] if !rest.is_empty() => Cmd::RecruitFaith(rest.join(" ")),
        ["evangelize", rest @ ..] if !rest.is_empty() => Cmd::Evangelize(rest.join(" ")),
        ["inquisition"] => Cmd::Inquisition,
        ["remove-heresy" | "rh"] => Cmd::RemoveHeresy,
        ["guru-heal" | "gh"] => Cmd::GuruHeal,
        ["claim", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::ClaimTile(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["reassign", q, r, city_idx] => {
            if let (Some((q, r)), Some(idx)) = (parse_coord(q, r), city_idx.parse::<usize>().ok()) {
                Cmd::ReassignTile(q, r, idx)
            } else { Cmd::Unknown(s.to_string()) }
        }
        ["send-trader" | "st", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::SendTrader(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["barb-hire" | "bh", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::BarbHire(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["barb-bribe" | "bb", q, r] => {
            parse_coord(q, r).map(|(q, r)| Cmd::BarbBribe(q, r)).unwrap_or(Cmd::Unknown(s.to_string()))
        }
        ["barb-incite" | "bi", q, r, rest @ ..] if !rest.is_empty() => {
            if let Some((q, r)) = parse_coord(q, r) {
                Cmd::BarbIncite(q, r, rest.join(" "))
            } else { Cmd::Unknown(s.to_string()) }
        }
        ["faith-buy" | "fb", rest @ ..] if !rest.is_empty() => Cmd::FaithBuy(rest.join(" ")),
        ["governor-promote" | "gp-promote", name, promo] => {
            Cmd::GovernorPromote(name.to_string(), promo.to_string())
        }
        _ => Cmd::Unknown(s.to_string()),
    }
}

fn print_help() {
    println!();
    println!("  Commands:");
    println!("    move <q> <r>       -- move selected unit to (q,r)     aliases: m, go");
    println!("    select <q> <r>     -- select unit at (q,r)            alias:  sel");
    println!("    units              -- list all units                   alias:  u");
    println!("    board              -- redraw the board");
    println!("    tile <q> <r>       -- inspect tile at (q,r)           alias:  t");
    println!("    city               -- show current city detail         alias:  c");
    println!("    cities             -- list all cities                  alias:  cs");
    println!("    assign <q> <r>     -- work tile (q,r)                 alias:  a");
    println!("    lock <q> <r>       -- work+lock tile (q,r)");
    println!("    unassign <q> <r>   -- stop working tile (q,r)         alias:  ua");
    println!("    build <name>       -- queue a unit for production      alias:  b");
    println!("    cancel             -- cancel the front production item");
    println!("    queue              -- show production queue            alias:  q");
    println!("    yields             -- show yield breakdown             alias:  y");
    println!("    status             -- show city and unit summary       alias:  s");
    println!("    settle [name]      -- found city with selected settler  alias: found");
    println!("    attack <q> <r>     -- attack unit at (q,r)             alias:  atk");
    println!("    techs              -- list researched technologies");
    println!("    civics             -- list completed civics");
    println!("    research <name>    -- queue a technology for research          alias:  res");
    println!("    improve <name>     -- build improvement with selected builder  alias:  imp");
    println!("    switch <n>         -- switch active city to index n (1-based)");
    println!("    trade <q> <r>      -- establish trade route to city at (q,r)  alias:  tr");
    println!("    routes             -- list active trade routes                 alias:  rt");
    println!("    scores             -- show score leaderboard                   alias:  sc");
    println!("    congress           -- show World Congress state                alias:  wc");
    println!("    district <n> <q> <r>  place a district at (q,r)               alias:  dist");
    println!("    road <q> <r>       -- build ancient road at (q,r) with builder");
    println!("    war <civ>          -- declare war on a civilization");
    println!("    peace <civ>        -- make peace with a civilization");
    println!("    government <name>  -- adopt a government                       alias:  gov");
    println!("    pantheon <belief>  -- found a pantheon with a belief");
    println!("    religion <n> <b1> <b2>  found a religion (needs Great Prophet) alias: rel");
    println!("    great-people       -- list available great people               alias:  gp");
    println!("    recruit <type>     -- recruit a great person by type (gold)");
    println!("    governor <n> <idx> -- assign governor to city index             ");
    println!("    diplomacy          -- show diplomatic status                    alias:  diplo");
    println!("    policy <name>      -- assign a policy to a slot");
    println!("    spread             -- spread religion with selected missionary/apostle");
    println!("    theo-combat <q> <r>-- theological combat at (q,r)              alias:  tc");
    println!("    bombard <q> <r>    -- city bombardment at enemy unit");
    println!("    retire <name>      -- retire a great person by name");
    println!("    great-work <name>  -- create great work from person            alias:  gw");
    println!("    recruit-faith <t>  -- recruit great prophet with faith         alias:  rf");
    println!("    evangelize <belief>-- evangelize belief with apostle");
    println!("    inquisition        -- launch inquisition with apostle");
    println!("    remove-heresy      -- remove heresy with inquisitor            alias:  rh");
    println!("    guru-heal          -- heal nearby religious units               alias:  gh");
    println!("    claim <q> <r>      -- claim tile for current city");
    println!("    reassign <q> <r> <n> reassign tile to city n (1-based)");
    println!("    send-trader <q> <r>-- send trader toward city at (q,r)         alias:  st");
    println!("    barb-hire <q> <r>  -- hire unit from barb camp at (q,r)        alias:  bh");
    println!("    barb-bribe <q> <r> -- bribe barb camp at (q,r)                 alias:  bb");
    println!("    barb-incite <q> <r> <civ> incite camp vs civ                   alias:  bi");
    println!("    faith-buy <name>   -- purchase unit/building with faith         alias:  fb");
    println!("    governor-promote <g> <p> promote governor                       alias:  gp-promote");
    println!("    end                -- end turn                         aliases: e, next, n");
    println!("    quit               -- exit                             alias:  exit");
    println!("    help               -- this message                     aliases: h, ?");
    println!();
    println!("  Legend:  # = city   @ = unit   . = unexplored   (lower) = fog");
    println!("  Terrain: G)rassland  P)lains  D)esert  M)ountain");
    println!("           C)oast  O)cean  T)undra  S)now");
}

// ── New command handlers ──────────────────────────────────────────────────────

/// Build an improvement on the selected builder unit's current tile.
fn cmd_improve(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected. Use 'select <q> <r>'.");
        return;
    };
    let Some(unit) = session.state.unit(uid) else {
        println!("  [error] Selected unit no longer exists.");
        return;
    };
    let type_name = unit_type_name(session, unit.unit_type());
    if type_name != "builder" {
        println!("  [error] Selected unit is a {type_name}, not a builder.");
        return;
    }
    let coord = unit.coord();

    let improvement = match name.to_ascii_lowercase().replace(' ', "_").as_str() {
        "farm"                        => BuiltinImprovement::Farm,
        "mine"                        => BuiltinImprovement::Mine,
        "lumber_mill" | "lumbermill"  => BuiltinImprovement::LumberMill,
        "trading_post" | "tradingpost"=> BuiltinImprovement::TradingPost,
        "fort"                        => BuiltinImprovement::Fort,
        "airstrip"                    => BuiltinImprovement::Airstrip,
        "missile_silo" | "missilesilo"=> BuiltinImprovement::MissileSilo,
        _ => {
            println!("  [error] Unknown improvement '{name}'.");
            println!("  Valid: farm, mine, lumber_mill, trading_post, fort, airstrip, missile_silo");
            return;
        }
    };

    match rules.place_improvement(&mut session.state, session.civ_id, coord, improvement, None) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::ImprovementPlaced { coord, improvement } = delta {
                    println!("  {} built at {}!", improvement.name(), fmtc(*coord));
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

/// Queue a technology for research by name.
fn cmd_research(session: &mut Session, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let found = session.state.tech_tree.nodes.values()
        .find(|n| n.name.to_ascii_lowercase() == name_lower)
        .map(|n| (n.id, n.name, n.cost));
    let Some((tech_id, tech_name, tech_cost)) = found else {
        println!("  [error] Unknown technology '{name}'. Use 'techs' to list available.");
        return;
    };
    let civ = session.state.civilizations.iter().find(|c| c.id == session.civ_id).unwrap();
    if civ.researched_techs.contains(&tech_id) {
        println!("  [info] {tech_name} is already researched.");
        return;
    }
    if civ.research_queue.iter().any(|tp| tp.tech_id == tech_id) {
        println!("  [info] {tech_name} is already queued.");
        return;
    }
    let researched = civ.researched_techs.clone();
    if !session.state.tech_tree.prerequisites_met(tech_id, &researched) {
        println!("  [error] Prerequisites not yet met for {tech_name}.");
        return;
    }
    let civ = session.state.civilizations.iter_mut().find(|c| c.id == session.civ_id).unwrap();
    civ.research_queue.push_back(TechProgress { tech_id, progress: 0, boosted: false });
    println!("  Queued: {tech_name} (cost: {tech_cost} science)");
}

/// Print all properties of the tile at (q, r).
fn cmd_tile(state: &GameState, q: i32, r: i32) {
    let coord = HexCoord::from_qr(q, r);
    let Some(tile) = state.board.tile(coord) else {
        println!("  [error] ({q},{r}) is outside the map.");
        return;
    };

    let yields = tile.total_yields();
    let terrain_name = tile.terrain.name();
    let feature_name = tile.feature
        .map(|f| f.name().to_string())
        .unwrap_or_else(|| "none".to_string());
    let resource_name = tile.resource
        .map(|r| format!("{r:?}"))
        .unwrap_or_else(|| "none".to_string());
    let improvement_name = tile.improvement
        .map(|i| i.name().to_string())
        .unwrap_or_else(|| "none".to_string());
    let road_name = tile.road.as_ref()
        .map(|r| r.as_def().name().to_string())
        .unwrap_or_else(|| "none".to_string());
    let owner_name = tile.owner
        .and_then(|oid| state.civilizations.iter().find(|c| c.id == oid))
        .map(|civ| civ.name.to_string())
        .unwrap_or_else(|| "none".to_string());

    // Worked-by lookup.
    let worked_by = state.cities.iter().find(|c| c.worked_tiles.contains(&coord));
    let worked_str = match worked_by {
        None => "none".to_string(),
        Some(city) => {
            if city.locked_tiles.contains(&coord) {
                format!("{} [locked]", city.name)
            } else {
                city.name.clone()
            }
        }
    };

    // Units on this tile.
    let units_here: Vec<String> = state.units.iter()
        .filter(|u| u.coord() == coord)
        .map(|u| {
            let owner = state.civilizations.iter()
                .find(|c| c.id == u.owner())
                .map(|c| c.name)
                .unwrap_or("?");
            format!("{:?} ({owner})  hp={}/{},  mv={}/{}",
                u.domain(),
                u.health(), u.max_health(),
                u.movement_left(), u.max_movement(),
            )
        })
        .collect();

    let hdr = format!("Tile ({q},{r})");
    hline(&hdr);
    println!("  Terrain    : {terrain_name}");
    println!("  Feature    : {feature_name}");
    println!("  Resource   : {resource_name}");
    println!("  Improvement: {improvement_name}");
    println!("  Road       : {road_name}");
    println!("  Yields     : food={}  prod={}  gold={}  sci={}  cul={}  faith={}",
        yields.food, yields.production, yields.gold,
        yields.science, yields.culture, yields.faith);
    println!("  Owner      : {owner_name}");
    println!("  Worked by  : {worked_str}");
    if units_here.is_empty() {
        println!("  Units      : none");
    } else {
        for (i, u) in units_here.iter().enumerate() {
            if i == 0 {
                println!("  Units      : {u}");
            } else {
                println!("               {u}");
            }
        }
    }
    hline_end();
}

/// Show the detail panel for the current city.
fn cmd_city(session: &Session, rules: &DefaultRulesEngine) {
    let state   = &session.state;
    let city_id = session.city_ids[session.current_city];
    let civ_id  = session.civ_id;

    let Some(city) = state.cities.iter().find(|c| c.id == city_id) else {
        println!("  [error] City not found.");
        return;
    };

    let yields = rules.compute_yields(state, civ_id);

    // Production queue front
    let (prod_name, prod_cost) = queue_front_info(session);
    let prod_bar = match prod_cost {
        Some(cost) => format!("{}  {} / {}",
            progress_bar(city.production_stored, cost), city.production_stored, cost),
        None => format!("{}  0 / --", progress_bar(0, 0)),
    };

    let capital_tag = if city.is_capital { " (capital)" } else { "" };
    let hdr = format!("{}{} pop={}", city.name, capital_tag, city.population);
    hline(&hdr);

    // Food and production bars
    let food_bar = format!("{}  {} / {}",
        progress_bar(city.food_stored, city.food_to_grow), city.food_stored, city.food_to_grow);
    println!("  Food      : {food_bar}");
    let prod_label = prod_name.map(|n| format!("  [{}]", capitalize(n))).unwrap_or_default();
    println!("  Production: {prod_bar}{prod_label}");
    println!("  Gold/turn : {:+}    Science/turn: {:+}    Culture/turn: {:+}",
        yields.gold, yields.science, yields.culture);

    // Worked tiles
    let pop = city.population as usize;
    let worked_count = city.worked_tiles.len();
    println!();
    println!("  Worked tiles ({worked_count} / {pop} citizens):");
    for coord in &city.worked_tiles {
        let tile_str = tile_yield_row(state, *coord);
        let center_tag  = if *coord == city.coord { " [center]" } else { "" };
        let lock_tag    = if city.locked_tiles.contains(coord) { " [locked]" } else { "" };
        println!("    {tile_str}{center_tag}{lock_tag}");
    }

    // Available tiles (not yet worked, sorted by total yield desc)
    let mut available: Vec<HexCoord> = (1u32..=3)
        .flat_map(|r| city.coord.ring(r))
        .filter(|coord| {
            state.board.tile(*coord).is_some() && !city.worked_tiles.contains(coord)
        })
        .collect();
    available.sort_by_key(|coord| {
        state.board.tile(*coord).map(|t| {
            let y = t.total_yields();
            -(y.food + y.production + y.gold + y.science + y.culture)
        }).unwrap_or(0)
    });
    if !available.is_empty() {
        println!();
        println!("  Available tiles (by yield):");
        for coord in available.iter().take(6) {
            println!("    {}", tile_yield_row(state, *coord));
        }
        if available.len() > 6 {
            println!("    ... ({} more)", available.len() - 6);
        }
    }

    // Production queue
    println!();
    println!("  Production queue:");
    if city.production_queue.is_empty() {
        println!("    (empty)");
    } else {
        for (i, item) in city.production_queue.iter().enumerate() {
            let (name, cost_str) = queue_item_display(session, item);
            if i == 0 {
                println!("    {}. {}  {}  stored: {}  [in progress]",
                    i + 1, capitalize(&name), cost_str, city.production_stored);
            } else {
                println!("    {}. {}  {}", i + 1, capitalize(&name), cost_str);
            }
        }
    }

    // Buildings
    println!();
    if city.buildings.is_empty() {
        println!("  Buildings: none");
    } else {
        println!("  Buildings: {} building(s)", city.buildings.len());
    }

    hline_end();
}

/// Table summary of all cities.
fn cmd_cities(session: &Session, rules: &DefaultRulesEngine) {
    let state  = &session.state;
    let civ_id = session.civ_id;
    let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
    let yields = rules.compute_yields(state, civ_id);
    println!("  Cities ({}):", civ.name);
    println!("    {:2}  {:12}  {:4}  {:7}  {:7}  {:7}  {:5}",
        "#", "Name", "Pop", "Food", "Prod", "Gold/t", "Sci/t");
    for (i, city) in state.cities.iter()
        .filter(|c| c.owner == civ_id)
        .enumerate()
    {
        let (_, prod_cost) = queue_front_info(session);
        let prod_str = match prod_cost {
            Some(cost) => format!("{}/{}", city.production_stored, cost),
            None       => format!("{}/--", city.production_stored),
        };
        println!("    {:2}  {:12}  {:4}  {:7}  {:7}  {:+6}  {:+5}",
            i + 1, city.name, city.population,
            format!("{}/{}", city.food_stored, city.food_to_grow),
            prod_str,
            yields.gold, yields.science,
        );
    }
}

/// Assign (or lock) a citizen to work a tile. Routes through the rules engine.
fn cmd_assign(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32, lock: bool) {
    let coord   = HexCoord::from_qr(q, r);
    let city_id = session.city_ids[session.current_city];
    match rules.assign_citizen(&mut session.state, city_id, coord, lock) {
        Ok(_) => {
            let label = tile_yield_row(&session.state, coord);
            let lock_note = if lock { " (locked)" } else { "" };
            println!("  Assigned citizen to {label}{lock_note}");
            let city = session.state.cities.iter().find(|c| c.id == city_id).unwrap();
            println!("  {} worked tiles: {}", city.name, city.worked_tiles.len());
        }
        Err(e) => println!("  [error] {e}"),
    }
}

/// Remove a citizen from a tile (direct state mutation, no rules check).
fn cmd_unassign(session: &mut Session, q: i32, r: i32) {
    let coord   = HexCoord::from_qr(q, r);
    let city_id = session.city_ids[session.current_city];
    let Some(city) = session.state.cities.iter_mut().find(|c| c.id == city_id) else {
        println!("  [error] City not found.");
        return;
    };
    city.worked_tiles.retain(|c| *c != coord);
    city.locked_tiles.remove(&coord);
    println!("  Unassigned ({},{}). {} now working {} tile(s).",
        q, r, city.name, city.worked_tiles.len());
}

/// Queue a unit or building for production by name (case-insensitive).
fn cmd_build(session: &mut Session, name: &str) {
    let key = name.trim().to_lowercase();
    let civ_id = session.civ_id;

    // Compute available units and buildings for this civ (tech-gated + replacement).
    let avail_units = libciv::game::available_unit_defs(&session.state, civ_id);
    let avail_buildings = libciv::game::available_building_defs(&session.state, civ_id);

    // Try unit types first (case-insensitive match against available units).
    if let Some(def) = avail_units.iter().find(|d| d.name.to_lowercase() == key) {
        let type_id  = def.id;
        let def_name = def.name;
        let def_cost = def.production_cost;
        let city_id  = session.city_ids[session.current_city];
        let city = session.state.cities.iter_mut().find(|c| c.id == city_id).unwrap();
        city.production_queue.push_back(ProductionItem::Unit(type_id));
        println!("  Queued: {} ({} prod) in {}", capitalize(def_name), def_cost, city.name);
        return;
    }

    // Try building defs (case-insensitive match against available buildings).
    if let Some(def) = avail_buildings.iter().find(|d| d.name.to_lowercase() == key) {
        let bld_id   = def.id;
        let bld_name = def.name;
        let bld_cost = def.cost;
        let city_id  = session.city_ids[session.current_city];
        let city = session.state.cities.iter_mut().find(|c| c.id == city_id).unwrap();
        city.production_queue.push_back(ProductionItem::Building(bld_id));
        println!("  Queued: {} ({} prod) in {}", capitalize(bld_name), bld_cost, city.name);
        return;
    }

    // Try project defs (case-insensitive match).
    if let Some(def) = session.state.project_defs.iter().find(|d| d.name.to_lowercase() == key) {
        let proj_id   = def.id;
        let proj_name = def.name;
        let proj_cost = def.production_cost;
        let city_id  = session.city_ids[session.current_city];
        let city = session.state.cities.iter_mut().find(|c| c.id == city_id).unwrap();
        city.production_queue.push_back(ProductionItem::Project(proj_id));
        println!("  Queued: {} ({} prod) in {}", capitalize(proj_name), proj_cost, city.name);
        return;
    }

    // Check if the name matches a known unit/building that is locked.
    let all_unit_match = session.state.unit_type_defs.iter().any(|d| d.name.to_lowercase() == key);
    let all_bldg_match = session.state.building_defs.iter().any(|d| d.name.to_lowercase() == key);
    if all_unit_match || all_bldg_match {
        println!("  [error] {:?} is not yet available (requires tech unlock or is exclusive to another civ).", key);
    } else {
        let mut available: Vec<&str> = avail_units.iter().map(|d| d.name).collect();
        available.extend(avail_buildings.iter().map(|d| d.name));
        available.extend(session.state.project_defs.iter().map(|d| d.name));
        println!("  [error] Unknown unit/building {:?}. Available: {}", key, available.join(", "));
    }
}

/// Cancel (pop) the front production item and reset stored production.
fn cmd_cancel(session: &mut Session) {
    let city_id = session.city_ids[session.current_city];
    let city = session.state.cities.iter_mut().find(|c| c.id == city_id).unwrap();
    match city.production_queue.pop_front() {
        None => println!("  Production queue is empty."),
        Some(item) => {
            city.production_stored = 0;
            println!("  Cancelled: {}. Production reset to 0.", item_name_str(item));
        }
    }
}

/// Show the full production queue for the current city.
fn cmd_queue(session: &Session) {
    let city = session.state.cities.iter().find(|c| c.id == session.city_ids[session.current_city]).unwrap();
    println!("  Production queue -- {}:", city.name);
    if city.production_queue.is_empty() {
        println!("    (empty)");
        return;
    }
    for (i, item) in city.production_queue.iter().enumerate() {
        let (name, cost_str) = queue_item_display(session, item);
        if i == 0 {
            println!("    {}. {}  {}  stored: {}",
                i + 1, capitalize(&name), cost_str, city.production_stored);
        } else {
            println!("    {}. {}  {}", i + 1, capitalize(&name), cost_str);
        }
    }
}

/// Show per-turn yield breakdown for the civilization.
fn cmd_yields(session: &Session, rules: &DefaultRulesEngine) {
    let state  = &session.state;
    let civ_id = session.civ_id;
    let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
    let y = rules.compute_yields(state, civ_id);
    println!("  Yields -- {} (turn {}):", civ.name, state.turn);
    println!("    Food     : {:+} / turn", y.food);
    println!("    Production: {:+} / turn", y.production);
    println!("    Gold     : {:+} / turn  (treasury: {})", y.gold, civ.gold);
    println!("    Science  : {:+} / turn", y.science);
    println!("    Culture  : {:+} / turn", y.culture);
    println!("    Faith    : {:+} / turn", y.faith);
}

/// List all units owned by the civilization.
fn cmd_units(session: &Session) {
    let state  = &session.state;
    let civ_id = session.civ_id;
    let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
    println!("  Units ({}):", civ.name);
    let owned: Vec<&BasicUnit> = state.units.iter().filter(|u| u.owner == civ_id).collect();
    if owned.is_empty() {
        println!("    (none)");
        return;
    }
    for (i, unit) in owned.iter().enumerate() {
        let sel = if session.selected_unit == Some(unit.id) { "  [selected]" } else { "" };
        let cs_str = unit.combat_strength.map(|v| format!("  cs={v}")).unwrap_or_default();
        println!("    {:2}  {:10}  {}  hp={}/{}  mv={}/{}{}{}",
            i + 1,
            capitalize(unit_type_name(session, unit.unit_type)),
            fmtc(unit.coord()),
            unit.health(), unit.max_health(),
            unit.movement_left(), unit.max_movement(),
            cs_str, sel,
        );
    }
}

/// List all active trade routes for the civilization.
fn cmd_routes(session: &Session) {
    let state  = &session.state;
    let civ_id = session.civ_id;
    let owned: Vec<_> = state.trade_routes.iter()
        .filter(|r| r.owner == civ_id)
        .collect();
    if owned.is_empty() {
        println!("  No active trade routes.");
        return;
    }
    println!("  Active trade routes ({}):", owned.len());
    for (i, route) in owned.iter().enumerate() {
        let origin_name = state.cities.iter()
            .find(|c| c.id == route.origin)
            .map(|c| c.name.as_str()).unwrap_or("?");
        let dest_name = state.cities.iter()
            .find(|c| c.id == route.destination)
            .map(|c| c.name.as_str()).unwrap_or("?");
        let turns_str = match route.turns_remaining {
            Some(t) => format!("{t} turns left"),
            None    => "permanent".to_string(),
        };
        println!("    {:2}.  {} -> {}   gold={:+}/turn   {}",
            i + 1, origin_name, dest_name,
            route.origin_yields.gold,
            turns_str,
        );
    }
}

/// Select the unit at (q, r) as the target for move commands.
fn cmd_select(session: &mut Session, q: i32, r: i32) {
    let coord  = HexCoord::from_qr(q, r);
    let civ_id = session.civ_id;
    match session.state.units.iter().find(|u| u.owner == civ_id && u.coord() == coord) {
        None => println!("  [error] No friendly unit at ({q},{r})."),
        Some(unit) => {
            let name = unit_type_name(session, unit.unit_type).to_string();
            session.selected_unit = Some(unit.id);
            println!("  Selected: {} at ({},{})", capitalize(&name), q, r);
        }
    }
}

/// Print the score leaderboard for all civilizations.
fn cmd_scores(session: &Session) {
    let scores = all_scores(&session.state);
    if scores.is_empty() {
        println!("  No civilizations to score.");
        return;
    }
    println!();
    hline("Scores");
    println!("  {:>3}  {:<20}  {:>6}", "#", "Civilization", "Score");
    println!("  {}  {}  {}", "-".repeat(3), "-".repeat(20), "-".repeat(6));
    for (rank, (civ_id, score)) in scores.iter().enumerate() {
        let name = session.state.civilizations.iter()
            .find(|c| c.id == *civ_id)
            .map(|c| c.name)
            .unwrap_or("?");
        let marker = if *civ_id == session.civ_id { " <" } else { "" };
        println!("  {:>3}  {:<20}  {:>6}{}", rank + 1, name, score, marker);
    }
    hline_end();
}

/// Show the current World Congress state: next session, VP standings.
fn cmd_congress(session: &Session) {
    let wc = &session.state.world_congress;
    hline("World Congress");
    println!("  Session interval : every {} turns", wc.session_interval);
    println!("  Next session     : turn {}", wc.next_session_turn);
    println!("  Active resolutions: {}", wc.active_resolutions.len());
    println!();
    println!("  Diplomatic Victory Points:");
    if wc.diplomatic_victory_points.is_empty() {
        println!("    (none yet)");
    } else {
        let mut entries: Vec<_> = wc.diplomatic_victory_points.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (civ_id, vp) in entries {
            let name = session.state.civilizations.iter()
                .find(|c| c.id == *civ_id)
                .map(|c| c.name)
                .unwrap_or("?");
            let marker = if *civ_id == session.civ_id { " <" } else { "" };
            println!("    {:<20}  {} VP{}", name, vp, marker);
        }
    }
    hline_end();
}

/// Place a district on a tile for the current city.
fn cmd_district(session: &mut Session, rules: &DefaultRulesEngine, name: &str, q: i32, r: i32) {
    let coord = HexCoord::from_qr(q, r);
    let city_id = session.city_ids[session.current_city];
    let district = match name.to_ascii_lowercase().replace(' ', "_").as_str() {
        "campus"                                        => BuiltinDistrict::Campus,
        "theater_square" | "theatersquare" | "theater"  => BuiltinDistrict::TheaterSquare,
        "commercial_hub" | "commercialhub" | "commercial" => BuiltinDistrict::CommercialHub,
        "harbor"                                        => BuiltinDistrict::Harbor,
        "holy_site" | "holysite" | "holy"               => BuiltinDistrict::HolySite,
        "encampment"                                    => BuiltinDistrict::Encampment,
        "industrial_zone" | "industrialzone" | "industrial" => BuiltinDistrict::IndustrialZone,
        "entertainment_complex" | "entertainmentcomplex" | "entertainment" => BuiltinDistrict::EntertainmentComplex,
        "aqueduct"                                      => BuiltinDistrict::Aqueduct,
        "water_park" | "waterpark"                      => BuiltinDistrict::WaterPark,
        _ => {
            println!("  [error] Unknown district '{name}'.");
            println!("  Valid: campus, theater_square, commercial_hub, harbor, holy_site,");
            println!("         encampment, industrial_zone, entertainment_complex, aqueduct, water_park");
            return;
        }
    };
    match rules.place_district(&mut session.state, city_id, district, coord) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::DistrictBuilt { coord, .. } = delta {
                    println!("  District placed at {}!", fmtc(*coord));
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

/// Build an ancient road on a tile using the selected builder.
fn cmd_road(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected. Use 'select <q> <r>'.");
        return;
    };
    let Some(unit) = session.state.unit(uid) else {
        println!("  [error] Selected unit no longer exists.");
        return;
    };
    let type_name = unit_type_name(session, unit.unit_type());
    if type_name != "builder" {
        println!("  [error] Selected unit is a {type_name}, not a builder.");
        return;
    }
    let coord = HexCoord::from_qr(q, r);
    match rules.place_road(&mut session.state, uid, coord, BuiltinRoad::Ancient(AncientRoad)) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::RoadPlaced { coord, .. } = delta {
                    println!("  Ancient road built at {}!", fmtc(*coord));
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

/// Declare war on a named civilization.
fn cmd_war(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let target = session.state.civilizations.iter()
        .find(|c| c.name.to_ascii_lowercase() == name_lower && c.id != session.civ_id)
        .map(|c| (c.id, c.name));
    let Some((target_id, target_name)) = target else {
        println!("  [error] Unknown or own civilization '{name}'.");
        return;
    };
    match rules.declare_war(&mut session.state, session.civ_id, target_id) {
        Ok(_) => println!("  War declared on {target_name}!"),
        Err(e) => println!("  [error] {e}"),
    }
}

/// Make peace with a named civilization.
fn cmd_peace(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let target = session.state.civilizations.iter()
        .find(|c| c.name.to_ascii_lowercase() == name_lower && c.id != session.civ_id)
        .map(|c| (c.id, c.name));
    let Some((target_id, target_name)) = target else {
        println!("  [error] Unknown or own civilization '{name}'.");
        return;
    };
    match rules.make_peace(&mut session.state, session.civ_id, target_id) {
        Ok(_) => println!("  Peace made with {target_name}."),
        Err(e) => println!("  [error] {e}"),
    }
}

/// Adopt a government by name (direct state mutation — no RulesEngine method exists).
fn cmd_government(session: &mut Session, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let found = session.state.governments.iter()
        .find(|g| g.name.to_ascii_lowercase() == name_lower)
        .map(|g| (g.id, g.name));
    let Some((_gov_id, gov_name)) = found else {
        let available: Vec<&str> = session.state.governments.iter().map(|g| g.name).collect();
        if available.is_empty() {
            println!("  [error] No governments registered.");
        } else {
            println!("  [error] Unknown government '{name}'. Available: {}", available.join(", "));
        }
        return;
    };
    let civ = session.state.civilizations.iter_mut()
        .find(|c| c.id == session.civ_id).unwrap();
    civ.current_government_name = Some(gov_name);
    println!("  Adopted government: {gov_name}");
}

/// Found a pantheon with the named belief.
fn cmd_pantheon(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let found = session.state.belief_defs.iter()
        .find(|b| b.name.to_ascii_lowercase() == name_lower)
        .map(|b| (b.id, b.name));
    let Some((belief_id, belief_name)) = found else {
        let available: Vec<&str> = session.state.belief_defs.iter().map(|b| b.name).collect();
        if available.is_empty() {
            println!("  [error] No beliefs registered.");
        } else {
            println!("  [error] Unknown belief '{name}'. Available: {}", available.join(", "));
        }
        return;
    };
    match rules.found_pantheon(&mut session.state, session.civ_id, belief_id) {
        Ok(_) => println!("  Pantheon founded with belief: {belief_name}!"),
        Err(e) => println!("  [error] {e}"),
    }
}

/// Found a religion using the selected Great Prophet unit.
fn cmd_religion(session: &mut Session, rules: &DefaultRulesEngine, name: &str, b1: &str, b2: &str) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected. Select a Great Prophet with 'select <q> <r>'.");
        return;
    };

    let find_belief = |s: &str| -> Option<(BeliefId, &'static str)> {
        let lower = s.to_ascii_lowercase();
        session.state.belief_defs.iter()
            .find(|b| b.name.to_ascii_lowercase() == lower)
            .map(|b| (b.id, b.name))
    };

    let Some((bid1, _)) = find_belief(b1) else {
        println!("  [error] Unknown belief '{b1}'.");
        return;
    };
    let Some((bid2, _)) = find_belief(b2) else {
        println!("  [error] Unknown belief '{b2}'.");
        return;
    };

    match rules.found_religion(&mut session.state, uid, name.to_string(), vec![bid1, bid2]) {
        Ok(_) => {
            println!("  Religion '{}' founded!", name);
            session.selected_unit = None;
        }
        Err(e) => println!("  [error] {e}"),
    }
}

/// List available great people and their recruitment costs.
fn cmd_great_people(session: &Session) {
    let state = &session.state;
    let civ = state.civilizations.iter().find(|c| c.id == session.civ_id).unwrap();

    println!("  Great People:");
    let types = [
        GreatPersonType::General,
        GreatPersonType::Admiral,
        GreatPersonType::Engineer,
        GreatPersonType::Merchant,
        GreatPersonType::Scientist,
        GreatPersonType::Writer,
        GreatPersonType::Artist,
        GreatPersonType::Musician,
        GreatPersonType::Prophet,
    ];
    let mut any = false;
    for gpt in &types {
        let threshold = recruitment_threshold(*gpt, state);
        let current = civ.great_person_points.get(gpt).copied().unwrap_or(0);
        if let Some(candidate) = next_candidate_name(*gpt, state) {
            let gold_cost = threshold.saturating_sub(current) * GP_PATRONAGE_GOLD_PER_POINT;
            println!("    {:10}  {:<20}  points: {}/{}  recruit cost: {} gold",
                format!("{:?}", gpt), candidate, current, threshold, gold_cost);
            any = true;
        }
    }
    if !any {
        println!("    (no great people available)");
    }

    // List already recruited great people.
    let owned: Vec<_> = state.great_people.iter()
        .filter(|gp| gp.owner == Some(session.civ_id))
        .collect();
    if !owned.is_empty() {
        println!();
        println!("  Recruited:");
        for gp in &owned {
            let status = if gp.is_retired { " [retired]" } else { "" };
            println!("    {:?} -- {}{}", gp.person_type, gp.name, status);
        }
    }
}

/// Recruit a great person by type (spending gold).
fn cmd_recruit(session: &mut Session, rules: &DefaultRulesEngine, type_name: &str) {
    let person_type = match type_name.to_ascii_lowercase().as_str() {
        "general"   => GreatPersonType::General,
        "admiral"   => GreatPersonType::Admiral,
        "engineer"  => GreatPersonType::Engineer,
        "merchant"  => GreatPersonType::Merchant,
        "scientist" => GreatPersonType::Scientist,
        "writer"    => GreatPersonType::Writer,
        "artist"    => GreatPersonType::Artist,
        "musician"  => GreatPersonType::Musician,
        "prophet"   => GreatPersonType::Prophet,
        _ => {
            println!("  [error] Unknown type '{type_name}'.");
            println!("  Valid: general, admiral, engineer, merchant, scientist, writer, artist, musician, prophet");
            return;
        }
    };
    match rules.recruit_great_person(&mut session.state, session.civ_id, person_type) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::GreatPersonPatronized { great_person, gold_spent, .. } = delta {
                    let name = session.state.great_people.iter()
                        .find(|gp| gp.id == *great_person)
                        .map(|gp| gp.name)
                        .unwrap_or("?");
                    println!("  Recruited {} for {} gold!", name, gold_spent);
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

/// Assign a governor to a city by name and city index.
fn cmd_governor(session: &mut Session, rules: &DefaultRulesEngine, name: &str, city_idx: usize) {
    if city_idx == 0 || city_idx > session.city_ids.len() {
        println!("  [error] City index out of range (1-{}).", session.city_ids.len());
        return;
    }
    let city_id = session.city_ids[city_idx - 1];
    let name_lower = name.to_ascii_lowercase();
    let governor = session.state.governors.iter()
        .find(|g| g.def_name.to_ascii_lowercase() == name_lower && g.owner == session.civ_id)
        .map(|g| g.id);
    let Some(governor_id) = governor else {
        let available: Vec<&str> = session.state.governors.iter()
            .filter(|g| g.owner == session.civ_id)
            .map(|g| g.def_name)
            .collect();
        if available.is_empty() {
            println!("  [error] No governors available. Earn governor titles first.");
        } else {
            println!("  [error] Unknown governor '{name}'. Available: {}", available.join(", "));
        }
        return;
    };
    match rules.assign_governor(&mut session.state, governor_id, city_id) {
        Ok(_) => {
            let city_name = session.state.cities.iter()
                .find(|c| c.id == city_id)
                .map(|c| c.name.as_str())
                .unwrap_or("?");
            println!("  Governor {name} assigned to {city_name}.");
        }
        Err(e) => println!("  [error] {e}"),
    }
}

/// Show diplomatic status with all known civilizations.
fn cmd_diplomacy(session: &Session) {
    let state = &session.state;
    let civ_id = session.civ_id;

    let other_civs: Vec<_> = state.civilizations.iter()
        .filter(|c| c.id != civ_id)
        .collect();

    if other_civs.is_empty() {
        println!("  No other civilizations known.");
        return;
    }

    println!("  Diplomatic Status:");
    for other in &other_civs {
        let relation = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == civ_id && r.civ_b == other.id)
                   || (r.civ_b == civ_id && r.civ_a == other.id));
        let (status_str, war_turns) = match relation {
            Some(rel) => {
                let war_info = if rel.turns_at_war > 0 {
                    format!("  (at war for {} turns)", rel.turns_at_war)
                } else {
                    String::new()
                };
                (format!("{:?}", rel.status), war_info)
            }
            None => ("No contact".to_string(), String::new()),
        };
        println!("    {:<12}  {}{}", other.name, status_str, war_turns);
    }
}

// ── Turn header ───────────────────────────────────────────────────────────────

fn print_turn_header(session: &Session, rules: &DefaultRulesEngine) {
    let state  = &session.state;
    let civ_id = session.civ_id;
    let city_id = session.city_ids[session.current_city];
    let civ  = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
    let city = state.cities.iter().find(|c| c.id == city_id).unwrap();
    let y    = rules.compute_yields(state, civ_id);

    // Production status
    let (prod_name, prod_cost) = queue_front_info(session);
    let prod_str = match (prod_name, prod_cost) {
        (Some(n), Some(cost)) =>
            format!("{}/{} [{}]", city.production_stored, cost, capitalize(n)),
        _ =>
            format!("{}/--", city.production_stored),
    };

    let score = libciv::compute_score(state, civ_id);
    let line1 = format!("Turn {}  |  {}", state.turn, civ.name);
    let line2 = format!(
        "  {} (pop {})  food={}/{}  prod={}  gold={:+}  sci={:+}  score={}",
        city.name, city.population,
        city.food_stored, city.food_to_grow,
        prod_str, y.gold, y.science, score,
    );
    let line3 = match session.selected_unit.and_then(|uid| state.unit(uid)) {
        Some(unit) => format!(
            "  {} {}  mv={}/{}  hp={}/{}  [selected]",
            capitalize(unit_type_name(session, unit.unit_type())),
            fmtc(unit.coord()),
            unit.movement_left(), unit.max_movement(),
            unit.health(), unit.max_health(),
        ),
        None => "  (no unit selected)".to_string(),
    };

    let width = 68usize;
    let sep   = "-".repeat(width);
    println!("+-- {} --+", &sep[..width.saturating_sub(line1.len() + 6).min(width)]);
    println!("{line1}");
    println!("{line2}");
    println!("{line3}");
    println!("+{sep}+");
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn banner(title: &str) {
    let w = title.len() + 4;
    println!("+{}+", "-".repeat(w));
    println!("|  {title}  |");
    println!("+{}+", "-".repeat(w));
}

fn hline(title: &str) {
    println!("  +-- {} {}", title, "-".repeat(40usize.saturating_sub(title.len()).max(4)));
}

fn hline_end() {
    println!("  +{}", "-".repeat(44));
}

fn fmtc(c: HexCoord) -> String {
    format!("({},{})", c.q, c.r)
}

fn progress_bar(current: u32, max: u32) -> String {
    let filled = if max > 0 {
        ((current as f32 / max as f32) * 10.0).min(10.0) as usize
    } else { 0 };
    format!("{}{}", "#".repeat(filled), ".".repeat(10 - filled))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None    => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Look up the human-readable unit type name from the unit type registry.
fn unit_type_name(session: &Session, type_id: UnitTypeId) -> &str {
    session.state.unit_type_defs.iter()
        .find(|d| d.id == type_id)
        .map(|d| d.name)
        .unwrap_or("unit")
}

/// One-line yield summary for a tile: "(q,r) TerrainName  food=F  prod=P  gold=G".
fn tile_yield_row(state: &GameState, coord: HexCoord) -> String {
    match state.board.tile(coord) {
        None => format!("({},{}) [off-map]", coord.q, coord.r),
        Some(tile) => {
            let terrain = tile.terrain.name();
            let feature = tile.feature
                .map(|f| format!("+{}", f.name()))
                .unwrap_or_default();
            let y = tile.total_yields();
            format!("({},{}) {:20}  food={:2}  prod={:2}  gold={:2}",
                coord.q, coord.r,
                format!("{}{}", terrain, feature),
                y.food, y.production, y.gold)
        }
    }
}

/// Return the name and cost of the front production item, if any.
fn queue_front_info(session: &Session) -> (Option<&'static str>, Option<u32>) {
    let city_id = session.city_ids[session.current_city];
    let city = session.state.cities.iter().find(|c| c.id == city_id).unwrap();
    match city.production_queue.front() {
        None => (None, None),
        Some(ProductionItem::Unit(tid)) => {
            match session.state.unit_type_defs.iter().find(|d| d.id == *tid) {
                Some(def) => (Some(def.name), Some(def.production_cost)),
                None      => (Some("?"), None),
            }
        }
        Some(ProductionItem::Building(bid)) => {
            match session.state.building_defs.iter().find(|d| d.id == *bid) {
                Some(def) => (Some(def.name), Some(def.cost)),
                None      => (Some("building"), None),
            }
        }
        Some(ProductionItem::District(_))  => (Some("district"), None),
        Some(ProductionItem::Wonder(_))    => (Some("wonder"), None),
        Some(ProductionItem::Project(pid)) => {
            match session.state.project_defs.iter().find(|d| d.id == *pid) {
                Some(def) => (Some(def.name), Some(def.production_cost)),
                None      => (Some("project"), None),
            }
        }
    }
}

/// Return the name and formatted cost of any production item.
fn queue_item_display(session: &Session, item: &ProductionItem) -> (String, String) {
    match item {
        ProductionItem::Unit(tid) => {
            match session.state.unit_type_defs.iter().find(|d| d.id == *tid) {
                Some(def) => (def.name.to_string(), format!("({} prod)", def.production_cost)),
                None      => ("? unit".to_string(), "(? prod)".to_string()),
            }
        }
        ProductionItem::Building(bid) => {
            match session.state.building_defs.iter().find(|d| d.id == *bid) {
                Some(def) => (def.name.to_string(), format!("({} prod)", def.cost)),
                None      => ("building".to_string(), "(? prod)".to_string()),
            }
        }
        ProductionItem::District(_) => ("district".to_string(), "(? prod)".to_string()),
        ProductionItem::Wonder(_)   => ("wonder".to_string(),   "(? prod)".to_string()),
        ProductionItem::Project(pid) => {
            match session.state.project_defs.iter().find(|d| d.id == *pid) {
                Some(def) => (def.name.to_string(), format!("({} prod)", def.production_cost)),
                None      => ("project".to_string(), "(? prod)".to_string()),
            }
        }
    }
}

/// Human-readable label for a production item (for cancel message).
fn item_name_str(item: ProductionItem) -> String {
    match item {
        ProductionItem::Unit(_)     => "unit".to_string(),
        ProductionItem::Building(_) => "building".to_string(),
        ProductionItem::District(_) => "district".to_string(),
        ProductionItem::Wonder(_)   => "wonder".to_string(),
        ProductionItem::Project(_)  => "project".to_string(),
    }
}

/// Clone board tiles into a row-major 2D grid for Visualizer.
fn board_grid(state: &GameState) -> Vec<Vec<WorldTile>> {
    let board = &state.board;
    (0..board.height as i32)
        .map(|r| {
            (0..board.width as i32)
                .filter_map(|q| board.tile(HexCoord::from_qr(q, r)).cloned())
                .collect()
        })
        .collect()
}

/// Render the board with fog of war. Visible tiles show normally (city=#, unit=@),
/// explored-but-foggy tiles use lowercase letters, unexplored tiles show as '.'.
fn print_board(session: &Session) {
    let state  = &session.state;
    let civ_id = session.civ_id;
    let visible = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .map(|c| &c.visible_tiles);
    let explored = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .map(|c| &c.explored_tiles);

    const N: usize = 1;
    let mut buf = Visualizer::new(board_grid(state)).render_buffer();

    // Overlay cities and units (only on visible tiles).
    for city in &state.cities {
        if visible.is_none_or(|v| v.contains(&city.coord)) {
            overlay(&mut buf, city.coord, '#', N);
        }
    }
    for unit in &state.units {
        if visible.is_none_or(|v| v.contains(&unit.coord())) {
            overlay(&mut buf, unit.coord(), '@', N);
        }
    }

    // Apply fog of war post-processing.
    if let (Some(vis), Some(exp)) = (visible, explored) {
        let board = &state.board;
        for r in 0..board.height as i32 {
            let half = N / 2;
            let line_idx = r as usize * N + half;
            for q in 0..board.width as i32 {
                let coord = HexCoord::from_qr(q, r);
                if vis.contains(&coord) {
                    // Fully visible: keep as-is.
                } else if exp.contains(&coord) {
                    // Explored but currently in fog: lowercase the tile character.
                    let indent = if r % 2 == 1 { N } else { 0 };
                    let char_idx = indent + q as usize * N + half;
                    if let Some(line) = buf.get_mut(line_idx) {
                        let mut chars: Vec<char> = line.chars().collect();
                        if let Some(ch) = chars.get_mut(char_idx) {
                            *ch = ch.to_lowercase().next().unwrap_or(*ch);
                        }
                        *line = chars.into_iter().collect();
                    }
                } else {
                    // Never explored: show as '.'.
                    overlay(&mut buf, coord, '.', N);
                }
            }
        }
    }

    for line in &buf {
        println!("  {line}");
    }
}

/// Replace the character of tile (q, r) in a rendered buffer.
fn overlay(buf: &mut [String], coord: HexCoord, ch: char, n: usize) {
    let q        = coord.q as usize;
    let r        = coord.r as usize;
    let half     = n / 2;
    let line_idx = r * n + half;
    let indent   = if r % 2 == 1 { n } else { 0 };
    let char_idx = indent + q * n + half;

    if let Some(line) = buf.get_mut(line_idx) {
        let mut chars: Vec<char> = line.chars().collect();
        if char_idx < chars.len() {
            chars[char_idx] = ch;
        }
        *line = chars.into_iter().collect();
    }
}

/// Apply UnitMoved deltas to state (move unit, deduct movement).
fn apply_diff(state: &mut GameState, diff: &GameStateDiff) {
    for delta in &diff.deltas {
        if let StateDelta::UnitMoved { unit, to, cost, .. } = delta
            && let Some(u) = state.unit_mut(*unit)
        {
            u.coord         = *to;
            u.movement_left  = u.movement_left.saturating_sub(*cost);
        }
    }
}

/// Reset all units' movement_left to max_movement (called at start of each new turn).
fn reset_movement(state: &mut GameState) {
    for unit in &mut state.units {
        unit.movement_left = unit.max_movement;
    }
}

/// Print human-readable turn events from a diff.
fn print_turn_events(diff: &GameStateDiff) {
    let mut any = false;
    for delta in &diff.deltas {
        match delta {
            StateDelta::TurnAdvanced { from, to } => {
                println!("  -- Turn {from} -> {to} ------------------------------------");
                any = true;
            }
            StateDelta::PopulationGrew { new_population, city, .. } => {
                let _ = city;
                println!("  Roma grew!  population -> {new_population}");
                any = true;
            }
            StateDelta::GoldChanged { delta, .. } => {
                println!("  Gold {delta:+}");
                any = true;
            }
            StateDelta::TechResearched { tech, .. } => {
                println!("  Tech researched: {tech}");
                any = true;
            }
            StateDelta::CivicCompleted { civic, .. } => {
                println!("  Civic completed: {civic}");
                any = true;
            }
            StateDelta::UnitCreated { coord, .. } => {
                println!("  Unit created at ({},{})", coord.q, coord.r);
                any = true;
            }
            StateDelta::CitizenAssigned { tile, .. } => {
                println!("  Citizen auto-assigned to ({},{})", tile.q, tile.r);
                any = true;
            }
            StateDelta::UnitAttacked { attacker_damage, defender_damage, attack_type, .. } => {
                let type_str = match attack_type {
                    AttackType::Melee       => "Melee",
                    AttackType::Ranged      => "Ranged",
                    AttackType::CityBombard => "CityBombard",
                };
                println!("  [{type_str}] attacker -{attacker_damage}, defender -{defender_damage}");
                any = true;
            }
            StateDelta::UnitDestroyed { .. } => {
                println!("  A unit was destroyed!");
                any = true;
            }
            StateDelta::CityFounded { coord, .. } => {
                println!("  City founded at ({},{})!", coord.q, coord.r);
                any = true;
            }
            StateDelta::TilesRevealed { coords, .. } => {
                println!("  {} new tile(s) explored.", coords.len());
                any = true;
            }
            StateDelta::TradeRouteExpired { .. } => {
                println!("  A trade route has expired.");
                any = true;
            }
            StateDelta::TileClaimed { coord, .. } => {
                println!("  Border expanded to ({},{}).", coord.q, coord.r);
                any = true;
            }
            _ => {}
        }
    }
    if !any {
        println!("  (no notable events this turn)");
    }
}

/// Print a brief summary of the AI adversary's actions after their turn.
fn print_ai_turn_events(state: &GameState, ai_civ_id: CivId, diff: &GameStateDiff) {
    let civ_name = state.civilizations.iter()
        .find(|c| c.id == ai_civ_id)
        .map(|c| c.name)
        .unwrap_or("AI");

    let mut moves    = 0usize;
    let mut attacks  = 0usize;
    let mut founded  = 0usize;

    for delta in &diff.deltas {
        match delta {
            StateDelta::UnitMoved  { .. }   => moves   += 1,
            StateDelta::UnitAttacked { .. } => attacks += 1,
            StateDelta::CityFounded  { .. } => founded += 1,
            _ => {}
        }
    }

    if moves == 0 && attacks == 0 && founded == 0 {
        println!("  [{civ_name}] (no actions)");
        return;
    }

    print!("  [{civ_name}]");
    if moves   > 0 { print!("  {moves} unit move(s)"); }
    if attacks > 0 { print!("  {attacks} attack(s)"); }
    if founded > 0 { print!("  founded {founded} city/cities"); }
    println!();
}

fn read_line() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(_) => None,
    }
}

// ── Remaining command handlers (covers all RulesEngine methods) ──────────────

fn cmd_policy(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let found = session.state.policies.iter()
        .find(|p| p.name.to_ascii_lowercase() == name_lower)
        .map(|p| p.id);
    match found {
        None => println!("  [error] Unknown policy '{name}'."),
        Some(pid) => match rules.assign_policy(&mut session.state, session.civ_id, pid) {
            Ok(diff) => {
                for delta in &diff.deltas {
                    if let StateDelta::PolicyAssigned { .. } = delta {
                        println!("  Policy '{name}' assigned.");
                    }
                }
            }
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_spread(session: &mut Session, rules: &DefaultRulesEngine) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected.");
        return;
    };
    match rules.spread_religion(&mut session.state, uid) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::ReligionSpread { city, followers_added, .. } = delta {
                    println!("  Religion spread to city {:?}: +{followers_added} followers.", city);
                }
                if let StateDelta::UnitDestroyed { .. } = delta {
                    println!("  Unit exhausted (no charges left).");
                    session.selected_unit = None;
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

fn cmd_theo_combat(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32) {
    let target_coord = HexCoord::from_qr(q, r);
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected.");
        return;
    };
    let defender = session.state.units.iter()
        .find(|u| u.coord() == target_coord && u.owner != session.civ_id && u.category == UnitCategory::Religious)
        .map(|u| u.id);
    match defender {
        None => println!("  [error] No enemy religious unit at {}.", fmtc(target_coord)),
        Some(def_id) => match rules.theological_combat(&mut session.state, uid, def_id) {
            Ok(diff) => {
                for delta in &diff.deltas {
                    if let StateDelta::TheologicalCombat { attacker_damage, defender_damage, .. } = delta {
                        println!("  [Theological] dealt {defender_damage} dmg, took {attacker_damage} dmg");
                    }
                    if let StateDelta::UnitDestroyed { .. } = delta {
                        println!("  Unit destroyed!");
                    }
                }
                if session.state.unit(uid).is_none() { session.selected_unit = None; }
            }
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_bombard(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32) {
    let target_coord = HexCoord::from_qr(q, r);
    let city_id = session.city_ids[session.current_city];
    let defender = session.state.units.iter()
        .find(|u| u.coord() == target_coord && u.owner != session.civ_id)
        .map(|u| u.id);
    match defender {
        None => println!("  [error] No enemy unit at {}.", fmtc(target_coord)),
        Some(def_id) => match rules.city_bombard(&mut session.state, city_id, def_id) {
            Ok(diff) => {
                for delta in &diff.deltas {
                    if let StateDelta::UnitAttacked { defender_damage, .. } = delta {
                        println!("  City bombardment dealt {defender_damage} damage.");
                    }
                    if let StateDelta::UnitDestroyed { .. } = delta {
                        println!("  Enemy unit destroyed!");
                    }
                }
            }
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_retire(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let found = session.state.great_people.iter()
        .find(|gp| gp.name.to_ascii_lowercase() == name_lower && gp.owner == Some(session.civ_id) && !gp.is_retired)
        .map(|gp| gp.id);
    match found {
        None => println!("  [error] No active great person named '{name}' owned by you."),
        Some(gp_id) => match rules.retire_great_person(&mut session.state, gp_id) {
            Ok(_) => println!("  Great person '{name}' retired."),
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_great_work(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let name_lower = name.to_ascii_lowercase();
    let found = session.state.great_people.iter()
        .find(|gp| gp.name.to_ascii_lowercase() == name_lower && gp.owner == Some(session.civ_id) && !gp.is_retired)
        .map(|gp| gp.id);
    match found {
        None => println!("  [error] No active great person named '{name}' owned by you."),
        Some(gp_id) => match rules.create_great_work(&mut session.state, gp_id) {
            Ok(diff) => {
                for delta in &diff.deltas {
                    if let StateDelta::GreatWorkCreated { work_name, city, .. } = delta {
                        println!("  Great Work '{work_name}' created in city {:?}!", city);
                    }
                }
            }
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_recruit_faith(session: &mut Session, rules: &DefaultRulesEngine, type_name: &str) {
    let person_type = match type_name.to_ascii_lowercase().as_str() {
        "prophet" => GreatPersonType::Prophet,
        _ => {
            println!("  [error] Faith recruitment only works for 'prophet'.");
            return;
        }
    };
    match rules.recruit_great_person_with_faith(&mut session.state, session.civ_id, person_type) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::GreatPersonRecruited { .. } = delta {
                    println!("  Great Prophet recruited with faith!");
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

fn cmd_evangelize(session: &mut Session, rules: &DefaultRulesEngine, belief_name: &str) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected (must be an Apostle).");
        return;
    };
    let name_lower = belief_name.to_ascii_lowercase();
    let found = session.state.belief_defs.iter()
        .find(|b| b.name.to_ascii_lowercase() == name_lower)
        .map(|b| b.id);
    match found {
        None => println!("  [error] Unknown belief '{belief_name}'."),
        Some(belief_id) => match rules.evangelize_belief(&mut session.state, uid, belief_id) {
            Ok(_) => println!("  Belief '{belief_name}' evangelized."),
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_inquisition(session: &mut Session, rules: &DefaultRulesEngine) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected (must be an Apostle).");
        return;
    };
    match rules.launch_inquisition(&mut session.state, uid) {
        Ok(_) => {
            println!("  Inquisition launched. Inquisitors can now be purchased.");
            session.selected_unit = None;
        }
        Err(e) => println!("  [error] {e}"),
    }
}

fn cmd_remove_heresy(session: &mut Session, rules: &DefaultRulesEngine) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected (must be an Inquisitor).");
        return;
    };
    match rules.remove_heresy(&mut session.state, uid) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::HeresyRemoved { followers_removed, .. } = delta {
                    println!("  Removed {followers_removed} foreign followers.");
                }
                if let StateDelta::UnitDestroyed { .. } = delta {
                    println!("  Inquisitor exhausted.");
                    session.selected_unit = None;
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

fn cmd_guru_heal(session: &mut Session, rules: &DefaultRulesEngine) {
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected (must be a Guru).");
        return;
    };
    match rules.guru_heal(&mut session.state, uid) {
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::ReligiousUnitsHealed { healed_count, .. } = delta {
                    println!("  Healed {healed_count} nearby religious units.");
                }
                if let StateDelta::UnitDestroyed { .. } = delta {
                    println!("  Guru exhausted.");
                    session.selected_unit = None;
                }
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

fn cmd_claim_tile(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32) {
    let coord = HexCoord::from_qr(q, r);
    let city_id = session.city_ids[session.current_city];
    match rules.claim_tile(&mut session.state, city_id, coord, false) {
        Ok(diff) => {
            if diff.is_empty() {
                println!("  Tile already owned by you.");
            } else {
                println!("  Tile {} claimed.", fmtc(coord));
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

fn cmd_reassign_tile(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32, city_idx: usize) {
    let coord = HexCoord::from_qr(q, r);
    let from_city = session.city_ids[session.current_city];
    if city_idx == 0 || city_idx > session.city_ids.len() {
        println!("  [error] City index out of range (1-{}).", session.city_ids.len());
        return;
    }
    let to_city = session.city_ids[city_idx - 1];
    match rules.reassign_tile(&mut session.state, from_city, to_city, coord) {
        Ok(diff) => {
            if diff.is_empty() {
                println!("  Tile already belongs to that city.");
            } else {
                println!("  Tile {} reassigned to city {}.", fmtc(coord), city_idx);
            }
        }
        Err(e) => println!("  [error] {e}"),
    }
}

fn cmd_send_trader(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32) {
    let dest_coord = HexCoord::from_qr(q, r);
    let Some(uid) = session.selected_unit else {
        println!("  [error] No unit selected. Select a trader.");
        return;
    };
    let dest = session.state.cities.iter()
        .find(|c| c.coord == dest_coord)
        .map(|c| c.id);
    match dest {
        None => println!("  [error] No city at {}.", fmtc(dest_coord)),
        Some(dest_id) => match rules.assign_trade_route(&mut session.state, uid, dest_id) {
            Ok(_) => println!("  Trader assigned to move toward {}.", fmtc(dest_coord)),
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn find_camp_at(state: &GameState, q: i32, r: i32) -> Option<BarbarianCampId> {
    let coord = HexCoord::from_qr(q, r);
    state.barbarian_camps.iter()
        .find(|c| c.coord == coord)
        .map(|c| c.id)
}

fn cmd_barb_hire(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32) {
    match find_camp_at(&session.state, q, r) {
        None => println!("  [error] No barbarian camp at ({q},{r})."),
        Some(camp_id) => match rules.hire_from_barbarian_camp(&mut session.state, camp_id, session.civ_id) {
            Ok(diff) => {
                for delta in &diff.deltas {
                    if let StateDelta::BarbarianClanHired { gold_spent, .. } = delta {
                        println!("  Hired unit from camp for {gold_spent} gold.");
                    }
                }
            }
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_barb_bribe(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32) {
    match find_camp_at(&session.state, q, r) {
        None => println!("  [error] No barbarian camp at ({q},{r})."),
        Some(camp_id) => match rules.bribe_barbarian_camp(&mut session.state, camp_id, session.civ_id) {
            Ok(diff) => {
                for delta in &diff.deltas {
                    if let StateDelta::BarbarianClanBribed { gold_spent, .. } = delta {
                        println!("  Camp bribed for {gold_spent} gold.");
                    }
                }
            }
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_barb_incite(session: &mut Session, rules: &DefaultRulesEngine, q: i32, r: i32, target_name: &str) {
    let target_lower = target_name.to_ascii_lowercase();
    let target = session.state.civilizations.iter()
        .find(|c| c.name.to_ascii_lowercase() == target_lower && c.id != session.civ_id)
        .map(|c| c.id);
    let Some(target_id) = target else {
        println!("  [error] Unknown civilization '{target_name}'.");
        return;
    };
    match find_camp_at(&session.state, q, r) {
        None => println!("  [error] No barbarian camp at ({q},{r})."),
        Some(camp_id) => match rules.incite_barbarian_camp(&mut session.state, camp_id, session.civ_id, target_id) {
            Ok(diff) => {
                for delta in &diff.deltas {
                    if let StateDelta::BarbarianClanIncited { gold_spent, .. } = delta {
                        println!("  Camp incited against {target_name} for {gold_spent} gold.");
                    }
                }
            }
            Err(e) => println!("  [error] {e}"),
        }
    }
}

fn cmd_faith_buy(session: &mut Session, rules: &DefaultRulesEngine, name: &str) {
    let city_id = session.city_ids[session.current_city];
    let item = FaithPurchaseItem::Unit(string_to_static(name));
    match rules.purchase_with_faith(&mut session.state, session.civ_id, city_id, item) {
        Ok(_) => println!("  Purchased '{name}' with faith."),
        Err(e) => println!("  [error] {e}"),
    }
}

/// Leak a String into a &'static str for FaithPurchaseItem.
/// Safe for a CLI that runs once; the allocation lives until process exit.
fn string_to_static(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn cmd_governor_promote(session: &mut Session, rules: &DefaultRulesEngine, gov_name: &str, promo_name: &str) {
    let name_lower = gov_name.to_ascii_lowercase();
    let found = session.state.governors.iter()
        .find(|g| g.def_name.to_ascii_lowercase() == name_lower && g.owner == session.civ_id)
        .map(|g| g.id);
    match found {
        None => println!("  [error] Governor '{gov_name}' not found or not owned by you."),
        Some(gid) => {
            let promo_static = string_to_static(promo_name);
            match rules.promote_governor(&mut session.state, gid, promo_static) {
                Ok(_) => println!("  Governor '{gov_name}' promoted with '{promo_name}'."),
                Err(e) => println!("  [error] {e}"),
            }
        }
    }
}
