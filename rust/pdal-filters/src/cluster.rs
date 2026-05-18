use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct ClusterFilter {
    min_points: usize,
    max_points: usize,
    tolerance: f64,
    is_3d: bool,
}

impl ClusterFilter {
    pub fn new(min_points: usize, max_points: usize, tolerance: f64, is_3d: bool) -> Self {
        Self {
            min_points,
            max_points,
            tolerance,
            is_3d,
        }
    }
}

impl Filter for ClusterFilter {
    fn name(&self) -> &str {
        "filters.cluster"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        let dims = if self.is_3d {
            vec![DimId::X, DimId::Y, DimId::Z]
        } else {
            vec![DimId::X, DimId::Y]
        };
        let mut processed = vec![false; view.len() as usize];
        let mut clusters = Vec::new();

        for idx in 0..view.len() {
            if processed[idx as usize] {
                continue;
            }

            let mut seed_queue = vec![idx];
            processed[idx as usize] = true;
            let mut queue_idx = 0;

            while queue_idx < seed_queue.len() {
                let point = seed_queue[queue_idx];
                let neighbors = index.radius_dims(point, self.tolerance, &dims);
                if neighbors.len() == 1 {
                    queue_idx += 1;
                    continue;
                }

                for neighbor in neighbors {
                    if processed[neighbor as usize] {
                        continue;
                    }
                    seed_queue.push(neighbor);
                    processed[neighbor as usize] = true;
                }
                queue_idx += 1;
            }

            if seed_queue.len() >= self.min_points && seed_queue.len() <= self.max_points {
                clusters.push(seed_queue);
            }
        }

        for (cluster_idx, cluster) in clusters.into_iter().enumerate() {
            let id = (cluster_idx + 1) as f64;
            for point_id in cluster {
                output.set_f64(point_id, &DimId::ClusterID, id);
            }
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ClusterFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::collections::BTreeSet;
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

    #[test]
    fn finds_two_clusters() {
        let shape = [
            (0.0, 0.0, 0.0),
            (0.5, 0.0, 0.0),
            (0.0, 0.5, 0.0),
            (0.5, 0.5, 0.0),
        ];
        let mut points = shape.to_vec();
        points.extend(shape.iter().map(|p| (p.0 + 100.0, p.1, p.2)));
        let view = view(&points);
        let mut filter = ClusterFilter::new(1, usize::MAX, 1.0, true);
        let out = filter.run(&view).unwrap().remove(0);

        let ids = (0..out.len())
            .map(|idx| out.get_f64(idx, &DimId::ClusterID) as i64)
            .collect::<BTreeSet<_>>();
        assert_eq!(ids, BTreeSet::from([1, 2]));
        for idx in 1..4 {
            assert_eq!(
                out.get_f64(idx, &DimId::ClusterID),
                out.get_f64(0, &DimId::ClusterID)
            );
            assert_eq!(
                out.get_f64(idx + 4, &DimId::ClusterID),
                out.get_f64(4, &DimId::ClusterID)
            );
        }
    }

    #[test]
    fn is3d_toggle_controls_z_distance() {
        let view = view(&[(0.0, 0.0, 0.0), (0.0, 0.0, 50.0)]);

        let mut filter = ClusterFilter::new(1, usize::MAX, 1.0, false);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(
            out.get_f64(0, &DimId::ClusterID),
            out.get_f64(1, &DimId::ClusterID)
        );

        let mut filter = ClusterFilter::new(1, usize::MAX, 1.0, true);
        let out = filter.run(&view).unwrap().remove(0);
        assert_ne!(
            out.get_f64(0, &DimId::ClusterID),
            out.get_f64(1, &DimId::ClusterID)
        );
    }
}
