//! Point-cloud comparison metrics.
//!
//! Port of the comparison logic behind PDAL's `hausdorff`, `chamfer`, and
//! `delta` kernels.

use crate::point::{DimId, PointId, PointView};

/// Read a point's `(X, Y, Z)` coordinates.
fn xyz(view: &PointView, idx: PointId) -> (f64, f64, f64) {
    (
        view.get_f64(idx, &DimId::X),
        view.get_f64(idx, &DimId::Y),
        view.get_f64(idx, &DimId::Z),
    )
}

/// For each point of `from`, the `(index, squared distance)` of its nearest
/// neighbor in `to`, in point order. Neighbor search is brute force; a future
/// spatial-index acceleration would replace the inner loop only. `to` must be
/// non-empty.
fn nearest_neighbors(from: &PointView, to: &PointView) -> Vec<(PointId, f64)> {
    let mut neighbors = Vec::with_capacity(from.len() as usize);
    for i in 0..from.len() {
        let (fx, fy, fz) = xyz(from, i);
        let mut best = 0;
        let mut best_sq = f64::MAX;
        for j in 0..to.len() {
            let (tx, ty, tz) = xyz(to, j);
            let dsq = (fx - tx).powi(2) + (fy - ty).powi(2) + (fz - tz).powi(2);
            if dsq < best_sq {
                best_sq = dsq;
                best = j;
            }
        }
        neighbors.push((best, best_sq));
    }
    neighbors
}

/// `(max, mean)` of the nearest-neighbor distance from each point of `from`
/// to the points of `to`.
fn directed(from: &PointView, to: &PointView) -> (f64, f64) {
    let mut max_distance = f64::MIN;
    let mut mean = 0.0f64;
    for (i, &(_, nearest_sq)) in nearest_neighbors(from, to).iter().enumerate() {
        if nearest_sq > max_distance {
            max_distance = nearest_sq;
        }
        // Welford running mean of the nearest-neighbor distances.
        let delta = nearest_sq.sqrt() - mean;
        mean += delta / (i as f64 + 1.0);
    }
    (max_distance.sqrt(), mean)
}

/// The original and modified Hausdorff distances between two point sets
/// (PDAL's `computeHausdorffPair`).
///
/// The original metric is the larger of the two directed
/// max-nearest-neighbor distances; the modified metric is the larger of the
/// two directed mean-nearest-neighbor distances. Both input views must be
/// non-empty.
pub fn hausdorff_pair(a: &PointView, b: &PointView) -> (f64, f64) {
    let a2b = directed(a, b);
    let b2a = directed(b, a);
    (a2b.0.max(b2a.0), a2b.1.max(b2a.1))
}

/// The Chamfer distance between two point sets (PDAL's `computeChamfer`):
/// the sum of squared nearest-neighbor distances taken in both directions.
pub fn chamfer_distance(a: &PointView, b: &PointView) -> f64 {
    let sum_sq = |from: &PointView, to: &PointView| -> f64 {
        nearest_neighbors(from, to)
            .iter()
            .map(|&(_, dsq)| dsq)
            .sum()
    };
    sum_sq(a, b) + sum_sq(b, a)
}

/// Min/mean/max of the signed delta of one dimension.
pub struct DeltaStat {
    pub dimension: &'static str,
    pub min: f64,
    pub mean: f64,
    pub max: f64,
}

/// Per-dimension `X`/`Y`/`Z` delta statistics (PDAL's `delta` kernel).
///
/// For each `source` point, the nearest `candidate` point is found, and the
/// signed difference `source - candidate` is accumulated per dimension. Both
/// views must be non-empty.
pub fn delta_summary(source: &PointView, candidate: &PointView) -> [DeltaStat; 3] {
    let dims = [DimId::X, DimId::Y, DimId::Z];
    let names = ["X", "Y", "Z"];
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    let mut mean = [0.0f64; 3];

    for (source_id, &(cand_id, _)) in nearest_neighbors(source, candidate).iter().enumerate() {
        let count = source_id as f64 + 1.0;
        for d in 0..3 {
            let sv = source.get_f64(source_id as PointId, &dims[d]);
            let cv = candidate.get_f64(cand_id, &dims[d]);
            let delta = sv - cv;
            if delta < min[d] {
                min[d] = delta;
            }
            if delta > max[d] {
                max[d] = delta;
            }
            // Welford running mean of the signed deltas.
            mean[d] += (delta - mean[d]) / count;
        }
    }

    std::array::from_fn(|d| DeltaStat {
        dimension: names[d],
        min: min[d],
        mean: mean[d],
        max: max[d],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn cloud(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for &(x, y, z) in points {
            let p = view.add_point();
            view.set_f64(p, &DimId::X, x);
            view.set_f64(p, &DimId::Y, y);
            view.set_f64(p, &DimId::Z, z);
        }
        view
    }

    #[test]
    fn identical_clouds_have_zero_distance() {
        let view = cloud(&[(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)]);
        let (hausdorff, modified) = hausdorff_pair(&view, &view);
        assert_eq!(hausdorff, 0.0);
        assert_eq!(modified, 0.0);
        assert_eq!(chamfer_distance(&view, &view), 0.0);
        for stat in delta_summary(&view, &view) {
            assert_eq!((stat.min, stat.mean, stat.max), (0.0, 0.0, 0.0));
        }
    }

    #[test]
    fn single_point_clouds_use_the_euclidean_distance() {
        let a = cloud(&[(0.0, 0.0, 0.0)]);
        let b = cloud(&[(3.0, 4.0, 0.0)]);
        let (hausdorff, modified) = hausdorff_pair(&a, &b);
        assert_eq!(hausdorff, 5.0);
        assert_eq!(modified, 5.0);
        // Chamfer sums squared distances: 5^2 each way.
        assert_eq!(chamfer_distance(&a, &b), 50.0);
    }

    #[test]
    fn directed_metrics_take_the_larger_side() {
        // A's points both coincide with one of B's; one B point is far.
        let a = cloud(&[(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]);
        let b = cloud(&[(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]);
        let (hausdorff, modified) = hausdorff_pair(&a, &b);
        // B -> A: distances 0 and 10 -> max 10, mean 5.
        assert_eq!(hausdorff, 10.0);
        assert_eq!(modified, 5.0);
        // Chamfer: A->B sums 0; B->A sums 0 + 10^2.
        assert_eq!(chamfer_distance(&a, &b), 100.0);
    }

    #[test]
    fn delta_reports_signed_per_dimension_differences() {
        let source = cloud(&[(1.0, 2.0, 3.0), (5.0, 5.0, 5.0)]);
        // Source point 0's nearest candidate is (0,0,0); point 1's is (6,6,6).
        let candidate = cloud(&[(0.0, 0.0, 0.0), (6.0, 6.0, 6.0)]);
        let stats = delta_summary(&source, &candidate);
        // X deltas: 1 - 0 = 1, 5 - 6 = -1.
        assert_eq!(
            (stats[0].min, stats[0].mean, stats[0].max),
            (-1.0, 0.0, 1.0)
        );
        // Y deltas: 2 and -1 -> min -1, mean 0.5, max 2.
        assert_eq!(
            (stats[1].min, stats[1].mean, stats[1].max),
            (-1.0, 0.5, 2.0)
        );
        // Z deltas: 3 and -1 -> min -1, mean 1, max 3.
        assert_eq!(
            (stats[2].min, stats[2].mean, stats[2].max),
            (-1.0, 1.0, 3.0)
        );
    }
}
