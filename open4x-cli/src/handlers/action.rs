//! Handler for the `action` CLI command.
//!
//! Dispatches an `ActionKind` to the corresponding `RulesEngine` method,
//! saves the updated state, and prints the result as JSON.

use std::path::Path;

use libciv::civ::diplomacy::AllianceType;
use libciv::civ::district::BuiltinDistrict;
use libciv::game::rules::FaithPurchaseItem;
use libciv::game::visibility::recalculate_visibility;
use libciv::world::improvement::BuiltinImprovement;
use libciv::world::road::BuiltinRoad;
use libciv::{
    apply_diff, BarbarianCampId, BeliefId, CityId, DefaultRulesEngine, GameStateDiff,
    GreatPersonId, GreatPersonType, RulesEngine, UnitId,
};
use libhexgrid::coord::HexCoord;

use crate::cli::ActionKind;
use crate::output::{self, ActionResult};
use crate::state_io;

use super::{find_civ_by_name, parse_ulid, validate_human};

/// Execute a game action for `player` and save the result.
pub fn handle_action(
    game_file: &Path,
    player: &str,
    action: &ActionKind,
) -> Result<(), String> {
    let mut state = state_io::load_game_file(game_file)?;
    let civ_id = find_civ_by_name(&state, player)?;
    validate_human(&state, player)?;

    if state.turn_done.contains(player) {
        return Err(format!("'{player}' has already ended their turn"));
    }

    let rules = DefaultRulesEngine;

    let result: Result<GameStateDiff, String> = dispatch_action(&rules, &mut state, civ_id, action);

    match result {
        Ok(diff) => {
            // Apply diffs that need explicit application (e.g. UnitMoved).
            apply_diff(&mut state, &diff);
            recalculate_visibility(&mut state, civ_id);
            state_io::save_game_file(game_file, &state)?;
            output::print_result(&ActionResult::ok(state.turn, diff));
        }
        Err(e) => {
            output::print_result(&ActionResult::err(state.turn, e.clone()));
            return Err(e);
        }
    }

    Ok(())
}

// ── ID parsing helpers ──────────────────────────────────────────────────────

fn parse_unit_id(s: &str) -> Result<UnitId, String> {
    parse_ulid(s).map(UnitId::from_ulid)
}

fn parse_city_id(s: &str) -> Result<CityId, String> {
    parse_ulid(s).map(CityId::from_ulid)
}

fn parse_great_person_id(s: &str) -> Result<GreatPersonId, String> {
    parse_ulid(s).map(GreatPersonId::from_ulid)
}

fn parse_belief_id(s: &str) -> Result<BeliefId, String> {
    parse_ulid(s).map(BeliefId::from_ulid)
}


fn parse_improvement(s: &str) -> Result<BuiltinImprovement, String> {
    match s.to_lowercase().as_str() {
        "farm" => Ok(BuiltinImprovement::Farm),
        "mine" => Ok(BuiltinImprovement::Mine),
        "lumbermill" | "lumber_mill" | "lumber mill" => Ok(BuiltinImprovement::LumberMill),
        "tradingpost" | "trading_post" | "trading post" => Ok(BuiltinImprovement::TradingPost),
        "fort" => Ok(BuiltinImprovement::Fort),
        "airstrip" => Ok(BuiltinImprovement::Airstrip),
        "missilesilo" | "missile_silo" | "missile silo" => Ok(BuiltinImprovement::MissileSilo),
        "quarry" => Ok(BuiltinImprovement::Quarry),
        "plantation" => Ok(BuiltinImprovement::Plantation),
        "camp" => Ok(BuiltinImprovement::Camp),
        "fishingboats" | "fishing_boats" | "fishing boats" => Ok(BuiltinImprovement::FishingBoats),
        "pasture" => Ok(BuiltinImprovement::Pasture),
        "sphinx" => Ok(BuiltinImprovement::Sphinx),
        "stepwell" => Ok(BuiltinImprovement::Stepwell),
        "oilwell" | "oil_well" | "oil well" => Ok(BuiltinImprovement::OilWell),
        "offshoreoilrig" | "offshore_oil_rig" | "offshore oil rig" => {
            Ok(BuiltinImprovement::OffshoreOilRig)
        }
        "beachresort" | "beach_resort" | "beach resort" => Ok(BuiltinImprovement::BeachResort),
        "chateau" => Ok(BuiltinImprovement::Chateau),
        "colossalhead" | "colossal_head" | "colossal head" => Ok(BuiltinImprovement::ColossalHead),
        "greatwall" | "great_wall" | "great wall" => Ok(BuiltinImprovement::GreatWall),
        "kurgan" => Ok(BuiltinImprovement::Kurgan),
        _ => Err(format!("unknown improvement '{s}'")),
    }
}

pub(crate) fn parse_district(s: &str) -> Result<BuiltinDistrict, String> {
    match s.to_lowercase().replace(' ', "").as_str() {
        "campus" => Ok(BuiltinDistrict::Campus),
        "theatersquare" | "theater_square" => Ok(BuiltinDistrict::TheaterSquare),
        "commercialhub" | "commercial_hub" => Ok(BuiltinDistrict::CommercialHub),
        "harbor" => Ok(BuiltinDistrict::Harbor),
        "holysite" | "holy_site" => Ok(BuiltinDistrict::HolySite),
        "encampment" => Ok(BuiltinDistrict::Encampment),
        "industrialzone" | "industrial_zone" => Ok(BuiltinDistrict::IndustrialZone),
        "entertainmentcomplex" | "entertainment_complex" => {
            Ok(BuiltinDistrict::EntertainmentComplex)
        }
        "waterpark" | "water_park" => Ok(BuiltinDistrict::WaterPark),
        "aqueduct" => Ok(BuiltinDistrict::Aqueduct),
        "dam" => Ok(BuiltinDistrict::Dam),
        "canal" => Ok(BuiltinDistrict::Canal),
        "aerodrome" => Ok(BuiltinDistrict::Aerodrome),
        "neighborhood" => Ok(BuiltinDistrict::Neighborhood),
        "spaceport" => Ok(BuiltinDistrict::Spaceport),
        "citycenter" | "city_center" => Ok(BuiltinDistrict::CityCenter),
        "lavra" => Ok(BuiltinDistrict::Lavra),
        "mbanza" => Ok(BuiltinDistrict::Mbanza),
        "streetcarnival" | "street_carnival" => Ok(BuiltinDistrict::StreetCarnival),
        "royalnavydockyard" | "royal_navy_dockyard" => Ok(BuiltinDistrict::RoyalNavyDockyard),
        _ => Err(format!("unknown district '{s}'")),
    }
}

fn parse_alliance_type(s: &str) -> Result<AllianceType, String> {
    match s.to_lowercase().as_str() {
        "research" => Ok(AllianceType::Research),
        "military" => Ok(AllianceType::Military),
        "economic" => Ok(AllianceType::Economic),
        "cultural" => Ok(AllianceType::Cultural),
        "religious" => Ok(AllianceType::Religious),
        _ => Err(format!("unknown alliance type '{s}'")),
    }
}

fn parse_great_person_type(s: &str) -> Result<GreatPersonType, String> {
    match s.to_lowercase().as_str() {
        "general" => Ok(GreatPersonType::General),
        "admiral" => Ok(GreatPersonType::Admiral),
        "engineer" => Ok(GreatPersonType::Engineer),
        "merchant" => Ok(GreatPersonType::Merchant),
        "musician" => Ok(GreatPersonType::Musician),
        "artist" => Ok(GreatPersonType::Artist),
        "writer" => Ok(GreatPersonType::Writer),
        "prophet" => Ok(GreatPersonType::Prophet),
        "scientist" => Ok(GreatPersonType::Scientist),
        _ => Err(format!("unknown great person type '{s}'")),
    }
}

// ── Action dispatch ─────────────────────────────────────────────────────────

pub(crate) fn dispatch_action(
    rules: &DefaultRulesEngine,
    state: &mut libciv::GameState,
    civ_id: libciv::CivId,
    action: &ActionKind,
) -> Result<GameStateDiff, String> {
    match action {
        // ── Movement & Combat ───────────────────────────────────────────
        ActionKind::Move { unit, to_q, to_r } => {
            let uid = parse_unit_id(unit)?;
            let to = HexCoord::from_qr(*to_q, *to_r);
            rules.move_unit(state, uid, to).map_err(|e| format!("{e:?}"))
        }
        ActionKind::Attack { unit, target } => {
            let uid = parse_unit_id(unit)?;
            let tid = parse_unit_id(target)?;
            rules.attack(state, uid, tid).map_err(|e| format!("{e:?}"))
        }
        ActionKind::CityBombard { city, target } => {
            let cid = parse_city_id(city)?;
            let tid = parse_unit_id(target)?;
            rules.city_bombard(state, cid, tid).map_err(|e| format!("{e:?}"))
        }
        ActionKind::TheologicalCombat { attacker, defender } => {
            let a = parse_unit_id(attacker)?;
            let d = parse_unit_id(defender)?;
            rules.theological_combat(state, a, d).map_err(|e| format!("{e:?}"))
        }
        ActionKind::PromoteUnit { unit, promotion } => {
            let uid = parse_unit_id(unit)?;
            rules.promote_unit(state, uid, promotion).map_err(|e| format!("{e:?}"))
        }
        ActionKind::RockBandPerform { unit } => {
            let uid = parse_unit_id(unit)?;
            rules.rock_band_perform(state, uid).map_err(|e| format!("{e:?}"))
        }

        // ── City & Production ───────────────────────────────────────────
        ActionKind::FoundCity { unit, name } => {
            let uid = parse_unit_id(unit)?;
            rules.found_city(state, uid, name.clone()).map_err(|e| format!("{e:?}"))
        }
        ActionKind::Build { city, item } => {
            let cid = parse_city_id(city)?;
            // Resolve the production item by name from unit_type_defs or building_defs.
            let prod_item = resolve_production_item(state, item)?;
            let city_obj = state
                .cities
                .iter_mut()
                .find(|c| c.id == cid)
                .ok_or("city not found")?;
            city_obj.production_queue.push_back(prod_item);
            Ok(GameStateDiff { deltas: Vec::new() })
        }
        ActionKind::CancelProduction { city } => {
            let cid = parse_city_id(city)?;
            let city_obj = state
                .cities
                .iter_mut()
                .find(|c| c.id == cid)
                .ok_or("city not found")?;
            city_obj.production_queue.pop_front();
            city_obj.production_stored = 0;
            Ok(GameStateDiff { deltas: Vec::new() })
        }
        ActionKind::PlaceDistrict {
            city,
            district,
            q,
            r,
        } => {
            let cid = parse_city_id(city)?;
            let dist = parse_district(district)?;
            let coord = HexCoord::from_qr(*q, *r);
            rules
                .place_district(state, cid, dist, coord)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::PlaceImprovement {
            coord_q,
            coord_r,
            improvement,
            builder,
        } => {
            let coord = HexCoord::from_qr(*coord_q, *coord_r);
            let imp = parse_improvement(improvement)?;
            let builder_id = builder
                .as_ref()
                .map(|b| parse_unit_id(b))
                .transpose()?;
            rules
                .place_improvement(state, civ_id, coord, imp, builder_id)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::PlaceRoad { unit, q, r } => {
            let uid = parse_unit_id(unit)?;
            let coord = HexCoord::from_qr(*q, *r);
            rules
                .place_road(state, uid, coord, BuiltinRoad::Ancient(libciv::world::road::AncientRoad))
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::AssignCitizen { city, q, r, lock } => {
            let cid = parse_city_id(city)?;
            let tile = HexCoord::from_qr(*q, *r);
            rules
                .assign_citizen(state, cid, tile, *lock)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::UnassignCitizen { city, q, r } => {
            let cid = parse_city_id(city)?;
            let tile = HexCoord::from_qr(*q, *r);
            // Unassign by removing from worked_tiles directly.
            let city_obj = state
                .cities
                .iter_mut()
                .find(|c| c.id == cid)
                .ok_or("city not found")?;
            city_obj.worked_tiles.retain(|t| *t != tile);
            city_obj.locked_tiles.retain(|t| *t != tile);
            Ok(GameStateDiff { deltas: Vec::new() })
        }
        ActionKind::ClaimTile { city, q, r, force } => {
            let cid = parse_city_id(city)?;
            let coord = HexCoord::from_qr(*q, *r);
            rules
                .claim_tile(state, cid, coord, *force)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::ReassignTile {
            from_city,
            to_city,
            q,
            r,
        } => {
            let from = parse_city_id(from_city)?;
            let to = parse_city_id(to_city)?;
            let coord = HexCoord::from_qr(*q, *r);
            rules
                .reassign_tile(state, from, to, coord)
                .map_err(|e| format!("{e:?}"))
        }

        // ── Tech & Civics ───────────────────────────────────────────────
        ActionKind::Research { tech } => {
            // Queue a tech for research by name.
            let tech_id = state
                .tech_tree
                .nodes
                .values()
                .find(|n| n.name == tech.as_str())
                .map(|n| n.id)
                .ok_or_else(|| format!("unknown tech '{tech}'"))?;
            let civ = state
                .civilizations
                .iter_mut()
                .find(|c| c.id == civ_id)
                .ok_or("civ not found")?;
            civ.research_queue.push_back(libciv::civ::TechProgress {
                tech_id,
                progress: 0,
                boosted: false,
            });
            Ok(GameStateDiff { deltas: Vec::new() })
        }
        ActionKind::StudyCivic { civic } => {
            // Set the current civic to research by name.
            let civic_id = state
                .civic_tree
                .nodes
                .values()
                .find(|n| n.name == civic.as_str())
                .map(|n| n.id)
                .ok_or_else(|| format!("unknown civic '{civic}'"))?;
            let civ = state
                .civilizations
                .iter_mut()
                .find(|c| c.id == civ_id)
                .ok_or("civ not found")?;
            civ.civic_in_progress = Some(libciv::civ::CivicProgress {
                civic_id,
                progress: 0,
                inspired: false,
            });
            Ok(GameStateDiff { deltas: Vec::new() })
        }
        ActionKind::AdoptGovernment { name } => {
            // Find government by name and adopt it.
            let gov_id = state
                .governments
                .iter()
                .find(|g| g.name == name.as_str())
                .map(|g| g.id)
                .ok_or_else(|| format!("unknown government '{name}'"))?;
            let civ = state
                .civilizations
                .iter_mut()
                .find(|c| c.id == civ_id)
                .ok_or("civ not found")?;
            civ.current_government = Some(gov_id);
            Ok(GameStateDiff { deltas: Vec::new() })
        }
        ActionKind::AssignPolicy { policy } => {
            let pol_id = state
                .policies
                .iter()
                .find(|p| p.name == policy.as_str())
                .map(|p| p.id)
                .ok_or_else(|| format!("unknown policy '{policy}'"))?;
            rules
                .assign_policy(state, civ_id, pol_id)
                .map_err(|e| format!("{e:?}"))
        }

        // ── Diplomacy ───────────────────────────────────────────────────
        ActionKind::DeclareWar { target } => {
            let target_id = find_civ_by_name(state, target)?;
            rules
                .declare_war(state, civ_id, target_id)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::MakePeace { target } => {
            let target_id = find_civ_by_name(state, target)?;
            rules
                .make_peace(state, civ_id, target_id)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::FormAlliance {
            target,
            alliance_type,
        } => {
            let target_id = find_civ_by_name(state, target)?;
            let at = parse_alliance_type(alliance_type)?;
            rules
                .form_alliance(state, civ_id, target_id, at)
                .map_err(|e| format!("{e:?}"))
        }

        // ── Trade ───────────────────────────────────────────────────────
        ActionKind::AssignTradeRoute { unit, dest_city } => {
            let uid = parse_unit_id(unit)?;
            let dest = parse_city_id(dest_city)?;
            rules
                .assign_trade_route(state, uid, dest)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::EstablishTradeRoute { unit, dest_city } => {
            let uid = parse_unit_id(unit)?;
            let dest = parse_city_id(dest_city)?;
            rules
                .establish_trade_route(state, uid, dest)
                .map_err(|e| format!("{e:?}"))
        }

        // ── Religion ────────────────────────────────────────────────────
        ActionKind::FoundPantheon { belief } => {
            let bid = parse_belief_id(belief)?;
            rules
                .found_pantheon(state, civ_id, bid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::FoundReligion {
            prophet,
            name,
            beliefs,
        } => {
            let pid = parse_unit_id(prophet)?;
            let bids: Result<Vec<BeliefId>, String> =
                beliefs.iter().map(|b| parse_belief_id(b)).collect();
            rules
                .found_religion(state, pid, name.clone(), bids?)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::SpreadReligion { unit } => {
            let uid = parse_unit_id(unit)?;
            rules
                .spread_religion(state, uid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::EvangelizeBelief { apostle, belief } => {
            let uid = parse_unit_id(apostle)?;
            let bid = parse_belief_id(belief)?;
            rules
                .evangelize_belief(state, uid, bid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::LaunchInquisition { apostle } => {
            let uid = parse_unit_id(apostle)?;
            rules
                .launch_inquisition(state, uid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::RemoveHeresy { inquisitor } => {
            let uid = parse_unit_id(inquisitor)?;
            rules
                .remove_heresy(state, uid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::GuruHeal { guru } => {
            let uid = parse_unit_id(guru)?;
            rules.guru_heal(state, uid).map_err(|e| format!("{e:?}"))
        }
        ActionKind::PurchaseWithFaith { city, item } => {
            let cid = parse_city_id(city)?;
            // Parse as unit name or worship building name.
            let faith_item = resolve_faith_purchase_item(item)?;
            rules
                .purchase_with_faith(state, civ_id, cid, faith_item)
                .map_err(|e| format!("{e:?}"))
        }

        // ── Great People ────────────────────────────────────────────────
        ActionKind::RetireGreatPerson { person } => {
            let gid = parse_great_person_id(person)?;
            rules
                .retire_great_person(state, gid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::CreateGreatWork { person } => {
            let gid = parse_great_person_id(person)?;
            rules
                .create_great_work(state, gid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::RecruitGreatPerson { person_type } => {
            let pt = parse_great_person_type(person_type)?;
            rules
                .recruit_great_person(state, civ_id, pt)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::RecruitGreatPersonFaith { person_type } => {
            let pt = parse_great_person_type(person_type)?;
            rules
                .recruit_great_person_with_faith(state, civ_id, pt)
                .map_err(|e| format!("{e:?}"))
        }

        // ── Governors ───────────────────────────────────────────────────
        ActionKind::AssignGovernor { governor, city } => {
            let gid =
                parse_ulid(governor).map(libciv::GovernorId::from_ulid)?;
            let cid = parse_city_id(city)?;
            rules
                .assign_governor(state, gid, cid)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::PromoteGovernor {
            governor,
            promotion,
        } => {
            let gid =
                parse_ulid(governor).map(libciv::GovernorId::from_ulid)?;
            // Leak to &'static str for the trait method.
            let promo: &'static str = Box::leak(promotion.clone().into_boxed_str());
            rules
                .promote_governor(state, gid, promo)
                .map_err(|e| format!("{e:?}"))
        }

        // ── Victory ─────────────────────────────────────────────────────
        ActionKind::CompleteScienceMilestone => rules
            .complete_science_milestone(state, civ_id)
            .map_err(|e| format!("{e:?}")),

        // ── Barbarians ──────────────────────────────────────────────────
        ActionKind::HireBarbarian { camp_q, camp_r } => {
            let camp_id = find_barbarian_camp(state, *camp_q, *camp_r)?;
            rules
                .hire_from_barbarian_camp(state, camp_id, civ_id)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::BribeBarbarian { camp_q, camp_r } => {
            let camp_id = find_barbarian_camp(state, *camp_q, *camp_r)?;
            rules
                .bribe_barbarian_camp(state, camp_id, civ_id)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::InciteBarbarian {
            camp_q,
            camp_r,
            target,
        } => {
            let camp_id = find_barbarian_camp(state, *camp_q, *camp_r)?;
            let target_id = find_civ_by_name(state, target)?;
            rules
                .incite_barbarian_camp(state, camp_id, civ_id, target_id)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::RaidBarbarian {
            unit,
            camp_q,
            camp_r,
        } => {
            let uid = parse_unit_id(unit)?;
            let camp_id = find_barbarian_camp(state, *camp_q, *camp_r)?;
            rules
                .raid_barbarian_camp(state, uid, camp_id)
                .map_err(|e| format!("{e:?}"))
        }
        ActionKind::ClearBarbarian { camp_q, camp_r } => {
            let camp_id = find_barbarian_camp(state, *camp_q, *camp_r)?;
            rules
                .clear_barbarian_camp(state, camp_id, civ_id)
                .map_err(|e| format!("{e:?}"))
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn find_barbarian_camp(
    state: &libciv::GameState,
    q: i32,
    r: i32,
) -> Result<BarbarianCampId, String> {
    let coord = HexCoord::from_qr(q, r);
    state
        .barbarian_camps
        .iter()
        .find(|c| c.coord == coord)
        .map(|c| c.id)
        .ok_or_else(|| format!("no barbarian camp at ({q}, {r})"))
}

fn resolve_production_item(
    state: &libciv::GameState,
    name: &str,
) -> Result<libciv::civ::ProductionItem, String> {
    // Try unit type first.
    if let Some(def) = state
        .unit_type_defs
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(name))
    {
        return Ok(libciv::civ::ProductionItem::Unit(def.id));
    }
    // Try building.
    if let Some(def) = state
        .building_defs
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(name))
    {
        return Ok(libciv::civ::ProductionItem::Building(def.id));
    }
    // Try wonder.
    if let Some(def) = state
        .wonder_defs
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(name))
    {
        return Ok(libciv::civ::ProductionItem::Wonder(def.id));
    }
    // Try project.
    if let Some(def) = state
        .project_defs
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(name))
    {
        return Ok(libciv::civ::ProductionItem::Project(def.id));
    }
    Err(format!("unknown production item '{name}'"))
}

fn resolve_faith_purchase_item(name: &str) -> Result<FaithPurchaseItem, String> {
    // Leak name to static for the enum variant.
    let static_name: &'static str = Box::leak(name.to_string().into_boxed_str());
    // Try as unit name first; the RulesEngine will validate further.
    Ok(FaithPurchaseItem::Unit(static_name))
}
