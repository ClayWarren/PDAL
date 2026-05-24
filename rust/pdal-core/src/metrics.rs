//! Point-cloud comparison metrics.
//!
//! Port of the comparison logic behind PDAL's `hausdorff`, `chamfer`,
//! `delta`, and `eval` kernels.

use crate::point::{DimId, PointId, PointView};
use rstar::{primitives::GeomWithData, PointDistance, RTree};

type IndexedPoint = GeomWithData<[f64; 3], PointId>;

/// Read a point's `(X, Y, Z)` coordinates.
fn xyz(view: &PointView, idx: PointId) -> (f64, f64, f64) {
    (
        view.get_f64(idx, &DimId::X),
        view.get_f64(idx, &DimId::Y),
        view.get_f64(idx, &DimId::Z),
    )
}

fn xyz_array(view: &PointView, idx: PointId) -> [f64; 3] {
    let (x, y, z) = xyz(view, idx);
    [x, y, z]
}

fn index_points(view: &PointView) -> RTree<IndexedPoint> {
    RTree::bulk_load(
        (0..view.len())
            .map(|idx| IndexedPoint::new(xyz_array(view, idx), idx))
            .collect(),
    )
}

/// For each point of `from`, the `(index, squared distance)` of its nearest
/// neighbor in `to`, in point order. `to` must be non-empty.
fn nearest_neighbors(from: &PointView, to: &PointView) -> Vec<(PointId, f64)> {
    let index = index_points(to);
    nearest_neighbors_with_index(from, &index)
}

fn nearest_neighbors_with_index(
    from: &PointView,
    index: &RTree<IndexedPoint>,
) -> Vec<(PointId, f64)> {
    let mut neighbors = Vec::with_capacity(from.len() as usize);
    for i in 0..from.len() {
        let query = xyz_array(from, i);
        let nearest = index.nearest_neighbor(&query).expect("non-empty index");
        let best = nearest.data;
        let best_sq = nearest.distance_2(&query);
        neighbors.push((best, best_sq));
    }
    neighbors
}

/// `(max, mean)` of the nearest-neighbor distance from each point of `from`
/// to the points of `to`.
fn directed_with_index(from: &PointView, index: &RTree<IndexedPoint>) -> (f64, f64) {
    let mut max_distance = f64::MIN;
    let mut mean = 0.0f64;
    for (i, &(_, nearest_sq)) in nearest_neighbors_with_index(from, index).iter().enumerate() {
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
    let a_index = index_points(a);
    let b_index = index_points(b);
    let a2b = directed_with_index(a, &b_index);
    let b2a = directed_with_index(b, &a_index);
    (a2b.0.max(b2a.0), a2b.1.max(b2a.1))
}

/// The Chamfer distance between two point sets (PDAL's `computeChamfer`):
/// the sum of squared nearest-neighbor distances taken in both directions.
pub fn chamfer_distance(a: &PointView, b: &PointView) -> f64 {
    let a_index = index_points(a);
    let b_index = index_points(b);
    let sum_sq = |from: &PointView, index: &RTree<IndexedPoint>| -> f64 {
        nearest_neighbors_with_index(from, index)
            .iter()
            .map(|&(_, dsq)| dsq)
            .sum()
    };
    sum_sq(a, &b_index) + sum_sq(b, &a_index)
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

/// Classification metrics for a single label (PDAL's `LabelStats`).
pub struct LabelMetrics {
    pub label: i64,
    pub support: u64,
    pub intersection_over_union: f64,
    pub f1_score: f64,
    pub sensitivity: f64,
    pub specificity: f64,
    pub precision: f64,
    pub accuracy: f64,
}

/// The full result of comparing predicted labels against truth labels.
pub struct EvalReport {
    pub labels: Vec<LabelMetrics>,
    pub mean_intersection_over_union: f64,
    pub overall_accuracy: f64,
    pub f1_score: f64,
    /// `(dim + 1) x (dim + 1)` confusion matrix, indexed `[truth][predicted]`;
    /// the trailing row/column collects labels outside the evaluated set.
    pub confusion_matrix: Vec<Vec<u64>>,
}

/// Evaluate predicted classification labels against truth labels (PDAL's
/// `eval` kernel).
///
/// For each `predicted` point the nearest `truth` point is found, and the
/// pair of labels is tallied into a confusion matrix over `labels` (sorted
/// ascending). Both views must be non-empty and must carry the named
/// dimensions; this is checked by the caller.
pub fn evaluate(
    predicted: &PointView,
    truth: &PointView,
    predicted_dim: &DimId,
    truth_dim: &DimId,
    labels: &[i64],
) -> EvalReport {
    let mut labels: Vec<i64> = labels.to_vec();
    labels.sort_unstable();
    labels.dedup();
    let dim = labels.len();

    // Confusion matrix indexed [truth][predicted]; index `dim` collects any
    // label that is not in the evaluated set.
    let mut matrix = vec![vec![0u64; dim + 1]; dim + 1];
    let class_index = |value: i64| labels.iter().position(|&l| l == value).unwrap_or(dim);

    for (predicted_id, &(truth_id, _)) in nearest_neighbors(predicted, truth).iter().enumerate() {
        let pc = predicted.get_f64(predicted_id as PointId, predicted_dim) as i64;
        let qc = truth.get_f64(truth_id, truth_dim) as i64;
        matrix[class_index(qc)][class_index(pc)] += 1;
    }

    // `top_sum` and `trace` cover only the rows for evaluated truth labels.
    let top_sum: u64 = matrix[..dim].iter().flatten().sum();
    let trace: u64 = (0..dim).map(|i| matrix[i][i]).sum();

    let metrics = |label: usize| -> LabelMetrics {
        let tp = matrix[label][label];
        let support: u64 = matrix[label].iter().sum();
        let col: u64 = matrix[..dim].iter().map(|row| row[label]).sum();
        let fp = col - tp;
        let fn_ = support - tp;
        let tn = top_sum - tp - fp - fn_;

        let iou = if tn == top_sum {
            0.0
        } else {
            tp as f64 / (tp + fp + fn_) as f64
        };
        let ratio = |num: u64, den: u64| -> f64 {
            if den == 0 {
                0.0
            } else {
                num as f64 / den as f64
            }
        };
        LabelMetrics {
            label: labels[label],
            support,
            intersection_over_union: iou,
            f1_score: 2.0 * iou / (1.0 + iou),
            sensitivity: ratio(tp, tp + fn_),
            specificity: ratio(tn, fp + tn),
            precision: ratio(tp, tp + fp),
            accuracy: if top_sum == 0 {
                0.0
            } else {
                (tp + tn) as f64 / top_sum as f64
            },
        }
    };

    let label_metrics: Vec<LabelMetrics> = (0..dim).map(metrics).collect();
    let mean_iou = if dim == 0 {
        0.0
    } else {
        label_metrics
            .iter()
            .map(|m| m.intersection_over_union)
            .sum::<f64>()
            / dim as f64
    };

    EvalReport {
        labels: label_metrics,
        mean_intersection_over_union: mean_iou,
        overall_accuracy: if top_sum == 0 {
            0.0
        } else {
            trace as f64 / top_sum as f64
        },
        f1_score: 2.0 * mean_iou / (1.0 + mean_iou),
        confusion_matrix: matrix,
    }
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

    /// Build a cloud whose points carry an extra `Classification` label.
    fn labeled_cloud(points: &[(f64, f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for &(x, y, z, c) in points {
            let p = view.add_point();
            view.set_f64(p, &DimId::X, x);
            view.set_f64(p, &DimId::Y, y);
            view.set_f64(p, &DimId::Z, z);
            view.set_f64(p, &DimId::Classification, c);
        }
        view
    }

    #[test]
    fn perfect_prediction_scores_one() {
        let truth = labeled_cloud(&[(0.0, 0.0, 0.0, 1.0), (1.0, 0.0, 0.0, 2.0)]);
        let report = evaluate(
            &truth,
            &truth,
            &DimId::Classification,
            &DimId::Classification,
            &[1, 2],
        );
        assert_eq!(report.overall_accuracy, 1.0);
        assert_eq!(report.mean_intersection_over_union, 1.0);
        for label in &report.labels {
            assert_eq!(label.intersection_over_union, 1.0);
            assert_eq!(label.precision, 1.0);
            assert_eq!(label.sensitivity, 1.0);
            assert_eq!(label.support, 1);
        }
        assert_eq!(
            report.confusion_matrix,
            vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 0],]
        );
    }

    #[test]
    fn one_swapped_label_halves_the_accuracy() {
        // Truth labels two co-located points 1 and 2; the prediction swaps
        // the second point's label.
        let truth = labeled_cloud(&[(0.0, 0.0, 0.0, 1.0), (10.0, 0.0, 0.0, 2.0)]);
        let predicted = labeled_cloud(&[(0.0, 0.0, 0.0, 1.0), (10.0, 0.0, 0.0, 1.0)]);
        let report = evaluate(
            &predicted,
            &truth,
            &DimId::Classification,
            &DimId::Classification,
            &[1, 2],
        );
        // Confusion matrix: truth 1 -> predicted 1; truth 2 -> predicted 1.
        assert_eq!(report.confusion_matrix[0][0], 1);
        assert_eq!(report.confusion_matrix[1][0], 1);
        assert_eq!(report.overall_accuracy, 0.5);
    }

    #[test]
    fn evaluate_sorts_deduplicates_labels_and_tracks_unknown_classes() {
        let truth = labeled_cloud(&[
            (0.0, 0.0, 0.0, 1.0),
            (10.0, 0.0, 0.0, 2.0),
            (20.0, 0.0, 0.0, 9.0),
        ]);
        let predicted = labeled_cloud(&[
            (0.0, 0.0, 0.0, 2.0),
            (10.0, 0.0, 0.0, 2.0),
            (20.0, 0.0, 0.0, 7.0),
        ]);

        let report = evaluate(
            &predicted,
            &truth,
            &DimId::Classification,
            &DimId::Classification,
            &[2, 1, 2],
        );

        assert_eq!(
            report.labels.iter().map(|m| m.label).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            report.confusion_matrix,
            vec![vec![0, 1, 0], vec![0, 1, 0], vec![0, 0, 1]]
        );
        assert_eq!(report.overall_accuracy, 0.5);
        assert_eq!(report.labels[0].support, 1);
        assert_eq!(report.labels[0].precision, 0.0);
        assert_eq!(report.labels[0].sensitivity, 0.0);
        assert_eq!(report.labels[0].specificity, 1.0);
        assert_eq!(report.labels[1].support, 1);
        assert_eq!(report.labels[1].precision, 0.5);
        assert_eq!(report.labels[1].sensitivity, 1.0);
        assert_eq!(report.labels[1].specificity, 0.0);
    }

    #[test]
    fn evaluate_with_no_requested_labels_reports_empty_metrics() {
        let truth = labeled_cloud(&[(0.0, 0.0, 0.0, 1.0)]);
        let predicted = labeled_cloud(&[(0.0, 0.0, 0.0, 1.0)]);

        let report = evaluate(
            &predicted,
            &truth,
            &DimId::Classification,
            &DimId::Classification,
            &[],
        );

        assert!(report.labels.is_empty());
        assert_eq!(report.mean_intersection_over_union, 0.0);
        assert_eq!(report.overall_accuracy, 0.0);
        assert_eq!(report.f1_score, 0.0);
        assert_eq!(report.confusion_matrix, vec![vec![1]]);
    }
}
