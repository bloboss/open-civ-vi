//! Phase 5 -- Resource placement.
//!
//! Places resources on tiles according to per-zone density quotas.
//! Strategic resources are placed first (rarest, most constrained),
//! then Luxury, then Bonus.

use std::collections::HashMap;

use rand::seq::SliceRandom;
use rand::rngs::SmallRng;

use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::game::board::WorldBoard;
use crate::world::feature::BuiltinFeature;
use crate::world::resource::BuiltinResource;
use crate::world::terrain::BuiltinTerrain;

use super::zones::EcoZone;
use super::features::is_land;

// ---------------------------------------------------------------------------
// Resource descriptor
// ---------------------------------------------------------------------------

/// Placement rule for one resource type.
struct ResourceDef {
    resource:      BuiltinResource,
    /// Which zones this resource appears in.
    zones:         &'static [EcoZone],
    /// Valid terrains (empty = any land).
    terrains:      &'static [BuiltinTerrain],
    /// If true, the resource prefers hills tiles (added again in candidate
    /// list to double the probability).
    prefers_hills: bool,
    /// If Some(f), only place on tiles that have this feature.
    requires_feature: Option<BuiltinFeature>,
    /// If true, only place on non-land (Coast/Ocean) tiles.
    water_only:    bool,
    /// 1 resource per this many eligible tiles.
    density:       u32,
}

const RESOURCES: &[ResourceDef] = &[
    // ---- Strategic ---------------------------------------------------------
    ResourceDef {
        resource: BuiltinResource::Horses,
        zones: &[EcoZone::Temperate, EcoZone::Tundra, EcoZone::Desert],
        terrains: &[BuiltinTerrain::Grassland, BuiltinTerrain::Plains],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 30,
    },
    ResourceDef {
        resource: BuiltinResource::Iron,
        zones: &[EcoZone::Temperate, EcoZone::Tundra, EcoZone::Desert],
        terrains: &[],
        prefers_hills: true, requires_feature: None, water_only: false,
        density: 30,
    },
    ResourceDef {
        resource: BuiltinResource::Coal,
        zones: &[EcoZone::Temperate],
        terrains: &[],
        prefers_hills: true, requires_feature: None, water_only: false,
        density: 35,
    },
    ResourceDef {
        resource: BuiltinResource::Oil,
        zones: &[EcoZone::Desert],
        terrains: &[BuiltinTerrain::Desert],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 40,
    },
    ResourceDef {
        resource: BuiltinResource::Aluminum,
        zones: &[EcoZone::Desert],
        terrains: &[],
        prefers_hills: true, requires_feature: None, water_only: false,
        density: 40,
    },
    ResourceDef {
        resource: BuiltinResource::Niter,
        zones: &[EcoZone::Desert, EcoZone::Tundra],
        terrains: &[],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 40,
    },
    ResourceDef {
        resource: BuiltinResource::Uranium,
        zones: &[EcoZone::Tundra, EcoZone::Temperate],
        terrains: &[],
        prefers_hills: true, requires_feature: None, water_only: false,
        density: 35,
    },
    // ---- Luxury ------------------------------------------------------------
    ResourceDef {
        resource: BuiltinResource::Wine,
        zones: &[EcoZone::Temperate],
        terrains: &[BuiltinTerrain::Plains],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 20,
    },
    ResourceDef {
        resource: BuiltinResource::Silk,
        zones: &[EcoZone::Temperate, EcoZone::Tropical],
        terrains: &[],
        prefers_hills: false,
        requires_feature: Some(BuiltinFeature::Forest),
        water_only: false,
        density: 20,
    },
    ResourceDef {
        resource: BuiltinResource::Spices,
        zones: &[EcoZone::Tropical],
        terrains: &[],
        prefers_hills: false,
        requires_feature: Some(BuiltinFeature::Rainforest),
        water_only: false,
        density: 18,
    },
    ResourceDef {
        resource: BuiltinResource::Incense,
        zones: &[EcoZone::Desert, EcoZone::Tropical],
        terrains: &[BuiltinTerrain::Desert, BuiltinTerrain::Plains],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 25,
    },
    ResourceDef {
        resource: BuiltinResource::Cotton,
        zones: &[EcoZone::Tropical, EcoZone::Temperate],
        terrains: &[BuiltinTerrain::Grassland, BuiltinTerrain::Plains],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 20,
    },
    ResourceDef {
        resource: BuiltinResource::Ivory,
        zones: &[EcoZone::Tropical, EcoZone::Desert],
        terrains: &[],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 25,
    },
    ResourceDef {
        resource: BuiltinResource::Sugar,
        zones: &[EcoZone::Tropical],
        terrains: &[BuiltinTerrain::Grassland],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 18,
    },
    ResourceDef {
        resource: BuiltinResource::Salt,
        zones: &[EcoZone::Desert, EcoZone::Tundra],
        terrains: &[],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 25,
    },
    // ---- Bonus -------------------------------------------------------------
    ResourceDef {
        resource: BuiltinResource::Wheat,
        zones: &[EcoZone::Temperate, EcoZone::Tropical],
        terrains: &[BuiltinTerrain::Grassland, BuiltinTerrain::Plains],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 8,
    },
    ResourceDef {
        resource: BuiltinResource::Rice,
        zones: &[EcoZone::Tropical, EcoZone::Temperate],
        terrains: &[BuiltinTerrain::Grassland],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 8,
    },
    ResourceDef {
        resource: BuiltinResource::Cattle,
        zones: &[EcoZone::Temperate, EcoZone::Tundra],
        terrains: &[BuiltinTerrain::Grassland, BuiltinTerrain::Plains],
        prefers_hills: false, requires_feature: None, water_only: false,
        density: 8,
    },
    ResourceDef {
        resource: BuiltinResource::Sheep,
        zones: &[EcoZone::Temperate, EcoZone::Tundra, EcoZone::Desert],
        terrains: &[],
        prefers_hills: true, requires_feature: None, water_only: false,
        density: 8,
    },
    ResourceDef {
        resource: BuiltinResource::Fish,
        zones: &[EcoZone::Polar, EcoZone::Tundra, EcoZone::Temperate,
                 EcoZone::Desert, EcoZone::Tropical],
        terrains: &[BuiltinTerrain::Coast, BuiltinTerrain::Ocean],
        prefers_hills: false, requires_feature: None, water_only: true,
        density: 10,
    },
    ResourceDef {
        resource: BuiltinResource::Stone,
        zones: &[EcoZone::Temperate, EcoZone::Tundra],
        terrains: &[],
        prefers_hills: true, requires_feature: None, water_only: false,
        density: 12,
    },
    ResourceDef {
        resource: BuiltinResource::Copper,
        zones: &[EcoZone::Temperate, EcoZone::Desert],
        terrains: &[],
        prefers_hills: true, requires_feature: None, water_only: false,
        density: 12,
    },
    ResourceDef {
        resource: BuiltinResource::Deer,
        zones: &[EcoZone::Temperate, EcoZone::Tundra],
        terrains: &[],
        prefers_hills: false,
        requires_feature: Some(BuiltinFeature::Forest),
        water_only: false,
        density: 12,
    },
];

// ---------------------------------------------------------------------------
// Phase entry point
// ---------------------------------------------------------------------------

pub fn generate(
    board:    &mut WorldBoard,
    zone_map: &HashMap<HexCoord, EcoZone>,
    rng:      &mut SmallRng,
) {
    for def in RESOURCES {
        place_resource(def, board, zone_map, rng);
    }
}

// ---------------------------------------------------------------------------
// Placement for a single resource type
// ---------------------------------------------------------------------------

fn place_resource(
    def:      &ResourceDef,
    board:    &mut WorldBoard,
    zone_map: &HashMap<HexCoord, EcoZone>,
    rng:      &mut SmallRng,
) {
    // Build candidate list.
    let mut candidates: Vec<HexCoord> = Vec::new();

    for coord in board.all_coords() {
        let Some(tile) = board.tile(coord) else { continue };

        // Already occupied.
        if tile.resource.is_some() { continue; }

        // Mountain tiles never get resources.
        if tile.terrain == BuiltinTerrain::Mountain { continue; }

        // Zone check.
        let zone = zone_map.get(&coord).copied().unwrap_or(EcoZone::Temperate);
        if !def.zones.contains(&zone) { continue; }

        // Water vs land check.
        let tile_is_water = !is_land(tile.terrain);
        if def.water_only  && !tile_is_water { continue; }
        if !def.water_only &&  tile_is_water { continue; }

        // Terrain check.
        if !def.terrains.is_empty() && !def.terrains.contains(&tile.terrain) { continue; }

        // Feature check.
        if let Some(req_feat) = def.requires_feature
            && tile.feature != Some(req_feat) { continue; }

        candidates.push(coord);
        if def.prefers_hills && tile.hills {
            candidates.push(coord); // 2x weight
        }
    }

    if candidates.is_empty() { return; }

    candidates.shuffle(rng);

    // Count eligible tiles for quota calculation.
    let eligible = candidates.len();
    let quota    = ((eligible as f32 / def.density as f32).ceil() as usize).max(1);

    let min_sep: u32 = 2;
    let mut placed: Vec<HexCoord> = Vec::new();

    for &coord in &candidates {
        if placed.len() >= quota { break; }
        let too_close = placed.iter().any(|p: &HexCoord| p.distance(&coord) < min_sep);
        if too_close { continue; }

        // Re-check that the tile still has no resource (might have been
        // placed by a different resource type earlier in this pass).
        if board.tile(coord).map(|t| t.resource.is_some()).unwrap_or(true) { continue; }

        if let Some(tile) = board.tile_mut(coord) {
            tile.resource = Some(def.resource);
            placed.push(coord);
        }
    }
}
