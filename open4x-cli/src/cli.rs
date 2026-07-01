use std::path::PathBuf;

use clap::{Parser, Subcommand};

// ── Top-level CLI ────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "open4x", about = "Civilization VI game engine CLI", version)]
pub struct Cli {
    /// API base URL of an open4x-server instance. When set, every
    /// subcommand dispatches over HTTP instead of running the rules
    /// engine in-process. See book/src/roadmap/cli-server-mode.md.
    #[arg(long, env = "OPEN4X_SERVER_URL", global = true)]
    pub server: Option<String>,

    /// Path to the JSON file holding the bearer token + game/civ IDs
    /// returned by `new-game` in server mode. `new-game` writes it;
    /// every other subcommand reads it. Ignored without `--server`.
    #[arg(
        long,
        env = "OPEN4X_TOKEN_FILE",
        global = true,
        default_value = ".open4x-session.json"
    )]
    pub token_file: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new game and write initial state to disk
    NewGame {
        /// Path to write the new game file (required in local mode;
        /// ignored in `--server` mode where the session JSON holds
        /// the game ID instead).
        #[arg(long)]
        game_file: Option<PathBuf>,
        /// RNG seed
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Map width
        #[arg(long, default_value_t = 80)]
        width: u32,
        /// Map height
        #[arg(long, default_value_t = 50)]
        height: u32,
        /// Human player civ names (can specify multiple)
        #[arg(long)]
        player: Vec<String>,
        /// AI player civ names (can specify multiple)
        #[arg(long)]
        ai: Vec<String>,
        /// Victory types to enable (score, culture, domination, religious, science, diplomatic)
        #[arg(long, default_value = "score")]
        victory: Vec<String>,
    },
    /// Perform a single game action
    Action {
        /// Path to the game state file (required in local mode;
        /// ignored in `--server` mode).
        #[arg(long)]
        game_file: Option<PathBuf>,
        /// Player performing the action (required in local mode;
        /// in `--server` mode the bearer token already identifies
        /// the civ).
        #[arg(long)]
        player: Option<String>,
        #[command(subcommand)]
        action: ActionKind,
    },
    /// End a player's turn
    EndTurn {
        /// Path to the game state file (required in local mode;
        /// ignored in `--server` mode).
        #[arg(long)]
        game_file: Option<PathBuf>,
        /// Player ending their turn (required in local mode;
        /// ignored in `--server` mode).
        #[arg(long)]
        player: Option<String>,
    },
    /// Export player-visible state (fog-of-war filtered)
    View {
        /// Path to the game state file (required in local mode;
        /// ignored in `--server` mode).
        #[arg(long)]
        game_file: Option<PathBuf>,
        /// Player whose view to export (required in local mode;
        /// ignored in `--server` mode).
        #[arg(long)]
        player: Option<String>,
    },
    /// Read-only queries against the game state
    Status {
        /// Path to the game state file (required in local mode;
        /// ignored in `--server` mode).
        #[arg(long)]
        game_file: Option<PathBuf>,
        /// Player perspective for fog-of-war filtering (required in
        /// local mode; ignored in `--server` mode).
        #[arg(long)]
        player: Option<String>,
        #[command(subcommand)]
        kind: StatusKind,
    },
    /// List available items
    List {
        /// Path to the game state file (required in local mode;
        /// ignored in `--server` mode).
        #[arg(long)]
        game_file: Option<PathBuf>,
        /// Player whose available items to list (required in local
        /// mode; ignored in `--server` mode).
        #[arg(long)]
        player: Option<String>,
        #[command(subcommand)]
        kind: ListKind,
    },
    /// Interactive REPL for an existing game file
    Repl {
        /// Path to the game state file
        #[arg(long)]
        game_file: PathBuf,
        /// Human player name
        #[arg(long)]
        player: String,
    },
    /// Legacy interactive REPL (backward compat)
    Play,
    /// Legacy demo mode
    Demo,
    /// Legacy AI demo mode
    AiDemo {
        /// Number of turns to simulate
        #[arg(short, long, default_value_t = 50)]
        turns: u32,
        /// RNG seed for terrain generation
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        /// Print the board every N turns; 0 = only at start and end
        #[arg(long, default_value_t = 10)]
        board_every: u32,
    },
}

// ── Action subcommands ───────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum ActionKind {
    // ── Movement & Combat ────────────────────────────────────────────────
    /// Move a unit to a hex coordinate
    Move {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        to_q: i32,
        #[arg(long)]
        to_r: i32,
    },
    /// Attack a target unit
    Attack {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        target: String,
    },
    /// Bombard a target unit from a city
    CityBombard {
        #[arg(long)]
        city: String,
        #[arg(long)]
        target: String,
    },
    /// Theological combat between religious units
    TheologicalCombat {
        #[arg(long)]
        attacker: String,
        #[arg(long)]
        defender: String,
    },
    /// Promote a unit
    PromoteUnit {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        promotion: String,
    },
    /// Perform with a Rock Band unit
    RockBandPerform {
        #[arg(long)]
        unit: String,
    },

    // ── City & Production ────────────────────────────────────────────────
    /// Found a city with a settler unit
    FoundCity {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        name: String,
    },
    /// Queue a production item in a city
    Build {
        #[arg(long)]
        city: String,
        #[arg(long)]
        item: String,
    },
    /// Cancel the current production in a city
    CancelProduction {
        #[arg(long)]
        city: String,
    },
    /// Place a district at a hex coordinate
    PlaceDistrict {
        #[arg(long)]
        city: String,
        #[arg(long)]
        district: String,
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
    },
    /// Place an improvement at a hex coordinate
    PlaceImprovement {
        #[arg(long)]
        coord_q: i32,
        #[arg(long)]
        coord_r: i32,
        #[arg(long)]
        improvement: String,
        #[arg(long)]
        builder: Option<String>,
    },
    /// Place a road at a hex coordinate using a builder unit
    PlaceRoad {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
    },
    /// Assign a citizen to work a tile in a city
    AssignCitizen {
        #[arg(long)]
        city: String,
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
        #[arg(long)]
        lock: bool,
    },
    /// Set the player-selected production focus for a city
    AssignCityFocus {
        #[arg(long)]
        city: String,
        /// One of: default, food, production, gold, science, culture, faith
        #[arg(long)]
        focus: String,
    },
    /// Rename a city
    RenameCity {
        #[arg(long)]
        city: String,
        #[arg(long)]
        name: String,
    },
    /// Unassign a citizen from a tile in a city
    UnassignCitizen {
        #[arg(long)]
        city: String,
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
    },
    /// Claim a tile for a city
    ClaimTile {
        #[arg(long)]
        city: String,
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
        #[arg(long)]
        force: bool,
    },
    /// Reassign a tile from one city to another
    ReassignTile {
        #[arg(long)]
        from_city: String,
        #[arg(long)]
        to_city: String,
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
    },

    // ── Tech & Civics ────────────────────────────────────────────────────
    /// Queue a technology for research
    Research {
        #[arg(long)]
        tech: String,
    },
    /// Cancel the active (front) tech in the player's research queue
    CancelResearch,
    /// Cancel the player's currently-studied civic (clears
    /// `civic_in_progress`)
    CancelCivic,
    /// Set the current civic to research
    StudyCivic {
        #[arg(long)]
        civic: String,
    },
    /// Adopt a government
    AdoptGovernment {
        #[arg(long)]
        name: String,
    },
    /// Assign a policy card
    AssignPolicy {
        #[arg(long)]
        policy: String,
    },

    // ── Diplomacy ────────────────────────────────────────────────────────
    /// Declare war on another civilization
    DeclareWar {
        #[arg(long)]
        target: String,
    },
    /// Make peace with another civilization
    MakePeace {
        #[arg(long)]
        target: String,
    },
    /// Form an alliance with another civilization
    FormAlliance {
        #[arg(long)]
        target: String,
        #[arg(long)]
        alliance_type: String,
    },

    // ── Trade ────────────────────────────────────────────────────────────
    /// Assign a trade route via a trader unit
    AssignTradeRoute {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        dest_city: String,
    },
    /// Establish a trade route via a trader unit
    EstablishTradeRoute {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        dest_city: String,
    },

    // ── Religion ─────────────────────────────────────────────────────────
    /// Found a pantheon with a chosen belief
    FoundPantheon {
        #[arg(long)]
        belief: String,
    },
    /// Found a religion with a Great Prophet
    FoundReligion {
        #[arg(long)]
        prophet: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        beliefs: Vec<String>,
    },
    /// Spread religion with a missionary or apostle
    SpreadReligion {
        #[arg(long)]
        unit: String,
    },
    /// Evangelize a belief with an apostle
    EvangelizeBelief {
        #[arg(long)]
        apostle: String,
        #[arg(long)]
        belief: String,
    },
    /// Launch an inquisition with an apostle
    LaunchInquisition {
        #[arg(long)]
        apostle: String,
    },
    /// Remove heresy with an inquisitor
    RemoveHeresy {
        #[arg(long)]
        inquisitor: String,
    },
    /// Heal nearby units with a guru
    GuruHeal {
        #[arg(long)]
        guru: String,
    },
    /// Purchase an item with faith in a city
    PurchaseWithFaith {
        #[arg(long)]
        city: String,
        #[arg(long)]
        item: String,
    },

    // ── Great People ─────────────────────────────────────────────────────
    /// Retire a Great Person for their unique ability
    RetireGreatPerson {
        #[arg(long)]
        person: String,
    },
    /// Create a Great Work with a Great Person
    CreateGreatWork {
        #[arg(long)]
        person: String,
    },
    /// Recruit a Great Person with gold/points
    RecruitGreatPerson {
        #[arg(long)]
        person_type: String,
    },
    /// Recruit a Great Person with faith
    RecruitGreatPersonFaith {
        #[arg(long)]
        person_type: String,
    },

    // ── Governors ────────────────────────────────────────────────────────
    /// Assign a governor to a city
    AssignGovernor {
        #[arg(long)]
        governor: String,
        #[arg(long)]
        city: String,
    },
    /// Promote a governor with a specific promotion
    PromoteGovernor {
        #[arg(long)]
        governor: String,
        #[arg(long)]
        promotion: String,
    },

    // ── Victory ──────────────────────────────────────────────────────────
    /// Complete a science victory milestone
    CompleteScienceMilestone,

    // ── Barbarians ───────────────────────────────────────────────────────
    /// Hire a barbarian from a camp
    HireBarbarian {
        #[arg(long)]
        camp_q: i32,
        #[arg(long)]
        camp_r: i32,
    },
    /// Bribe a barbarian camp
    BribeBarbarian {
        #[arg(long)]
        camp_q: i32,
        #[arg(long)]
        camp_r: i32,
    },
    /// Incite a barbarian camp against a target civilization
    InciteBarbarian {
        #[arg(long)]
        camp_q: i32,
        #[arg(long)]
        camp_r: i32,
        #[arg(long)]
        target: String,
    },
    /// Raid a barbarian camp with a unit
    RaidBarbarian {
        #[arg(long)]
        unit: String,
        #[arg(long)]
        camp_q: i32,
        #[arg(long)]
        camp_r: i32,
    },
    /// Clear a barbarian camp
    ClearBarbarian {
        #[arg(long)]
        camp_q: i32,
        #[arg(long)]
        camp_r: i32,
    },
}

// ── Status subcommands ───────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum StatusKind {
    /// Show scores for all civilizations
    Scores,
    /// Show details for a specific city
    City {
        #[arg(long)]
        id: String,
    },
    /// Show details for a specific unit
    Unit {
        #[arg(long)]
        id: String,
    },
    /// Show details for a specific tile
    Tile {
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
    },
    /// Show diplomatic relations
    Diplomacy,
    /// Show World Congress state
    Congress,
    /// Show yield summary
    Yields,
    /// Show researched and available technologies
    Techs,
    /// Show researched and available civics
    Civics,
    /// Show pending turn actions (research/civic to choose, units with
    /// movement, cities with empty production queues)
    Pending,
    /// Show victory progress for each registered condition
    Victory,
    /// Show available actions for a specific unit
    UnitActions {
        #[arg(long)]
        id: String,
    },
    /// Show a deterministic combat preview for an attacker engaging the
    /// unit at (q, r)
    CombatPreview {
        #[arg(long)]
        attacker: String,
        #[arg(long)]
        q: i32,
        #[arg(long)]
        r: i32,
    },
    /// Show the policy-card catalogue for the player (active /
    /// available / locked)
    Policies,
}

// ── List subcommands ─────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum ListKind {
    /// List all units
    Units,
    /// List all cities
    Cities,
    /// List available production items for a city
    Production {
        #[arg(long)]
        city: String,
    },
    /// List available Great People
    GreatPeople,
    /// List active trade routes
    Routes,
    /// List governors and their assignments
    Governors,
    /// List buildings
    Buildings,
    /// List improvements
    Improvements,
}
