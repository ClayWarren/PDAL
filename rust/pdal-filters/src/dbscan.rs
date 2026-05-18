use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct DbscanFilter {
    min_points: usize,
    eps: f64,
    dims: Vec<DimId>,
}

impl DbscanFilter {
    pub fn new(min_points: usize, eps: f64, dim_names: Vec<String>) -> Self {
        Self {
            min_points,
            eps,
            dims: dim_names
                .into_iter()
                .map(|name| DimId::from_name(&name))
                .collect(),
        }
    }
}

impl Filter for DbscanFilter {
    fn name(&self) -> &str {
        "filters.dbscan"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        let neighbors = (0..view.len())
            .map(|idx| index.radius_dims(idx, self.eps, &self.dims))
            .collect::<Vec<_>>();

        for idx in 0..view.len() {
            output.set_f64(idx, &DimId::ClusterID, -2.0);
        }

        let mut cluster_label = 0.0;
        for idx in 0..view.len() {
            if output.get_f64(idx, &DimId::ClusterID) != -2.0 {
                continue;
            }

            if neighbors[idx as usize].len() < self.min_points {
                output.set_f64(idx, &DimId::ClusterID, -1.0);
                continue;
            }

            let mut next = neighbors[idx as usize].clone();
            let mut visited = vec![idx];
            output.set_f64(idx, &DimId::ClusterID, cluster_label);

            while let Some(point) = next.pop() {
                if visited.contains(&point) {
                    continue;
                }
                visited.push(point);

                if output.get_f64(point, &DimId::ClusterID) == -1.0 {
                    output.set_f64(point, &DimId::ClusterID, cluster_label);
                }

                if output.get_f64(point, &DimId::ClusterID) != -2.0 {
                    continue;
                }

                output.set_f64(point, &DimId::ClusterID, cluster_label);
                if neighbors[point as usize].len() >= self.min_points {
                    for neighbor in &neighbors[point as usize] {
                        if !visited.contains(neighbor) && !next.contains(neighbor) {
                            next.push(*neighbor);
                        }
                    }
                }
            }

            cluster_label += 1.0;
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for DbscanFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
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

    fn cube(cx: f64, cy: f64, cz: f64) -> Vec<(f64, f64, f64)> {
        (0..8)
            .map(|i| {
                (
                    cx + if i & 1 != 0 { 0.3 } else { 0.0 },
                    cy + if i & 2 != 0 { 0.3 } else { 0.0 },
                    cz + if i & 4 != 0 { 0.3 } else { 0.0 },
                )
            })
            .collect()
    }

    #[test]
    fn finds_two_clusters_and_noise() {
        let mut points = cube(0.0, 0.0, 0.0);
        points.extend(cube(50.0, 50.0, 50.0));
        points.push((500.0, 500.0, 500.0));
        let view = view(&points);
        let mut filter = DbscanFilter::new(6, 1.0, vec!["X".into(), "Y".into(), "Z".into()]);
        let out = filter.run(&view).unwrap().remove(0);

        for idx in 1..8 {
            assert_eq!(
                out.get_f64(idx, &DimId::ClusterID),
                out.get_f64(0, &DimId::ClusterID)
            );
            assert_eq!(
                out.get_f64(idx + 8, &DimId::ClusterID),
                out.get_f64(8, &DimId::ClusterID)
            );
        }
        assert_ne!(
            out.get_f64(0, &DimId::ClusterID),
            out.get_f64(8, &DimId::ClusterID)
        );
        assert_eq!(out.get_f64(16, &DimId::ClusterID), -1.0);
    }

    #[test]
    fn dimensions_restrict_clustering() {
        let points = (0..6)
            .map(|i| (0.1 * (i & 1) as f64, 0.1 * (i & 1) as f64, i as f64 * 30.0))
            .collect::<Vec<_>>();
        let view = view(&points);

        let mut filter = DbscanFilter::new(6, 1.0, vec!["X".into(), "Y".into()]);
        let out = filter.run(&view).unwrap().remove(0);
        assert!((0..out.len()).all(|idx| out.get_f64(idx, &DimId::ClusterID) >= 0.0));

        let mut filter = DbscanFilter::new(6, 1.0, vec!["X".into(), "Y".into(), "Z".into()]);
        let out = filter.run(&view).unwrap().remove(0);
        assert!((0..out.len()).any(|idx| out.get_f64(idx, &DimId::ClusterID) < 0.0));
    }
}
