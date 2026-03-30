//! Production handlers: `place_improvement`, `place_road`, `place_district`.

use crate::{CityId, CivId, UnitId};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::HexTile;

use super::RulesError;
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;
use super::movement::decrement_builder_charges;

/// Place an improvement on `coord`.
pub(crate) fn place_improvement(
    state: &mut GameState,
    civ_id: CivId,
    coord: HexCoord,
    improvement: crate::world::improvement::BuiltinImprovement,
    builder: Option<UnitId>,
) -> Result<GameStateDiff, RulesError> {
    use crate::world::improvement::{ElevationReq, ProximityReq};
    use libhexgrid::types::Elevation;

    let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

    // Validate builder if provided.
    if let Some(uid) = builder {
        let unit = state.unit(uid).ok_or(RulesError::UnitNotFound)?;
        if unit.charges.is_none() {
            return Err(RulesError::NotABuilder);
        }
        if unit.charges == Some(0) {
            return Err(RulesError::NoChargesRemaining);
        }
        if unit.coord != coord {
            return Err(RulesError::InvalidCoord);
        }
    }

    let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
    let req  = improvement.requirements(&state.tech_refs, &state.civic_refs);

    if tile.owner != Some(civ_id) {
        return Err(RulesError::TileNotOwned);
    }

    if req.requires_land && !tile.terrain.is_land() {
        return Err(RulesError::InvalidImprovement);
    }
    if req.requires_water && tile.terrain.is_land() {
        return Err(RulesError::InvalidImprovement);
    }

    let elev = tile.elevation();
    let elev_ok = match req.elevation {
        ElevationReq::Any         => true,
        ElevationReq::Flat        => elev < Elevation::HILLS,
        ElevationReq::HillsOrMore => elev >= Elevation::HILLS && elev != Elevation::High,
        ElevationReq::NotMountain => elev != Elevation::High,
    };
    if !elev_ok {
        return Err(RulesError::InvalidImprovement);
    }

    if req.blocked_terrains.contains(&tile.terrain) {
        return Err(RulesError::InvalidImprovement);
    }

    if let Some(req_feat) = req.required_feature
        && tile.feature != Some(req_feat)
    {
        return Err(RulesError::InvalidImprovement);
    }

    let terrain = tile.terrain;
    let feature = tile.feature;
    for &(cond_terrain, allowed_features) in req.conditional_features {
        if terrain == cond_terrain {
            let ok = feature.is_some_and(|f| allowed_features.contains(&f));
            if !ok {
                return Err(RulesError::InvalidImprovement);
            }
        }
    }

    if let Some(req_res) = req.required_resource
        && tile.resource != Some(req_res)
    {
        return Err(RulesError::ResourceRequired);
    }

    if let Some(prox) = req.proximity {
        let ok = state.board.neighbors(coord).iter().any(|&nb| {
            state.board.tile(nb).is_some_and(|t| match prox {
                ProximityReq::AdjacentTerrain(tt) => t.terrain == tt,
                ProximityReq::AdjacentFeature(f)  => t.feature == Some(f),
                ProximityReq::AdjacentResource(r) => t.resource == Some(r),
            })
        });
        if !ok {
            return Err(RulesError::ProximityRequired);
        }
    }

    let civ = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .ok_or(RulesError::CivNotFound)?;

    if let Some(tech_id) = req.required_tech
        && !civ.researched_techs.contains(&tech_id)
    {
        return Err(RulesError::TechRequired);
    }

    if let Some(civic_id) = req.required_civic
        && !civ.completed_civics.contains(&civic_id)
    {
        return Err(RulesError::CivicRequired);
    }

    if let Some(tile) = state.board.tile_mut(coord) {
        tile.improvement = Some(improvement);
        tile.improvement_pillaged = false;
    }

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::ImprovementPlaced { coord, improvement });

    if let Some(uid) = builder {
        decrement_builder_charges(state, uid, &mut diff);
    }

    Ok(diff)
}

/// Place a road on `coord` using a builder unit.
pub(crate) fn place_road(
    state: &mut GameState,
    unit_id: UnitId,
    coord: HexCoord,
    road: crate::world::road::BuiltinRoad,
) -> Result<GameStateDiff, RulesError> {
    let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

    let unit = state.unit(unit_id).ok_or(RulesError::UnitNotFound)?;
    if unit.charges.is_none() {
        return Err(RulesError::NotABuilder);
    }
    if unit.charges == Some(0) {
        return Err(RulesError::NoChargesRemaining);
    }
    if unit.coord != coord {
        return Err(RulesError::InvalidCoord);
    }
    let civ_id = unit.owner;

    let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
    if !tile.terrain.is_land() {
        return Err(RulesError::InvalidImprovement);
    }
    if tile.owner != Some(civ_id) {
        return Err(RulesError::TileNotOwned);
    }

    if let Some(existing) = &tile.road
        && road.tier() <= existing.tier()
    {
        return Err(RulesError::RoadDowngrade);
    }

    if let Some(tech_name) = road.required_tech() {
        let civ = state.civilizations.iter()
            .find(|c| c.id == civ_id)
            .ok_or(RulesError::CivNotFound)?;
        let has_tech = civ.researched_techs.iter().any(|&tid| {
            state.tech_tree.get(tid)
                .map(|node| node.name == tech_name)
                .unwrap_or(false)
        });
        if !has_tech {
            return Err(RulesError::TechRequired);
        }
    }

    if let Some(tile) = state.board.tile_mut(coord) {
        tile.road = Some(road);
    }

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::RoadPlaced { coord, road });

    decrement_builder_charges(state, unit_id, &mut diff);

    Ok(diff)
}

/// Place a district for `city_id` at `coord`.
pub(crate) fn place_district(
    state: &mut GameState,
    city_id: CityId,
    district: crate::civ::district::BuiltinDistrict,
    coord: HexCoord,
) -> Result<GameStateDiff, RulesError> {
    let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

    let (city_coord, civ_id) = state.cities.iter()
        .find(|c| c.id == city_id)
        .map(|c| (c.coord, c.owner))
        .ok_or(RulesError::CityNotFound)?;

    let dist = city_coord.distance(&coord);
    if !(1..=3).contains(&dist) {
        return Err(RulesError::TileNotInCityRange);
    }

    let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
    if tile.owner != Some(civ_id) {
        return Err(RulesError::TileNotOwned);
    }

    if state.placed_districts.iter().any(|d| d.coord == coord) {
        return Err(RulesError::TileOccupiedByDistrict);
    }

    if state.cities.iter().any(|c| c.coord == coord) {
        return Err(RulesError::TileOccupied);
    }

    let already_has = state.cities.iter()
        .find(|c| c.id == city_id)
        .is_some_and(|c| c.districts.contains(&district));
    if already_has {
        return Err(RulesError::DistrictAlreadyPresent);
    }

    let req = district.requirements(&state.tech_refs, &state.civic_refs);

    if req.requires_land && !tile.terrain.is_land() {
        return Err(RulesError::InvalidDistrict);
    }
    if req.requires_water && tile.terrain.is_land() {
        return Err(RulesError::InvalidDistrict);
    }

    if req.forbidden_terrains.contains(&tile.terrain) {
        return Err(RulesError::InvalidDistrict);
    }

    if let Some(tech_id) = req.required_tech {
        let civ = state.civilizations.iter()
            .find(|c| c.id == civ_id)
            .ok_or(RulesError::CivNotFound)?;
        if !civ.researched_techs.contains(&tech_id) {
            return Err(RulesError::TechRequired);
        }
    }

    if let Some(civic_id) = req.required_civic {
        let civ = state.civilizations.iter()
            .find(|c| c.id == civ_id)
            .ok_or(RulesError::CivNotFound)?;
        if !civ.completed_civics.contains(&civic_id) {
            return Err(RulesError::CivicRequired);
        }
    }

    state.placed_districts.push(crate::civ::district::PlacedDistrict::new(
        district, city_id, coord,
    ));
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        city.districts.push(district);
    }

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::DistrictBuilt { city: city_id, district, coord });
    Ok(diff)
}
