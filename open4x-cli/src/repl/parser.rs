//! Text command parser for the interactive REPL.
//!
//! Converts a single line of user input into a `ReplCommand` that the
//! session loop can dispatch.

use libciv::{CityId, UnitId};
use libhexgrid::coord::HexDir;

use crate::cli::ActionKind;

/// A parsed REPL command.
pub enum ReplCommand {
    /// Delegate to `dispatch_action` with the constructed `ActionKind`.
    Action(ActionKind),
    /// Read-only query; printed inline.
    Query(QueryKind),
    /// End the current turn (run AI, advance, save).
    EndTurn,
    /// Redraw the board.
    Board,
    /// Select a unit at the given (q, r) coordinate.
    SelectUnit(i32, i32),
    /// Move the selected unit in a hex direction.
    MoveDirection(HexDir),
    /// Select a unit by ID suffix and show its details.
    UnitSelect(String),
    /// Select a city by name or ID suffix and show its details.
    CitySelect(String),
    /// Save the game state to disk.
    Save,
    /// Print the help text.
    Help,
    /// Quit (save and exit).
    Quit,
    /// Unrecognized input.
    Unknown(String),
}

/// Read-only query variants.
pub enum QueryKind {
    Units,
    Cities,
    Yields,
    Techs,
    Civics,
    Scores,
    Diplomacy,
    Tile(i32, i32),
    BuildList,
    ResearchList,
    CivicsList,
}

/// Parse a line of REPL input into a `ReplCommand`.
///
/// `selected_unit` and `current_city` are used as implicit arguments for
/// commands that need a unit or city context.
pub fn parse_command(
    line: &str,
    selected_unit: Option<UnitId>,
    current_city: Option<CityId>,
) -> ReplCommand {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ReplCommand::Unknown(String::new());
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let cmd = parts[0].to_lowercase();

    // Helper: get the ULID string for the selected unit, or a placeholder
    // that will cause dispatch_action to produce a clear error.
    let unit_str = || -> String {
        selected_unit
            .map(|u| u.to_string())
            .unwrap_or_else(|| "no-unit-selected".to_string())
    };
    let city_str = || -> String {
        current_city
            .map(|c| c.to_string())
            .unwrap_or_else(|| "no-city-selected".to_string())
    };

    match cmd.as_str() {
        // ── Movement & Combat ───────────────────────────────────────────
        "move" | "m" | "go" => {
            if parts.len() >= 2 {
                // Try direction first: `move e`, `move sw`, etc.
                if let Some(dir) = parse_direction(parts[1]) {
                    return ReplCommand::MoveDirection(dir);
                }
            }
            if parts.len() >= 3
                && let (Ok(q), Ok(r)) = (parts[parts.len() - 2].parse::<i32>(), parts[parts.len() - 1].parse::<i32>())
            {
                let u = if parts.len() >= 4 { parts[1].to_string() } else { unit_str() };
                return ReplCommand::Action(ActionKind::Move { unit: u, to_q: q, to_r: r });
            }
            ReplCommand::Unknown("Usage: move <dir> | move [unit] <q> <r>".to_string())
        }

        // Bare direction shortcuts: `e`, `sw`, `nw`, etc.
        _ if parse_direction(cmd.as_str()).is_some() => {
            ReplCommand::MoveDirection(parse_direction(cmd.as_str()).unwrap())
        }

        "attack" | "atk" => {
            if parts.len() >= 2 {
                let (u, target) = if parts.len() >= 3 {
                    (parts[1].to_string(), parts[2].to_string())
                } else {
                    (unit_str(), parts[1].to_string())
                };
                return ReplCommand::Action(ActionKind::Attack { unit: u, target });
            }
            ReplCommand::Unknown("Usage: attack [unit] <target>".to_string())
        }

        "settle" | "found-city" | "found" => {
            let (u, name) = if parts.len() >= 3 {
                (parts[1].to_string(), parts[2..].join(" "))
            } else if parts.len() == 2 {
                (unit_str(), parts[1].to_string())
            } else {
                (unit_str(), "New City".to_string())
            };
            ReplCommand::Action(ActionKind::FoundCity { unit: u, name })
        }

        // ── Production ──────────────────────────────────────────────────
        "build" | "b" => {
            if parts.len() >= 2 {
                if parts[1].eq_ignore_ascii_case("list") || parts[1].eq_ignore_ascii_case("ls") {
                    return ReplCommand::Query(QueryKind::BuildList);
                }
                let item = parts[1..].join(" ");
                return ReplCommand::Action(ActionKind::Build { city: city_str(), item });
            }
            ReplCommand::Unknown("Usage: build <item> | build list".to_string())
        }

        "cancel-production" | "cancel" => {
            ReplCommand::Action(ActionKind::CancelProduction { city: city_str() })
        }

        // ── Research ────────────────────────────────────────────────────
        "research" | "res" => {
            if parts.len() >= 2 {
                if parts[1].eq_ignore_ascii_case("list") || parts[1].eq_ignore_ascii_case("ls") {
                    return ReplCommand::Query(QueryKind::ResearchList);
                }
                let tech = parts[1..].join(" ");
                return ReplCommand::Action(ActionKind::Research { tech });
            }
            ReplCommand::Unknown("Usage: research <tech> | research list".to_string())
        }

        "culture" | "civic" => {
            if parts.len() >= 2 {
                if parts[1].eq_ignore_ascii_case("list") || parts[1].eq_ignore_ascii_case("ls") {
                    return ReplCommand::Query(QueryKind::CivicsList);
                }
                let civic = parts[1..].join(" ");
                return ReplCommand::Action(ActionKind::StudyCivic { civic });
            }
            ReplCommand::Unknown("Usage: culture <civic> | culture list".to_string())
        }

        // ── Improvements & Roads ────────────────────────────────────────
        "improve" | "imp" => {
            if parts.len() >= 2 {
                let improvement = parts[1..].join(" ");
                // Use selected unit's coord -- dispatch_action needs explicit coords,
                // but we pass 0,0 as placeholders. The REPL will need to supply real
                // coords from the selected unit.
                return ReplCommand::Action(ActionKind::PlaceImprovement {
                    coord_q: 0,
                    coord_r: 0,
                    improvement,
                    builder: Some(unit_str()),
                });
            }
            ReplCommand::Unknown("Usage: improve <type>".to_string())
        }

        "road" => {
            if parts.len() >= 3
                && let (Ok(q), Ok(r)) = (parts[parts.len() - 2].parse::<i32>(), parts[parts.len() - 1].parse::<i32>())
            {
                let u = if parts.len() >= 4 { parts[1].to_string() } else { unit_str() };
                return ReplCommand::Action(ActionKind::PlaceRoad { unit: u, q, r });
            }
            ReplCommand::Unknown("Usage: road [unit] <q> <r>".to_string())
        }

        // ── Districts ───────────────────────────────────────────────────
        "district" | "dist" => {
            if parts.len() >= 4 {
                let district = parts[1].to_string();
                if let (Ok(q), Ok(r)) = (parts[2].parse::<i32>(), parts[3].parse::<i32>()) {
                    return ReplCommand::Action(ActionKind::PlaceDistrict {
                        city: city_str(),
                        district,
                        q,
                        r,
                    });
                }
            }
            ReplCommand::Unknown("Usage: district <type> <q> <r>".to_string())
        }

        // ── Diplomacy ───────────────────────────────────────────────────
        "declare-war" | "war" => {
            if parts.len() >= 2 {
                return ReplCommand::Action(ActionKind::DeclareWar { target: parts[1..].join(" ") });
            }
            ReplCommand::Unknown("Usage: declare-war <civ>".to_string())
        }

        "make-peace" | "peace" => {
            if parts.len() >= 2 {
                return ReplCommand::Action(ActionKind::MakePeace { target: parts[1..].join(" ") });
            }
            ReplCommand::Unknown("Usage: make-peace <civ>".to_string())
        }

        "form-alliance" | "alliance" => {
            if parts.len() >= 3 {
                return ReplCommand::Action(ActionKind::FormAlliance {
                    target: parts[1].to_string(),
                    alliance_type: parts[2].to_string(),
                });
            }
            ReplCommand::Unknown("Usage: form-alliance <civ> <type>".to_string())
        }

        // ── Government & Policy ─────────────────────────────────────────
        "assign-policy" | "policy" => {
            if parts.len() >= 2 {
                return ReplCommand::Action(ActionKind::AssignPolicy { policy: parts[1..].join(" ") });
            }
            ReplCommand::Unknown("Usage: assign-policy <policy>".to_string())
        }

        "adopt-government" | "gov" => {
            if parts.len() >= 2 {
                return ReplCommand::Action(ActionKind::AdoptGovernment { name: parts[1..].join(" ") });
            }
            ReplCommand::Unknown("Usage: adopt-government <name>".to_string())
        }

        // ── Trade ───────────────────────────────────────────────────────
        "trade" => {
            if parts.len() >= 3 {
                return ReplCommand::Action(ActionKind::EstablishTradeRoute {
                    unit: parts[1].to_string(),
                    dest_city: parts[2].to_string(),
                });
            } else if parts.len() == 2 {
                return ReplCommand::Action(ActionKind::EstablishTradeRoute {
                    unit: unit_str(),
                    dest_city: parts[1].to_string(),
                });
            }
            ReplCommand::Unknown("Usage: trade [unit] <dest_city>".to_string())
        }

        // ── Religion ────────────────────────────────────────────────────
        "found-pantheon" | "pantheon" => {
            if parts.len() >= 2 {
                return ReplCommand::Action(ActionKind::FoundPantheon { belief: parts[1].to_string() });
            }
            ReplCommand::Unknown("Usage: found-pantheon <belief>".to_string())
        }

        "found-religion" | "religion" => {
            if parts.len() >= 4 {
                let prophet = parts[1].to_string();
                let name = parts[2].to_string();
                let beliefs = parts[3..].iter().map(|s| s.to_string()).collect();
                return ReplCommand::Action(ActionKind::FoundReligion { prophet, name, beliefs });
            }
            ReplCommand::Unknown("Usage: found-religion <prophet> <name> <belief1> [belief2...]".to_string())
        }

        "spread" => {
            let u = if parts.len() >= 2 { parts[1].to_string() } else { unit_str() };
            ReplCommand::Action(ActionKind::SpreadReligion { unit: u })
        }

        // ── Great People ────────────────────────────────────────────────
        "recruit-gp" | "recruit" => {
            if parts.len() >= 2 {
                return ReplCommand::Action(ActionKind::RecruitGreatPerson {
                    person_type: parts[1].to_string(),
                });
            }
            ReplCommand::Unknown("Usage: recruit-gp <type>".to_string())
        }

        // ── Governors ───────────────────────────────────────────────────
        "assign-governor" | "governor" => {
            if parts.len() >= 3 {
                return ReplCommand::Action(ActionKind::AssignGovernor {
                    governor: parts[1].to_string(),
                    city: parts[2].to_string(),
                });
            }
            ReplCommand::Unknown("Usage: assign-governor <governor> <city>".to_string())
        }

        // ── Promotions ──────────────────────────────────────────────────
        "promote-unit" | "promote" => {
            if parts.len() >= 2 {
                let (u, promo) = if parts.len() >= 3 {
                    (parts[1].to_string(), parts[2..].join(" "))
                } else {
                    (unit_str(), parts[1].to_string())
                };
                return ReplCommand::Action(ActionKind::PromoteUnit { unit: u, promotion: promo });
            }
            ReplCommand::Unknown("Usage: promote [unit] <promotion>".to_string())
        }

        // ── Turn management ─────────────────────────────────────────────
        "end-turn" | "end" | "n" | "next" => ReplCommand::EndTurn,

        // ── Selection ───────────────────────────────────────────────────
        "select" | "sel" => {
            if parts.len() >= 3
                && let (Ok(q), Ok(r)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>())
            {
                return ReplCommand::SelectUnit(q, r);
            }
            ReplCommand::Unknown("Usage: select <q> <r>".to_string())
        }

        // ── Queries ─────────────────────────────────────────────────────
        "units" | "u" => ReplCommand::Query(QueryKind::Units),
        "cities" | "cs" => ReplCommand::Query(QueryKind::Cities),
        "yields" | "y" => ReplCommand::Query(QueryKind::Yields),
        "techs" => ReplCommand::Query(QueryKind::Techs),
        "civics" => ReplCommand::Query(QueryKind::Civics),
        "scores" | "sc" => ReplCommand::Query(QueryKind::Scores),
        "diplomacy" | "diplo" => ReplCommand::Query(QueryKind::Diplomacy),

        "tile" | "t" => {
            if parts.len() >= 3
                && let (Ok(q), Ok(r)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>())
            {
                return ReplCommand::Query(QueryKind::Tile(q, r));
            }
            ReplCommand::Unknown("Usage: tile <q> <r>".to_string())
        }

        "city" => {
            if parts.len() >= 2 {
                return ReplCommand::CitySelect(parts[1..].join(" "));
            }
            ReplCommand::Unknown("Usage: city <name_or_suffix>".to_string())
        }

        "unit" => {
            if parts.len() >= 2 {
                return ReplCommand::UnitSelect(parts[1].to_string());
            }
            ReplCommand::Unknown("Usage: unit <id>".to_string())
        }

        // ── Board / Save / Help / Quit ──────────────────────────────────
        "board" | "map" => ReplCommand::Board,
        "save" => ReplCommand::Save,
        "help" | "h" | "?" => ReplCommand::Help,
        "quit" | "exit" | "q" => ReplCommand::Quit,

        _ => ReplCommand::Unknown(format!("Unknown command: '{}'", parts[0])),
    }
}

/// Parse a direction string into a `HexDir`.
fn parse_direction(s: &str) -> Option<HexDir> {
    match s.to_lowercase().as_str() {
        "e" | "east" => Some(HexDir::E),
        "w" | "west" => Some(HexDir::W),
        "ne" | "northeast" => Some(HexDir::NE),
        "nw" | "northwest" => Some(HexDir::NW),
        "se" | "southeast" => Some(HexDir::SE),
        "sw" | "southwest" => Some(HexDir::SW),
        _ => None,
    }
}
