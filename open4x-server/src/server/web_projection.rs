//! Builders that turn `GameView` into wire types under
//! [`open4x_protocol::v1::web`].
//!
//! See `book/src/roadmap/web-ui.md` §5.2. Each builder is a pure function
//! over the player-projected `GameView` plus a small amount of room
//! configuration where needed (e.g. `turn_limit` for `/player-state`).
//!
//! Builders for endpoints that haven't been wired yet (Phase 2+) still
//! return `Default::default()`.

#![allow(dead_code, unused_variables)]

use open4x_protocol::v1::enums::{BoardTopology, BuiltinTerrain, TileVisibility};
use open4x_protocol::v1::view::GameView;
use open4x_protocol::v1::web::*;

// ── /player-state ────────────────────────────────────────────────────────────

pub fn build_player_state(view: &GameView, turn_limit: Option<u32>) -> player_state::PlayerState {
    let civ = &view.my_civ;
    let yields = &civ.yields;

    let resources = player_state::Resources {
        gold: player_state::Bucket {
            value: Some(civ.gold),
            per_turn: yields.gold,
        },
        science: player_state::Bucket {
            value: None,
            per_turn: yields.science,
        },
        culture: player_state::Bucket {
            value: None,
            per_turn: yields.culture,
        },
        faith: player_state::Bucket {
            value: Some(civ.faith as i32),
            per_turn: yields.faith,
        },
        food: player_state::Bucket {
            value: None,
            per_turn: yields.food,
        },
        production: player_state::Bucket {
            value: None,
            per_turn: yields.production,
        },
    };

    let strategic: std::collections::BTreeMap<String, u32> = civ
        .strategic_resources
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    player_state::PlayerState {
        turn: view.turn,
        turn_max: turn_limit.unwrap_or(0),
        era: format!("{:?}", civ.current_era),
        era_progress: 0.0, // TODO(WEBUI-ERA): plumb era progress through GameView
        resources,
        happiness: yields.amenities,
        strategic,
    }
}

// ── /world/snapshot ──────────────────────────────────────────────────────────

/// Project a `WorldSnapshot` from the player-visible `GameView`.
///
/// `radius == 0` returns every explored tile in the view; any other value
/// keeps only tiles within that hex distance from the camera centre `(q, r)`.
pub fn build_world_snapshot(
    view: &GameView,
    cam_q: i32,
    cam_r: i32,
    radius: u32,
) -> world::WorldSnapshot {
    let board = &view.board;

    let (wrap_x, wrap_y) = match board.topology {
        BoardTopology::Flat => (false, false),
        BoardTopology::CylindricalEW => (true, false),
        BoardTopology::Toroidal => (true, true),
    };

    let world_meta = world::WorldMeta {
        width: board.width,
        height: board.height,
        wrap_x,
        wrap_y,
        seed: 0, // TODO(WEBUI-SEED): plumb world seed through GameView
        turn: view.turn,
    };

    let camera = world::Camera {
        x: cam_q,
        y: cam_r,
        zoom: 1.0,
        selection: Some(world::TileCoord { q: cam_q, r: cam_r }),
    };

    let legend = world::Legend {
        terrains: vec![
            "Grassland".into(),
            "Plains".into(),
            "Desert".into(),
            "Tundra".into(),
            "Snow".into(),
            "Coast".into(),
            "Ocean".into(),
            "Mountain".into(),
        ],
        resources: vec![
            "Wheat".into(),
            "Cattle".into(),
            "Iron".into(),
            "Horses".into(),
            "Fish".into(),
            "Pearls".into(),
        ],
        edge_kinds: vec!["river".into(), "border".into(), "coast".into()],
    };

    // Pre-compute tile lookups for cities/units placed on tiles.
    let cities_by_coord: std::collections::HashMap<_, _> =
        view.cities.iter().map(|c| (c.coord, c)).collect();
    let units_by_coord: std::collections::HashMap<_, _> =
        view.units.iter().map(|u| (u.coord, u)).collect();
    let unit_type_by_id: std::collections::HashMap<_, _> =
        view.unit_type_defs.iter().map(|d| (d.id, d)).collect();
    let my_civ_id = view.my_civ_id;

    let tiles = board
        .tiles
        .iter()
        .filter(|t| {
            radius == 0 || hex_distance(cam_q, cam_r, t.coord.q, t.coord.r) <= radius as i32
        })
        .map(|t| {
            let owner = t.owner.map(|id| {
                if id == my_civ_id {
                    view.my_civ.name.clone()
                } else {
                    view.other_civs
                        .iter()
                        .find(|c| c.id == id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| id.to_string())
                }
            });

            let city = cities_by_coord.get(&t.coord).map(|c| world::TileCity {
                id: c.id.as_ulid().to_string(),
                name: c.name.clone(),
                pop: c.population,
                capital: c.is_capital,
            });

            let unit = units_by_coord.get(&t.coord).map(|u| {
                let kind = unit_type_by_id
                    .get(&u.unit_type)
                    .map(|d| capitalize(&d.name))
                    .unwrap_or_else(|| "Unit".into());
                world::TileUnit {
                    id: u.id.as_ulid().to_string(),
                    kind,
                    name: format!("{:?}", u.category),
                    hp: format!("{}/{}", u.health, 100),
                }
            });

            world::TileView {
                q: t.coord.q,
                r: t.coord.r,
                terrain: terrain_label(t.terrain, t.hills),
                yields: world::TileYields::default(),
                appeal: 0,
                flood: 0,
                fog: matches!(t.visibility, TileVisibility::Foggy),
                owner,
                resource: t.resource.map(|r| format!("{r:?}")),
                improvement: t.improvement.map(|i| format!("{i:?}")),
                city,
                unit,
            }
        })
        .collect();

    world::WorldSnapshot {
        world: world_meta,
        camera,
        legend,
        tiles,
    }
}

fn terrain_label(t: BuiltinTerrain, hills: bool) -> String {
    if hills {
        format!("{t:?}+Hills")
    } else {
        format!("{t:?}")
    }
}

/// Cube-coordinate hex distance using only the axial `(q, r)` pair.
fn hex_distance(a_q: i32, a_r: i32, b_q: i32, b_r: i32) -> i32 {
    let dq = a_q - b_q;
    let dr = a_r - b_r;
    let ds = -dq - dr;
    (dq.abs() + dr.abs() + ds.abs()) / 2
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// ── /cities ──────────────────────────────────────────────────────────────────

pub fn build_cities(view: &GameView) -> city_data::CityData {
    use open4x_protocol::v1::enums::ProductionItemView;
    let unit_type_by_id: std::collections::HashMap<_, _> =
        view.unit_type_defs.iter().map(|d| (d.id, d)).collect();
    let building_def_by_id: std::collections::HashMap<_, _> =
        view.building_defs.iter().map(|d| (d.id, d)).collect();
    let my_civ_id = view.my_civ_id;

    let cities = view
        .cities
        .iter()
        .map(|c| {
            let owner = if c.owner == my_civ_id {
                view.my_civ.name.clone()
            } else {
                view.other_civs
                    .iter()
                    .find(|p| p.id == c.owner)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| c.owner.to_string())
            };
            let production_queue: Vec<String> = c
                .production_queue
                .iter()
                .map(|item| match item {
                    ProductionItemView::Unit(t) => unit_type_by_id
                        .get(t)
                        .map(|d| capitalize(&d.name))
                        .unwrap_or_else(|| "Unit".into()),
                    ProductionItemView::Building(b) => building_def_by_id
                        .get(b)
                        .map(|d| capitalize(&d.name))
                        .unwrap_or_else(|| "Building".into()),
                    ProductionItemView::District(d) => format!("{d:?}"),
                    ProductionItemView::Wonder(_) => "Wonder".into(),
                    ProductionItemView::Project(_) => "Project".into(),
                })
                .collect();
            city_data::CityRow {
                id: c.id.as_ulid().to_string(),
                name: c.name.clone(),
                owner,
                capital: c.is_capital,
                is_own: c.is_own,
                position: world::TileCoord {
                    q: c.coord.q,
                    r: c.coord.r,
                },
                population: c.population,
                food_stored: c.food_stored,
                food_to_grow: c.food_to_grow,
                production_stored: c.production_stored,
                production_queue,
                worked_tile_count: c.worked_tiles.len() as u32,
                territory_count: c.territory.len() as u32,
                focus: format!("{:?}", c.focus).to_lowercase(),
            }
        })
        .collect();

    city_data::CityData { cities }
}

pub fn build_city_tiles(view: &GameView, city_id: &str) -> Option<city_tiles::CityTiles> {
    let target_ulid: ulid::Ulid = city_id.parse().ok()?;
    let target = open4x_protocol::v1::ids::CityId::from_ulid(target_ulid);
    let city = view.cities.iter().find(|c| c.id == target)?;

    let worked: std::collections::HashSet<_> = city.worked_tiles.iter().collect();

    let tiles = city
        .territory
        .iter()
        .map(|coord| city_tiles::TileEntry {
            q: coord.q,
            r: coord.r,
            worked: worked.contains(coord),
            // GameView doesn't currently expose locked_tiles; default false.
            locked: false,
            is_center: *coord == city.coord,
        })
        .collect();

    Some(city_tiles::CityTiles {
        city_id: city_id.to_string(),
        center: world::TileCoord {
            q: city.coord.q,
            r: city.coord.r,
        },
        tiles,
    })
}

// ── /units ───────────────────────────────────────────────────────────────────

/// Map a libciv [`libciv::UnitActionKind`] to the wire `(id, label, hotkey)`
/// triple the wireframe expects. Cosmetic strings live here, not in the
/// engine.
fn unit_action_wire(
    kind: libciv::UnitActionKind,
) -> (&'static str, &'static str, Option<&'static str>) {
    use libciv::UnitActionKind as K;
    match kind {
        K::Move => ("move", "Move", Some("M")),
        K::Attack => ("attack", "Attack", Some("A")),
        K::Fortify => ("fortify", "Fortify", Some("F")),
        K::Sleep => ("sleep", "Sleep", Some("Z")),
        K::FoundCity => ("found_city", "Found City", Some("B")),
        K::Build => ("build", "Build", Some("E")),
        K::TradeRoute => ("trade_route", "Trade Route", Some("T")),
        K::SpreadReligion => ("spread_religion", "Spread Religion", Some("R")),
    }
}

/// Authoritative builder: looks up each visible unit's available actions
/// via [`libciv::RulesEngine::available_unit_actions`] against the room's
/// `GameState`. Use this from REST handlers; [`build_units`] below is the
/// `GameView`-only fallback for tests / contexts without room access.
pub fn build_units_from_room(
    view: &GameView,
    room: &crate::server::state::GameRoom,
) -> unit_data::UnitData {
    use libciv::RulesEngine;

    let unit_type_by_id: std::collections::HashMap<_, _> =
        view.unit_type_defs.iter().map(|d| (d.id, d)).collect();
    let my_civ_id = view.my_civ_id;

    let units = view
        .units
        .iter()
        .map(|u| {
            let def = unit_type_by_id.get(&u.unit_type);
            let kind = def
                .map(|d| capitalize(&d.name))
                .unwrap_or_else(|| "Unit".into());
            let owner = if u.owner == my_civ_id {
                view.my_civ.name.clone()
            } else {
                view.other_civs
                    .iter()
                    .find(|p| p.id == u.owner)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| u.owner.to_string())
            };

            // Engine-derived action set. Foreign units get an empty set
            // (the player can't act on units they don't own).
            let actions: Vec<_> = if u.is_own {
                let libciv_unit_id = libciv::UnitId::from_ulid(u.id.as_ulid());
                room.rules
                    .available_unit_actions(&room.state, libciv_unit_id)
                    .into_iter()
                    .map(|a| {
                        let (id, label, hotkey) = unit_action_wire(a.kind);
                        unit_data::UnitAction {
                            id: id.into(),
                            label: label.into(),
                            hotkey: hotkey.map(|s| s.into()),
                            enabled: a.enabled,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            unit_data::Unit {
                id: u.id.as_ulid().to_string(),
                name: kind.clone(),
                kind,
                owner,
                is_own: u.is_own,
                hp: u.health,
                hp_max: 100,
                mp: u.movement_left / 100,
                mp_max: u.max_movement / 100,
                position: world::TileCoord {
                    q: u.coord.q,
                    r: u.coord.r,
                },
                status: "idle".into(),
                combat_strength: u.combat_strength,
                range: u.range,
                vision_range: u.vision_range,
                category: format!("{:?}", u.category),
                domain: format!("{:?}", u.domain),
                actions,
            }
        })
        .collect();

    unit_data::UnitData { units }
}

pub fn build_units(view: &GameView) -> unit_data::UnitData {
    let unit_type_by_id: std::collections::HashMap<_, _> =
        view.unit_type_defs.iter().map(|d| (d.id, d)).collect();
    let my_civ_id = view.my_civ_id;

    let units = view
        .units
        .iter()
        .map(|u| {
            let def = unit_type_by_id.get(&u.unit_type);
            let kind = def
                .map(|d| capitalize(&d.name))
                .unwrap_or_else(|| "Unit".into());
            let owner = if u.owner == my_civ_id {
                view.my_civ.name.clone()
            } else {
                view.other_civs
                    .iter()
                    .find(|p| p.id == u.owner)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| u.owner.to_string())
            };

            // Static placeholder action set — Phase 2 RulesEngine extension
            // will replace this with `available_unit_actions(gs, unit)`.
            let mut actions = vec![
                unit_data::UnitAction {
                    id: "move".into(),
                    label: "Move".into(),
                    hotkey: Some("M".into()),
                    enabled: u.movement_left > 0,
                },
                unit_data::UnitAction {
                    id: "fortify".into(),
                    label: "Fortify".into(),
                    hotkey: Some("F".into()),
                    enabled: u.is_own && u.combat_strength.is_some(),
                },
                unit_data::UnitAction {
                    id: "sleep".into(),
                    label: "Sleep".into(),
                    hotkey: Some("Z".into()),
                    enabled: u.is_own,
                },
            ];
            if u.is_own && u.combat_strength.is_some() {
                actions.insert(
                    1,
                    unit_data::UnitAction {
                        id: "attack".into(),
                        label: "Attack".into(),
                        hotkey: Some("A".into()),
                        enabled: u.movement_left > 0,
                    },
                );
            }
            if def.is_some_and(|d| d.can_found_city) {
                actions.push(unit_data::UnitAction {
                    id: "found_city".into(),
                    label: "Found City".into(),
                    hotkey: Some("B".into()),
                    enabled: u.is_own && u.movement_left > 0,
                });
            }

            unit_data::Unit {
                id: u.id.as_ulid().to_string(),
                name: kind.clone(),
                kind,
                owner,
                is_own: u.is_own,
                hp: u.health,
                hp_max: 100,
                mp: u.movement_left / 100,
                mp_max: u.max_movement / 100,
                position: world::TileCoord {
                    q: u.coord.q,
                    r: u.coord.r,
                },
                status: "idle".into(),
                combat_strength: u.combat_strength,
                range: u.range,
                vision_range: u.vision_range,
                category: format!("{:?}", u.category),
                domain: format!("{:?}", u.domain),
                actions,
            }
        })
        .collect();

    unit_data::UnitData { units }
}

pub fn build_armies(view: &GameView) -> army_data::ArmyData {
    army_data::ArmyData::default()
}

// ── /combat/preview ──────────────────────────────────────────────────────────

/// Compute a heuristic combat preview: the attacker's combat strength vs.
/// the defender's, with a simple expected-damage estimate.
///
/// This does not call into `RulesEngine` yet — the proper implementation is
/// a Phase 2 libciv extension (`RulesEngine::preview_combat`). For now the
/// projector is good enough to drive the wireframe's combat-odds bar.
/// Authoritative `/combat/preview` builder — calls into
/// [`libciv::RulesEngine::preview_combat`] against the room's `GameState`.
/// Returns `None` when the attacker can't be found or when the engine
/// returns `None` (no defender / out of range / non-combat attacker).
///
/// `defender_kind` for the wire shape comes from the room's `unit_type_defs`
/// since the engine returns the bare `UnitId`.
pub fn build_combat_preview_from_room(
    view: &GameView,
    room: &crate::server::state::GameRoom,
    attacker_id: &str,
    defender_q: i32,
    defender_r: i32,
) -> Option<combat_preview::CombatPreview> {
    use libciv::RulesEngine;

    let attacker_ulid: ulid::Ulid = attacker_id.parse().ok()?;
    let libciv_attacker = libciv::UnitId::from_ulid(attacker_ulid);
    let coord = libhexgrid::coord::HexCoord::from_qr(defender_q, defender_r);

    let preview = room
        .rules
        .preview_combat(&room.state, libciv_attacker, coord)?;

    // Look up the defender's unit-type name from the engine's units.
    let defender_unit = room.state.units.iter().find(|u| u.id == preview.defender);
    let defender_kind = defender_unit
        .and_then(|u| {
            room.state
                .unit_type_defs
                .iter()
                .find(|d| d.id == u.unit_type)
        })
        .map(|d| capitalize(d.name))
        .unwrap_or_else(|| "Enemy".into());

    let defender_info = combat_preview::DefenderInfo {
        id: preview.defender.as_ulid().to_string(),
        kind: defender_kind,
        q: defender_q,
        r: defender_r,
    };

    // The wire shape's `_strength` fields are i32; cast through.
    let _ = view;
    Some(combat_preview::CombatPreview {
        attacker_id: preview.attacker.as_ulid().to_string(),
        defender: Some(defender_info),
        attacker_strength: preview.attacker_effective_cs as i32,
        defender_strength: preview.defender_effective_cs as i32,
        predicted_attacker_damage: preview.predicted_attacker_damage as i32,
        predicted_defender_damage: preview.predicted_defender_damage as i32,
        note: format!("{:?} preview at rng=1.0", preview.attack_type),
    })
}

/// Legacy `GameView`-only fallback. Returns the same wire shape but uses
/// a simple CS-difference heuristic since the GameView doesn't carry the
/// modifier pipeline. Use [`build_combat_preview_from_room`] from REST
/// handlers.
pub fn build_combat_preview(
    view: &GameView,
    attacker_id: &str,
    defender_q: i32,
    defender_r: i32,
) -> Option<combat_preview::CombatPreview> {
    let attacker_ulid: ulid::Ulid = attacker_id.parse().ok()?;
    let attacker_uid = open4x_protocol::v1::ids::UnitId::from_ulid(attacker_ulid);
    let attacker = view.units.iter().find(|u| u.id == attacker_uid)?;
    let attacker_str = attacker.combat_strength? as i32;

    let unit_type_by_id: std::collections::HashMap<_, _> =
        view.unit_type_defs.iter().map(|d| (d.id, d)).collect();

    let defender = view
        .units
        .iter()
        .find(|u| u.coord.q == defender_q && u.coord.r == defender_r && !u.is_own);
    let (defender_info, defender_str) = match defender {
        Some(d) => {
            let kind = unit_type_by_id
                .get(&d.unit_type)
                .map(|t| capitalize(&t.name))
                .unwrap_or_else(|| "Enemy".into());
            (
                Some(combat_preview::DefenderInfo {
                    id: d.id.as_ulid().to_string(),
                    kind,
                    q: d.coord.q,
                    r: d.coord.r,
                }),
                d.combat_strength.unwrap_or(0) as i32,
            )
        }
        None => (None, 0),
    };

    let diff = attacker_str - defender_str;
    let predicted_atk = (30 - diff).clamp(0, 60);
    let predicted_def = (30 + diff).clamp(0, 60);

    Some(combat_preview::CombatPreview {
        attacker_id: attacker.id.as_ulid().to_string(),
        defender: defender_info,
        attacker_strength: attacker_str,
        defender_strength: defender_str,
        predicted_attacker_damage: predicted_atk,
        predicted_defender_damage: predicted_def,
        note: "heuristic preview (GameView-only fallback)".into(),
    })
}

pub fn build_tech_tree(view: &GameView) -> tech_tree::TechTreeView {
    let researched: std::collections::HashSet<_> = view.my_civ.researched_techs.iter().collect();
    let queue_progress: std::collections::HashMap<_, _> = view
        .my_civ
        .research_queue
        .iter()
        .map(|p| (p.tech_id, p))
        .collect();
    let active = view.my_civ.research_queue.first().map(|p| p.tech_id);

    let techs = view
        .tech_tree
        .nodes
        .iter()
        .map(|n| {
            let prereq_ids: Vec<String> = n
                .prerequisites
                .iter()
                .map(|p| p.as_ulid().to_string())
                .collect();
            let status = if researched.contains(&n.id) {
                "done".to_string()
            } else if Some(n.id) == active {
                "current".to_string()
            } else if n.prerequisites.iter().all(|p| researched.contains(p)) {
                "available".to_string()
            } else {
                "locked".to_string()
            };
            let progress = queue_progress.get(&n.id).map(|p| p.progress);
            tech_tree::TechNode {
                id: n.id.as_ulid().to_string(),
                name: n.name.clone(),
                cost: n.cost,
                progress,
                unlocks: n.eureka_description.clone(),
                era: era_for_tech_cost(n.cost),
                status,
                prereqs: prereq_ids,
            }
        })
        .collect();

    let research_queue = view
        .my_civ
        .research_queue
        .iter()
        .map(|p| p.tech_id.as_ulid().to_string())
        .collect();

    tech_tree::TechTreeView {
        techs,
        research_queue,
    }
}

pub fn build_civics_tree(view: &GameView) -> civics_tree::CivicsTreeView {
    let completed: std::collections::HashSet<_> = view.my_civ.completed_civics.iter().collect();
    let active = view.my_civ.civic_in_progress.as_ref().map(|p| p.civic_id);
    let active_progress = view.my_civ.civic_in_progress.as_ref().map(|p| p.progress);

    let civics = view
        .civic_tree
        .nodes
        .iter()
        .map(|n| {
            let prereq_ids: Vec<String> = n
                .prerequisites
                .iter()
                .map(|p| p.as_ulid().to_string())
                .collect();
            let status = if completed.contains(&n.id) {
                "done".to_string()
            } else if Some(n.id) == active {
                "current".to_string()
            } else if n.prerequisites.iter().all(|p| completed.contains(p)) {
                "available".to_string()
            } else {
                "locked".to_string()
            };
            let progress = if Some(n.id) == active {
                active_progress
            } else {
                None
            };
            civics_tree::CivicNode {
                id: n.id.as_ulid().to_string(),
                name: n.name.clone(),
                cost: n.cost,
                progress,
                unlocks: n.inspiration_description.clone(),
                era: era_for_tech_cost(n.cost),
                status,
                prereqs: prereq_ids,
            }
        })
        .collect();

    let civic_queue = active
        .map(|id| vec![id.as_ulid().to_string()])
        .unwrap_or_default();

    civics_tree::CivicsTreeView {
        civics,
        civic_queue,
    }
}

/// Heuristic era classification by cost — replaces server-side era lookup
/// while libciv hasn't yet exposed era per node. Replaced in Phase 3 final
/// pass with the real era field once `RulesEngine::tech_era(node)` lands.
fn era_for_tech_cost(cost: u32) -> String {
    match cost {
        0..=49 => "Ancient".into(),
        50..=99 => "Classical".into(),
        100..=199 => "Medieval".into(),
        200..=299 => "Renaissance".into(),
        300..=499 => "Industrial".into(),
        _ => "Modern+".into(),
    }
}

/// Authoritative `/government` builder: populates the catalogue from
/// [`libciv::RulesEngine::policy_catalogue`]. Use from REST handlers;
/// [`build_government`] below remains as the GameView-only fallback
/// (returns an empty catalogue).
pub fn build_government_from_room(
    view: &GameView,
    room: &crate::server::state::GameRoom,
    civ: open4x_protocol::v1::ids::CivId,
) -> government::GovernmentPolicies {
    use libciv::{PolicyCardStatus, RulesEngine};

    let active_policies: Vec<government::ActivePolicy> = view
        .my_civ
        .active_policies
        .iter()
        .enumerate()
        .map(|(i, pid)| government::ActivePolicy {
            slot: format!("slot_{i}"),
            id: pid.as_ulid().to_string(),
            name: format!("Policy {}", i + 1),
            effect: String::new(),
        })
        .collect();

    let government = government::Government {
        id: view
            .my_civ
            .current_government
            .clone()
            .map(|s| s.to_lowercase().replace(' ', "_"))
            .unwrap_or_else(|| "chiefdom".into()),
        name: view
            .my_civ
            .current_government
            .clone()
            .unwrap_or_else(|| "Chiefdom".into()),
        era: format!("{:?}", view.my_civ.current_era),
        slots: government::Slots {
            military: 2,
            economic: 2,
            diplomatic: 1,
            wildcard: 0,
        },
        legacy_bonus: String::new(),
    };

    let libciv_civ = libciv::CivId::from_ulid(civ.as_ulid());
    let catalogue: Vec<government::PolicyCard> = room
        .rules
        .policy_catalogue(&room.state, libciv_civ)
        .into_iter()
        .map(|e| {
            let status = match e.status {
                PolicyCardStatus::Active => "active",
                PolicyCardStatus::Available => "available",
                PolicyCardStatus::Locked => "locked",
            };
            government::PolicyCard {
                id: e.policy_id.as_ulid().to_string(),
                name: e.name.into(),
                kind: format!("{:?}", e.policy_type).to_lowercase(),
                era: String::new(),
                dark_age: false,
                status: status.into(),
                unlock_civic: Some(e.prereq_civic.into()),
                effect: String::new(),
            }
        })
        .collect();

    government::GovernmentPolicies {
        government,
        active_policies,
        catalogue,
    }
}

pub fn build_government(view: &GameView) -> government::GovernmentPolicies {
    let government::GovernmentPolicies { catalogue, .. } =
        government::GovernmentPolicies::default();
    let active_policies: Vec<government::ActivePolicy> = view
        .my_civ
        .active_policies
        .iter()
        .enumerate()
        .map(|(i, pid)| government::ActivePolicy {
            slot: format!("slot_{i}"),
            id: pid.as_ulid().to_string(),
            name: format!("Policy {}", i + 1),
            effect: String::new(),
        })
        .collect();

    let government = government::Government {
        id: view
            .my_civ
            .current_government
            .clone()
            .map(|s| s.to_lowercase().replace(' ', "_"))
            .unwrap_or_else(|| "chiefdom".into()),
        name: view
            .my_civ
            .current_government
            .clone()
            .unwrap_or_else(|| "Chiefdom".into()),
        era: format!("{:?}", view.my_civ.current_era),
        slots: government::Slots {
            military: 2,
            economic: 2,
            diplomatic: 1,
            wildcard: 0,
        },
        legacy_bonus: String::new(),
    };

    government::GovernmentPolicies {
        government,
        active_policies,
        catalogue,
    }
}

// ── /diplomacy ───────────────────────────────────────────────────────────────

pub fn build_diplomacy(view: &GameView) -> diplomacy::Diplomacy {
    use open4x_protocol::v1::enums::DiplomaticStatus;

    let civs = view
        .other_civs
        .iter()
        .map(|c| {
            let relation = match c.diplomatic_status {
                DiplomaticStatus::War => "At War",
                DiplomaticStatus::Denounced => "Unfriendly",
                DiplomaticStatus::Neutral => "Neutral",
                DiplomaticStatus::Friendly => "Friendly",
                DiplomaticStatus::Alliance => "Allied",
            };
            let cities_known = view.cities.iter().filter(|city| city.owner == c.id).count() as u32;
            diplomacy::CivRow {
                id: c.id.as_ulid().to_string(),
                name: c.name.clone(),
                leader: c.leader_name.clone(),
                relation: relation.into(),
                relation_score: 0,
                agenda: String::new(),
                government: String::new(),
                cities_known,
                treaties: Vec::new(),
                modifiers: Vec::new(),
            }
        })
        .collect();

    diplomacy::Diplomacy {
        civs,
        city_states: Vec::new(),
        active_civ: None,
        deal_draft: None,
    }
}

// ── /empire/overview ─────────────────────────────────────────────────────────

pub fn build_empire_overview(view: &GameView) -> empire_overview::EmpireOverview {
    let my_cities: Vec<_> = view.cities.iter().filter(|c| c.is_own).collect();
    let total_pop = my_cities.iter().map(|c| c.population).sum();
    let military_units = view
        .units
        .iter()
        .filter(|u| u.is_own && u.combat_strength.is_some())
        .count() as u32;

    let summary = empire_overview::Summary {
        cities: my_cities.len() as u32,
        population: total_pop,
        treasury: view.my_civ.gold,
        treasury_per_turn: view.my_civ.yields.gold,
        military_units,
    };

    let cities = my_cities
        .iter()
        .map(|c| empire_overview::CityRow {
            name: c.name.clone(),
            capital: c.is_capital,
            pop: c.population,
        })
        .collect();

    let strategic_resources = view
        .my_civ
        .strategic_resources
        .iter()
        .map(|(name, value)| empire_overview::ResourceRow {
            name: name.clone(),
            value: *value,
            per_turn: 0,
        })
        .collect();

    empire_overview::EmpireOverview {
        summary,
        cities,
        strategic_resources,
        luxury_resources: Vec::new(),
        trade_routes: Vec::new(),
        trade_slots_total: 0,
    }
}

// ── /victory ─────────────────────────────────────────────────────────────────

/// Map a libciv `BuiltinVictoryCondition` to its wire-id string. The
/// wire shape uses stable lowercase ids so the UI doesn't have to care
/// about the random per-game `VictoryId` ULIDs.
fn victory_condition_wire_id(c: &libciv::game::victory::BuiltinVictoryCondition) -> &'static str {
    use libciv::game::victory::BuiltinVictoryCondition as V;
    match c {
        V::Score { .. } => "score",
        V::Culture { .. } => "culture",
        V::Domination { .. } => "domination",
        V::Science { .. } => "science",
        V::Diplomatic { .. } => "diplomatic",
        V::Religious { .. } => "religious",
    }
}

const VICTORY_CONDITION_WIRE: [(&str, &str); 6] = [
    ("score", "Score"),
    ("culture", "Culture"),
    ("domination", "Domination"),
    ("science", "Science"),
    ("religious", "Religious"),
    ("diplomatic", "Diplomatic"),
];

/// Authoritative builder that calls into `RulesEngine::victory_progress`
/// against the room's `GameState`. Use this from REST handlers; the
/// [`build_victory`] helper is kept as a `GameView`-only fallback (returns
/// the stable 6-condition shape with `player_pct=0`).
pub fn build_victory_from_room(
    view: &GameView,
    room: &crate::server::state::GameRoom,
    civ: open4x_protocol::v1::ids::CivId,
    turn_limit: Option<u32>,
) -> victory::Victory {
    use libciv::RulesEngine;

    let my_civ_id = view.my_civ_id;
    let my_score = view
        .scores
        .iter()
        .find(|(id, _)| *id == my_civ_id)
        .map(|(_, s)| *s as i32)
        .unwrap_or(0);

    // Leaderboard from scores Vec.
    let mut leaderboard: Vec<_> = view
        .scores
        .iter()
        .map(|(id, score)| (*id, *score as i32))
        .collect();
    leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
    let leaderboard = leaderboard
        .into_iter()
        .enumerate()
        .map(|(i, (id, score))| {
            let name = if id == view.my_civ_id {
                view.my_civ.name.clone()
            } else {
                view.other_civs
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Unknown".into())
            };
            victory::LeaderRow {
                rank: (i + 1) as u32,
                name,
                is_player: id == view.my_civ_id,
                score,
            }
        })
        .collect::<Vec<_>>();

    let rank = leaderboard
        .iter()
        .position(|r| r.is_player)
        .map(|i| (i + 1) as u32)
        .unwrap_or(1);

    // Compute engine-side progress for the player's civ, indexed by wire-id.
    let libciv_civ = libciv::CivId::from_ulid(civ.as_ulid());
    let progresses = room.rules.victory_progress(&room.state, libciv_civ);
    let pct_by_wire_id: std::collections::HashMap<&'static str, u32> = progresses
        .iter()
        .zip(room.state.victory_conditions.iter())
        .map(|(p, c)| {
            let pct = p.percentage().clamp(0.0, 100.0).round() as u32;
            (victory_condition_wire_id(c), pct)
        })
        .collect();

    // Stable 6-element wire shape; overlay engine percentages where present.
    let conditions: Vec<_> = VICTORY_CONDITION_WIRE
        .iter()
        .map(|(id, name)| victory::Condition {
            id: (*id).into(),
            name: (*name).into(),
            player_pct: pct_by_wire_id.get(*id).copied().unwrap_or(0),
        })
        .collect();

    let (leading_condition, leading_pct) = conditions
        .iter()
        .max_by_key(|c| c.player_pct)
        .map(|c| (c.id.clone(), c.player_pct))
        .unwrap_or_else(|| ("score".into(), 0));

    victory::Victory {
        turn: view.turn,
        turn_max: turn_limit.unwrap_or(500),
        leading_condition,
        leading_pct,
        score: my_score,
        rank,
        rank_of: leaderboard.len() as u32,
        conditions,
        leaderboard,
    }
}

/// Legacy `GameView`-only builder. Returns the stable 6-condition wire
/// shape with `player_pct=0`. Use [`build_victory_from_room`] from REST
/// handlers.
pub fn build_victory(view: &GameView) -> victory::Victory {
    let my_civ_id = view.my_civ_id;
    let my_score = view
        .scores
        .iter()
        .find(|(id, _)| *id == my_civ_id)
        .map(|(_, s)| *s as i32)
        .unwrap_or(0);

    let mut leaderboard: Vec<_> = view
        .scores
        .iter()
        .map(|(id, score)| (*id, *score as i32))
        .collect();
    leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
    let leaderboard = leaderboard
        .into_iter()
        .enumerate()
        .map(|(i, (id, score))| {
            let name = if id == view.my_civ_id {
                view.my_civ.name.clone()
            } else {
                view.other_civs
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Unknown".into())
            };
            victory::LeaderRow {
                rank: (i + 1) as u32,
                name,
                is_player: id == view.my_civ_id,
                score,
            }
        })
        .collect::<Vec<_>>();

    let rank = leaderboard
        .iter()
        .position(|r| r.is_player)
        .map(|i| (i + 1) as u32)
        .unwrap_or(1);

    let conditions = VICTORY_CONDITION_WIRE
        .iter()
        .map(|(id, name)| victory::Condition {
            id: (*id).into(),
            name: (*name).into(),
            player_pct: 0,
        })
        .collect();

    victory::Victory {
        turn: view.turn,
        turn_max: 500,
        leading_condition: "score".into(),
        leading_pct: 0,
        score: my_score,
        rank,
        rank_of: leaderboard.len() as u32,
        conditions,
        leaderboard,
    }
}

// ── /notifications ───────────────────────────────────────────────────────────

pub fn build_notifications(view: &GameView) -> notifications::Notifications {
    // Default impl when no room context; handler with room access uses
    // build_notifications_from_room below.
    notifications::Notifications {
        turn: view.turn,
        notifications: Vec::new(),
    }
}

pub fn build_notifications_from_room(
    view: &GameView,
    room: &crate::server::state::GameRoom,
    civ: open4x_protocol::v1::ids::CivId,
) -> notifications::Notifications {
    let recs = room.notifications.for_civ(civ);
    let items = recs
        .into_iter()
        .map(|r| notifications::Notification {
            id: r.id,
            kind: r.kind.as_wire().to_string(),
            category: r.category.to_string(),
            title: r.title,
            desc: r.desc,
            target: r.target.map(|t| notifications::NotificationTarget {
                screen: t.screen,
                q: t.q,
                r: t.r,
            }),
        })
        .collect();
    notifications::Notifications {
        turn: view.turn,
        notifications: items,
    }
}

// ── /turn-queue ──────────────────────────────────────────────────────────────

/// Authoritative turn-queue builder — calls
/// `RulesEngine::pending_actions` against the room's `GameState` and maps
/// each [`libciv::PendingAction`] into a wire-shape
/// [`turn_queue::TurnQueueItem`]. Use this from REST handlers; the
/// [`build_turn_queue`] helper below is kept as a `GameView`-only fallback
/// for tests / contexts without room access.
pub fn build_turn_queue_from_room(
    view: &GameView,
    room: &crate::server::state::GameRoom,
    civ: open4x_protocol::v1::ids::CivId,
) -> turn_queue::TurnQueue {
    use libciv::{PendingActionKind, RulesEngine};

    let libciv_civ = libciv::CivId::from_ulid(civ.as_ulid());
    let pending = room.rules.pending_actions(&room.state, libciv_civ);

    let items = pending
        .into_iter()
        .map(|a| match a.kind {
            PendingActionKind::ChooseResearch => turn_queue::TurnQueueItem {
                id: "choose_research".into(),
                kind: "research".into(),
                required: a.required,
                title: "Choose next research".into(),
                desc: "No tech is queued — pick one to start researching.".into(),
                skip_label: None,
                target: Some(notifications::NotificationTarget {
                    screen: "tech".into(),
                    q: None,
                    r: None,
                }),
            },
            PendingActionKind::ChooseCivic => turn_queue::TurnQueueItem {
                id: "choose_civic".into(),
                kind: "civic".into(),
                required: a.required,
                title: "Choose next civic".into(),
                desc: "No civic is in progress — pick one to start studying.".into(),
                skip_label: None,
                target: Some(notifications::NotificationTarget {
                    screen: "culture".into(),
                    q: None,
                    r: None,
                }),
            },
            PendingActionKind::UnitNeedsOrders { unit_id, coord } => {
                // Look up the unit on the GameView so the desc can include
                // movement info; fall back to bare title if the unit isn't
                // visible to the player (shouldn't happen — they own it).
                let view_unit = view
                    .units
                    .iter()
                    .find(|u| u.id.as_ulid() == unit_id.as_ulid());
                let desc = view_unit
                    .map(|u| {
                        format!(
                            "{}/{} MP available",
                            u.movement_left / 100,
                            u.max_movement / 100
                        )
                    })
                    .unwrap_or_else(|| "Has movement available".into());
                turn_queue::TurnQueueItem {
                    id: format!("unit_{}", unit_id.as_ulid()),
                    kind: "unit".into(),
                    required: a.required,
                    title: format!("Unit at ({}, {})", coord.q, coord.r),
                    desc,
                    skip_label: Some("Sleep".into()),
                    target: Some(notifications::NotificationTarget {
                        screen: "hud".into(),
                        q: Some(coord.q),
                        r: Some(coord.r),
                    }),
                }
            }
            PendingActionKind::CityNeedsProduction { city_id } => {
                // Use the city's name + coord from GameState (authoritative).
                let city = room
                    .state
                    .cities
                    .iter()
                    .find(|c| c.id.as_ulid() == city_id.as_ulid());
                let (title, q, r) = match city {
                    Some(c) => (
                        format!("{} has nothing in production", c.name),
                        Some(c.coord.q),
                        Some(c.coord.r),
                    ),
                    None => ("City has nothing in production".into(), None, None),
                };
                turn_queue::TurnQueueItem {
                    id: format!("city_{}", city_id.as_ulid()),
                    kind: "city".into(),
                    required: a.required,
                    title,
                    desc: "Pick a unit, building, or district to start producing.".into(),
                    skip_label: None,
                    target: Some(notifications::NotificationTarget {
                        screen: "city".into(),
                        q,
                        r,
                    }),
                }
            }
        })
        .collect();

    turn_queue::TurnQueue {
        turn: view.turn,
        items,
    }
}

/// Legacy `GameView`-only builder. Kept for tests and any caller without
/// room access; mirrors only the two cases that can be derived from the
/// view (research choice + units with movement).
pub fn build_turn_queue(view: &GameView) -> turn_queue::TurnQueue {
    let mut items = Vec::new();

    if view.my_civ.research_queue.is_empty() && !view.tech_tree.nodes.is_empty() {
        items.push(turn_queue::TurnQueueItem {
            id: "choose_research".into(),
            kind: "research".into(),
            required: true,
            title: "Choose next research".into(),
            desc: "No tech is queued — pick one to start researching.".into(),
            skip_label: None,
            target: Some(notifications::NotificationTarget {
                screen: "tech".into(),
                q: None,
                r: None,
            }),
        });
    }

    for u in view
        .units
        .iter()
        .filter(|u| u.is_own && u.movement_left > 0)
    {
        items.push(turn_queue::TurnQueueItem {
            id: format!("unit_{}", u.id.as_ulid()),
            kind: "unit".into(),
            required: false,
            title: format!("Unit at ({}, {})", u.coord.q, u.coord.r),
            desc: format!(
                "{}/{} MP available",
                u.movement_left / 100,
                u.max_movement / 100
            ),
            skip_label: Some("Sleep".into()),
            target: Some(notifications::NotificationTarget {
                screen: "hud".into(),
                q: Some(u.coord.q),
                r: Some(u.coord.r),
            }),
        });
    }

    turn_queue::TurnQueue {
        turn: view.turn,
        items,
    }
}

// ── /map/overlays ────────────────────────────────────────────────────────────

pub fn build_registry(view: &GameView) -> registry::Registry {
    let unit_types = view
        .unit_type_defs
        .iter()
        .map(|d| registry::UnitType {
            id: d.id.as_ulid().to_string(),
            name: d.name.clone(),
            production_cost: d.production_cost,
            combat_strength: d.combat_strength,
            max_movement: d.max_movement,
            category: format!("{:?}", d.category),
            domain: format!("{:?}", d.domain),
            can_found_city: d.can_found_city,
        })
        .collect();

    let buildings = view
        .building_defs
        .iter()
        .map(|d| registry::Building {
            id: d.id.as_ulid().to_string(),
            name: d.name.clone(),
            cost: d.cost,
            maintenance: d.maintenance,
        })
        .collect();

    registry::Registry {
        unit_types,
        buildings,
    }
}

pub fn build_map_overlays(view: &GameView) -> map_overlays::MapOverlays {
    let _ = view;
    map_overlays::MapOverlays {
        overlays: vec![
            map_overlays::Overlay {
                id: "yields".into(),
                label: "Yields".into(),
                active: false,
            },
            map_overlays::Overlay {
                id: "appeal".into(),
                label: "Appeal".into(),
                active: false,
            },
            map_overlays::Overlay {
                id: "religion".into(),
                label: "Religion".into(),
                active: false,
            },
            map_overlays::Overlay {
                id: "borders".into(),
                label: "Borders".into(),
                active: true,
            },
            map_overlays::Overlay {
                id: "resources".into(),
                label: "Resources".into(),
                active: true,
            },
        ],
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use open4x_protocol::v1::coord::HexCoord;
    use open4x_protocol::v1::ids::CivId;
    use open4x_protocol::v1::view::*;

    fn empty_game_view() -> GameView {
        let civ_id = CivId::from_ulid(ulid::Ulid::new());
        GameView {
            turn: 7,
            my_civ_id: civ_id,
            board: BoardView {
                width: 10,
                height: 10,
                topology: BoardTopology::CylindricalEW,
                tiles: Vec::new(),
                river_edges: Vec::new(),
            },
            my_civ: CivView {
                id: civ_id,
                name: "Egypt".into(),
                adjective: "Egyptian".into(),
                leader_name: "Cleopatra".into(),
                gold: 100,
                current_era: open4x_protocol::v1::enums::AgeType::Classical,
                researched_techs: Vec::new(),
                research_queue: Vec::new(),
                completed_civics: Vec::new(),
                civic_in_progress: None,
                current_government: None,
                active_policies: Vec::new(),
                unlocked_units: Vec::new(),
                unlocked_buildings: Vec::new(),
                unlocked_improvements: Vec::new(),
                strategic_resources: [("Iron".into(), 3u32), ("Horses".into(), 2u32)]
                    .into_iter()
                    .collect(),
                yields: YieldBundleView {
                    food: 1,
                    production: 2,
                    gold: 5,
                    science: 7,
                    culture: 4,
                    faith: 1,
                    housing: 0,
                    amenities: 2,
                    tourism: 0,
                    great_person_points: 0,
                },
                faith: 11,
                pantheon_belief: None,
                founded_religion: None,
            },
            other_civs: Vec::new(),
            cities: Vec::new(),
            units: Vec::new(),
            tech_tree: TechTreeView { nodes: Vec::new() },
            civic_tree: CivicTreeView { nodes: Vec::new() },
            trade_routes: Vec::new(),
            unit_type_defs: Vec::new(),
            building_defs: Vec::new(),
            scores: Vec::new(),
            religions: Vec::new(),
            game_over: None,
        }
    }

    #[test]
    fn player_state_projects_yields_and_strategic_resources() {
        let view = empty_game_view();
        let ps = build_player_state(&view, Some(500));

        assert_eq!(ps.turn, 7);
        assert_eq!(ps.turn_max, 500);
        assert_eq!(ps.era, "Classical");
        assert_eq!(ps.resources.gold.value, Some(100));
        assert_eq!(ps.resources.gold.per_turn, 5);
        assert!(ps.resources.science.value.is_none());
        assert_eq!(ps.resources.science.per_turn, 7);
        assert_eq!(ps.resources.faith.value, Some(11));
        assert_eq!(ps.happiness, 2);
        assert_eq!(ps.strategic.get("Iron").copied(), Some(3));
        assert_eq!(ps.strategic.get("Horses").copied(), Some(2));
    }

    #[test]
    fn world_snapshot_topology_maps_to_wrap_flags() {
        let view = empty_game_view();
        let snap = build_world_snapshot(&view, 0, 0, 0);
        assert_eq!(snap.world.width, 10);
        assert_eq!(snap.world.height, 10);
        assert!(snap.world.wrap_x);
        assert!(!snap.world.wrap_y);
        assert_eq!(snap.world.turn, 7);
    }

    #[test]
    fn cities_projector_includes_owner_and_queue() {
        use open4x_protocol::v1::enums::ProductionItemView;
        use open4x_protocol::v1::ids::{CityId, UnitTypeId};

        let mut view = empty_game_view();
        let unit_type_id = UnitTypeId::from_ulid(ulid::Ulid::new());
        view.unit_type_defs.push(UnitTypeDefView {
            id: unit_type_id,
            name: "warrior".into(),
            production_cost: 40,
            domain: open4x_protocol::v1::enums::UnitDomain::Land,
            category: open4x_protocol::v1::enums::UnitCategory::Combat,
            max_movement: 200,
            combat_strength: Some(20),
            range: 0,
            vision_range: 2,
            can_found_city: false,
            resource_cost: None,
        });

        view.cities.push(CityView {
            id: CityId::from_ulid(ulid::Ulid::new()),
            name: "Alexandria".into(),
            owner: view.my_civ.id,
            coord: HexCoord::from_qr(2, 3),
            is_capital: true,
            population: 4,
            food_stored: 5,
            food_to_grow: 20,
            production_stored: 12,
            production_queue: vec![ProductionItemView::Unit(unit_type_id)],
            buildings: Vec::new(),
            worked_tiles: vec![HexCoord::from_qr(2, 3)],
            territory: vec![HexCoord::from_qr(2, 3), HexCoord::from_qr(3, 3)],
            ownership: open4x_protocol::v1::enums::CityOwnership::Normal,
            walls: open4x_protocol::v1::enums::WallLevel::None,
            religious_followers: std::collections::HashMap::new(),
            majority_religion: None,
            is_own: true,
            focus: open4x_protocol::v1::enums::CityFocus::default(),
        });

        let cities = build_cities(&view);
        assert_eq!(cities.cities.len(), 1);
        let c = &cities.cities[0];
        assert_eq!(c.name, "Alexandria");
        assert_eq!(c.owner, "Egypt");
        assert!(c.capital);
        assert!(c.is_own);
        assert_eq!(c.population, 4);
        assert_eq!(c.production_queue, vec!["Warrior".to_string()]);
        assert_eq!(c.worked_tile_count, 1);
        assert_eq!(c.territory_count, 2);
    }

    #[test]
    fn units_projector_emits_action_set() {
        use open4x_protocol::v1::ids::{UnitId, UnitTypeId};

        let mut view = empty_game_view();
        let unit_type_id = UnitTypeId::from_ulid(ulid::Ulid::new());
        view.unit_type_defs.push(UnitTypeDefView {
            id: unit_type_id,
            name: "settler".into(),
            production_cost: 80,
            domain: open4x_protocol::v1::enums::UnitDomain::Land,
            category: open4x_protocol::v1::enums::UnitCategory::Civilian,
            max_movement: 200,
            combat_strength: None,
            range: 0,
            vision_range: 2,
            can_found_city: true,
            resource_cost: None,
        });
        view.units.push(UnitView {
            id: UnitId::from_ulid(ulid::Ulid::new()),
            unit_type: unit_type_id,
            owner: view.my_civ.id,
            coord: HexCoord::from_qr(0, 0),
            domain: open4x_protocol::v1::enums::UnitDomain::Land,
            category: open4x_protocol::v1::enums::UnitCategory::Civilian,
            movement_left: 200,
            max_movement: 200,
            combat_strength: None,
            health: 100,
            range: 0,
            vision_range: 2,
            is_own: true,
        });

        let units = build_units(&view);
        assert_eq!(units.units.len(), 1);
        let u = &units.units[0];
        assert_eq!(u.kind, "Settler");
        assert!(u.is_own);
        assert_eq!(u.hp, 100);
        assert_eq!(u.mp, 2);
        // Found City should be present and enabled for an own civilian with movement.
        let action_ids: Vec<_> = u.actions.iter().map(|a| a.id.as_str()).collect();
        assert!(action_ids.contains(&"move"));
        assert!(action_ids.contains(&"found_city"));
        // No combat actions for a non-combat unit.
        assert!(!action_ids.contains(&"attack"));
    }

    #[test]
    fn world_snapshot_radius_filters_tiles() {
        let mut view = empty_game_view();
        for q in -3..=3 {
            for r in -3..=3 {
                view.board.tiles.push(TileView {
                    coord: HexCoord::from_qr(q, r),
                    terrain: open4x_protocol::v1::enums::BuiltinTerrain::Plains,
                    hills: false,
                    feature: None,
                    resource: None,
                    improvement: None,
                    road: None,
                    owner: None,
                    visibility: TileVisibility::Visible,
                });
            }
        }
        // Radius 0 means "no filter": keep all tiles.
        assert_eq!(build_world_snapshot(&view, 0, 0, 0).tiles.len(), 49);
        // Radius 1 = centre + 6 neighbours = 7 tiles.
        assert_eq!(build_world_snapshot(&view, 0, 0, 1).tiles.len(), 7);
        // Radius 2 = 1 + 6 + 12 = 19 tiles.
        assert_eq!(build_world_snapshot(&view, 0, 0, 2).tiles.len(), 19);
    }
}
