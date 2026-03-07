use clap::{Parser, Subcommand};
use libciv::{GameState, TurnEngine, DefaultRulesEngine};

#[derive(Parser, Debug)]
#[command(name = "civsim", about = "Open Civ VI simulation CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start a new game
    New {
        /// Random seed for deterministic generation
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        /// Map width in tiles
        #[arg(short = 'W', long, default_value_t = 80)]
        width: u32,
        /// Map height in tiles
        #[arg(short = 'H', long, default_value_t = 50)]
        height: u32,
    },
    /// Run the simulation for N turns
    Run {
        /// Number of turns to simulate
        #[arg(short, long, default_value_t = 1)]
        turns: u32,
        /// Seed of the game to run
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::New { seed, width, height } => {
            let state = GameState::new(seed, width, height);
            println!(
                "New game created: {}×{} map, seed={}, turn={}",
                state.board.width,
                state.board.height,
                seed,
                state.turn,
            );
        }
        Command::Run { turns, seed } => {
            let mut state = GameState::new(seed, 80, 50);
            let engine = TurnEngine::new();
            let rules = DefaultRulesEngine;
            println!("Simulating {turns} turns (seed={seed})…");
            for t in 1..=turns {
                // Phase 2: process_turn will apply rules; for now it's a stub
                let _diff = engine.process_turn(&mut state, &rules);
                println!("  Turn {t} complete.");
            }
        }
    }
}
