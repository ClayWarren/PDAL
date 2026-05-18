use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

pub struct VoxelCentroidNearestNeighborFilter {
    cell: f64,
}

impl VoxelCentroidNearestNeighborFilter {
    pub fn new(cell: f64) -> Self {
        Self { cell }
    }
}

impl Filter for VoxelCentroidNearestNeighborFilter {
    fn name(&self) -> &str {
        "filters.voxelcentroidnearestneighbor"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        if view.is_empty() {
            return Ok(vec![output]);
        }

        let x0 = view.get_f64(0, &DimId::X);
        let y0 = view.get_f64(0, &DimId::Y);
        let z0 = view.get_f64(0, &DimId::Z);

        let mut voxels = BTreeMap::<(isize, isize, isize), Vec<PointId>>::new();
        for idx in 0..view.len() {
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            let z = view.get_f64(idx, &DimId::Z);
            let r = ((y - y0) / self.cell).floor() as isize;
            let c = ((x - x0) / self.cell).floor() as isize;
            let d = ((z - z0) / self.cell).floor() as isize;
            voxels.entry((r, c, d)).or_default().push(idx);
        }

        for (key, ids) in voxels {
            let keep = match ids.as_slice() {
                [single] => *single,
                [first, second] => {
                    let center = (
                        x0 + (key.1 as f64 + 0.5) * self.cell,
                        y0 + (key.0 as f64 + 0.5) * self.cell,
                        z0 + (key.2 as f64 + 0.5) * self.cell,
                    );
                    let first_dist = squared_distance_to(view, *first, center);
                    let second_dist = squared_distance_to(view, *second, center);
                    if first_dist < second_dist {
                        *first
                    } else {
                        *second
                    }
                }
                many => {
                    let centroid = centroid(view, many);
                    many.iter()
                        .copied()
                        .min_by(|a, b| {
                            squared_distance_to(view, *a, centroid)
                                .total_cmp(&squared_distance_to(view, *b, centroid))
                        })
                        .unwrap()
                }
            };
            output.append_point(view, keep);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for VoxelCentroidNearestNeighborFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

fn centroid(view: &PointView, ids: &[PointId]) -> (f64, f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    for id in ids {
        x += view.get_f64(*id, &DimId::X);
        y += view.get_f64(*id, &DimId::Y);
        z += view.get_f64(*id, &DimId::Z);
    }
    let count = ids.len() as f64;
    (x / count, y / count, z / count)
}

fn squared_distance_to(
    view: &PointView,
    idx: pdal_core::point::PointId,
    point: (f64, f64, f64),
) -> f64 {
    let dx = point.0 - view.get_f64(idx, &DimId::X);
    let dy = point.1 - view.get_f64(idx, &DimId::Y);
    let dz = point.2 - view.get_f64(idx, &DimId::Z);
    dx * dx + dy * dy + dz * dz
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
    fn matches_existing_synthetic_case() {
        let view = view(&[
            (5.0, 5.0, 5.0),
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (11.0, 11.0, 11.0),
            (21.0, 21.0, 21.0),
        ]);
        let mut filter = VoxelCentroidNearestNeighborFilter::new(10.0);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.len(), 3);
        assert_eq!(out.source_index(0), 1);
        assert_eq!(out.source_index(1), 3);
        assert_eq!(out.source_index(2), 4);
    }

    #[test]
    fn more_than_two_points_use_centroid() {
        let view = view(&[(0.0, 0.0, 0.0), (4.0, 0.0, 0.0), (5.0, 0.0, 0.0)]);
        let mut filter = VoxelCentroidNearestNeighborFilter::new(10.0);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.len(), 1);
        assert_eq!(out.source_index(0), 1);
    }
}
