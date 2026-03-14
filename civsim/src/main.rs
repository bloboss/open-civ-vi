use std::io::{self, BufRead, Write};

use clap::{Parser, Subcommand};
use libciv::{
    CityId, CivId, GameState, GameStateDiff, DefaultRulesEngine, RulesEngine, TurnEngine,
    UnitCategory, UnitDomain, UnitId, UnitTypeId,
};
use libciv::ai::{Agent, HeuristicAgent};
use libciv::civ::{Agenda, BasicUnit, City, Civilization, Leader, ProductionItem, TechProgress, Unit};
use libciv::game::{AttackType, RulesError, StateDelta, recalculate_visibility};
use libciv::game::state::UnitTypeDef;
use libciv::visualize::Visualizer;
use libciv::world::improvement::BuiltinImprovement;
use libciv::world::terrain::BuiltinTerrain;
use libciv::world::tile::WorldTile;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::Rng;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "open4x", about = "open4x simulation CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start a new game and report board dimensions
    New {
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        #[arg(short = 'W', long, default_value_t = 80)]
        width: u32,
        #[arg(short = 'H', long, default_value_t = 50)]
        height: u32,
    },
    /// Run the simulation for N turns without displaying anything
    Run {
        #[arg(short, long, default_value_t = 1)]
        turns: u32,
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
    },
    /// Non-interactive demo: create game, move once, advance one turn
    Demo,
    /// AI agent demo: two HeuristicAgents play against each other for N turns
    AiDemo {
        /// Number of turns to simulate (default: 50)
        #[arg(short, long, default_value_t = 50)]
        turns: u32,
        /// RNG seed for terrain generation (default: 42)
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        /// Print the board every N turns; 0 = only at start and end (default: 10)
        #[arg(long, default_value_t = 10)]
        board_every: u32,
    },
    /// Interactive: move your Warrior turn-by-turn
    Play,
}

// ── Agenda stub ───────────────────────────────────────────────────────────────

struct NoOpAgenda;

impl std::fmt::Debug for NoOpAgenda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoOpAgenda")
    }
}

impl Agenda for NoOpAgenda {
    fn name(&self) -> &'static str { "Expansionist" }
    fn description(&self) -> &'static str { "Likes open land." }
    fn attitude(&self, _toward: CivId) -> i32 { 0 }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::New { seed, width, height } => {
            let state = GameState::new(seed, width, height);
            println!(
                "New game: {}x{} map, seed={}, turn={}",
                state.board.width, state.board.height, seed, state.turn,
            );
        }
        Command::Run { turns, seed } => {
            let mut state = GameState::new(seed, 80, 50);
            let engine = TurnEngine::new();
            let rules  = DefaultRulesEngine;
            println!("Simulating {turns} turns (seed={seed})...");
            for t in 1..=turns {
                engine.process_turn(&mut state, &rules);
                println!("  Turn {t} done.");
            }
        }
        Command::Demo => run_demo(),
        Command::AiDemo { turns, seed, board_every } => run_ai_demo(turns, seed, board_every),
        Command::Play => run_play(),
    }
}

// ── Session ───────────────────────────────────────────────────────────────────

struct Session {
    state:         GameState,
    civ_id:        CivId,
    city_ids:      Vec<CityId>,
    current_city:  usize,
    selected_unit: Option<UnitId>,
}

/// Randomly assign terrain to every board tile using a seeded RNG.
/// Tiles within 1 hex of `safe_coord` are kept habitable (Grassland/Plains only).
/// Weighted distribution: 35% Grassland, 25% Plains, 15% Desert, 10% Tundra,
///   8% Mountain, 4% Ocean, 3% (rolled as Ocean -> Coast in the visualizer).
fn randomize_terrain(state: &mut GameState, seed: u64, safe_coord: HexCoord) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let safe: std::collections::HashSet<HexCoord> = {
        let mut s = std::collections::HashSet::new();
        s.insert(safe_coord);
        for n in state.board.neighbors(safe_coord) { s.insert(n); }
        s
    };
    let coords: Vec<HexCoord> = state.board.all_coords();
    for coord in coords {
        let terrain = if safe.contains(&coord) {
            // Near the capital: only habitable terrain.
            match rng.random_range(0u8..4) {
                0 | 1 => BuiltinTerrain::Grassland,
                _     => BuiltinTerrain::Plains,
            }
        } else {
            match rng.random_range(0u8..100) {
                0..35  => BuiltinTerrain::Grassland,
                35..60 => BuiltinTerrain::Plains,
                60..75 => BuiltinTerrain::Desert,
                75..85 => BuiltinTerrain::Tundra,
                85..93 => BuiltinTerrain::Mountain,
                _      => BuiltinTerrain::Ocean,
            }
        };
        if let Some(tile) = state.board.tile_mut(coord) {
            tile.terrain = terrain;
        }
    }
}

fn build_session() -> Session {
    let seed = 42u64;
    let mut state = GameState::new(seed, 14, 8);

    // Civilization: Rome / Caesar
    let civ_id = state.id_gen.next_civ_id();
    let leader = Leader {
        name: "Caesar",
        civ_id,
        abilities: Vec::new(),
        agenda: Box::new(NoOpAgenda),
    };
    state.civilizations.push(Civilization::new(civ_id, "Rome", "Roman", leader));

    // Capital city at (3, 3)
    let city_coord = HexCoord::from_qr(3, 3);
    let city_id    = state.id_gen.next_city_id();
    let mut city   = City::new(city_id, "Roma".to_string(), civ_id, city_coord);
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations[0].cities.push(city_id);

    // Randomize terrain, keeping the city's immediate neighborhood habitable.
    randomize_terrain(&mut state, seed, city_coord);

    // Populate unit type registry. Each def has a stable ID used for production lookups.
    let warrior_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let builder_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let slinger_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef { id: warrior_type_id, name: "warrior", production_cost: 40,
                      max_movement: 200, combat_strength: Some(20),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None },
        UnitTypeDef { id: settler_type_id, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None },
        UnitTypeDef { id: builder_type_id, name: "builder", production_cost: 50,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None },
        UnitTypeDef { id: slinger_type_id, name: "slinger", production_cost: 35,
                      max_movement: 200, combat_strength: Some(10),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 2, vision_range: 2, can_found_city: false, resource_cost: None },
    ]);

    // Starting Warrior at (7, 3).
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id:              unit_id,
        unit_type:       warrior_type_id,
        owner:           civ_id,
        coord:           HexCoord::from_qr(7, 3),
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
        health:          100,
        range:           0,
        vision_range:    2,
    });

    recalculate_visibility(&mut state, civ_id);

    Session { state, civ_id, city_ids: vec![city_id], current_city: 0, selected_unit: Some(unit_id) }
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

    // ── Unit-type registry ────────────────────────────────────────────────
    let warrior_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let settler_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    let slinger_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.extend([
        UnitTypeDef { id: warrior_type, name: "warrior", production_cost: 40,
                      max_movement: 200, combat_strength: Some(20),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 0, vision_range: 2, can_found_city: false, resource_cost: None },
        UnitTypeDef { id: settler_type, name: "settler", production_cost: 80,
                      max_movement: 200, combat_strength: None,
                      domain: UnitDomain::Land, category: UnitCategory::Civilian,
                      range: 0, vision_range: 2, can_found_city: true, resource_cost: None },
        UnitTypeDef { id: slinger_type, name: "slinger", production_cost: 35,
                      max_movement: 200, combat_strength: Some(10),
                      domain: UnitDomain::Land, category: UnitCategory::Combat,
                      range: 2, vision_range: 2, can_found_city: false, resource_cost: None },
    ]);

    // ── Rome (west side) ──────────────────────────────────────────────────
    let rome_id = state.id_gen.next_civ_id();
    state.civilizations.push(Civilization::new(rome_id, "Rome", "Roman",
        Leader { name: "Caesar", civ_id: rome_id, abilities: Vec::new(),
                 agenda: Box::new(NoOpAgenda) }));

    let rome_city_coord = HexCoord::from_qr(3, 4);
    randomize_terrain(&mut state, seed, rome_city_coord);

    let rome_city = state.id_gen.next_city_id();
    let mut rc = City::new(rome_city, "Roma".to_string(), rome_id, rome_city_coord);
    rc.is_capital = true;
    state.cities.push(rc);
    state.civilizations.iter_mut().find(|c| c.id == rome_id).unwrap()
        .cities.push(rome_city);

    // Rome's starting warrior.
    let rome_warrior = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: rome_warrior, unit_type: warrior_type, owner: rome_id,
        coord: HexCoord::from_qr(5, 4),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        health: 100, range: 0, vision_range: 2,
    });

    // ── Babylon (east side) ───────────────────────────────────────────────
    let babylon_id = state.id_gen.next_civ_id();
    state.civilizations.push(Civilization::new(babylon_id, "Babylon", "Babylonian",
        Leader { name: "Hammurabi", civ_id: babylon_id, abilities: Vec::new(),
                 agenda: Box::new(NoOpAgenda) }));

    let babylon_city_coord = HexCoord::from_qr(16, 7);
    randomize_terrain(&mut state, seed.wrapping_add(1), babylon_city_coord);

    let babylon_city = state.id_gen.next_city_id();
    let mut bc = City::new(babylon_city, "Babylon".to_string(), babylon_id, babylon_city_coord);
    bc.is_capital = true;
    state.cities.push(bc);
    state.civilizations.iter_mut().find(|c| c.id == babylon_id).unwrap()
        .cities.push(babylon_city);

    // Babylon's starting warrior.
    let babylon_warrior = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: babylon_warrior, unit_type: warrior_type, owner: babylon_id,
        coord: HexCoord::from_qr(14, 7),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        health: 100, range: 0, vision_range: 2,
    });

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
                                                            AttackType::Melee  => "Melee",
                                                            AttackType::Ranged => "Ranged",
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
                    if let Some(prog) = civ.research_queue.front() {
                        if let Some(node) = session.state.tech_tree.get(prog.tech_id) {
                            println!("  In progress: {} ({}/{})",
                                node.name, prog.progress, node.cost);
                        }
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

                Cmd::EndTurn => break,

                Cmd::Quit => {
                    println!("  Goodbye.");
                    return;
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

    match rules.place_improvement(&mut session.state, session.civ_id, coord, improvement) {
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

/// Queue a unit for production by name (case-insensitive).
fn cmd_build(session: &mut Session, name: &str) {
    let key = name.trim().to_lowercase();
    let def_idx = session.state.unit_type_defs.iter().position(|d| d.name == key);
    match def_idx {
        None => {
            let available: Vec<&str> = session.state.unit_type_defs.iter().map(|d| d.name).collect();
            println!("  [error] Unknown unit {:?}. Available: {}", key, available.join(", "));
        }
        Some(idx) => {
            let type_id  = session.state.unit_type_defs[idx].id;
            let def_name = session.state.unit_type_defs[idx].name;
            let def_cost = session.state.unit_type_defs[idx].production_cost;
            let city_id  = session.city_ids[session.current_city];
            let city = session.state.cities.iter_mut().find(|c| c.id == city_id).unwrap();
            city.production_queue.push_back(ProductionItem::Unit(type_id));
            println!("  Queued: {} ({} prod) in {}", capitalize(def_name), def_cost, city.name);
        }
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

    let line1 = format!("Turn {}  |  {}", state.turn, civ.name);
    let line2 = format!(
        "  {} (pop {})  food={}/{}  prod={}  gold={:+}  sci={:+}",
        city.name, city.population,
        city.food_stored, city.food_to_grow,
        prod_str, y.gold, y.science,
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
        Some(ProductionItem::Building(_))  => (Some("building"), None),
        Some(ProductionItem::District(_))  => (Some("district"), None),
        Some(ProductionItem::Wonder(_))    => (Some("wonder"), None),
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
        ProductionItem::Building(_) => ("building".to_string(), "(? prod)".to_string()),
        ProductionItem::District(_) => ("district".to_string(), "(? prod)".to_string()),
        ProductionItem::Wonder(_)   => ("wonder".to_string(),   "(? prod)".to_string()),
    }
}

/// Human-readable label for a production item (for cancel message).
fn item_name_str(item: ProductionItem) -> String {
    match item {
        ProductionItem::Unit(_)     => "unit".to_string(),
        ProductionItem::Building(_) => "building".to_string(),
        ProductionItem::District(_) => "district".to_string(),
        ProductionItem::Wonder(_)   => "wonder".to_string(),
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
        if let StateDelta::UnitMoved { unit, to, cost, .. } = delta {
            if let Some(u) = state.unit_mut(*unit) {
                u.coord         = *to;
                u.movement_left  = u.movement_left.saturating_sub(*cost);
            }
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
                    AttackType::Melee  => "Melee",
                    AttackType::Ranged => "Ranged",
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
            _ => {}
        }
    }
    if !any {
        println!("  (no notable events this turn)");
    }
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
