use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

use libhexgrid::board::{BoardTopology, HexBoard};
use libhexgrid::coord::{HexCoord, HexDir};
use libhexgrid::types::{Elevation, MovementCost};
use libhexgrid::{HexEdge, HexTile};
use crate::world::edge::WorldEdge;
use crate::world::terrain::BuiltinTerrain;
use crate::world::tile::WorldTile;

/// Concrete hex board for the world map.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldBoard {
    pub width: u32,
    pub height: u32,
    topology: BoardTopology,
    /// Tiles stored in row-major order: index = r * width + q (offset coords).
    tiles: Vec<WorldTile>,
    /// Edges keyed by canonical (HexCoord, HexDir) pair.
    /// Forward-half directions only: {E, NE, NW}.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_hashmap_as_vec"))]
    edges: HashMap<(HexCoord, HexDir), WorldEdge>,
}

impl WorldBoard {
    pub fn new(width: u32, height: u32) -> Self {
        let mut tiles = Vec::with_capacity((width * height) as usize);
        for r in 0..height as i32 {
            for q in 0..width as i32 {
                let coord = HexCoord::from_qr(q, r);
                tiles.push(WorldTile::new(coord, BuiltinTerrain::Grassland));
            }
        }
        Self {
            width,
            height,
            topology: BoardTopology::CylindricalEW,
            tiles,
            edges: HashMap::new(),
        }
    }

    fn coord_to_index(&self, coord: HexCoord) -> Option<usize> {
        let q = coord.q;
        let r = coord.r;
        if r < 0 || r >= self.height as i32 {
            return None;
        }
        let q_wrapped = match self.topology {
            BoardTopology::CylindricalEW | BoardTopology::Toroidal => {
                q.rem_euclid(self.width as i32)
            }
            BoardTopology::Flat => {
                if q < 0 || q >= self.width as i32 {
                    return None;
                }
                q
            }
        };
        Some((r as u32 * self.width + q_wrapped as u32) as usize)
    }

    fn normalize_coord(&self, coord: HexCoord) -> Option<HexCoord> {
        let r = coord.r;
        if r < 0 || r >= self.height as i32 {
            return None;
        }
        let q = match self.topology {
            BoardTopology::CylindricalEW | BoardTopology::Toroidal => {
                coord.q.rem_euclid(self.width as i32)
            }
            BoardTopology::Flat => {
                if coord.q < 0 || coord.q >= self.width as i32 {
                    return None;
                }
                coord.q
            }
        };
        Some(HexCoord::from_qr(q, r))
    }

    /// Canonicalize an edge reference to its forward-half form.
    /// Forward half: {E, NE, NW}. Backward half: {W, SW, SE} → step to neighbor, flip.
    fn canonical(coord: HexCoord, dir: HexDir) -> (HexCoord, HexDir) {
        match dir {
            HexDir::E | HexDir::NE | HexDir::NW => (coord, dir),
            backward => (coord + backward.unit_vec(), backward.opposite()),
        }
    }

    /// Insert an edge into the board, canonicalizing its direction automatically.
    pub fn set_edge(&mut self, coord: HexCoord, dir: HexDir, edge: WorldEdge) {
        let (canon_coord, canon_dir) = Self::canonical(coord, dir);
        self.edges.insert((canon_coord, canon_dir), edge);
    }

    /// Dijkstra pathfinding. Returns `Some(path)` or `None` if unreachable.
    ///
    /// `terrain_passable` is an optional filter: if provided, tiles where it
    /// returns `false` are treated as impassable. Use this to enforce domain
    /// restrictions (e.g. land units cannot enter water tiles).
    pub fn find_path(
        &self,
        start: HexCoord,
        goal: HexCoord,
        movement_budget: u32,
    ) -> Option<Vec<HexCoord>> {
        self.find_path_filtered(start, goal, movement_budget, |_| true)
    }

    /// Dijkstra pathfinding for land units: excludes water tiles (Coast, Ocean).
    pub fn find_path_land(
        &self,
        start: HexCoord,
        goal: HexCoord,
        movement_budget: u32,
    ) -> Option<Vec<HexCoord>> {
        self.find_path_filtered(start, goal, movement_budget, |t| t.terrain.is_land())
    }

    /// Dijkstra pathfinding for sea units: excludes land tiles.
    pub fn find_path_sea(
        &self,
        start: HexCoord,
        goal: HexCoord,
        movement_budget: u32,
    ) -> Option<Vec<HexCoord>> {
        self.find_path_filtered(start, goal, movement_budget, |t| t.terrain.is_water())
    }

    /// Dijkstra pathfinding with a tile filter predicate.
    fn find_path_filtered(
        &self,
        start: HexCoord,
        goal: HexCoord,
        movement_budget: u32,
        tile_passable: impl Fn(&WorldTile) -> bool,
    ) -> Option<Vec<HexCoord>> {
        let start = self.normalize_coord(start)?;
        let goal = self.normalize_coord(goal)?;

        let mut dist: HashMap<HexCoord, u32> = HashMap::new();
        let mut prev: HashMap<HexCoord, HexCoord> = HashMap::new();
        let mut heap: BinaryHeap<(Reverse<u32>, HexCoord)> = BinaryHeap::new();

        dist.insert(start, 0);
        heap.push((Reverse(0), start));

        while let Some((Reverse(cost), coord)) = heap.pop() {
            if coord == goal {
                break;
            }
            if cost > *dist.get(&coord).unwrap_or(&u32::MAX) {
                continue;
            }
            if cost > movement_budget {
                break;
            }

            for dir in HexDir::ALL {
                let neighbor_raw = coord + dir.unit_vec();
                let Some(neighbor) = self.normalize_coord(neighbor_raw) else { continue };
                let Some(tile) = self.tile(neighbor) else { continue };

                // Domain filter: skip tiles incompatible with the unit's domain.
                if !tile_passable(tile) {
                    continue;
                }

                // Roads override terrain movement cost when present.
                let base_cost = match tile.road.as_ref() {
                    Some(road) => road.as_def().movement_cost(),
                    None       => tile.movement_cost(),
                };
                let tile_cost = match base_cost {
                    MovementCost::Impassable => continue,
                    MovementCost::Cost(c)    => c,
                };

                let edge_cost = self
                    .edge(coord, dir)
                    .map(|e| match e.crossing_cost() {
                        MovementCost::Impassable => u32::MAX,
                        MovementCost::Cost(c) => c,
                    })
                    .unwrap_or(0);

                if edge_cost == u32::MAX {
                    continue;
                }

                let next_cost = cost + tile_cost + edge_cost;
                let prev_best = *dist.get(&neighbor).unwrap_or(&u32::MAX);
                if next_cost < prev_best {
                    dist.insert(neighbor, next_cost);
                    prev.insert(neighbor, coord);
                    heap.push((Reverse(next_cost), neighbor));
                }
            }
        }

        let goal_cost = dist.get(&goal).copied().unwrap_or(u32::MAX);
        if goal_cost > movement_budget {
            return None;
        }

        let mut path = vec![goal];
        let mut cur = goal;
        while cur != start {
            cur = *prev.get(&cur)?;
            path.push(cur);
        }
        path.reverse();
        Some(path)
    }

    /// Line-of-sight check. Returns true if `from` can see `to`.
    ///
    /// Blocking rules:
    /// 1. `Elevation::High` (Mountain) at any intermediate tile always blocks.
    /// 2. An intermediate tile blocks if its elevation is strictly above
    ///    `max(from_elev, to_elev)` — i.e., above both endpoints.
    /// 3. Cliff detection: if any consecutive tile pair along the ray (including
    ///    the from→first and last→to steps) has an elevation level difference > 1,
    ///    LOS is blocked (emergent cliff from topology).
    pub fn has_los(&self, from: HexCoord, to: HexCoord) -> bool {
        let Some(from) = self.normalize_coord(from) else { return false };
        let Some(to)   = self.normalize_coord(to)   else { return false };

        if from == to {
            return true;
        }

        let from_elev = self.tile(from).map(|t| t.elevation()).unwrap_or(Elevation::FLAT);
        let to_elev   = self.tile(to).map(|t| t.elevation()).unwrap_or(Elevation::FLAT);
        let max_elev  = from_elev.max(to_elev);

        let dist = from.distance(&to) as i32;

        // Helper: numeric level for cliff diff checks (High = 255).
        let level = |e: Elevation| -> i32 {
            match e {
                Elevation::Low      => -1,
                Elevation::Level(n) => n as i32,
                Elevation::High     => 255,
            }
        };

        let mut prev_level = level(from_elev);

        for step in 1..dist {
            let frac_q = from.q as f32 + (to.q - from.q) as f32 * step as f32 / dist as f32;
            let frac_r = from.r as f32 + (to.r - from.r) as f32 * step as f32 / dist as f32;
            let mid = hex_round(frac_q, frac_r);
            let Some(mid) = self.normalize_coord(mid) else { continue };

            let mid_elev = self.tile(mid).map(|t| t.elevation()).unwrap_or(Elevation::FLAT);

            // Rule 1: Mountain always blocks.
            if mid_elev == Elevation::High {
                return false;
            }

            // Rule 2: Intermediate tile rises above both endpoints.
            if mid_elev > max_elev {
                return false;
            }

            // Rule 3: Cliff — elevation jump > 1 from previous tile.
            let mid_level = level(mid_elev);
            if (mid_level - prev_level).abs() > 1 {
                return false;
            }

            prev_level = mid_level;
        }

        // Rule 3 applied to the final step (last intermediate → destination).
        if (level(to_elev) - prev_level).abs() > 1 {
            return false;
        }

        true
    }
}

fn hex_round(frac_q: f32, frac_r: f32) -> HexCoord {
    let frac_s = -frac_q - frac_r;
    let mut q = frac_q.round() as i32;
    let mut r = frac_r.round() as i32;
    let mut s = frac_s.round() as i32;

    let q_diff = (q as f32 - frac_q).abs();
    let r_diff = (r as f32 - frac_r).abs();
    let s_diff = (s as f32 - frac_s).abs();

    if q_diff > r_diff && q_diff > s_diff {
        q = -r - s;
    } else if r_diff > s_diff {
        r = -q - s;
    } else {
        s = -q - r;
        let _ = s;
    }

    HexCoord::from_qr(q, r)
}

impl HexBoard for WorldBoard {
    type Tile = WorldTile;
    type Edge = WorldEdge;

    fn topology(&self) -> BoardTopology { self.topology }
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }

    fn tile(&self, coord: HexCoord) -> Option<&WorldTile> {
        let idx = self.coord_to_index(coord)?;
        self.tiles.get(idx)
    }

    fn tile_mut(&mut self, coord: HexCoord) -> Option<&mut WorldTile> {
        let idx = self.coord_to_index(coord)?;
        self.tiles.get_mut(idx)
    }

    fn edge(&self, coord: HexCoord, dir: HexDir) -> Option<&WorldEdge> {
        let coord = self.normalize_coord(coord)?;
        let (canon_coord, canon_dir) = Self::canonical(coord, dir);
        self.edges.get(&(canon_coord, canon_dir))
    }

    fn neighbors(&self, coord: HexCoord) -> Vec<HexCoord> {
        coord
            .neighbors()
            .into_iter()
            .filter_map(|n| self.normalize_coord(n))
            .collect()
    }

    fn normalize(&self, coord: HexCoord) -> Option<HexCoord> {
        self.normalize_coord(coord)
    }

    fn all_coords(&self) -> Vec<HexCoord> {
        self.tiles.iter().map(|t| t.coord).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::feature::BuiltinFeature;
    use crate::world::edge::{BuiltinEdgeFeature, River};

    fn small_board() -> WorldBoard {
        WorldBoard::new(10, 10)
    }

    #[test]
    fn test_wraparound_east_west() {
        let board = small_board();
        let east_edge = HexCoord::from_qr(9, 2);
        let step_east = east_edge + HexDir::E.unit_vec();
        let normalized = board.normalize(step_east).expect("should wrap");
        assert_eq!(normalized.q, 0);
    }

    #[test]
    fn test_edge_lookup_both_directions() {
        let mut board = small_board();
        let a = HexCoord::from_qr(3, 3);
        let river = WorldEdge::new(a, HexDir::E).with_feature(BuiltinEdgeFeature::River(River));
        board.set_edge(a, HexDir::E, river);
        // Forward lookup
        assert!(board.edge(a, HexDir::E).is_some());
        // Reverse lookup (W from the neighbour) must find the same edge
        let b = HexCoord::from_qr(4, 3);
        assert!(board.edge(b, HexDir::W).is_some());
        assert_eq!(
            board.edge(a, HexDir::E).unwrap().crossing_cost(),
            board.edge(b, HexDir::W).unwrap().crossing_cost(),
        );
    }

    #[test]
    fn test_los_blocked_by_high() {
        let mut board = small_board();
        // Place a Mountain (Elevation::High) between from and to.
        let mid = HexCoord::from_qr(5, 5);
        if let Some(t) = board.tile_mut(mid) {
            t.terrain = crate::world::terrain::BuiltinTerrain::Mountain;
        }
        // from and to are both at Elevation::FLAT (Grassland).
        // The Mountain in between must block line-of-sight.
        let from = HexCoord::from_qr(3, 5);
        let to   = HexCoord::from_qr(7, 5);
        assert!(!board.has_los(from, to), "mountain should block LOS");

        // Sanity check: unobstructed sightline on the same row works.
        let clear_from = HexCoord::from_qr(0, 0);
        let clear_to   = HexCoord::from_qr(2, 0);
        assert!(board.has_los(clear_from, clear_to), "clear path should have LOS");
    }

    #[test]
    fn test_dijkstra_avoids_impassable() {
        let mut board = WorldBoard::new(10, 10);
        let blocker = HexCoord::from_qr(3, 3);
        if let Some(t) = board.tile_mut(blocker) {
            t.feature = Some(BuiltinFeature::Ice);
        }
        let start = HexCoord::from_qr(2, 3);
        let goal = HexCoord::from_qr(5, 3);
        let path = board.find_path(start, goal, 10_000);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(!path.contains(&blocker), "path must not pass through impassable tile");
    }

    #[test]
    fn test_dijkstra_prefers_roads() {
        use crate::world::road::{AncientRoad, BuiltinRoad};

        // A 5×3 board. All tiles start as Grassland (movement cost ONE = 100).
        // start=(0,1), goal=(2,1). Direct path: (0,1)→(1,1)→(2,1).
        //
        // Without road: cost = 100 (enter 1,1) + 100 (enter 2,1) = 200.
        // With AncientRoad on (1,1): cost = 50 + 100 = 150.
        //
        // Budget set to 160: enough with the road (150 ≤ 160),
        // not enough without it (200 > 160). If road cost is not applied,
        // find_path returns None; if it is applied, the path is found.
        let mut board = WorldBoard::new(5, 3);
        let roaded = HexCoord::from_qr(1, 1);
        if let Some(t) = board.tile_mut(roaded) {
            t.road = Some(BuiltinRoad::Ancient(AncientRoad));
        }

        let start = HexCoord::from_qr(0, 1);
        let goal  = HexCoord::from_qr(2, 1);

        let path = board.find_path(start, goal, 160);
        assert!(path.is_some(), "road should make the path reachable within budget 160");

        let path = path.unwrap();
        assert!(path.contains(&roaded), "path should pass through the roaded tile");
        assert_eq!(*path.last().unwrap(), goal);

        // Confirm that without the road the same budget is insufficient.
        let board_no_road = WorldBoard::new(5, 3);
        // (no road set)
        let path_no_road = board_no_road.find_path(start, goal, 160);
        assert!(path_no_road.is_none(), "without road the budget of 160 should be insufficient");
    }
}
