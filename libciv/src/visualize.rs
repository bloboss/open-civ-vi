use crate::world::feature::BuiltinFeature;
use crate::world::terrain::BuiltinTerrain;
use crate::world::tile::WorldTile;

/// A type that can be rendered as an n×n square ASCII block.
pub trait Visualize {
    /// Side length of the rendered square.
    fn size(&self) -> usize;
    /// Render into an `size() × size()` grid of characters.
    fn render(&self) -> Vec<Vec<char>>;
}

/// Stores a 2D grid of visualizable tiles and produces a terminal-printable
/// string buffer with staggered hex-row alignment.
///
/// Odd-indexed hex rows are indented by `size` spaces relative to even rows,
/// approximating the offset typical of pointy-top hex grids.
pub struct Visualizer<T: Visualize> {
    /// Row-major: `storage[r][q]`.
    pub storage: Vec<Vec<T>>,
}

impl<T: Visualize> Visualizer<T> {
    pub fn new(storage: Vec<Vec<T>>) -> Self {
        Self { storage }
    }

    /// Render the board to a list of strings suitable for terminal output.
    ///
    /// Each hex tile occupies an `n × n` character block. Tiles in the same
    /// hex row are placed side-by-side. Odd-indexed hex rows are indented by
    /// `n` spaces to produce the staggered hexagonal offset.
    pub fn render_buffer(&self) -> Vec<String> {
        if self.storage.is_empty() {
            return Vec::new();
        }
        let n = self.storage[0].first().map(|t| t.size()).unwrap_or(3);

        let mut lines: Vec<String> = Vec::new();

        for (r, row) in self.storage.iter().enumerate() {
            let indent = if r % 2 == 1 { " ".repeat(n) } else { String::new() };
            // Each hex row produces `n` sub-rows; pre-fill each with the indent.
            let mut sub_rows: Vec<String> = vec![indent; n];
            for tile in row {
                let block = tile.render();
                for (sub_r, char_row) in block.iter().enumerate() {
                    let s: String = char_row.iter().collect();
                    sub_rows[sub_r].push_str(&s);
                }
            }
            lines.extend(sub_rows);
        }

        lines
    }
}

// ── WorldTile rendering ──────────────────────────────────────────────────────

impl Visualize for WorldTile {
    fn size(&self) -> usize {
        1
    }

    /// Renders a single character representing this tile's terrain or feature.
    fn render(&self) -> Vec<Vec<char>> {
        let ch = match self.feature {
            Some(f) => feature_char(f),
            None    => terrain_char(self.terrain),
        };
        vec![vec![ch]]
    }
}

fn terrain_char(t: BuiltinTerrain) -> char {
    match t {
        BuiltinTerrain::Grassland => 'G',
        BuiltinTerrain::Plains    => 'P',
        BuiltinTerrain::Desert    => 'D',
        BuiltinTerrain::Tundra    => 'T',
        BuiltinTerrain::Snow      => 'S',
        BuiltinTerrain::Coast     => 'C',
        BuiltinTerrain::Ocean     => 'O',
        BuiltinTerrain::Mountain  => 'M',
    }
}

fn feature_char(f: BuiltinFeature) -> char {
    match f {
        BuiltinFeature::Forest       => 'f',
        BuiltinFeature::Rainforest   => 'r',
        BuiltinFeature::Marsh        => 'm',
        BuiltinFeature::Floodplain   => 'F',
        BuiltinFeature::Reef         => 'R',
        BuiltinFeature::Ice          => 'i',
        BuiltinFeature::VolcanicSoil => 'v',
        BuiltinFeature::Oasis        => 'o',
        BuiltinFeature::GeothermalFissure  => 'g',
        BuiltinFeature::Volcano            => 'V',
        BuiltinFeature::FloodplainGrassland => 'F',
        BuiltinFeature::FloodplainPlains   => 'F',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libhexgrid::coord::HexCoord;
    fn grassland_tile(q: i32, r: i32) -> WorldTile {
        WorldTile::new(HexCoord::from_qr(q, r), BuiltinTerrain::Grassland)
    }

    #[test]
    fn test_size_is_1() {
        assert_eq!(grassland_tile(0, 0).size(), 1);
    }

    #[test]
    fn test_render_grassland() {
        let block = grassland_tile(0, 0).render();
        assert_eq!(block.len(), 1, "block must have 1 row");
        assert_eq!(block[0], vec!['G']);
    }

    #[test]
    fn test_render_mountain() {
        let tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Mountain);
        let block = tile.render();
        assert_eq!(block[0][0], 'M', "mountain char should be 'M'");
    }

    #[test]
    fn test_render_feature_overrides_terrain() {
        use crate::world::feature::BuiltinFeature;
        let mut tile = grassland_tile(0, 0);
        tile.feature = Some(BuiltinFeature::Forest);
        let block = tile.render();
        assert_eq!(block[0][0], 'f', "forest feature should override terrain char");
    }

    #[test]
    fn test_visualizer_even_row_no_indent() {
        let row0 = vec![grassland_tile(0, 0), grassland_tile(1, 0)];
        let vis = Visualizer::new(vec![row0]);
        let buf = vis.render_buffer();
        // Even row (r=0): no indent — first character must be 'G'.
        assert_eq!(buf.len(), 1);
        assert!(buf[0].starts_with('G'), "even row must not be indented");
    }

    #[test]
    fn test_visualizer_odd_row_indented() {
        let vis = Visualizer::new(vec![
            vec![grassland_tile(0, 0)],
            vec![grassland_tile(0, 1)],
        ]);
        let buf = vis.render_buffer();
        // 1 sub-row per hex row → 2 total lines.
        assert_eq!(buf.len(), 2);
        // Row 0 (even): no leading spaces.
        assert!(!buf[0].starts_with(' '), "even row 0 should not be indented");
        // Row 1 (odd): indented by 1 space.
        assert!(buf[1].starts_with(' '), "odd row 1 must start with 1 space");
        assert!(!buf[1].starts_with("  "), "odd row 1 must have exactly 1 space indent");
    }

    #[test]
    fn test_visualizer_tile_count_in_buffer() {
        // 2 rows × 3 columns → 1 sub-row each → 2 lines total.
        let make_row = |r| (0..3).map(|q| grassland_tile(q, r)).collect::<Vec<_>>();
        let vis = Visualizer::new(vec![make_row(0), make_row(1)]);
        let buf = vis.render_buffer();
        assert_eq!(buf.len(), 2);
        // Row 0: 3 tiles × 1 char each = 3 chars, no indent.
        assert_eq!(buf[0].len(), 3, "row 0 should be 3 chars wide");
        // Row 1: 1 space indent + 3 chars = 4 chars total.
        assert_eq!(buf[1].len(), 4, "row 1 should be 4 chars (1 indent + 3 tiles)");
    }
}
