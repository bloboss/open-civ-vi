//! Converts a `GameState` into a fog-of-war-filtered `GameView` for one player.

use libciv::game::state::GameState;
use libciv::{CivId, DefaultRulesEngine, RulesEngine};
use libciv::game::score::{all_scores, compute_score};
use libhexgrid::board::{HexBoard, HexEdge};

use crate::types::enums::*;
use crate::types::view::*;
use crate::types::coord::HexCoord as ApiCoord;
use crate::types::ids as api;

// ── Coordinate conversion ────────────────────────────────────────────────────

fn conv_coord(c: libhexgrid::coord::HexCoord) -> ApiCoord {
    ApiCoord { q: c.q, r: c.r, s: c.s }
}

fn conv_civ_id(id: libciv::CivId) -> api::CivId {
    api::CivId::from_ulid(id.as_ulid())
}

fn conv_city_id(id: libciv::CityId) -> api::CityId {
    api::CityId::from_ulid(id.as_ulid())
}

fn conv_unit_id(id: libciv::UnitId) -> api::UnitId {
    api::UnitId::from_ulid(id.as_ulid())
}

fn conv_tech_id(id: libciv::TechId) -> api::TechId {
    api::TechId::from_ulid(id.as_ulid())
}

fn conv_civic_id(id: libciv::CivicId) -> api::CivicId {
    api::CivicId::from_ulid(id.as_ulid())
}

fn conv_policy_id(id: libciv::PolicyId) -> api::PolicyId {
    api::PolicyId::from_ulid(id.as_ulid())
}

fn conv_building_id(id: libciv::BuildingId) -> api::BuildingId {
    api::BuildingId::from_ulid(id.as_ulid())
}

fn conv_unit_type_id(id: libciv::UnitTypeId) -> api::UnitTypeId {
    api::UnitTypeId::from_ulid(id.as_ulid())
}

fn conv_religion_id(id: libciv::ReligionId) -> api::ReligionId {
    api::ReligionId::from_ulid(id.as_ulid())
}

fn conv_belief_id(id: libciv::BeliefId) -> api::BeliefId {
    api::BeliefId::from_ulid(id.as_ulid())
}

fn conv_trade_route_id(id: libciv::TradeRouteId) -> api::TradeRouteId {
    api::TradeRouteId::from_ulid(id.as_ulid())
}

fn conv_wonder_id(id: libciv::WonderId) -> api::WonderId {
    api::WonderId::from_ulid(id.as_ulid())
}

fn conv_project_id(id: libciv::ProjectId) -> api::ProjectId {
    api::ProjectId::from_ulid(id.as_ulid())
}

fn conv_terrain(t: libciv::world::terrain::BuiltinTerrain) -> BuiltinTerrain {
    match t {
        libciv::world::terrain::BuiltinTerrain::Grassland => BuiltinTerrain::Grassland,
        libciv::world::terrain::BuiltinTerrain::Plains    => BuiltinTerrain::Plains,
        libciv::world::terrain::BuiltinTerrain::Desert    => BuiltinTerrain::Desert,
        libciv::world::terrain::BuiltinTerrain::Tundra    => BuiltinTerrain::Tundra,
        libciv::world::terrain::BuiltinTerrain::Snow      => BuiltinTerrain::Snow,
        libciv::world::terrain::BuiltinTerrain::Coast     => BuiltinTerrain::Coast,
        libciv::world::terrain::BuiltinTerrain::Ocean     => BuiltinTerrain::Ocean,
        libciv::world::terrain::BuiltinTerrain::Mountain  => BuiltinTerrain::Mountain,
    }
}

fn conv_feature(f: libciv::world::feature::BuiltinFeature) -> BuiltinFeature {
    match f {
        libciv::world::feature::BuiltinFeature::Forest      => BuiltinFeature::Forest,
        libciv::world::feature::BuiltinFeature::Rainforest  => BuiltinFeature::Rainforest,
        libciv::world::feature::BuiltinFeature::Marsh       => BuiltinFeature::Marsh,
        libciv::world::feature::BuiltinFeature::Floodplain  => BuiltinFeature::Floodplain,
        libciv::world::feature::BuiltinFeature::Reef        => BuiltinFeature::Reef,
        libciv::world::feature::BuiltinFeature::Ice         => BuiltinFeature::Ice,
        libciv::world::feature::BuiltinFeature::VolcanicSoil => BuiltinFeature::VolcanicSoil,
        libciv::world::feature::BuiltinFeature::Oasis       => BuiltinFeature::Oasis,
        libciv::world::feature::BuiltinFeature::GeothermalFissure  => BuiltinFeature::GeothermalFissure,
        libciv::world::feature::BuiltinFeature::Volcano            => BuiltinFeature::Volcano,
        libciv::world::feature::BuiltinFeature::FloodplainGrassland => BuiltinFeature::FloodplainGrassland,
        libciv::world::feature::BuiltinFeature::FloodplainPlains   => BuiltinFeature::FloodplainPlains,
    }
}

fn conv_resource(r: libciv::world::resource::BuiltinResource) -> BuiltinResource {
    // Both enums have identical variant names; match exhaustively.
    use libciv::world::resource::BuiltinResource as R;
    match r {
        R::Wheat => BuiltinResource::Wheat, R::Rice => BuiltinResource::Rice,
        R::Cattle => BuiltinResource::Cattle, R::Sheep => BuiltinResource::Sheep,
        R::Fish => BuiltinResource::Fish, R::Stone => BuiltinResource::Stone,
        R::Copper => BuiltinResource::Copper, R::Deer => BuiltinResource::Deer,
        R::Wine => BuiltinResource::Wine, R::Silk => BuiltinResource::Silk,
        R::Spices => BuiltinResource::Spices, R::Incense => BuiltinResource::Incense,
        R::Cotton => BuiltinResource::Cotton, R::Ivory => BuiltinResource::Ivory,
        R::Sugar => BuiltinResource::Sugar, R::Salt => BuiltinResource::Salt,
        R::Horses => BuiltinResource::Horses, R::Iron => BuiltinResource::Iron,
        R::Coal => BuiltinResource::Coal, R::Oil => BuiltinResource::Oil,
        R::Aluminum => BuiltinResource::Aluminum, R::Niter => BuiltinResource::Niter,
        R::Uranium => BuiltinResource::Uranium,
        R::Bananas => BuiltinResource::Bananas, R::Crabs => BuiltinResource::Crabs,
        R::Citrus => BuiltinResource::Citrus, R::Cocoa => BuiltinResource::Cocoa,
        R::Coffee => BuiltinResource::Coffee, R::Diamonds => BuiltinResource::Diamonds,
        R::Dyes => BuiltinResource::Dyes, R::Furs => BuiltinResource::Furs,
        R::Gypsum => BuiltinResource::Gypsum, R::Jade => BuiltinResource::Jade,
        R::Marble => BuiltinResource::Marble, R::Mercury => BuiltinResource::Mercury,
        R::Pearls => BuiltinResource::Pearls, R::Silver => BuiltinResource::Silver,
        R::Tea => BuiltinResource::Tea, R::Tobacco => BuiltinResource::Tobacco,
        R::Truffles => BuiltinResource::Truffles, R::Whales => BuiltinResource::Whales,
    }
}

fn conv_improvement(i: libciv::world::improvement::BuiltinImprovement) -> BuiltinImprovement {
    use libciv::world::improvement::BuiltinImprovement as I;
    match i {
        I::Farm => BuiltinImprovement::Farm, I::Mine => BuiltinImprovement::Mine,
        I::LumberMill => BuiltinImprovement::LumberMill, I::TradingPost => BuiltinImprovement::TradingPost,
        I::Fort => BuiltinImprovement::Fort, I::Airstrip => BuiltinImprovement::Airstrip,
        I::MissileSilo => BuiltinImprovement::MissileSilo,
        I::Quarry => BuiltinImprovement::Quarry, I::Plantation => BuiltinImprovement::Plantation,
        I::Camp => BuiltinImprovement::Camp, I::FishingBoats => BuiltinImprovement::FishingBoats,
        I::Pasture => BuiltinImprovement::Pasture,
        I::Sphinx => BuiltinImprovement::Sphinx, I::Stepwell => BuiltinImprovement::Stepwell,
        I::OilWell => BuiltinImprovement::OilWell, I::OffshoreOilRig => BuiltinImprovement::OffshoreOilRig,
        I::BeachResort => BuiltinImprovement::BeachResort, I::Chateau => BuiltinImprovement::Chateau,
        I::ColossalHead => BuiltinImprovement::ColossalHead, I::GreatWall => BuiltinImprovement::GreatWall,
        I::Kurgan => BuiltinImprovement::Kurgan, I::Mission => BuiltinImprovement::Mission,
        I::RomanFort => BuiltinImprovement::RomanFort, I::Ziggurat => BuiltinImprovement::Ziggurat,
        I::SolarFarm => BuiltinImprovement::SolarFarm, I::WindFarm => BuiltinImprovement::WindFarm,
        I::OffshoreWindFarm => BuiltinImprovement::OffshoreWindFarm, I::GeothermalPlant => BuiltinImprovement::GeothermalPlant,
        I::Seastead => BuiltinImprovement::Seastead, I::MountainTunnel => BuiltinImprovement::MountainTunnel,
        I::SkiResort => BuiltinImprovement::SkiResort,
    }
}

fn conv_road(r: &libciv::world::road::BuiltinRoad) -> BuiltinRoad {
    use libciv::world::road::BuiltinRoad as R;
    match r {
        R::Ancient(_)    => BuiltinRoad::Ancient,
        R::Medieval(_)   => BuiltinRoad::Medieval,
        R::Industrial(_) => BuiltinRoad::Industrial,
        R::Railroad(_)   => BuiltinRoad::Railroad,
    }
}

fn conv_domain(d: libciv::UnitDomain) -> UnitDomain {
    match d {
        libciv::UnitDomain::Land => UnitDomain::Land,
        libciv::UnitDomain::Sea  => UnitDomain::Sea,
        libciv::UnitDomain::Air  => UnitDomain::Air,
    }
}

fn conv_category(c: libciv::UnitCategory) -> UnitCategory {
    match c {
        libciv::UnitCategory::Civilian    => UnitCategory::Civilian,
        libciv::UnitCategory::Combat      => UnitCategory::Combat,
        libciv::UnitCategory::Support     => UnitCategory::Support,
        libciv::UnitCategory::Religious   => UnitCategory::Religious,
        libciv::UnitCategory::GreatPerson => UnitCategory::GreatPerson,
        libciv::UnitCategory::Trader      => UnitCategory::Trader,
    }
}

fn conv_diplo_status(s: libciv::civ::DiplomaticStatus) -> DiplomaticStatus {
    match s {
        libciv::civ::DiplomaticStatus::War       => DiplomaticStatus::War,
        libciv::civ::DiplomaticStatus::Denounced  => DiplomaticStatus::Denounced,
        libciv::civ::DiplomaticStatus::Neutral    => DiplomaticStatus::Neutral,
        libciv::civ::DiplomaticStatus::Friendly   => DiplomaticStatus::Friendly,
        libciv::civ::DiplomaticStatus::Alliance   => DiplomaticStatus::Alliance,
    }
}

fn conv_wall_level(w: libciv::civ::city::WallLevel) -> WallLevel {
    match w {
        libciv::civ::city::WallLevel::None        => WallLevel::None,
        libciv::civ::city::WallLevel::Ancient      => WallLevel::Ancient,
        libciv::civ::city::WallLevel::Medieval     => WallLevel::Medieval,
        libciv::civ::city::WallLevel::Renaissance  => WallLevel::Renaissance,
    }
}

fn conv_ownership(o: libciv::civ::city::CityOwnership) -> CityOwnership {
    match o {
        libciv::civ::city::CityOwnership::Normal   => CityOwnership::Normal,
        libciv::civ::city::CityOwnership::Occupied  => CityOwnership::Occupied,
        libciv::civ::city::CityOwnership::Puppet    => CityOwnership::Puppet,
        libciv::civ::city::CityOwnership::Razed     => CityOwnership::Razed,
    }
}

fn conv_topology(t: libhexgrid::board::BoardTopology) -> BoardTopology {
    match t {
        libhexgrid::board::BoardTopology::Flat          => BoardTopology::Flat,
        libhexgrid::board::BoardTopology::CylindricalEW => BoardTopology::CylindricalEW,
        libhexgrid::board::BoardTopology::Toroidal      => BoardTopology::Toroidal,
    }
}

fn conv_yields(y: &libciv::YieldBundle) -> YieldBundleView {
    YieldBundleView {
        food: y.food, production: y.production, gold: y.gold,
        science: y.science, culture: y.culture, faith: y.faith,
        housing: y.housing, amenities: y.amenities, tourism: y.tourism,
        great_person_points: y.great_person_points,
    }
}

fn conv_production_item(item: &libciv::civ::ProductionItem) -> ProductionItemView {
    match item {
        libciv::civ::ProductionItem::Unit(id)     => ProductionItemView::Unit(conv_unit_type_id(*id)),
        libciv::civ::ProductionItem::Building(id)  => ProductionItemView::Building(conv_building_id(*id)),
        libciv::civ::ProductionItem::District(d)   => ProductionItemView::District(conv_district(*d)),
        libciv::civ::ProductionItem::Wonder(id)    => ProductionItemView::Wonder(conv_wonder_id(*id)),
        libciv::civ::ProductionItem::Project(id)   => ProductionItemView::Project(conv_project_id(*id)),
    }
}

fn conv_district(d: libciv::civ::district::BuiltinDistrict) -> BuiltinDistrict {
    use libciv::civ::district::BuiltinDistrict as D;
    match d {
        D::Campus               => BuiltinDistrict::Campus,
        D::TheaterSquare         => BuiltinDistrict::TheaterSquare,
        D::CommercialHub         => BuiltinDistrict::CommercialHub,
        D::Harbor                => BuiltinDistrict::Harbor,
        D::HolySite              => BuiltinDistrict::HolySite,
        D::Encampment            => BuiltinDistrict::Encampment,
        D::IndustrialZone        => BuiltinDistrict::IndustrialZone,
        D::EntertainmentComplex  => BuiltinDistrict::EntertainmentComplex,
        D::WaterPark             => BuiltinDistrict::WaterPark,
        D::Aqueduct              => BuiltinDistrict::Aqueduct,
        D::Dam                   => BuiltinDistrict::Dam,
        D::Canal                 => BuiltinDistrict::Canal,
        D::Aerodrome             => BuiltinDistrict::Aerodrome,
        D::Neighborhood          => BuiltinDistrict::Neighborhood,
        D::Spaceport             => BuiltinDistrict::Spaceport,
        D::CityCenter            => BuiltinDistrict::CityCenter,
        D::Lavra                 => BuiltinDistrict::Lavra,
        D::Mbanza                => BuiltinDistrict::Mbanza,
        D::StreetCarnival        => BuiltinDistrict::StreetCarnival,
        D::RoyalNavyDockyard     => BuiltinDistrict::RoyalNavyDockyard,
    }
}

// ── Main projection ──────────────────────────────────────────────────────────

/// Build a `GameView` for `viewer`, filtering by fog-of-war.
pub fn project_game_view(state: &GameState, viewer: CivId) -> GameView {
    let civ = state.civilizations.iter()
        .find(|c| c.id == viewer)
        .expect("viewer civ must exist");

    let rules = DefaultRulesEngine;
    let yields = rules.compute_yields(state, viewer);
    let civ_yields = conv_yields(&yields);

    // ── Board ────────────────────────────────────────────────────────────
    let board_w = state.board.width();
    let board_h = state.board.height();

    let mut tiles = Vec::new();
    for coord in state.board.all_coords() {
        if !civ.explored_tiles.contains(&coord) {
            continue;
        }
        let Some(tile) = state.board.tile(coord) else { continue };
        let visibility = if civ.visible_tiles.contains(&coord) {
            TileVisibility::Visible
        } else {
            TileVisibility::Foggy
        };

        // Resource gating: only show if civ has revealed this resource
        // (via researching the appropriate tech).
        let resource = tile.resource.and_then(|r| {
            if r.reveal_tech().is_none() || civ.revealed_resources.contains(&r) {
                Some(conv_resource(r))
            } else {
                None
            }
        });

        tiles.push(TileView {
            coord: conv_coord(coord),
            terrain: conv_terrain(tile.terrain),
            hills: tile.hills,
            feature: tile.feature.map(conv_feature),
            resource,
            improvement: tile.improvement.map(conv_improvement),
            road: tile.road.as_ref().map(conv_road),
            owner: tile.owner.map(conv_civ_id),
            visibility,
        });
    }

    // River edges: iterate all board edges, collect those with river features.
    let mut river_edges: Vec<(ApiCoord, ApiCoord)> = Vec::new();
    for coord in state.board.all_coords() {
        use libhexgrid::coord::HexDir;
        for dir in [HexDir::E, HexDir::NE, HexDir::NW] {
            if let Some(edge) = state.board.edge(coord, dir)
                && edge.feature.as_ref().is_some_and(|f| matches!(f, libciv::world::edge::BuiltinEdgeFeature::River(_)))
            {
                let (a, b) = edge.endpoints();
                river_edges.push((conv_coord(a), conv_coord(b)));
            }
        }
    }

    let topology = conv_topology(state.board.topology());
    let board = BoardView { width: board_w, height: board_h, topology, tiles, river_edges };

    // ── Own civilization ──────────────────────────────────────────────────
    let my_civ = CivView {
        id: conv_civ_id(civ.id),
        name: civ.name.to_string(),
        adjective: civ.adjective.to_string(),
        leader_name: civ.leader.name.to_string(),
        gold: civ.gold,
        current_era: AgeType::Ancient, // TODO: map from civ.current_era
        researched_techs: civ.researched_techs.iter().map(|t| conv_tech_id(*t)).collect(),
        research_queue: civ.research_queue.iter().map(|tp| TechProgressView {
            tech_id: conv_tech_id(tp.tech_id),
            progress: tp.progress,
            boosted: tp.boosted,
        }).collect(),
        completed_civics: civ.completed_civics.iter().map(|c| conv_civic_id(*c)).collect(),
        civic_in_progress: civ.civic_in_progress.as_ref().map(|cp| CivicProgressView {
            civic_id: conv_civic_id(cp.civic_id),
            progress: cp.progress,
            inspired: cp.inspired,
        }),
        current_government: civ.current_government
            .and_then(|g| state.governments.iter().find(|gov| gov.id == g))
            .map(|g| g.name.to_string()),
        active_policies: civ.active_policies.iter().map(|p| conv_policy_id(*p)).collect(),
        unlocked_units: civ.unlocked_units.iter().map(|s| s.to_string()).collect(),
        unlocked_buildings: civ.unlocked_buildings.iter().map(|s| s.to_string()).collect(),
        unlocked_improvements: civ.unlocked_improvements.iter().map(|s| s.to_string()).collect(),
        strategic_resources: civ.strategic_resources.iter()
            .map(|(r, &qty)| (format!("{r:?}"), qty))
            .collect(),
        yields: civ_yields,
        faith: civ.faith,
        pantheon_belief: civ.pantheon_belief.map(conv_belief_id),
        founded_religion: civ.founded_religion.map(conv_religion_id),
    };

    // ── Other civilizations ──────────────────────────────────────────────
    let other_civs: Vec<PublicCivView> = state.civilizations.iter()
        .filter(|c| c.id != viewer)
        .map(|c| {
            let diplo_status = state.diplomatic_relations.iter()
                .find(|r| (r.civ_a == viewer && r.civ_b == c.id) ||
                          (r.civ_a == c.id && r.civ_b == viewer))
                .map(|r| conv_diplo_status(r.status))
                .unwrap_or(DiplomaticStatus::Neutral);
            PublicCivView {
                id: conv_civ_id(c.id),
                name: c.name.to_string(),
                leader_name: c.leader.name.to_string(),
                score: compute_score(state, c.id),
                diplomatic_status: diplo_status,
            }
        })
        .collect();

    // ── Cities ───────────────────────────────────────────────────────────
    let cities: Vec<CityView> = state.cities.iter()
        .filter(|c| civ.explored_tiles.contains(&c.coord))
        .map(|c| {
            let is_own = c.owner == viewer;
            CityView {
                id: conv_city_id(c.id),
                name: c.name.clone(),
                owner: conv_civ_id(c.owner),
                coord: conv_coord(c.coord),
                is_capital: c.is_capital,
                population: c.population,
                food_stored: c.food_stored,
                food_to_grow: c.food_to_grow,
                production_stored: c.production_stored,
                production_queue: c.production_queue.iter().map(conv_production_item).collect(),
                buildings: c.buildings.iter().map(|b| conv_building_id(*b)).collect(),
                worked_tiles: c.worked_tiles.iter().map(|t| conv_coord(*t)).collect(),
                territory: c.territory.iter().map(|t| conv_coord(*t)).collect(),
                ownership: conv_ownership(c.ownership),
                walls: conv_wall_level(c.walls),
                religious_followers: c.religious_followers.iter()
                    .map(|(&rid, &count)| (conv_religion_id(rid), count))
                    .collect(),
                majority_religion: c.majority_religion().map(conv_religion_id),
                is_own,
            }
        })
        .collect();

    // ── Units (only visible) ─────────────────────────────────────────────
    let units: Vec<UnitView> = state.units.iter()
        .filter(|u| civ.visible_tiles.contains(&u.coord))
        .map(|u| UnitView {
            id: conv_unit_id(u.id),
            unit_type: conv_unit_type_id(u.unit_type),
            owner: conv_civ_id(u.owner),
            coord: conv_coord(u.coord),
            domain: conv_domain(u.domain),
            category: conv_category(u.category),
            movement_left: u.movement_left,
            max_movement: u.max_movement,
            combat_strength: u.combat_strength,
            health: u.health,
            range: u.range,
            vision_range: u.vision_range,
            is_own: u.owner == viewer,
        })
        .collect();

    // ── Tech tree ────────────────────────────────────────────────────────
    let tech_tree = TechTreeView {
        nodes: state.tech_tree.nodes.values().map(|n| TechNodeView {
            id: conv_tech_id(n.id),
            name: n.name.to_string(),
            cost: n.cost,
            prerequisites: n.prerequisites.iter().map(|t| conv_tech_id(*t)).collect(),
            eureka_description: n.eureka_description.to_string(),
        }).collect(),
    };

    let civic_tree = CivicTreeView {
        nodes: state.civic_tree.nodes.values().map(|n| CivicNodeView {
            id: conv_civic_id(n.id),
            name: n.name.to_string(),
            cost: n.cost,
            prerequisites: n.prerequisites.iter().map(|c| conv_civic_id(*c)).collect(),
            inspiration_description: n.inspiration_description.to_string(),
        }).collect(),
    };

    // ── Trade routes ─────────────────────────────────────────────────────
    let trade_routes: Vec<TradeRouteView> = state.trade_routes.iter()
        .filter(|tr| tr.owner == viewer)
        .map(|tr| TradeRouteView {
            id: conv_trade_route_id(tr.id),
            origin: conv_city_id(tr.origin),
            destination: conv_city_id(tr.destination),
            owner: conv_civ_id(tr.owner),
            origin_yields: conv_yields(&tr.origin_yields),
            destination_yields: conv_yields(&tr.destination_yields),
            turns_remaining: tr.turns_remaining,
        })
        .collect();

    // ── Unit type defs ───────────────────────────────────────────────────
    let unit_type_defs: Vec<UnitTypeDefView> = state.unit_type_defs.iter().map(|d| {
        UnitTypeDefView {
            id: conv_unit_type_id(d.id),
            name: d.name.to_string(),
            production_cost: d.production_cost,
            domain: conv_domain(d.domain),
            category: conv_category(d.category),
            max_movement: d.max_movement,
            combat_strength: d.combat_strength,
            range: d.range,
            vision_range: d.vision_range,
            can_found_city: d.can_found_city,
            resource_cost: d.resource_cost.map(|(r, q)| (conv_resource(r), q)),
        }
    }).collect();

    let building_defs: Vec<BuildingDefView> = state.building_defs.iter().map(|d| {
        BuildingDefView {
            id: conv_building_id(d.id),
            name: d.name.to_string(),
            cost: d.cost,
            maintenance: d.maintenance,
            yields: conv_yields(&d.yields),
        }
    }).collect();

    // ── Scores ───────────────────────────────────────────────────────────
    let scores: Vec<(api::CivId, u32)> = all_scores(state).into_iter()
        .map(|(cid, s)| (conv_civ_id(cid), s))
        .collect();

    // ── Game over ────────────────────────────────────────────────────────
    let game_over = state.game_over.as_ref().map(|go| GameOverView {
        winner: conv_civ_id(go.winner),
        condition: go.condition.to_string(),
        turn: go.turn,
    });

    GameView {
        turn: state.turn,
        my_civ_id: conv_civ_id(viewer),
        board,
        my_civ,
        other_civs,
        cities,
        units,
        tech_tree,
        civic_tree,
        trade_routes,
        unit_type_defs,
        building_defs,
        scores,
        religions: state.religions.iter().map(|r| {
            ReligionView {
                id: conv_religion_id(r.id),
                name: r.name.clone(),
                founded_by: conv_civ_id(r.founded_by),
                holy_city: conv_city_id(r.holy_city),
                beliefs: r.beliefs.iter().filter_map(|bid| {
                    state.belief_defs.iter().find(|b| b.id == *bid).map(|b| {
                        BeliefView {
                            id: conv_belief_id(b.id),
                            name: b.name.to_string(),
                            description: b.description.to_string(),
                            category: match b.category {
                                libciv::civ::religion::BeliefCategory::Pantheon => BeliefCategory::Pantheon,
                                libciv::civ::religion::BeliefCategory::Founder => BeliefCategory::Founder,
                                libciv::civ::religion::BeliefCategory::Follower => BeliefCategory::Follower,
                                libciv::civ::religion::BeliefCategory::Worship => BeliefCategory::Worship,
                                libciv::civ::religion::BeliefCategory::Enhancer => BeliefCategory::Enhancer,
                            },
                        }
                    })
                }).collect(),
                total_followers: r.total_followers(&state.cities),
            }
        }).collect(),
        game_over,
    }
}
