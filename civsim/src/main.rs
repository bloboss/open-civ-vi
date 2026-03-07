use std::io::{self, BufRead, Write};

use clap::{Parser, Subcommand};
use libciv::{
    CivId, GameState, GameStateDiff, DefaultRulesEngine, RulesEngine, TurnEngine, UnitCategory,
    UnitDomain, UnitId, UnitTypeId,
};
use libciv::civ::{Agenda, BasicUnit, City, Civilization, Leader, Unit};
use libciv::game::{RulesError, StateDelta};
use libciv::visualize::Visualizer;
use libciv::world::tile::WorldTile;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "civsim", about = "Open Civ VI simulation CLI", version)]
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
                "New game: {}×{} map, seed={}, turn={}",
                state.board.width, state.board.height, seed, state.turn,
            );
        }
        Command::Run { turns, seed } => {
            let mut state = GameState::new(seed, 80, 50);
            let engine = TurnEngine::new();
            let rules  = DefaultRulesEngine;
            println!("Simulating {turns} turns (seed={seed})…");
            for t in 1..=turns {
                engine.process_turn(&mut state, &rules);
                println!("  Turn {t} done.");
            }
        }
        Command::Demo => run_demo(),
        Command::Play => run_play(),
    }
}

// ── Shared game setup ─────────────────────────────────────────────────────────

struct Session {
    state:    GameState,
    _civ_id:  CivId,
    _city_id: libciv::CityId,
    unit_id:  UnitId,
}

fn build_session() -> Session {
    let mut state = GameState::new(42, 14, 8);

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
    state.civilizations[0].capital = Some(city_id);

    // Warrior at (7, 3) — 200 movement points (two Grassland tiles per turn)
    let unit_id   = state.id_gen.next_unit_id();
    let unit_type = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.units.push(BasicUnit {
        id:              unit_id,
        unit_type,
        owner:           civ_id,
        coord:           HexCoord::from_qr(7, 3),
        domain:          UnitDomain::Land,
        category:        UnitCategory::Combat,
        movement_left:   200,
        max_movement:    200,
        combat_strength: Some(20),
        promotions:      Vec::new(),
        health:          100,
    });

    Session { state, _civ_id: civ_id, _city_id: city_id, unit_id }
}

// ── Non-interactive demo ──────────────────────────────────────────────────────

fn run_demo() {
    let rules = DefaultRulesEngine;
    let Session { mut state, unit_id, .. } = build_session();

    let unit_start = state.unit(unit_id).unwrap().coord;
    let city_coord = state.cities[0].coord;

    banner("Turn 0  |  Rome  —  before any action");
    println!("  Legend:  # = city   @ = unit");
    println!("  Roma {}   Warrior {}   movement {}/{}",
        fmtc(city_coord), fmtc(unit_start),
        state.unit(unit_id).unwrap().movement_left,
        state.unit(unit_id).unwrap().max_movement,
    );
    println!();
    print_board(&state);

    let target = HexCoord::from_qr(8, 3);
    println!("\n  >>> Moving Warrior {} → {}…", fmtc(unit_start), fmtc(target));

    match rules.move_unit(&state, unit_id, target) {
        Ok(diff) => {
            apply_diff(&mut state, &diff);
            println!("      Move OK");
        }
        Err(RulesError::InsufficientMovement(diff)) => {
            apply_diff(&mut state, &diff);
            println!("      Partial move (insufficient movement)");
        }
        Err(e) => println!("      Move failed: {e}"),
    }

    let diff = rules.advance_turn(&mut state);
    reset_movement(&mut state);

    println!();
    banner(&format!("Turn {}  |  Rome", state.turn));
    print_board(&state);
    print_turn_events(&diff);

    let city = &state.cities[0];
    let unit = state.unit(unit_id).unwrap();
    println!("\n  Roma pop={}  food {}/{}   Warrior {}  mv {}/{}",
        city.population, city.food_stored, city.food_to_grow,
        fmtc(unit.coord), unit.movement_left, unit.max_movement,
    );
}

// ── Interactive play loop ─────────────────────────────────────────────────────

fn run_play() {
    let rules = DefaultRulesEngine;
    let Session { mut state, unit_id, .. } = build_session();

    print_help();

    loop {
        // ── Turn header ───────────────────────────────────────────────────────
        println!();
        let unit = state.unit(unit_id).unwrap();
        let city = &state.cities[0];
        banner(&format!(
            "Turn {}  |  Rome  |  Roma pop={}  food={}/{}  |  Warrior {}  mv={}/{}",
            state.turn,
            city.population, city.food_stored, city.food_to_grow,
            fmtc(unit.coord), unit.movement_left, unit.max_movement,
        ));
        println!();
        print_board(&state);
        println!();

        // ── Command loop for this turn ────────────────────────────────────────
        loop {
            print!("  > ");
            io::stdout().flush().ok();

            let line = match read_line() {
                Some(l) => l,
                None    => return, // EOF / Ctrl-D
            };

            match parse_cmd(&line) {
                Cmd::Move(q, r) => {
                    let target = HexCoord::from_qr(q, r);
                    let before = state.unit(unit_id).unwrap().coord;

                    match rules.move_unit(&state, unit_id, target) {
                        Ok(diff) => {
                            apply_diff(&mut state, &diff);
                            let after = state.unit(unit_id).unwrap().coord;
                            println!("  Warrior moved {} → {}   mv={}/{}",
                                fmtc(before), fmtc(after),
                                state.unit(unit_id).unwrap().movement_left,
                                state.unit(unit_id).unwrap().max_movement,
                            );
                            println!();
                            print_board(&state);
                        }
                        Err(RulesError::InsufficientMovement(diff)) if !diff.is_empty() => {
                            apply_diff(&mut state, &diff);
                            let after = state.unit(unit_id).unwrap().coord;
                            println!("  [partial] Warrior moved {} → {}  (ran out of movement)",
                                fmtc(before), fmtc(after));
                            println!("  Warrior mv={}/{}",
                                state.unit(unit_id).unwrap().movement_left,
                                state.unit(unit_id).unwrap().max_movement,
                            );
                            println!();
                            print_board(&state);
                        }
                        Err(RulesError::InsufficientMovement(_)) => {
                            println!("  [error] Not enough movement points to take even one step.");
                        }
                        Err(RulesError::DestinationImpassable) => {
                            println!("  [error] {} is impassable or unreachable.", fmtc(target));
                        }
                        Err(RulesError::InvalidCoord) => {
                            println!("  [error] ({},{}) is outside the map (0..{}, 0..{}).",
                                q, r,
                                state.board.width.saturating_sub(1),
                                state.board.height.saturating_sub(1),
                            );
                        }
                        Err(e) => {
                            println!("  [error] {e}");
                        }
                    }
                }

                Cmd::Board => {
                    println!();
                    print_board(&state);
                }

                Cmd::Status => {
                    let unit = state.unit(unit_id).unwrap();
                    let city = &state.cities[0];
                    println!("  Turn {}  |  Roma pop={}  food {}/{}",
                        state.turn, city.population, city.food_stored, city.food_to_grow);
                    println!("  Warrior {}  mv={}/{}  hp={}/{}",
                        fmtc(unit.coord),
                        unit.movement_left, unit.max_movement,
                        unit.health, unit.max_health(),
                    );
                }

                Cmd::EndTurn => break,

                Cmd::Quit => {
                    println!("  Goodbye.");
                    return;
                }

                Cmd::Help => print_help(),

                Cmd::Unknown(s) => {
                    println!("  [error] Unknown command {:?} — type 'help' for a list.", s);
                }
            }
        }

        // ── End of turn ───────────────────────────────────────────────────────
        let diff = rules.advance_turn(&mut state);
        reset_movement(&mut state);
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
    Unknown(String),
}

fn parse_cmd(raw: &str) -> Cmd {
    let s = raw.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();
    match parts.as_slice() {
        ["move" | "m", q, r] | ["go", q, r] => {
            match (q.parse::<i32>(), r.parse::<i32>()) {
                (Ok(q), Ok(r)) => Cmd::Move(q, r),
                _ => Cmd::Unknown(s.to_string()),
            }
        }
        ["board" | "b"] => Cmd::Board,
        ["status" | "s"] => Cmd::Status,
        ["end" | "e" | "next" | "n"] => Cmd::EndTurn,
        ["quit" | "q" | "exit"] => Cmd::Quit,
        ["help" | "h" | "?"] => Cmd::Help,
        _ => Cmd::Unknown(s.to_string()),
    }
}

fn print_help() {
    println!();
    println!("  Commands:");
    println!("    move <q> <r>   — move Warrior to tile (q, r)   aliases: m, go");
    println!("    board          — redraw the board               alias:  b");
    println!("    status         — show city and unit stats       alias:  s");
    println!("    end            — end turn and advance           aliases: e, next, n");
    println!("    quit           — exit                           aliases: q, exit");
    println!("    help           — this message                   aliases: h, ?");
    println!();
    println!("  Legend:  # = city   @ = unit");
    println!("  Terrain: G)rassland  P)lains  D)esert  M)ountain");
    println!("           C)oast  O)cean  T)undra  S)now");
    println!("  Movement cost: each Grassland tile costs 100; budget resets to 200 per turn.");
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn banner(title: &str) {
    let w = title.len() + 4;
    println!("┌{}┐", "─".repeat(w));
    println!("│  {}  │", title);
    println!("└{}┘", "─".repeat(w));
}

fn fmtc(c: HexCoord) -> String {
    format!("({},{})", c.q, c.r)
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

/// Render the board, then overlay city (#) and unit (@) symbols.
fn print_board(state: &GameState) {
    const N: usize = 1;
    let mut buf = Visualizer::new(board_grid(state)).render_buffer();
    for city in &state.cities {
        overlay(&mut buf, city.coord, '#', N);
    }
    for unit in &state.units {
        overlay(&mut buf, unit.coord, '@', N);
    }
    for line in &buf {
        println!("  {}", line);
    }
}

/// Replace the character of tile (q, r) in a rendered buffer.
/// Works for any tile size n: the representative row and column are both n/2.
fn overlay(buf: &mut Vec<String>, coord: HexCoord, ch: char, n: usize) {
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

/// Apply UnitMoved deltas to state and zero out remaining movement.
fn apply_diff(state: &mut GameState, diff: &GameStateDiff) {
    for delta in &diff.deltas {
        if let StateDelta::UnitMoved { unit, to, cost, .. } = delta {
            if let Some(u) = state.unit_mut(*unit) {
                u.coord = *to;
                u.movement_left -= cost;
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
                println!("  ── Turn {} → {} ─────────────────────────", from, to);
                any = true;
            }
            StateDelta::PopulationGrew { new_population, .. } => {
                println!("  Roma grew!  population → {}", new_population);
                any = true;
            }
            StateDelta::GoldChanged { delta, .. } => {
                println!("  Gold Δ{:+}", delta);
                any = true;
            }
            StateDelta::TechResearched { tech, .. } => {
                println!("  Tech researched: {}", tech);
                any = true;
            }
            StateDelta::CivicCompleted { civic, .. } => {
                println!("  Civic completed: {}", civic);
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
