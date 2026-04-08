//! Interactive REPL for playing a game from a saved state file.
//!
//! The REPL loads the game state into memory, accepts text commands,
//! dispatches them via the same `dispatch_action` used by the file-backed
//! CLI, and saves on end-turn and quit.

mod formatter;
mod parser;

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use libciv::ai::{Agent, HeuristicAgent};
use libciv::game::visibility::recalculate_visibility;
use libciv::visualize::Visualizer;
use libciv::world::tile::WorldTile;
use libciv::{
    apply_diff, compute_score, CityId, CivId, DefaultRulesEngine, GameStateDiff,
    GameState, RulesEngine, UnitId,
};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::handlers;
use crate::state_io;

use self::parser::{QueryKind, ReplCommand};

/// Holds the in-memory game state and session context for the REPL.
pub struct ReplSession {
    state: GameState,
    rules: DefaultRulesEngine,
    civ_id: CivId,
    civ_name: &'static str,
    selected_unit: Option<UnitId>,
    game_file: PathBuf,
    ai_civ_ids: Vec<CivId>,
}

impl ReplSession {
    /// Create a new REPL session by loading a game file.
    pub fn new(game_file: &Path, player: &str) -> Result<Self, String> {
        let state = state_io::load_game_file(game_file)?;
        let civ_id = handlers::find_civ_by_name(&state, player)?;
        handlers::validate_human(&state, player)?;

        let civ_name = state
            .civilizations
            .iter()
            .find(|c| c.id == civ_id)
            .map(|c| c.name)
            .unwrap_or("?");

        // Collect AI civ IDs.
        let ai_civ_ids: Vec<CivId> = state
            .player_config
            .iter()
            .filter(|s| s.is_ai)
            .filter_map(|s| {
                state
                    .civilizations
                    .iter()
                    .find(|c| c.name == s.civ_name.as_str())
                    .map(|c| c.id)
            })
            .collect();

        // Select the first owned unit, if any.
        let selected_unit = state
            .units
            .iter()
            .find(|u| u.owner == civ_id)
            .map(|u| u.id);

        Ok(Self {
            state,
            rules: DefaultRulesEngine,
            civ_id,
            civ_name,
            selected_unit,
            game_file: game_file.to_path_buf(),
            ai_civ_ids,
        })
    }

    /// Run the interactive REPL loop.
    pub fn run(&mut self) {
        println!("open4x REPL -- {} ({})", self.civ_name, self.civ_id);
        println!("Type 'help' for commands, 'quit' to exit.\n");
        self.print_turn_header();
        self.print_board();

        let stdin = io::stdin();
        let mut reader = stdin.lock();
        loop {
            // Prompt.
            {
                let sel = self
                    .selected_unit
                    .map(|u| {
                        let id_str = u.to_string();
                        let short = &id_str[..8.min(id_str.len())];
                        format!(" [{}]", short)
                    })
                    .unwrap_or_default();
                print!("T{}{sel}> ", self.state.turn);
                let _ = io::stdout().flush();
            }

            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Err(e) => {
                    eprintln!("Read error: {e}");
                    break;
                }
                Ok(_) => {}
            }

            let current_city = self.current_city_id();
            let cmd = parser::parse_command(&line, self.selected_unit, current_city);

            match cmd {
                ReplCommand::Action(kind) => self.handle_action(&kind),
                ReplCommand::Query(q) => self.handle_query(&q),
                ReplCommand::EndTurn => self.handle_end_turn(),
                ReplCommand::Board => self.print_board(),
                ReplCommand::SelectUnit(q, r) => self.handle_select(q, r),
                ReplCommand::Save => self.handle_save(),
                ReplCommand::Help => Self::print_help(),
                ReplCommand::Quit => {
                    self.handle_save();
                    println!("Goodbye.");
                    return;
                }
                ReplCommand::Unknown(msg) => {
                    if !msg.is_empty() {
                        println!("  {msg}");
                    }
                }
            }
        }
    }

    // ── Command handlers ────────────────────────────────────────────────────

    fn handle_action(&mut self, kind: &crate::cli::ActionKind) {
        // For PlaceImprovement with the selected unit, fill in coords from unit position.
        let action;
        if let crate::cli::ActionKind::PlaceImprovement {
            coord_q: 0,
            coord_r: 0,
            improvement,
            builder,
        } = kind
        {
            // Resolve coord from selected unit.
            let coord = self
                .selected_unit
                .and_then(|uid| self.state.units.iter().find(|u| u.id == uid))
                .map(|u| u.coord)
                .unwrap_or(HexCoord::from_qr(0, 0));
            action = crate::cli::ActionKind::PlaceImprovement {
                coord_q: coord.q,
                coord_r: coord.r,
                improvement: improvement.clone(),
                builder: builder.clone(),
            };
        } else {
            // Clone-by-ref won't work since ActionKind isn't Clone.
            // We use the reference directly.
            return self.dispatch_action_ref(kind);
        }
        self.dispatch_action_ref(&action);
    }

    fn dispatch_action_ref(&mut self, kind: &crate::cli::ActionKind) {
        match handlers::action::dispatch_action(
            &self.rules,
            &mut self.state,
            self.civ_id,
            kind,
        ) {
            Ok(diff) => {
                apply_diff(&mut self.state, &diff);
                recalculate_visibility(&mut self.state, self.civ_id);
                formatter::print_deltas(&diff, &self.state);
            }
            Err(e) => {
                println!("  Error: {e}");
            }
        }
    }

    fn handle_end_turn(&mut self) {
        let mut combined_diff = GameStateDiff { deltas: Vec::new() };

        // Run AI turns.
        for cid in &self.ai_civ_ids.clone() {
            let agent = HeuristicAgent::new(*cid);
            let diff = agent.take_turn(&mut self.state, &self.rules);
            combined_diff.deltas.extend(diff.deltas);
        }

        // Advance turn.
        let turn_diff = self.rules.advance_turn(&mut self.state);
        combined_diff.deltas.extend(turn_diff.deltas);

        // Reset movement for all units.
        for unit in &mut self.state.units {
            unit.movement_left = unit.max_movement;
        }

        // Recalculate visibility for all civs.
        let civ_ids: Vec<CivId> = self.state.civilizations.iter().map(|c| c.id).collect();
        for cid in civ_ids {
            recalculate_visibility(&mut self.state, cid);
        }

        // Save.
        if let Err(e) = state_io::save_game_file(&self.game_file, &self.state) {
            eprintln!("  Save error: {e}");
        }

        // Print events from this turn.
        formatter::print_deltas(&combined_diff, &self.state);

        // Check game over.
        if let Some(ref go) = self.state.game_over {
            let winner = self
                .state
                .civilizations
                .iter()
                .find(|c| c.id == go.winner)
                .map(|c| c.name)
                .unwrap_or("?");
            println!("\n  === GAME OVER ===");
            println!("  Winner: {winner}");
            println!("  Condition: {}", go.condition);
            println!("  Turn: {}", go.turn);
        }

        println!();
        self.print_turn_header();
        self.print_board();
    }

    fn handle_query(&self, query: &QueryKind) {
        match query {
            QueryKind::Units => formatter::print_units(&self.state, self.civ_id),
            QueryKind::Cities => formatter::print_cities(&self.state, self.civ_id),
            QueryKind::Yields => formatter::print_yields(&self.state, self.civ_id),
            QueryKind::Techs => formatter::print_techs(&self.state, self.civ_id),
            QueryKind::Civics => formatter::print_civics(&self.state, self.civ_id),
            QueryKind::Scores => formatter::print_scores(&self.state),
            QueryKind::Diplomacy => formatter::print_diplomacy(&self.state, self.civ_id),
            QueryKind::Tile(q, r) => {
                formatter::print_tile(&self.state, HexCoord::from_qr(*q, *r));
            }
            QueryKind::CityDetail(id_or_name) => {
                // Try to find by name first, then by ID prefix.
                let city = self
                    .state
                    .cities
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(id_or_name))
                    .or_else(|| {
                        self.state
                            .cities
                            .iter()
                            .find(|c| c.id.to_string().starts_with(id_or_name.as_str()))
                    });
                match city {
                    Some(c) => {
                        println!("  City: {} ({})", c.name, c.id);
                        println!("    Coord: ({}, {})", c.coord.q, c.coord.r);
                        println!("    Pop: {}  Capital: {}", c.population, c.is_capital);
                        println!("    Food: {}/{}", c.food_stored, c.food_to_grow);
                        println!("    Production stored: {}", c.production_stored);
                        let q_len = c.production_queue.len();
                        println!("    Production queue ({q_len}):");
                        for item in &c.production_queue {
                            println!("      {item:?}");
                        }
                    }
                    None => println!("  City not found: {id_or_name}"),
                }
            }
            QueryKind::UnitDetail(id_str) => {
                let unit = self
                    .state
                    .units
                    .iter()
                    .find(|u| u.id.to_string().starts_with(id_str.as_str()));
                match unit {
                    Some(u) => {
                        let type_name = self
                            .state
                            .unit_type_defs
                            .iter()
                            .find(|d| d.id == u.unit_type)
                            .map(|d| d.name)
                            .unwrap_or("?");
                        println!("  Unit: {} ({})", type_name, u.id);
                        println!("    Coord: ({}, {})", u.coord.q, u.coord.r);
                        println!("    HP: {}  Movement: {}/{}", u.health, u.movement_left, u.max_movement);
                        if let Some(cs) = u.combat_strength {
                            println!("    Combat strength: {cs}");
                        }
                        if let Some(charges) = u.charges {
                            println!("    Charges: {charges}");
                        }
                    }
                    None => println!("  Unit not found: {id_str}"),
                }
            }
        }
    }

    fn handle_select(&mut self, q: i32, r: i32) {
        let coord = HexCoord::from_qr(q, r);
        let unit = self
            .state
            .units
            .iter()
            .find(|u| u.owner == self.civ_id && u.coord == coord);
        match unit {
            Some(u) => {
                let type_name = self
                    .state
                    .unit_type_defs
                    .iter()
                    .find(|d| d.id == u.unit_type)
                    .map(|d| d.name)
                    .unwrap_or("?");
                self.selected_unit = Some(u.id);
                println!("  Selected: {type_name} at ({q}, {r})");
            }
            None => {
                println!("  No owned unit at ({q}, {r}).");
            }
        }
    }

    fn handle_save(&self) {
        match state_io::save_game_file(&self.game_file, &self.state) {
            Ok(()) => println!("  Saved to {}", self.game_file.display()),
            Err(e) => eprintln!("  Save error: {e}"),
        }
    }

    // ── Display helpers ─────────────────────────────────────────────────────

    fn print_turn_header(&self) {
        let gold = self
            .state
            .civilizations
            .iter()
            .find(|c| c.id == self.civ_id)
            .map(|c| c.gold)
            .unwrap_or(0);
        let score = compute_score(&self.state, self.civ_id);
        println!(
            "Turn {} | {} | Gold: {} | Score: {}",
            self.state.turn, self.civ_name, gold, score
        );
    }

    fn print_board(&self) {
        let visible = self
            .state
            .civilizations
            .iter()
            .find(|c| c.id == self.civ_id)
            .map(|c| &c.visible_tiles);
        let explored = self
            .state
            .civilizations
            .iter()
            .find(|c| c.id == self.civ_id)
            .map(|c| &c.explored_tiles);

        const N: usize = 1;
        let mut buf = Visualizer::new(board_grid(&self.state)).render_buffer();

        // Overlay cities and units (only on visible tiles).
        for city in &self.state.cities {
            if visible.is_none_or(|v| v.contains(&city.coord)) {
                overlay(&mut buf, city.coord, '#', N);
            }
        }
        for unit in &self.state.units {
            if visible.is_none_or(|v| v.contains(&unit.coord)) {
                overlay(&mut buf, unit.coord, '@', N);
            }
        }

        // Apply fog of war.
        if let (Some(vis), Some(exp)) = (visible, explored) {
            let board = &self.state.board;
            for r in 0..board.height as i32 {
                let half = N / 2;
                let line_idx = r as usize * N + half;
                for q in 0..board.width as i32 {
                    let coord = HexCoord::from_qr(q, r);
                    if vis.contains(&coord) {
                        // Fully visible.
                    } else if exp.contains(&coord) {
                        // Explored but foggy: lowercase.
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
                        // Never explored.
                        overlay(&mut buf, coord, '.', N);
                    }
                }
            }
        }

        for line in &buf {
            println!("  {line}");
        }
    }

    /// Get the first city ID owned by this civ, if any.
    fn current_city_id(&self) -> Option<CityId> {
        self.state
            .cities
            .iter()
            .find(|c| c.owner == self.civ_id)
            .map(|c| c.id)
    }

    fn print_help() {
        println!(
            "\
  Movement & Combat:
    move [unit] <q> <r>       Move unit to coordinate
    attack [unit] <target>    Attack a target unit
    settle [unit] [name]      Found a city with settler

  Production & Building:
    build <item>              Queue production item
    cancel                    Cancel current production
    district <type> <q> <r>   Place a district
    improve <type>            Place improvement at selected unit
    road [unit] <q> <r>       Build road at coordinate

  Research & Government:
    research <tech>           Queue tech research
    adopt-government <name>   Adopt a government
    assign-policy <policy>    Assign a policy card

  Diplomacy:
    declare-war <civ>         Declare war
    make-peace <civ>          Make peace
    form-alliance <civ> <type>  Form alliance

  Trade & Religion:
    trade [unit] <dest_city>  Establish trade route
    pantheon <belief>         Found a pantheon
    religion <prophet> <name> <beliefs...>
    spread [unit]             Spread religion

  Great People & Governors:
    recruit <type>            Recruit great person
    governor <gov> <city>     Assign governor
    promote [unit] <promo>    Promote a unit

  Queries:
    units / cities / yields / techs / civics / scores / diplomacy
    tile <q> <r>              Inspect a tile
    city <name_or_id>         City details
    unit <id>                 Unit details

  Other:
    select <q> <r>            Select unit at coordinate
    end / next / n            End turn
    board / map               Redraw map
    save                      Save game
    help / h / ?              Show this help
    quit / exit / q           Save and quit"
        );
    }
}

// ── Board rendering helpers ─────────────────────────────────────────────────

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

fn overlay(buf: &mut [String], coord: HexCoord, ch: char, n: usize) {
    let q = coord.q as usize;
    let r = coord.r as usize;
    let half = n / 2;
    let line_idx = r * n + half;
    let indent = if r % 2 == 1 { n } else { 0 };
    let char_idx = indent + q * n + half;

    if let Some(line) = buf.get_mut(line_idx) {
        let mut chars: Vec<char> = line.chars().collect();
        if char_idx < chars.len() {
            chars[char_idx] = ch;
        }
        *line = chars.into_iter().collect();
    }
}
