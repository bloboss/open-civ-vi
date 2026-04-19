//! Human-readable formatting for state deltas and queries.

use libciv::civ::district::BuiltinDistrict;
use libciv::game::diff::StateDelta;
use libciv::game::production_helpers::available_buildings_for_city;
use libciv::{all_scores, CityId, CivId, GameState, UnitId, UnitTypeId};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use super::short_ids::ShortIds;

/// Format a single delta into a one-line human-readable string.
/// Returns `None` for deltas that should be suppressed (noise).
pub fn format_delta(delta: &StateDelta, state: &GameState) -> Option<String> {
    match delta {
        StateDelta::TurnAdvanced { to, .. } => {
            Some(format!("--- Turn {to} ---"))
        }
        StateDelta::UnitMoved { to, .. } => {
            Some(format!("Unit moved to ({}, {})", to.q, to.r))
        }
        StateDelta::UnitCreated { coord, .. } => {
            Some(format!("New unit at ({}, {})", coord.q, coord.r))
        }
        StateDelta::UnitDestroyed { unit } => {
            Some(format!("Unit destroyed: {unit}"))
        }
        StateDelta::UnitAttacked { attacker_damage, defender_damage, .. } => {
            Some(format!("Combat: dealt {defender_damage} dmg, took {attacker_damage}"))
        }
        StateDelta::CityFounded { coord, .. } => {
            // Try to find the city name from state.
            let name = state.cities.iter()
                .find(|c| c.coord == *coord)
                .map(|c| c.name.as_str())
                .unwrap_or("?");
            Some(format!("City founded: {name} at ({}, {})", coord.q, coord.r))
        }
        StateDelta::PopulationGrew { city, new_population } => {
            let name = city_name(state, *city);
            Some(format!("{name}: population grew to {new_population}"))
        }
        StateDelta::GoldChanged { delta, .. } => {
            Some(format!("Gold: {delta:+}"))
        }
        StateDelta::TechResearched { tech, .. } => {
            Some(format!("Technology researched: {tech}"))
        }
        StateDelta::CivicCompleted { civic, .. } => {
            Some(format!("Civic completed: {civic}"))
        }
        StateDelta::BuildingCompleted { city, building } => {
            let name = city_name(state, *city);
            Some(format!("{name}: building completed: {building}"))
        }
        StateDelta::DistrictBuilt { city, district, coord } => {
            let name = city_name(state, *city);
            Some(format!("{name}: {district:?} built at ({}, {})", coord.q, coord.r))
        }
        StateDelta::WonderBuilt { wonder, city, .. } => {
            let name = city_name(state, *city);
            Some(format!("{name}: wonder completed: {wonder}"))
        }
        StateDelta::ProductionStarted { city, item } => {
            let name = city_name(state, *city);
            Some(format!("{name}: started producing {item}"))
        }
        StateDelta::ImprovementPlaced { coord, improvement } => {
            Some(format!("{improvement:?} built at ({}, {})", coord.q, coord.r))
        }
        StateDelta::RoadPlaced { coord, .. } => {
            Some(format!("Road built at ({}, {})", coord.q, coord.r))
        }
        StateDelta::ExperienceGained { amount, .. } => {
            Some(format!("+{amount} XP"))
        }
        StateDelta::UnitPromoted { promotion_name, .. } => {
            Some(format!("Unit promoted: {promotion_name}"))
        }
        StateDelta::VictoryAchieved { condition, civ } => {
            let civ_name = state.civilizations.iter()
                .find(|c| c.id == *civ)
                .map(|c| c.name)
                .unwrap_or("?");
            Some(format!("VICTORY! {civ_name} wins by {condition}!"))
        }
        StateDelta::DiplomacyChanged { civ_a, civ_b, new_status } => {
            let a = civ_display_name(state, *civ_a);
            let b = civ_display_name(state, *civ_b);
            Some(format!("Diplomacy: {a} <-> {b} now {new_status:?}"))
        }
        StateDelta::TradeRouteEstablished { origin, destination, .. } => {
            let o = city_name(state, *origin);
            let d = city_name(state, *destination);
            Some(format!("Trade route established: {o} -> {d}"))
        }
        StateDelta::TradeRouteExpired { .. } => {
            Some("Trade route expired".to_string())
        }
        StateDelta::ReligionFounded { name, .. } => {
            Some(format!("Religion founded: {name}"))
        }
        StateDelta::FaithChanged { delta, .. } => {
            Some(format!("Faith: {delta:+}"))
        }
        StateDelta::ChargesChanged { remaining, .. } => {
            Some(format!("Builder charges remaining: {remaining}"))
        }
        StateDelta::UnitHealed { new_health, .. } => {
            Some(format!("Unit healed to {new_health} HP"))
        }
        StateDelta::GreatPersonRecruited { person_type, .. } => {
            Some(format!("Great {person_type:?} recruited"))
        }
        StateDelta::GovernorAssigned { .. } => {
            Some("Governor assigned".to_string())
        }
        StateDelta::CityRevolted { city, .. } => {
            let name = city_name(state, *city);
            Some(format!("{name} revolted!"))
        }
        StateDelta::EraAdvanced { civ, new_era, era_age } => {
            let name = civ_display_name(state, *civ);
            Some(format!("{name} entered {new_era:?} ({era_age:?})"))
        }
        StateDelta::UnitEmbarked { coord, .. } => {
            Some(format!("Unit embarked at ({}, {})", coord.q, coord.r))
        }
        StateDelta::UnitDisembarked { coord, .. } => {
            Some(format!("Unit disembarked at ({}, {})", coord.q, coord.r))
        }
        StateDelta::EmbarkCoastUnlocked { .. } => {
            Some("Coast embarkation unlocked!".to_string())
        }
        StateDelta::EmbarkOceanUnlocked { .. } => {
            Some("Ocean embarkation unlocked!".to_string())
        }
        // Suppress noise.
        StateDelta::TilesRevealed { .. } => None,
        StateDelta::CitizenAssigned { .. } => None,
        StateDelta::TileClaimed { .. } => None,
        _ => None,
    }
}

/// Print all non-suppressed deltas from a diff.
pub fn print_deltas(diff: &libciv::GameStateDiff, state: &GameState) {
    for delta in &diff.deltas {
        if let Some(line) = format_delta(delta, state) {
            println!("  {line}");
        }
    }
}

// ── Query formatters ────────────────────────────────────────────────────────

/// Print a tabular list of owned units, followed by visible foreign units.
pub fn print_units(
    state: &GameState,
    civ_id: CivId,
    short_ids: &ShortIds<UnitId>,
    other_short_ids: &ShortIds<UnitId>,
) {
    // ── Our units ──────────────────────────────────────────────────────────
    let own: Vec<_> = state.units.iter().filter(|u| u.owner == civ_id).collect();
    if own.is_empty() {
        println!("  No units.");
    } else {
        println!(
            "  {:<28} {:<14} {:>8} {:>4}/{:<4} {:>3}",
            "ID", "Type", "Coord", "HP", "Max", "Mv"
        );
        println!("  {}", "-".repeat(70));
        for u in &own {
            let type_name = unit_type_name(state, u.unit_type);
            let id_display = short_ids.format_bold(u.id);
            let visible_len = short_ids.display_len(u.id);
            let pad = if visible_len < 28 {
                28 - visible_len
            } else {
                1
            };
            print!("  {id_display}{:pad$}", "");
            println!(
                "{:<14} ({:>3},{:>3}) {:>4}/{:<4} {:>3}",
                type_name, u.coord.q, u.coord.r, u.health, 100, u.movement_left
            );
        }
    }

    // ── Other visible units ────────────────────────────────────────────────
    let visible = state
        .civilizations
        .iter()
        .find(|c| c.id == civ_id)
        .map(|c| &c.visible_tiles);

    let others: Vec<_> = state
        .units
        .iter()
        .filter(|u| {
            u.owner != civ_id
                && visible.is_none_or(|v| v.contains(&u.coord))
        })
        .collect();

    if others.is_empty() {
        return;
    }

    println!();
    println!("  Other units:");
    println!(
        "  {:<28} {:<14} {:>8} {:>4}/{:<4} {:<16}",
        "ID", "Type", "Coord", "HP", "Max", "Owner"
    );
    println!("  {}", "-".repeat(82));
    for u in &others {
        let type_name = unit_type_name(state, u.unit_type);
        let owner = owner_label(state, u.owner);
        let id_display = other_short_ids.format_bold(u.id);
        let visible_len = other_short_ids.display_len(u.id);
        let pad = if visible_len < 28 {
            28 - visible_len
        } else {
            1
        };
        print!("  {id_display}{:pad$}", "");
        println!(
            "{:<14} ({:>3},{:>3}) {:>4}/{:<4} {:<16}",
            type_name, u.coord.q, u.coord.r, u.health, 100, owner
        );
    }
}

/// Print a tabular list of cities owned by the given civ, with districts
/// indented under each city.
pub fn print_cities(state: &GameState, civ_id: CivId, short_ids: &ShortIds<CityId>) {
    let cities: Vec<_> = state.cities.iter().filter(|c| c.owner == civ_id).collect();
    if cities.is_empty() {
        println!("  No cities.");
        return;
    }
    println!(
        "  {:<28} {:<16} {:>8} {:>4} {:<20}",
        "ID", "Name", "Coord", "Pop", "Producing"
    );
    println!("  {}", "-".repeat(80));
    for c in &cities {
        let id_display = short_ids.format_bold(c.id);
        let visible_len = short_ids.display_len(c.id);
        let pad = if visible_len < 28 {
            28 - visible_len
        } else {
            1
        };
        let producing = c
            .production_queue
            .front()
            .map(|p| format!("{p:?}"))
            .unwrap_or_else(|| "-".to_string());
        let prod_display = if producing.len() > 20 {
            format!("{}...", &producing[..17])
        } else {
            producing
        };
        print!("  {id_display}{:pad$}", "");
        println!(
            "{:<16} ({:>3},{:>3}) {:>4} {:<20}",
            c.name, c.coord.q, c.coord.r, c.population, prod_display
        );

        // Show districts indented under the city.
        for dist in &c.districts {
            let placed = state.placed_districts.iter()
                .find(|pd| pd.city_id == c.id && pd.district_type == *dist);
            let coord_str = placed
                .map(|pd| format!("({:>3},{:>3})", pd.coord.q, pd.coord.r))
                .unwrap_or_else(|| "       -".to_string());
            let building_count = placed.map(|pd| pd.buildings.len()).unwrap_or(0);
            let bldg_str = if building_count > 0 {
                format!("{building_count} bldg")
            } else {
                String::new()
            };
            println!(
                "    {:<26} {:<16} {:>8}      {:<20}",
                "", dist.name(), coord_str, bldg_str
            );
        }
    }
}

/// Print available buildings that can be produced in the given city.
///
/// When `district_filter` is `Some`, only buildings belonging to that district
/// are shown (or "City Center" buildings when `CityCenter` is selected).
pub fn print_available_buildings(
    state: &GameState,
    civ_id: CivId,
    city_id: CityId,
    district_filter: Option<BuiltinDistrict>,
) {
    let city = match state.cities.iter().find(|c| c.id == city_id) {
        Some(c) => c,
        None => {
            println!("  City not found.");
            return;
        }
    };

    let mut available = available_buildings_for_city(state, civ_id, city_id);

    // Filter by selected district.
    if let Some(dist) = district_filter {
        let dist_name = dist.name();
        available.retain(|d| {
            d.requires_district
                .is_some_and(|r| r.eq_ignore_ascii_case(dist_name))
        });
    }

    if available.is_empty() {
        let scope = district_filter
            .map(|d| format!("{} in {}", d.name(), city.name))
            .unwrap_or_else(|| city.name.clone());
        println!("  No buildings available for {scope}.");
        return;
    }

    let header = district_filter
        .map(|d| format!("Available buildings for {} ({}):", city.name, d.name()))
        .unwrap_or_else(|| format!("Available buildings for {}:", city.name));
    println!("  {header}");
    println!(
        "  {:<24} {:>6} {:>6}  District",
        "Name", "Cost", "Maint"
    );
    println!("  {}", "-".repeat(60));
    for d in &available {
        let district = d.requires_district.unwrap_or("-");
        println!(
            "  {:<24} {:>6} {:>6}  {}",
            d.name, d.cost, d.maintenance, district
        );
    }
}

/// Print districts available for placement in the given city.
///
/// A district is available if:
/// 1. The city does not already have that district type.
/// 2. The required tech (if any) has been researched.
/// 3. The required civic (if any) has been completed.
/// 4. It is not CityCenter (implicit).
pub fn print_available_districts(state: &GameState, civ_id: CivId, city_id: CityId) {
    let city = match state.cities.iter().find(|c| c.id == city_id) {
        Some(c) => c,
        None => {
            println!("  City not found.");
            return;
        }
    };
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => {
            println!("  Civilization not found.");
            return;
        }
    };

    let all_districts = [
        BuiltinDistrict::Campus,
        BuiltinDistrict::TheaterSquare,
        BuiltinDistrict::CommercialHub,
        BuiltinDistrict::Harbor,
        BuiltinDistrict::HolySite,
        BuiltinDistrict::Encampment,
        BuiltinDistrict::IndustrialZone,
        BuiltinDistrict::EntertainmentComplex,
        BuiltinDistrict::WaterPark,
        BuiltinDistrict::Aqueduct,
        BuiltinDistrict::Dam,
        BuiltinDistrict::Canal,
        BuiltinDistrict::Aerodrome,
        BuiltinDistrict::Neighborhood,
        BuiltinDistrict::Spaceport,
        BuiltinDistrict::Lavra,
        BuiltinDistrict::Mbanza,
        BuiltinDistrict::StreetCarnival,
        BuiltinDistrict::RoyalNavyDockyard,
    ];

    let mut available: Vec<(BuiltinDistrict, &str)> = Vec::new();
    for &dist in &all_districts {
        // Skip if already built.
        if city.districts.contains(&dist) {
            continue;
        }
        let reqs = dist.requirements(&state.tech_refs, &state.civic_refs);
        // Check tech prerequisite.
        if let Some(tid) = reqs.required_tech
            && !civ.researched_techs.contains(&tid)
        {
            continue;
        }
        // Check civic prerequisite.
        if let Some(cid) = reqs.required_civic
            && !civ.completed_civics.contains(&cid)
        {
            continue;
        }
        // Terrain note for display.
        let terrain_note = if reqs.requires_water {
            "water"
        } else if reqs.requires_land {
            "land"
        } else {
            "-"
        };
        available.push((dist, terrain_note));
    }

    if available.is_empty() {
        println!("  No districts available for {}.", city.name);
        return;
    }

    println!("  Available districts for {}:", city.name);
    println!("  {:<24} {:>6}  Terrain", "Name", "Cost");
    println!("  {}", "-".repeat(40));
    for (dist, terrain) in &available {
        println!("  {:<24} {:>6}  {}", dist.name(), dist.base_cost(), terrain);
    }
}

/// Print yield summary for a civ.
pub fn print_yields(state: &GameState, civ_id: CivId) {
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => { println!("  Civilization not found."); return; }
    };
    println!("  Gold: {}", civ.gold);
    println!("  Faith: {}", civ.faith);
    let city_count = state.cities.iter().filter(|c| c.owner == civ_id).count();
    let pop: u32 = state.cities.iter().filter(|c| c.owner == civ_id).map(|c| c.population).sum();
    println!("  Cities: {city_count}  Population: {pop}");
}

/// Print researched techs and current research queue.
pub fn print_techs(state: &GameState, civ_id: CivId) {
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => { println!("  Civilization not found."); return; }
    };
    println!("  Researched:");
    if civ.researched_techs.is_empty() {
        println!("    (none)");
    } else {
        for tid in &civ.researched_techs {
            let name = state.tech_tree.nodes.values()
                .find(|n| n.id == *tid)
                .map(|n| n.name)
                .unwrap_or("?");
            println!("    - {name}");
        }
    }
    println!("  Research queue:");
    if civ.research_queue.is_empty() {
        println!("    (empty)");
    } else {
        for tp in &civ.research_queue {
            let name = state.tech_tree.nodes.values()
                .find(|n| n.id == tp.tech_id)
                .map(|n| n.name)
                .unwrap_or("?");
            println!("    - {name} ({}/???)", tp.progress);
        }
    }
}

/// Print completed civics and current civic in progress.
pub fn print_civics(state: &GameState, civ_id: CivId) {
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => { println!("  Civilization not found."); return; }
    };
    println!("  Completed civics:");
    if civ.completed_civics.is_empty() {
        println!("    (none)");
    } else {
        for cid in &civ.completed_civics {
            let name = state.civic_tree.nodes.values()
                .find(|n| n.id == *cid)
                .map(|n| n.name)
                .unwrap_or("?");
            println!("    - {name}");
        }
    }
    println!("  Current civic:");
    match &civ.civic_in_progress {
        Some(cp) => {
            let name = state.civic_tree.nodes.values()
                .find(|n| n.id == cp.civic_id)
                .map(|n| n.name)
                .unwrap_or("?");
            println!("    {name} ({}/???)", cp.progress);
        }
        None => println!("    (none)"),
    }
}

/// Print available techs that can be researched by the given civ.
///
/// A tech is available if:
/// 1. It has not already been researched.
/// 2. It is not already in the research queue.
/// 3. All prerequisites have been researched.
pub fn print_available_techs(state: &GameState, civ_id: CivId) {
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => {
            println!("  Civilization not found.");
            return;
        }
    };

    let queued_ids: Vec<_> = civ.research_queue.iter().map(|tp| tp.tech_id).collect();

    let mut available: Vec<_> = state
        .tech_tree
        .nodes
        .values()
        .filter(|n| {
            !civ.researched_techs.contains(&n.id)
                && !queued_ids.contains(&n.id)
                && state
                    .tech_tree
                    .prerequisites_met(n.id, &civ.researched_techs)
        })
        .collect();
    available.sort_by_key(|n| n.cost);

    if available.is_empty() {
        println!("  No techs available for research.");
        return;
    }

    println!("  Available techs:");
    println!("  {:<24} {:>6}  Prerequisites", "Name", "Cost");
    println!("  {}", "-".repeat(60));
    for n in &available {
        let prereqs: Vec<&str> = n
            .prerequisites
            .iter()
            .filter_map(|pid| state.tech_tree.get(*pid).map(|p| p.name))
            .collect();
        let prereq_str = if prereqs.is_empty() {
            "-".to_string()
        } else {
            prereqs.join(", ")
        };
        println!("  {:<24} {:>6}  {}", n.name, n.cost, prereq_str);
    }
}

/// Print available civics that can be researched by the given civ.
///
/// A civic is available if:
/// 1. It has not already been completed.
/// 2. It is not the current civic in progress.
/// 3. All prerequisites have been completed.
pub fn print_available_civics(state: &GameState, civ_id: CivId) {
    let civ = match state.civilizations.iter().find(|c| c.id == civ_id) {
        Some(c) => c,
        None => {
            println!("  Civilization not found.");
            return;
        }
    };

    let in_progress_id = civ.civic_in_progress.as_ref().map(|cp| cp.civic_id);

    let mut available: Vec<_> = state
        .civic_tree
        .nodes
        .values()
        .filter(|n| {
            !civ.completed_civics.contains(&n.id)
                && in_progress_id != Some(n.id)
                && state
                    .civic_tree
                    .prerequisites_met(n.id, &civ.completed_civics)
        })
        .collect();
    available.sort_by_key(|n| n.cost);

    if available.is_empty() {
        println!("  No civics available for research.");
        return;
    }

    println!("  Available civics:");
    println!("  {:<24} {:>6}  Prerequisites", "Name", "Cost");
    println!("  {}", "-".repeat(60));
    for n in &available {
        let prereqs: Vec<&str> = n
            .prerequisites
            .iter()
            .filter_map(|pid| state.civic_tree.get(*pid).map(|p| p.name))
            .collect();
        let prereq_str = if prereqs.is_empty() {
            "-".to_string()
        } else {
            prereqs.join(", ")
        };
        println!("  {:<24} {:>6}  {}", n.name, n.cost, prereq_str);
    }
}

/// Print a leaderboard of all civ scores.
pub fn print_scores(state: &GameState) {
    let mut scores = all_scores(state);
    scores.sort_by(|a, b| b.1.cmp(&a.1));
    println!("  {:<20} {:>6}", "Civilization", "Score");
    println!("  {}", "-".repeat(28));
    for (cid, score) in &scores {
        let name = state.civilizations.iter()
            .find(|c| c.id == *cid)
            .map(|c| c.name)
            .unwrap_or("?");
        println!("  {:<20} {:>6}", name, score);
    }
}

/// Print diplomatic relations for a civ.
pub fn print_diplomacy(state: &GameState, civ_id: CivId) {
    let rels: Vec<_> = state.diplomatic_relations.iter()
        .filter(|r| r.civ_a == civ_id || r.civ_b == civ_id)
        .collect();
    if rels.is_empty() {
        println!("  No diplomatic relations.");
        return;
    }
    for r in &rels {
        let other = if r.civ_a == civ_id { r.civ_b } else { r.civ_a };
        let other_name = civ_display_name(state, other);
        println!("  {other_name}: {:?}", r.status);
    }
}

/// Print tile details at a coordinate.
pub fn print_tile(state: &GameState, coord: HexCoord) {
    match state.board.tile(coord) {
        Some(tile) => {
            println!("  Tile ({}, {}):", coord.q, coord.r);
            println!("    Terrain: {:?}", tile.terrain);
            if tile.hills {
                println!("    Hills: yes");
            }
            if let Some(f) = tile.feature {
                println!("    Feature: {f:?}");
            }
            if let Some(r) = tile.resource {
                println!("    Resource: {r:?}");
            }
            if let Some(imp) = tile.improvement {
                println!("    Improvement: {imp:?}");
            }
            if tile.road.is_some() {
                println!("    Road: yes");
            }
            if let Some(owner) = tile.owner {
                let name = civ_display_name(state, owner);
                println!("    Owner: {name}");
            }
            // Units at this coord.
            let units_here: Vec<_> = state.units.iter()
                .filter(|u| u.coord == coord)
                .collect();
            if !units_here.is_empty() {
                println!("    Units:");
                for u in &units_here {
                    let type_name = state.unit_type_defs.iter()
                        .find(|d| d.id == u.unit_type)
                        .map(|d| d.name)
                        .unwrap_or("?");
                    let owner = civ_display_name(state, u.owner);
                    println!("      {type_name} ({owner}) HP:{}", u.health);
                }
            }
        }
        None => println!("  No tile at ({}, {}).", coord.q, coord.r),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn city_name(state: &GameState, city_id: libciv::CityId) -> String {
    state.cities.iter()
        .find(|c| c.id == city_id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "?".to_string())
}

fn civ_display_name(state: &GameState, civ_id: CivId) -> &'static str {
    state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .map(|c| c.name)
        .unwrap_or("?")
}

fn unit_type_name(state: &GameState, unit_type: UnitTypeId) -> &'static str {
    state
        .unit_type_defs
        .iter()
        .find(|d| d.id == unit_type)
        .map(|d| d.name)
        .unwrap_or("?")
}

/// Classify a unit's owner for display: civ name, "City-State", or "Barbarian".
fn owner_label(state: &GameState, owner: CivId) -> String {
    if state.barbarian_civ == Some(owner) {
        return "Barbarian".to_string();
    }
    if state.city_state_by_civ(owner).is_some() {
        let name = civ_display_name(state, owner);
        return format!("{name} (CS)");
    }
    civ_display_name(state, owner).to_string()
}
