//! `filters.lloydkmeans`: segment a point cloud into `k` clusters with Lloyd's
//! algorithm, writing each point's cluster index to the `ClusterID` dimension.
//!
//! Initial cluster centers are the `k` spatially farthest-apart points (see
//! [`farthest_point_sampling`]). Each iteration assigns every point to its
//! nearest center over the configured clustering dimensions, then recomputes
//! the centers as the per-cluster means.

use crate::farthestpointsampling::farthest_point_sampling;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct LloydKMeansFilter {
    k: usize,
    maxiters: usize,
    dims: Vec<DimId>,
}

impl LloydKMeansFilter {
    pub fn new(k: usize, maxiters: usize, dims: Vec<DimId>) -> Self {
        Self { k, maxiters, dims }
    }
}

impl Filter for LloydKMeansFilter {
    fn name(&self) -> &str {
        "filters.lloydkmeans"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::ClusterID, DimType::F64)]
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let np = view.len();
        let mut out = view.make_new();
        for idx in 0..np {
            out.append_point(view, idx);
        }

        // Mirror C++: nothing to do for an empty view or fewer points than k.
        if np == 0 || self.k == 0 || np < self.k as u64 || self.dims.is_empty() {
            return Ok(vec![out]);
        }

        let ndims = self.dims.len();

        // Initial cluster centers: k spatially farthest-apart points.
        let center_ids = farthest_point_sampling(view, self.k as u64);
        let mut centers: Vec<Vec<f64>> = center_ids
            .iter()
            .map(|&id| self.dims.iter().map(|d| view.get_f64(id, d)).collect())
            .collect();

        for _ in 0..self.maxiters {
            // Welford running mean per (dimension, cluster) and cluster sizes.
            let mut means = vec![vec![0.0f64; self.k]; ndims];
            let mut counts = vec![0u64; self.k];

            for p in 0..np {
                let coords: Vec<f64> = self.dims.iter().map(|d| view.get_f64(p, d)).collect();

                // Nearest center; brute force is fine since k is small.
                let mut best = 0usize;
                let mut best_dist = f64::MAX;
                for (c, center) in centers.iter().enumerate() {
                    let dist: f64 = coords
                        .iter()
                        .zip(center)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum();
                    if dist < best_dist {
                        best_dist = dist;
                        best = c;
                    }
                }

                out.set_f64(p, &DimId::ClusterID, best as f64);

                counts[best] += 1;
                let n = counts[best] as f64;
                for i in 0..ndims {
                    means[i][best] += (coords[i] - means[i][best]) / n;
                }
            }

            // Recompute centers from the cluster means.
            for c in 0..self.k {
                for i in 0..ndims {
                    centers[c][i] = means[i][c];
                }
            }
        }

        Ok(vec![out])
    }
}

impl Streamable for LloydKMeansFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        // Clustering needs the whole view at once; it is not streamable.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::PointLayout;
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ClusterID, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    fn xyz() -> Vec<DimId> {
        vec![DimId::X, DimId::Y, DimId::Z]
    }

    #[test]
    fn keeps_every_point() {
        let v = view(&[
            (0.0, 0.0, 0.0),
            (0.1, 0.0, 0.0),
            (10.0, 0.0, 0.0),
            (10.1, 0.0, 0.0),
        ]);
        let mut filter = LloydKMeansFilter::new(2, 10, xyz());
        let out = filter.run_one(&v).unwrap().pop().unwrap();
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn separates_two_well_isolated_clusters() {
        let v = view(&[
            (0.0, 0.0, 0.0),
            (0.1, 0.1, 0.0),
            (100.0, 0.0, 0.0),
            (100.1, 0.1, 0.0),
        ]);
        let mut filter = LloydKMeansFilter::new(2, 10, xyz());
        let out = filter.run_one(&v).unwrap().pop().unwrap();
        // The two near points share a cluster, distinct from the far pair.
        let c0 = out.get_f64(0, &DimId::ClusterID);
        let c1 = out.get_f64(1, &DimId::ClusterID);
        let c2 = out.get_f64(2, &DimId::ClusterID);
        let c3 = out.get_f64(3, &DimId::ClusterID);
        assert_eq!(c0, c1);
        assert_eq!(c2, c3);
        assert_ne!(c0, c2);
    }

    #[test]
    fn passes_through_when_fewer_points_than_k() {
        let v = view(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)]);
        let mut filter = LloydKMeansFilter::new(10, 10, xyz());
        let out = filter.run_one(&v).unwrap().pop().unwrap();
        assert_eq!(out.len(), 2);
    }
}
