# libhexgrid -- Hex Geometry

`libhexgrid` is a standalone hex-grid geometry library with **zero game knowledge**. It provides coordinate math, board abstractions, pathfinding, and line-of-sight computation. The game layer (`libciv`) implements the traits defined here.

## Coordinates

### HexCoord

Cube coordinates with the invariant `q + r + s = 0`:

```rust
struct HexCoord { q: i32, r: i32, s: i32 }
```

- **Construction**: `HexCoord::new(q, r, s)` validates the invariant; `HexCoord::from_qr(q, r)` auto-computes `s = -q - r`
- **Distance**: `a.distance(b)` returns the hex-grid Manhattan distance
- **Neighbors**: `coord.neighbors()` returns all 6 adjacent coordinates
- **Rings**: `coord.ring(radius)` returns all coordinates at exactly that distance
- **Arithmetic**: `Add`, `Sub`, `Neg`, `Mul<i32>` all preserve the cube invariant

### HexDir

Six cardinal directions, each with a unit vector:

```
     NW   NE
      \ /
  W ---+--- E
      / \
     SW   SE
```

- `HexDir::unit_vec()` -- the `HexCoord` offset for one step in this direction
- `HexDir::opposite()` -- the reverse direction
- **Canonical set**: `{E, NE, NW}` (forward-half) is used for edge storage. Backward-half directions `{W, SW, SE}` are normalized by looking up the adjacent tile with the opposite direction.

## Supporting Types

### MovementCost

```rust
enum MovementCost {
    Impassable,
    Cost(u32),  // scaled by 100: ONE=100, TWO=200, THREE=300
}
```

### Elevation

```rust
enum Elevation {
    Low,         // below sea level (e.g., ocean depths)
    Level(u8),   // 0=coastal, 1=flat, 2=hills
    High,        // impassable mountain peaks
}
```

Elevation is ordered (`Low < Level(0) < Level(1) < Level(2) < High`) and used for line-of-sight blocking.

### Other Types

| Type | Description |
|------|-------------|
| `Vision` | `Blind`, `Radius(u8)`, or `Omniscient` |
| `MovementProfile` | `Ground`, `Naval`, `Air`, `Embarked`, `Amphibious` |
| `BoardTopology` | `Flat`, `CylindricalEW` (wraps east-west), `Toroidal` |

## Traits

### HexTile

```rust
trait HexTile {
    fn coord(&self) -> HexCoord;
    fn elevation(&self) -> Elevation;
    fn movement_cost(&self) -> MovementCost;
    fn vision_bonus(&self) -> i8;
}
```

Implemented by `WorldTile` in `libciv`.

### HexEdge

```rust
trait HexEdge {
    fn coord(&self) -> HexCoord;       // canonical forward-half endpoint
    fn dir(&self) -> HexDir;            // always E, NE, or NW
    fn crossing_cost(&self) -> MovementCost;
}
```

Implemented by `WorldEdge` in `libciv`. Edges represent rivers, canals, and mountain passes.

### HexBoard

```rust
trait HexBoard {
    type Tile: HexTile;
    type Edge: HexEdge;

    fn topology(&self) -> BoardTopology;
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn tile(&self, coord: HexCoord) -> Option<&Self::Tile>;
    fn tile_mut(&mut self, coord: HexCoord) -> Option<&mut Self::Tile>;
    fn edge(&self, coord: HexCoord, dir: HexDir) -> Option<&Self::Edge>;
    fn neighbors(&self, coord: HexCoord) -> Vec<HexCoord>;
    fn normalize(&self, coord: HexCoord) -> Option<HexCoord>;
    fn all_coords(&self) -> Vec<HexCoord>;
}
```

The board handles coordinate normalization for wrapping topologies (cylindrical and toroidal maps). Implemented by `WorldBoard` in `libciv`.

## Edge Canonicalization

Edges are stored as `(HexCoord, HexDir)` using only the forward-half directions `{E, NE, NW}`. When a backward-half direction is queried:

1. Compute the neighbor in the backward direction
2. Look up the edge from that neighbor using the opposite (forward-half) direction

This ensures each edge is stored exactly once regardless of which side queries it. `WorldBoard::set_edge()` handles canonicalization automatically.
