//! Per-node processing for the COPC pyramid build.
//!
//! Port of `io/private/copcwriter/Processor.{hpp,cpp}`. A `Processor` takes one
//! `VoxelInfo` (a node and its eight children), merges sparse children up into
//! the parent, subsamples representative points from the children into the
//! parent via the node's occupancy grid, and reports the LAZ chunks to write
//! plus the parent octant to queue at the next level up.
//!
//! Note on parity: the C++ `sample()` shuffles each child with `std::mt19937`
//! before greedily accepting the first point seen per occupancy-grid cell, so
//! the exact per-node point membership depends on the standard library's RNG
//! and shuffle. That cannot be reproduced byte-for-byte in Rust; we keep the
//! same algorithm with a deterministic PRNG. The observable COPC contract
//! (every point retained, valid octree, resolution/bounds queryability) is
//! preserved; node-for-node point identity is not guaranteed.

use pdal_core::point::{DimId, PointView};

use super::common::{MINIMUM_POINTS, MINIMUM_TOTAL_POINTS};
use super::octant_info::OctantInfo;
use super::voxel_info::VoxelInfo;
use super::voxel_key::VoxelKey;

/// Small deterministic PRNG (PCG-style LCG, matching the `faux` reader's
/// approach) used to shuffle child points before subsampling.
pub struct SampleRng {
    state: u64,
}

impl SampleRng {
    pub fn new(seed: u64) -> Self {
        SampleRng {
            state: seed.wrapping_mul(6364136223846793005).wrapping_add(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // Xorshift the high bits down for better low-bit quality.
        let x = self.state;
        x ^ (x >> 31)
    }

    /// Uniform integer in `[0, n)`.
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        self.next_u64() % n
    }
}

/// In-place Fisher-Yates shuffle of a view's points (mirrors `std::shuffle` over
/// `PointView::begin()/end()`).
fn shuffle_view(view: &mut PointView, rng: &mut SampleRng) {
    let len = view.len();
    if len < 2 {
        return;
    }
    for i in (1..len).rev() {
        let j = rng.below(i + 1);
        view.swap_points(i, j);
    }
}

/// A LAZ chunk to emit: the node key and the points assigned to it. Empty views
/// still produce a chunk (an empty hierarchy entry), matching the C++
/// `writeCompressed` empty path.
pub struct Chunk {
    pub key: VoxelKey,
    pub view: PointView,
}

pub struct Processor {
    vi: VoxelInfo,
}

impl Processor {
    pub fn new(vi: VoxelInfo) -> Self {
        Processor { vi }
    }

    /// Run redistribution + subsampling for this node. Returns the chunks to
    /// write (child nodes that have points or must be written, plus the root
    /// node itself) and the parent octant to queue at the next level. Mirrors
    /// `Processor::run` minus the LAZ-encode/file-write, which the Output layer
    /// owns.
    pub fn run(mut self, rng: &mut SampleRng) -> (Vec<Chunk>, OctantInfo) {
        self.vi.init_parent_octant();
        self.redistribute();
        self.sample(rng);
        let chunks = self.collect_chunks();
        let key = self.vi.key();
        let octant = std::mem::replace(self.vi.octant_mut(), OctantInfo::new(key));
        (chunks, octant)
    }

    /// Merge sparse children up into the parent (C++ `run` point-moving).
    fn redistribute(&mut self) {
        let counts: [usize; 8] = std::array::from_fn(|i| self.vi.child(i).num_points());
        let total_points: usize = counts.iter().sum();

        for (i, &count) in counts.iter().enumerate() {
            if count < MINIMUM_POINTS {
                self.merge_child_up(i);
            }
        }
        if total_points < MINIMUM_TOTAL_POINTS {
            for i in 0..8 {
                self.merge_child_up(i);
            }
        }
    }

    /// Append child `i`'s points to the parent octant, emptying the child.
    /// Equivalent to C++ `octant().movePoints(child)` (the parent source was
    /// initialized by `init_parent_octant`).
    fn merge_child_up(&mut self, i: usize) {
        let Some(child_src) = self.vi.child_mut(i).take_source() else {
            return;
        };
        match self.vi.octant_mut().source_mut() {
            Some(parent) => parent.append(&child_src),
            None => self.vi.octant_mut().set_source(child_src),
        }
    }

    /// Subsample child points into the parent via the occupancy grid (C++
    /// `sample`). Accepted points go to the parent and occupy a grid cell;
    /// rejected points stay in their child.
    fn sample(&mut self, rng: &mut SampleRng) {
        let mut accepted = self
            .vi
            .octant_mut()
            .take_source()
            .expect("init_parent_octant set the parent source");
        let mut grid = std::mem::take(self.vi.grid());

        for i in 0..8 {
            if self.vi.child(i).num_points() == 0 {
                continue;
            }
            let mut child_view = self
                .vi
                .child_mut(i)
                .take_source()
                .expect("non-empty child has a source");
            shuffle_view(&mut child_view, rng);
            let mut rejected = child_view.make_new();

            let len = child_view.len();
            for idx in 0..len {
                let x = child_view.get_f64(idx, &DimId::X);
                let y = child_view.get_f64(idx, &DimId::Y);
                let z = child_view.get_f64(idx, &DimId::Z);
                let key = self.vi.grid_key(x, y, z);
                if grid.insert(key) {
                    // Cell was empty -> accept into the parent.
                    accepted.append_point(&child_view, idx);
                } else {
                    rejected.append_point(&child_view, idx);
                }
            }
            self.vi.child_mut(i).set_source(rejected);
        }

        self.vi.octant_mut().set_source(accepted);
        *self.vi.grid() = grid;
    }

    /// Children with points (or flagged must-write), plus the root parent.
    /// Sets the parent's must-write flag when any child is written (C++
    /// `write`).
    fn collect_chunks(&mut self) -> Vec<Chunk> {
        let is_root = self.vi.key() == VoxelKey::ROOT;
        let mut chunks = Vec::new();
        let mut any_child = false;
        for i in 0..8 {
            let child = self.vi.child(i);
            let has = child.num_points() != 0 || child.must_write();
            if has {
                any_child = true;
                let key = child.key();
                let view = self
                    .vi
                    .child_mut(i)
                    .take_source()
                    .unwrap_or_else(|| panic!("child {i} flagged for write has no source"));
                chunks.push(Chunk { key, view });
            }
        }
        if any_child {
            self.vi.octant_mut().set_must_write(true);
        }
        if is_root {
            let key = self.vi.octant().key();
            if let Some(view) = self.vi.octant_mut().take_source() {
                chunks.push(Chunk { key, view });
            }
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::bounds::Bounds3D;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn cube() -> Bounds3D {
        Bounds3D {
            minx: 0.0,
            miny: 0.0,
            minz: 0.0,
            maxx: 16.0,
            maxy: 16.0,
            maxz: 16.0,
        }
    }

    fn xyz_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        PointView::new(Rc::new(layout))
    }

    fn add(view: &mut PointView, x: f64, y: f64, z: f64) {
        let id = view.add_point();
        view.set_f64(id, &DimId::X, x);
        view.set_f64(id, &DimId::Y, y);
        view.set_f64(id, &DimId::Z, z);
    }

    #[test]
    fn shuffle_preserves_point_count() {
        let mut v = xyz_view();
        for i in 0..20 {
            add(&mut v, i as f64, 0.0, 0.0);
        }
        let mut rng = SampleRng::new(1234);
        shuffle_view(&mut v, &mut rng);
        assert_eq!(v.len(), 20);
    }

    #[test]
    fn small_total_merges_all_children_into_root() {
        // Few points total (< MINIMUM_TOTAL_POINTS): everything ends up in the
        // root node and one root chunk is written; no point is lost.
        let mut vi = VoxelInfo::new(cube(), VoxelKey::ROOT);
        let mut total = 0;
        for dir in 0..8 {
            let mut cv = xyz_view();
            for p in 0..10 {
                add(&mut cv, dir as f64 + 0.1, p as f64 * 0.1, 0.0);
                total += 1;
            }
            vi.child_mut(dir).set_source(cv);
        }
        let mut rng = SampleRng::new(1234);
        let (chunks, parent) = Processor::new(vi).run(&mut rng);

        // All points retained across the emitted chunks + queued parent.
        let chunk_points: usize = chunks.iter().map(|c| c.view.len() as usize).sum();
        assert_eq!(chunk_points + parent.num_points(), total);
        // The root chunk is present.
        assert!(chunks.iter().any(|c| c.key == VoxelKey::ROOT));
    }

    #[test]
    fn sampling_retains_every_point_between_parent_and_children() {
        // Larger child populations: subsampling keeps some in the parent and
        // rejects the rest back to children, but no point is dropped.
        let mut vi = VoxelInfo::new(cube(), VoxelKey::ROOT);
        let mut total = 0;
        for dir in 0..8 {
            let mut cv = xyz_view();
            for p in 0..300 {
                // Spread within the child's octant so grid cells vary.
                let base = (dir & 1) as f64 * 8.0;
                add(&mut cv, base + (p as f64) * 0.02, (p as f64) * 0.01, 0.0);
                total += 1;
            }
            vi.child_mut(dir).set_source(cv);
        }
        let mut rng = SampleRng::new(1234);
        let key = vi.key();
        let (chunks, parent) = Processor::new(vi).run(&mut rng);
        let chunk_points: usize = chunks.iter().map(|c| c.view.len() as usize).sum();
        assert_eq!(chunk_points + parent.num_points(), total);
        // At the root, the parent is also emitted as a chunk.
        if key == VoxelKey::ROOT {
            assert!(chunks.iter().any(|c| c.key == VoxelKey::ROOT));
        }
    }
}
