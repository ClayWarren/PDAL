//! Bottom-up COPC octree build driver.
//!
//! Port of `io/private/copcwriter/BuPyramid.{hpp,cpp}` and the orchestration in
//! `PyramidManager.{hpp,cpp}`. The C++ version processes nodes on a thread pool;
//! the build is order-independent (a parent is processed only once all eight
//! children complete), so this port runs it sequentially.
//!
//! Input is a [`CellManager`] of finest-grid leaf cells (produced by binning
//! every point through [`super::grid::Grid`]). Output is the ordered list of
//! LAZ chunks to encode, the per-node point counts, and the cumulative child
//! counts the hierarchy emission needs. Actual LAZ encoding and file layout
//! belong to the (separate) Output layer.

use std::collections::{HashMap, VecDeque};

use pdal_core::bounds::Bounds3D;

use super::cell_manager::CellManager;
use super::octant_info::OctantInfo;
use super::processor::{Chunk, Processor, SampleRng};
use super::voxel_info::VoxelInfo;
use super::voxel_key::VoxelKey;

/// Result of the pyramid build, consumed by the Output layer.
pub struct PyramidResult {
    /// LAZ chunks in write order (`(key, points)`); empty must-write nodes
    /// produce a zero-point chunk.
    pub chunks: Vec<Chunk>,
    /// Per-node point count for every written node (including zero-point ones).
    pub written: HashMap<VoxelKey, i32>,
    /// Cumulative descendant counts per node (C++ `calcCounts`), used to decide
    /// hierarchy sub-pages.
    pub child_counts: HashMap<VoxelKey, i64>,
    /// Total points written across all chunks.
    pub total_points: u64,
}

pub struct Pyramid {
    bounds: Bounds3D,
    queue: VecDeque<OctantInfo>,
    completes: HashMap<VoxelKey, OctantInfo>,
    written: HashMap<VoxelKey, i32>,
    chunks: Vec<Chunk>,
    total_points: u64,
    rng: SampleRng,
}

impl Pyramid {
    pub fn new(bounds: Bounds3D, seed: u64) -> Self {
        Pyramid {
            bounds,
            queue: VecDeque::new(),
            completes: HashMap::new(),
            written: HashMap::new(),
            chunks: Vec::new(),
            total_points: 0,
            rng: SampleRng::new(seed),
        }
    }

    /// Build the COPC octree from the finest-grid `cells`. Mirrors
    /// `BuPyramid::run`: queue the leaves (plus the empty ancestor children
    /// needed so every processed parent has all eight children), then process
    /// bottom-up to the root.
    pub fn run(mut self, cells: CellManager) -> PyramidResult {
        self.queue_work(cells);
        self.process_queue();
        let child_counts = self.calc_counts();
        PyramidResult {
            chunks: self.chunks,
            written: self.written,
            child_counts,
            total_points: self.total_points,
        }
    }

    /// Queue leaf octants and the empty ancestor children required to process
    /// every parent up to the root (C++ `BuPyramid::queueWork`).
    fn queue_work(&mut self, mut cells: CellManager) {
        let keys: Vec<VoxelKey> = cells.iter().map(|(k, _)| *k).collect();

        let mut needed: std::collections::HashSet<VoxelKey> = std::collections::HashSet::new();
        let mut have: Vec<OctantInfo> = Vec::with_capacity(keys.len());

        for key in &keys {
            let view = cells.remove(*key).expect("key came from the manager");
            let mut o = OctantInfo::new(*key);
            o.set_source(view);
            have.push(o);

            // Walk up to the root, recording every child of every ancestor as
            // potentially needed.
            let mut k = *key;
            while k != VoxelKey::ROOT {
                k = k.parent();
                for i in 0..8 {
                    needed.insert(k.child(i));
                }
            }
        }

        // Remove the nodes we actually have (and their ancestors) from needed.
        for o in &have {
            let mut k = o.key();
            while k != VoxelKey::ROOT {
                needed.remove(&k);
                k = k.parent();
            }
        }

        for o in have {
            self.queue.push_back(o);
        }
        for k in needed {
            self.queue.push_back(OctantInfo::new(k));
        }
    }

    fn process_queue(&mut self) {
        while let Some(o) = self.queue.pop_front() {
            if o.key() == VoxelKey::ROOT {
                break;
            }
            self.process(o);
        }
    }

    /// Stash a completed octant; once all eight children of its parent are
    /// complete, build and run the parent (C++ `PyramidManager::process`).
    fn process(&mut self, o: OctantInfo) {
        let parent_key = o.key().parent();
        self.completes.insert(o.key(), o);
        if !self.children_complete(parent_key) {
            return;
        }

        let mut vi = VoxelInfo::new(self.bounds, parent_key);
        for i in 0..8usize {
            let child_key = parent_key.child(i as i32);
            let child = self
                .remove_complete(child_key)
                .unwrap_or_else(|| OctantInfo::new(child_key));
            *vi.child_mut(i) = child;
        }

        if !vi.has_points() {
            // Nothing here: queue the (empty) parent so its parent still sees a
            // complete child set.
            let key = vi.key();
            let octant = std::mem::replace(vi.octant_mut(), OctantInfo::new(key));
            self.queue.push_back(octant);
            return;
        }

        let (chunks, parent_octant) = Processor::new(vi).run(&mut self.rng);
        for chunk in chunks {
            self.new_chunk(chunk);
        }
        self.queue.push_back(parent_octant);
    }

    fn children_complete(&self, parent: VoxelKey) -> bool {
        (0..8).all(|i| self.completes.contains_key(&parent.child(i)))
    }

    fn remove_complete(&mut self, key: VoxelKey) -> Option<OctantInfo> {
        self.completes.remove(&key)
    }

    /// Record a chunk's point count and stash it for encoding (C++
    /// `PyramidManager::newChunk` + `Output::newChunk` bookkeeping).
    fn new_chunk(&mut self, chunk: Chunk) {
        let count = chunk.view.len() as i32;
        self.written.insert(chunk.key, count);
        self.total_points += count as u64;
        self.chunks.push(chunk);
    }

    /// Cumulative descendant counts per node (C++ `PyramidManager::run`'s
    /// `calcCounts`): each node maps to the total of its written descendants.
    fn calc_counts(&self) -> HashMap<VoxelKey, i64> {
        let mut counts = HashMap::new();
        self.calc_counts_rec(VoxelKey::ROOT, &mut counts);
        counts
    }

    fn calc_counts_rec(&self, k: VoxelKey, counts: &mut HashMap<VoxelKey, i64>) -> i64 {
        let mut count = 0;
        for i in 0..8 {
            let c = k.child(i);
            if self.written.contains_key(&c) {
                count += self.calc_counts_rec(c, counts);
            }
        }
        counts.insert(k, count);
        count + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
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

    fn source() -> PointView {
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

    /// Bin points to a finest grid (mimicking what the writer does with `Grid`)
    /// and return a `CellManager`.
    fn cells_from(points: &[(f64, f64, f64)], cells_per_side: i32) -> (CellManager, usize) {
        use super::super::voxel_key::VoxelKey;
        let mut mgr = CellManager::new(source());
        let level = (cells_per_side as f64).log2() as i32;
        let cell = 16.0 / cells_per_side as f64;
        for &(x, y, z) in points {
            let xi = ((x / cell) as i32).min(cells_per_side - 1);
            let yi = ((y / cell) as i32).min(cells_per_side - 1);
            let zi = ((z / cell) as i32).min(cells_per_side - 1);
            add(mgr.get(VoxelKey::new(xi, yi, zi, level)), x, y, z);
        }
        (mgr, points.len())
    }

    #[test]
    fn single_leaf_builds_root_chunk_with_all_points() {
        let pts: Vec<(f64, f64, f64)> = (0..50).map(|i| (i as f64 * 0.1, 1.0, 1.0)).collect();
        let (cells, total) = cells_from(&pts, 2);
        let result = Pyramid::new(cube(), 1234).run(cells);

        assert_eq!(result.total_points as usize, total);
        // Every input point appears across the written chunks.
        let chunk_total: usize = result.chunks.iter().map(|c| c.view.len() as usize).sum();
        assert_eq!(chunk_total, total);
        // The root node is written and counted.
        assert!(result.written.contains_key(&VoxelKey::ROOT));
        assert!(result.child_counts.contains_key(&VoxelKey::ROOT));
    }

    #[test]
    fn points_spread_across_octants_are_all_retained() {
        let mut pts = Vec::new();
        for xi in 0..2 {
            for yi in 0..2 {
                for zi in 0..2 {
                    for p in 0..40 {
                        pts.push((
                            xi as f64 * 8.0 + p as f64 * 0.05,
                            yi as f64 * 8.0 + 0.5,
                            zi as f64 * 8.0 + 0.5,
                        ));
                    }
                }
            }
        }
        let total = pts.len();
        let (cells, _) = cells_from(&pts, 2);
        let result = Pyramid::new(cube(), 1234).run(cells);
        let chunk_total: usize = result.chunks.iter().map(|c| c.view.len() as usize).sum();
        assert_eq!(chunk_total, total);
        assert_eq!(result.total_points as usize, total);
    }

    #[test]
    fn empty_input_produces_no_chunks() {
        let mgr = CellManager::new(source());
        let result = Pyramid::new(cube(), 1234).run(mgr);
        assert_eq!(result.total_points, 0);
        assert!(result.chunks.is_empty());
    }
}
