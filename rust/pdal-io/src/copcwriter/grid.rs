//! Octree grid sizing and point-to-voxel mapping for the COPC writer.
//!
//! Port of `io/private/copcwriter/Grid.{hpp,cpp}`. Given the data bounds and the
//! total point count, `Grid` picks the octree depth so leaf nodes hold roughly
//! `MAX_POINTS_PER_NODE` points, exposes the cubic processing bounds, and maps a
//! point to its finest-level `VoxelKey`. It also derives the COPC header
//! scale/offset.

use pdal_core::bounds::Bounds3D;

use super::common::MAX_POINTS_PER_NODE;
use super::voxel_key::VoxelKey;

/// The COPC writer always works in cubic bounds, matching the C++ default
/// (`m_cubic = true`).
pub struct Grid {
    grid_size: i32,
    max_level: i32,
    bounds: Bounds3D,
    cubic_bounds: Bounds3D,
    million_points: usize,
    xsize: f64,
    ysize: f64,
    zsize: f64,
}

impl Grid {
    /// Build a grid for `bounds` containing `points` points. Mirrors the C++
    /// `Grid::Grid`: it grows a cube from the conforming bounds, rounds the
    /// point count to millions, and selects the octree level.
    pub fn new(bounds: Bounds3D, points: usize) -> Self {
        let xside = bounds.maxx - bounds.minx;
        let yside = bounds.maxy - bounds.miny;
        let zside = bounds.maxz - bounds.minz;
        let side = xside.max(yside).max(zside);
        let cubic_bounds = Bounds3D {
            minx: bounds.minx,
            miny: bounds.miny,
            minz: bounds.minz,
            maxx: bounds.minx + side,
            maxy: bounds.miny + side,
            maxz: bounds.minz + side,
        };

        // C++ rounds to the N-million points via `size_t(points / 1e6)`.
        let million_points = (points as f64 / 1_000_000.0) as usize;

        let mut grid = Grid {
            grid_size: -1,
            max_level: 0,
            bounds,
            cubic_bounds,
            million_points,
            xsize: 0.0,
            ysize: 0.0,
            zsize: 0.0,
        };
        let level = grid.calc_level();
        grid.reset_level(level);
        grid
    }

    fn calc_level(&self) -> i32 {
        let mut level = 0;
        let mut mp = self.million_points as f64;

        let xside = self.bounds.maxx - self.bounds.minx;
        let yside = self.bounds.maxy - self.bounds.miny;
        let zside = self.bounds.maxz - self.bounds.minz;
        let mut side = xside.max(yside).max(zside);

        while mp > MAX_POINTS_PER_NODE as f64 / 1_000_000.0 {
            // Cubic mode (the COPC default): halve per axis that spans the cube.
            if xside >= side {
                mp /= 2.0;
            }
            if yside >= side {
                mp /= 2.0;
            }
            if zside >= side {
                mp /= 2.0;
            }
            side /= 2.0;
            level += 1;
        }
        level
    }

    fn reset_level(&mut self, level: i32) {
        // Need at least level 1 or sampling breaks (matches C++).
        self.max_level = level.max(1);
        self.grid_size = 2_i32.pow(self.max_level as u32);

        // Cubic mode: all axes share the cube cell size.
        self.xsize = (self.cubic_bounds.maxx - self.cubic_bounds.minx) / self.grid_size as f64;
        self.ysize = self.xsize;
        self.zsize = self.xsize;
    }

    /// The finest-level `VoxelKey` containing `(x, y, z)`. Matches `Grid::key`.
    pub fn key(&self, x: f64, y: f64, z: f64) -> VoxelKey {
        let xi = ((x - self.bounds.minx) / self.xsize).floor() as i32;
        let yi = ((y - self.bounds.miny) / self.ysize).floor() as i32;
        let zi = ((z - self.bounds.minz) / self.zsize).floor() as i32;
        let clamp = |v: i32| v.max(0).min(self.grid_size - 1);
        VoxelKey::new(clamp(xi), clamp(yi), clamp(zi), self.max_level)
    }

    pub fn max_level(&self) -> i32 {
        self.max_level
    }

    pub fn cubic_bounds(&self) -> Bounds3D {
        self.cubic_bounds
    }

    pub fn conforming_bounds(&self) -> Bounds3D {
        self.bounds
    }

    /// COPC header offset: the center of the conforming bounds. Matches
    /// `Grid::offset` (divides before summing to avoid overflow).
    pub fn offset(&self) -> [f64; 3] {
        [
            self.bounds.maxx / 2.0 + self.bounds.minx / 2.0,
            self.bounds.maxy / 2.0 + self.bounds.miny / 2.0,
            self.bounds.maxz / 2.0 + self.bounds.minz / 2.0,
        ]
    }

    /// COPC header scale per axis. Matches `Grid::scale`.
    pub fn scale(&self) -> [f64; 3] {
        let calc_scale = |low: f64, high: f64| {
            // Center around 0 via the offset, so scale covers half the range.
            let val = high / 2.0 - low / 2.0;
            let power = (val / 2_000_000_000.0).log10().ceil();
            10_f64.powf(power.max(-4.0))
        };
        [
            calc_scale(self.bounds.minx, self.bounds.maxx),
            calc_scale(self.bounds.miny, self.bounds.maxy),
            calc_scale(self.bounds.minz, self.bounds.maxz),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(minx: f64, miny: f64, minz: f64, maxx: f64, maxy: f64, maxz: f64) -> Bounds3D {
        Bounds3D {
            minx,
            miny,
            minz,
            maxx,
            maxy,
            maxz,
        }
    }

    #[test]
    fn small_point_count_uses_level_one() {
        // < 1M points rounds to 0 million -> calc_level returns 0 -> clamped to
        // max_level 1, grid_size 2 (matches C++ resetLevel minimum).
        let g = Grid::new(bounds(0.0, 0.0, 0.0, 10.0, 10.0, 10.0), 100);
        assert_eq!(g.max_level(), 1);
    }

    #[test]
    fn larger_point_count_increases_level() {
        // 8M points over a cube: each level halves mp per axis (3 per level in
        // cubic mode -> /8), so 8M -> 1M after one level still exceeds 0.1, etc.
        let g = Grid::new(bounds(0.0, 0.0, 0.0, 100.0, 100.0, 100.0), 8_000_000);
        assert!(g.max_level() >= 2);
    }

    #[test]
    fn key_maps_corners_and_clamps() {
        // Cube [0,8]^3, small count -> level 1, grid_size 2, cell size 4.
        let g = Grid::new(bounds(0.0, 0.0, 0.0, 8.0, 8.0, 8.0), 10);
        assert_eq!(g.key(0.0, 0.0, 0.0), VoxelKey::new(0, 0, 0, 1));
        assert_eq!(g.key(7.9, 7.9, 7.9), VoxelKey::new(1, 1, 1, 1));
        // Out-of-range clamps into [0, grid_size-1].
        assert_eq!(g.key(100.0, -5.0, 4.0), VoxelKey::new(1, 0, 1, 1));
    }

    #[test]
    fn cubic_bounds_grows_to_max_side() {
        let g = Grid::new(bounds(0.0, 0.0, 0.0, 10.0, 4.0, 2.0), 10);
        let cb = g.cubic_bounds();
        assert_eq!(cb.maxx - cb.minx, 10.0);
        assert_eq!(cb.maxy - cb.miny, 10.0);
        assert_eq!(cb.maxz - cb.minz, 10.0);
    }

    #[test]
    fn offset_is_center_of_conforming_bounds() {
        let g = Grid::new(bounds(0.0, 2.0, 4.0, 10.0, 12.0, 14.0), 10);
        assert_eq!(g.offset(), [5.0, 7.0, 9.0]);
    }
}
