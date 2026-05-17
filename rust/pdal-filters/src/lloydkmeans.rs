use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct LloydKMeansFilter {
    k: usize,
    dimensions: Vec<DimId>,
    maxiters: usize,
}

impl LloydKMeansFilter {
    pub fn new(k: usize, dimensions: Vec<DimId>, maxiters: usize) -> Self {
        Self {
            k,
            dimensions,
            maxiters,
        }
    }
}

impl Filter for LloydKMeansFilter {
    fn name(&self) -> &str {
        "filters.lloydkmeans"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        if output.len() < self.k as u64 || self.k == 0 {
            return Ok(vec![output]);
        }

        let center_ids = farthest_point_ids(&output, self.k);
        let mut centers: Vec<Vec<f64>> = center_ids
            .iter()
            .map(|id| {
                self.dimensions
                    .iter()
                    .map(|dim| output.get_f64(*id, dim))
                    .collect()
            })
            .collect();

        for _ in 0..self.maxiters {
            let mut means = vec![vec![0.0; self.k]; self.dimensions.len()];
            let mut counts = vec![0_u64; self.k];

            for point_id in 0..output.len() {
                let cluster = nearest_center(&output, point_id, &self.dimensions, &centers);
                output.set_f64(point_id, &DimId::ClusterID, cluster as f64);
                counts[cluster] += 1;
                let n = counts[cluster] as f64;
                for (dim_idx, dim) in self.dimensions.iter().enumerate() {
                    let delta = output.get_f64(point_id, dim) - means[dim_idx][cluster];
                    means[dim_idx][cluster] += delta / n;
                }
            }

            for cluster in 0..self.k {
                for dim_idx in 0..self.dimensions.len() {
                    centers[cluster][dim_idx] = means[dim_idx][cluster];
                }
            }
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for LloydKMeansFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

fn farthest_point_ids(view: &PointView, count: usize) -> Vec<PointId> {
    let mut ids = vec![0; count];
    let mut min_dists = vec![0.0; view.len() as usize];
    for point_id in 0..view.len() {
        min_dists[point_id as usize] = xyz_distance2(view, point_id, 0);
    }

    for id in ids.iter_mut().skip(1) {
        let (max_idx, _) = min_dists
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .unwrap_or((0, &0.0));
        *id = max_idx as PointId;

        for point_id in 0..view.len() {
            let d2 = xyz_distance2(view, point_id, *id);
            if d2 < min_dists[point_id as usize] {
                min_dists[point_id as usize] = d2;
            }
        }
    }
    ids
}

fn xyz_distance2(view: &PointView, left: PointId, right: PointId) -> f64 {
    let dx = view.get_f64(left, &DimId::X) - view.get_f64(right, &DimId::X);
    let dy = view.get_f64(left, &DimId::Y) - view.get_f64(right, &DimId::Y);
    let dz = view.get_f64(left, &DimId::Z) - view.get_f64(right, &DimId::Z);
    dx * dx + dy * dy + dz * dz
}

fn nearest_center(
    view: &PointView,
    point_id: PointId,
    dimensions: &[DimId],
    centers: &[Vec<f64>],
) -> usize {
    let mut best = 0;
    let mut best_dist = f64::INFINITY;
    for (center_idx, center) in centers.iter().enumerate() {
        let mut dist = 0.0;
        for (dim_idx, dim) in dimensions.iter().enumerate() {
            let delta = view.get_f64(point_id, dim) - center[dim_idx];
            dist += delta * delta;
        }
        if dist < best_dist {
            best_dist = dist;
            best = center_idx;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::collections::BTreeSet;
    use std::rc::Rc;

    #[test]
    fn separates_two_obvious_clusters() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ClusterID, DimType::U64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y) in [(0.0, 0.0), (0.1, 0.0), (100.0, 0.0), (100.1, 0.0)] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, 0.0);
        }

        let mut filter = LloydKMeansFilter::new(2, vec![DimId::X, DimId::Y, DimId::Z], 3);
        let out = filter.run(&view).unwrap().remove(0);
        let left: BTreeSet<_> = [
            out.get_f64(0, &DimId::ClusterID) as u64,
            out.get_f64(1, &DimId::ClusterID) as u64,
        ]
        .into_iter()
        .collect();
        let right: BTreeSet<_> = [
            out.get_f64(2, &DimId::ClusterID) as u64,
            out.get_f64(3, &DimId::ClusterID) as u64,
        ]
        .into_iter()
        .collect();

        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_ne!(left, right);
    }
}
