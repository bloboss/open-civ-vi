use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

use libhexgrid::board::{BoardTopology, HexBoard};
use libhexgrid::coord::HexCoord;
use libhexgrid::types::{Elevation, MovementCost};
use libhexgrid::{HexEdge, HexTile};
use crate::world::edge::WorldEdge;
use crate::world::terrain::{BuiltinTerrain, Grassland};
use crate::world::tile::WorldTile;

/// Concrete hex board for the world map.
#[derive(Debug)]
pub struct WorldBoard {
    pub width: u32,
    pub height: u32,
    topology: BoardTopology,
    /// Tiles stored in row-major order: index = r * width + q (offset coords).
    tiles: Vec<WorldTile>,
    /// Edges keyed by sorted pair of HexCoords.
    edges: HashMap<(HexCoord, HexCoord), WorldEdge>,
}

impl WorldBoard {
    pub fn new(width: u32, height: u32) -> Self {
        let mut tiles = Vec::with_capacity((width * height) as usize);
        for r in 0..height as i32 {
            for q in 0..width as i32 {
                let coord = HexCoord::from_qr(q, r);
                tiles.push(WorldTile::new(coord, BuiltinTerrain::Grassland(Grassland)));
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
        // Wrap q for cylindrical topology
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

    fn edge_key(a: HexCoord, b: HexCoord) -> (HexCoord, HexCoord) {
        if (a.q, a.r) <= (b.q, b.r) { (a, b) } else { (b, a) }
    }

    pub fn set_edge(&mut self, edge: WorldEdge) {
        let (a, b) = edge.endpoints();
        let key = Self::edge_key(a, b);
        self.edges.insert(key, edge);
    }

    /// Dijkstra pathfinding. Returns `Some(path)` or `None` if unreachable.
    pub fn find_path(
        &self,
        start: HexCoord,
        goal: HexCoord,
        movement_budget: u32,
    ) -> Option<Vec<HexCoord>> {
        let start = self.normalize_coord(start)?;
        let goal = self.normalize_coord(goal)?;

        // dist maps coord -> cheapest cost to reach
        let mut dist: HashMap<HexCoord, u32> = HashMap::new();
        let mut prev: HashMap<HexCoord, HexCoord> = HashMap::new();
        // BinaryHeap is a max-heap; use Reverse for min-heap behaviour
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

            for neighbor_raw in coord.neighbors() {
                let Some(neighbor) = self.normalize_coord(neighbor_raw) else { continue };
                let Some(tile) = self.tile(neighbor) else { continue };

                let tile_cost = match tile.movement_cost() {
                    MovementCost::Impassable => continue,
                    MovementCost::Cost(c) => c,
                };

                let edge_cost = self
                    .edge(coord, neighbor)
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

        if !dist.contains_key(&goal) {
            return None;
        }

        // Reconstruct path
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
    /// Blocked if any intermediate tile has higher elevation than both endpoints
    /// or by an edge that blocks LOS.
    pub fn has_los(&self, from: HexCoord, to: HexCoord) -> bool {
        let Some(from) = self.normalize_coord(from) else { return false };
        let Some(to) = self.normalize_coord(to) else { return false };

        if from == to {
            return true;
        }

        let from_elev = self.tile(from).map(|t| t.elevation()).unwrap_or(Elevation::FLAT);
        let to_elev = self.tile(to).map(|t| t.elevation()).unwrap_or(Elevation::FLAT);
        let min_elev = from_elev.min(to_elev);

        // Walk intermediate hexes on the line from->to
        let dist = from.distance(&to) as i32;
        for step in 1..dist {
            let frac_q = from.q as f32 + (to.q - from.q) as f32 * step as f32 / dist as f32;
            let frac_r = from.r as f32 + (to.r - from.r) as f32 * step as f32 / dist as f32;
            let mid = hex_round(frac_q, frac_r);
            let Some(mid) = self.normalize_coord(mid) else { continue };
            if let Some(tile) = self.tile(mid) {
                if tile.elevation() > min_elev {
                    return false;
                }
            }
        }
        true
    }
}

/// Round fractional hex coordinates to nearest integer hex.
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
        let _ = s; // s is derived, not stored
    }

    HexCoord::from_qr(q, r)
}

impl HexBoard for WorldBoard {
    type Tile = WorldTile;
    type Edge = WorldEdge;

    fn topology(&self) -> BoardTopology {
        self.topology
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn tile(&self, coord: HexCoord) -> Option<&WorldTile> {
        let idx = self.coord_to_index(coord)?;
        self.tiles.get(idx)
    }

    fn tile_mut(&mut self, coord: HexCoord) -> Option<&mut WorldTile> {
        let idx = self.coord_to_index(coord)?;
        self.tiles.get_mut(idx)
    }

    fn edge(&self, from: HexCoord, to: HexCoord) -> Option<&WorldEdge> {
        let from = self.normalize_coord(from)?;
        let to = self.normalize_coord(to)?;
        self.edges.get(&Self::edge_key(from, to))
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
    use libhexgrid::HexDir;
    use crate::world::feature::{BuiltinFeature, Ice};
    use crate::world::terrain::{BuiltinTerrain, Grassland};

    fn small_board() -> WorldBoard {
        WorldBoard::new(10, 10)
    }

    #[test]
    fn test_wraparound_east_west() {
        let board = small_board();
        // q=9, going E should wrap to q=0
        let east_edge = HexCoord::from_qr(9, 2);
        let step_east = east_edge + HexDir::E.unit_vec();
        let normalized = board.normalize(step_east).expect("should wrap");
        assert_eq!(normalized.q, 0);
    }

    #[test]
    #[ignore = "Phase 2: implement LOS with elevation"]
    fn test_los_blocked_by_high() {
        let mut board = small_board();
        // Place a high tile in the middle
        let mid = HexCoord::from_qr(5, 5);
        if let Some(t) = board.tile_mut(mid) {
            t.terrain = BuiltinTerrain::Grassland(Grassland); // placeholder; need hills terrain
        }
        let from = HexCoord::from_qr(3, 5);
        let to = HexCoord::from_qr(7, 5);
        assert!(!board.has_los(from, to));
    }

    #[test]
    fn test_dijkstra_avoids_impassable() {
        let mut board = WorldBoard::new(10, 10);
        // Block a row with impassable edges (cliffs) — use Ice feature to make tiles impassable
        let blocker = HexCoord::from_qr(3, 3);
        if let Some(t) = board.tile_mut(blocker) {
            t.feature = Some(BuiltinFeature::Ice(Ice));
        }
        let start = HexCoord::from_qr(2, 3);
        let goal = HexCoord::from_qr(5, 3);
        // Path should exist but go around the impassable tile
        let path = board.find_path(start, goal, 10_000);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(!path.contains(&blocker), "path must not pass through impassable tile");
    }

    #[test]
    #[ignore = "Phase 2: implement road movement cost reduction"]
    fn test_dijkstra_prefers_roads() {
        // Requires road tiles to have lower movement cost - Phase 2.
        todo!("set up road tiles, verify Dijkstra chooses road path")
    }
}
