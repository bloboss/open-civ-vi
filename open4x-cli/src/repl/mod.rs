//! Interactive REPL for playing a game from a saved state file.
//!
//! The REPL loads the game state into memory, accepts text commands,
//! dispatches them via the same `dispatch_action` used by the file-backed
//! CLI, and saves on end-turn and quit.

mod formatter;
mod parser;
pub mod short_ids;

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use libciv::ai::{Agent, HeuristicAgent};
use libciv::game::visibility::recalculate_visibility;
use libciv::visualize::Visualizer;
use libciv::world::tile::WorldTile;
use libciv::civ::district::BuiltinDistrict;
use libciv::{
    apply_diff, compute_score, CityId, CivId, DefaultRulesEngine, GameStateDiff,
    GameState, RulesEngine, UnitId,
};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::{HexCoord, HexDir};

use crate::handlers;
use crate::state_io;

use self::parser::{QueryKind, ReplCommand};
use self::short_ids::ShortIds;

/// Holds the in-memory game state and session context for the REPL.
pub struct ReplSession {
    state: GameState,
    rules: DefaultRulesEngine,
    civ_id: CivId,
    civ_name: &'static str,
    selected_unit: Option<UnitId>,
    selected_city: Option<CityId>,
    selected_district: Option<BuiltinDistrict>,
    unit_short_ids: ShortIds<UnitId>,
    /// Short IDs for visible foreign units (for attack targeting).
    other_unit_short_ids: ShortIds<UnitId>,
    city_short_ids: ShortIds<CityId>,
    game_file: PathBuf,
    log_path: PathBuf,
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

        // Select the first owned city, if any.
        let selected_city = state
            .cities
            .iter()
            .find(|c| c.owner == civ_id)
            .map(|c| c.id);

        let unit_short_ids = Self::build_unit_short_ids(&state, civ_id);
        let other_unit_short_ids = Self::build_other_unit_short_ids(&state, civ_id);
        let city_short_ids = Self::build_city_short_ids(&state, civ_id);

        let log_path = game_file.with_extension("json.log");

        Ok(Self {
            state,
            rules: DefaultRulesEngine,
            civ_id,
            civ_name,
            selected_unit,
            selected_city,
            selected_district: None,
            unit_short_ids,
            other_unit_short_ids,
            city_short_ids,
            game_file: game_file.to_path_buf(),
            log_path,
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
                    .map(|u| format!(" [{}]", self.unit_short_ids.short(u)))
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
                ReplCommand::MoveDirection(dir) => self.handle_move_direction(dir),
                ReplCommand::UnitSelect(ref id_str) => self.handle_unit_select(id_str),
                ReplCommand::CitySelect(ref id_str) => self.handle_city_select(id_str),
                ReplCommand::DistrictSelect(ref name) => self.handle_district_select(name),
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
        let action = match kind {
            // Resolve attack target by suffix against visible enemy units.
            crate::cli::ActionKind::Attack { unit, target } => {
                let resolved_target = self
                    .other_unit_short_ids
                    .find_by_suffix(target)
                    .map(|uid| uid.to_string())
                    .unwrap_or_else(|| target.clone());
                crate::cli::ActionKind::Attack {
                    unit: unit.clone(),
                    target: resolved_target,
                }
            }
            // Resolve coords from selected unit for PlaceImprovement.
            crate::cli::ActionKind::PlaceImprovement {
                coord_q: 0,
                coord_r: 0,
                improvement,
                builder,
            } => {
                let coord = self
                    .selected_unit
                    .and_then(|uid| self.state.units.iter().find(|u| u.id == uid))
                    .map(|u| u.coord)
                    .unwrap_or(HexCoord::from_qr(0, 0));
                crate::cli::ActionKind::PlaceImprovement {
                    coord_q: coord.q,
                    coord_r: coord.r,
                    improvement: improvement.clone(),
                    builder: builder.clone(),
                }
            }
            _ => {
                return self.dispatch_action_ref(kind);
            }
        };
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
                self.recalculate_short_ids();
                self.trace_diff("action", &diff);
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

        // Recalculate short IDs (units may have been created/destroyed).
        self.recalculate_short_ids();

        // Log the combined diff.
        self.trace_diff("end-turn", &combined_diff);

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
            QueryKind::Units => {
                formatter::print_units(
                    &self.state,
                    self.civ_id,
                    &self.unit_short_ids,
                    &self.other_unit_short_ids,
                );
            }
            QueryKind::Cities => {
                formatter::print_cities(&self.state, self.civ_id, &self.city_short_ids);
            }
            QueryKind::Yields => formatter::print_yields(&self.state, self.civ_id),
            QueryKind::Techs => formatter::print_techs(&self.state, self.civ_id),
            QueryKind::Civics => formatter::print_civics(&self.state, self.civ_id),
            QueryKind::Scores => formatter::print_scores(&self.state),
            QueryKind::Diplomacy => formatter::print_diplomacy(&self.state, self.civ_id),
            QueryKind::Tile(q, r) => {
                formatter::print_tile(&self.state, HexCoord::from_qr(*q, *r));
            }
            QueryKind::ResearchList => {
                formatter::print_available_techs(&self.state, self.civ_id);
            }
            QueryKind::CivicsList => {
                formatter::print_available_civics(&self.state, self.civ_id);
            }
            QueryKind::BuildListAll => {
                if let Some(city_id) = self.current_city_id() {
                    formatter::print_build_list_all(
                        &self.state,
                        self.civ_id,
                        city_id,
                        self.selected_district,
                    );
                } else {
                    println!("  No city selected.");
                }
            }
            QueryKind::BuildListUnits => {
                formatter::print_available_units(&self.state, self.civ_id);
            }
            QueryKind::BuildListBuildings => {
                if let Some(city_id) = self.current_city_id() {
                    formatter::print_available_buildings(
                        &self.state,
                        self.civ_id,
                        city_id,
                        self.selected_district,
                    );
                } else {
                    println!("  No city selected.");
                }
            }
            QueryKind::BuildListWonders => {
                formatter::print_available_wonders(&self.state, self.civ_id);
            }
            QueryKind::BuildListProjects => {
                if let Some(city_id) = self.current_city_id() {
                    formatter::print_available_projects(
                        &self.state,
                        self.civ_id,
                        city_id,
                    );
                } else {
                    println!("  No city selected.");
                }
            }
            QueryKind::DistrictList => {
                if let Some(city_id) = self.current_city_id() {
                    formatter::print_available_districts(
                        &self.state,
                        self.civ_id,
                        city_id,
                    );
                } else {
                    println!("  No city selected.");
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

    fn handle_move_direction(&mut self, dir: HexDir) {
        let uid = match self.selected_unit {
            Some(uid) => uid,
            None => {
                println!("  No unit selected.");
                return;
            }
        };
        let coord = match self.state.units.iter().find(|u| u.id == uid) {
            Some(u) => u.coord,
            None => {
                println!("  Selected unit no longer exists.");
                self.selected_unit = None;
                return;
            }
        };
        let target = coord + dir.unit_vec();
        let action = crate::cli::ActionKind::Move {
            unit: uid.to_string(),
            to_q: target.q,
            to_r: target.r,
        };
        self.dispatch_action_ref(&action);
    }

    fn handle_unit_select(&mut self, input: &str) {
        match self.resolve_unit(input) {
            Some(uid) => {
                self.selected_unit = Some(uid);
                let unit = self.state.units.iter().find(|u| u.id == uid).unwrap();
                let type_name = self
                    .state
                    .unit_type_defs
                    .iter()
                    .find(|d| d.id == unit.unit_type)
                    .map(|d| d.name)
                    .unwrap_or("?");
                println!("  Selected: {type_name} ({})", self.unit_short_ids.format_bold(uid));
                println!("    Coord: ({}, {})", unit.coord.q, unit.coord.r);
                println!(
                    "    HP: {}  Movement: {}/{}",
                    unit.health, unit.movement_left, unit.max_movement
                );
                if let Some(cs) = unit.combat_strength {
                    println!("    Combat strength: {cs}");
                }
                if let Some(charges) = unit.charges {
                    println!("    Charges: {charges}");
                }
            }
            None => println!("  No unit found matching '{input}'"),
        }
    }

    /// Resolve a user-provided string to a UnitId via suffix matching.
    fn resolve_unit(&self, input: &str) -> Option<UnitId> {
        self.unit_short_ids.find_by_suffix(input)
    }

    fn handle_city_select(&mut self, input: &str) {
        match self.resolve_city(input) {
            Some(cid) => {
                self.selected_city = Some(cid);
                self.selected_district = None;
                let city = self.state.cities.iter().find(|c| c.id == cid).unwrap();
                println!(
                    "  Selected: {} ({})",
                    city.name,
                    self.city_short_ids.format_bold(cid)
                );
                println!("    Coord: ({}, {})", city.coord.q, city.coord.r);
                println!("    Pop: {}  Capital: {}", city.population, city.is_capital);
                println!("    Food: {}/{}", city.food_stored, city.food_to_grow);
                println!("    Production stored: {}", city.production_stored);
                let q_len = city.production_queue.len();
                println!("    Production queue ({q_len}):");
                for item in &city.production_queue {
                    println!("      {item:?}");
                }
            }
            None => println!("  No city found matching '{input}'"),
        }
    }

    fn handle_district_select(&mut self, name: &str) {
        let city_id = match self.current_city_id() {
            Some(id) => id,
            None => {
                println!("  No city selected.");
                return;
            }
        };
        let city = match self.state.cities.iter().find(|c| c.id == city_id) {
            Some(c) => c,
            None => {
                println!("  City not found.");
                return;
            }
        };

        // Parse the district name.
        let district = match crate::handlers::action::parse_district(name) {
            Ok(d) => d,
            Err(e) => {
                println!("  {e}");
                return;
            }
        };

        // Check the city actually has this district.
        if !city.districts.contains(&district) {
            println!("  {} does not have a {}.", city.name, district.name());
            return;
        }

        self.selected_district = Some(district);

        // Show the district and its buildings.
        let placed = self.state.placed_districts.iter()
            .find(|pd| pd.city_id == city_id && pd.district_type == district);
        println!("  Selected district: {}", district.name());
        if let Some(pd) = placed {
            println!("    Coord: ({}, {})", pd.coord.q, pd.coord.r);
            if pd.buildings.is_empty() {
                println!("    Buildings: (none)");
            } else {
                println!("    Buildings:");
                for bid in &pd.buildings {
                    let bname = self.state.building_defs.iter()
                        .find(|d| d.id == *bid)
                        .map(|d| d.name)
                        .unwrap_or("?");
                    println!("      - {bname}");
                }
            }
        }
        println!("    Use 'build list' to see available buildings for this district.");
    }

    /// Resolve a user-provided string to a CityId.
    /// Tries name match first, then ID suffix match.
    fn resolve_city(&self, input: &str) -> Option<CityId> {
        // Name match (case-insensitive).
        if let Some(city) = self
            .state
            .cities
            .iter()
            .find(|c| c.owner == self.civ_id && c.name.eq_ignore_ascii_case(input))
        {
            return Some(city.id);
        }
        // ID suffix match.
        self.city_short_ids.find_by_suffix(input)
    }

    fn recalculate_short_ids(&mut self) {
        self.unit_short_ids = Self::build_unit_short_ids(&self.state, self.civ_id);
        self.other_unit_short_ids = Self::build_other_unit_short_ids(&self.state, self.civ_id);
        self.city_short_ids = Self::build_city_short_ids(&self.state, self.civ_id);
    }

    fn build_unit_short_ids(state: &GameState, civ_id: CivId) -> ShortIds<UnitId> {
        ShortIds::new(
            state
                .units
                .iter()
                .filter(|u| u.owner == civ_id)
                .map(|u| (u.id, u.id.as_ulid().to_string())),
        )
    }

    fn build_other_unit_short_ids(state: &GameState, civ_id: CivId) -> ShortIds<UnitId> {
        let visible = state
            .civilizations
            .iter()
            .find(|c| c.id == civ_id)
            .map(|c| &c.visible_tiles);
        ShortIds::new(
            state
                .units
                .iter()
                .filter(|u| {
                    u.owner != civ_id
                        && visible.is_none_or(|v| v.contains(&u.coord))
                })
                .map(|u| (u.id, u.id.as_ulid().to_string())),
        )
    }

    fn build_city_short_ids(state: &GameState, civ_id: CivId) -> ShortIds<CityId> {
        ShortIds::new(
            state
                .cities
                .iter()
                .filter(|c| c.owner == civ_id)
                .map(|c| (c.id, c.id.as_ulid().to_string())),
        )
    }

    fn handle_save(&self) {
        match state_io::save_game_file(&self.game_file, &self.state) {
            Ok(()) => println!("  Saved to {}", self.game_file.display()),
            Err(e) => eprintln!("  Save error: {e}"),
        }
    }

    // ── Trace logging ───────────────────────────────────────────────────────

    /// Append a JSON log entry for a game state diff.
    fn trace_diff(&self, source: &str, diff: &GameStateDiff) {
        if diff.is_empty() {
            return;
        }
        let entry = serde_json::json!({
            "turn": self.state.turn,
            "source": source,
            "diff": diff,
        });
        if let Err(e) = state_io::append_log(&self.log_path, &entry) {
            eprintln!("  Trace log error: {e}");
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

    /// Get the selected city ID, falling back to the first owned city.
    fn current_city_id(&self) -> Option<CityId> {
        self.selected_city.or_else(|| {
            self.state
                .cities
                .iter()
                .find(|c| c.owner == self.civ_id)
                .map(|c| c.id)
        })
    }

    fn print_help() {
        println!(
            "\
  Movement & Combat:
    move <dir>                Move selected unit in direction (e/w/ne/nw/se/sw)
    move [unit] <q> <r>       Move unit to coordinate
    e/w/ne/nw/se/sw           Direction shortcut (same as move <dir>)
    attack [unit] <target>    Attack a target unit
    settle [unit] [name]      Found a city with settler

  Production & Building:
    build <item>              Queue any production item by name
    build list                List all buildable items for selected city
    build unit [list|<name>]  List or queue a unit
    build bdg [list|<name>]   List or queue a building (alias: building)
    build wonder [list|<name>]  List or queue a wonder
    build project [list|<name>] List or queue a project
    cancel                    Cancel current production
    district <type> <q> <r>   Place a district
    district list             List available districts for selected city
    district select <name>    Select a district (scopes build list)
    improve <type>            Place improvement at selected unit
    road [unit] <q> <r>       Build road at coordinate

  Research & Government:
    research <tech>           Queue tech research
    research list             List available techs for research
    culture <civic>           Set current civic to research
    culture list              List available civics for research
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
    city <name_or_suffix>      Select city by name/ID suffix + show details
    unit <suffix>             Select unit by ID suffix + show details

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
