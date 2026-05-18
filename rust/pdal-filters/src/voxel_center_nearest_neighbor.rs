use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

pub struct VoxelCenterNearestNeighborFilter {
    cell: f64,
}

impl VoxelCenterNearestNeighborFilter {
    pub fn new(cell: f64) -> Self {
        Self { cell }
    }
}

impl Filter for VoxelCenterNearestNeighborFilter {
    fn name(&self) -> &str {
        "filters.voxelcenternearestneighbor"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        if view.is_empty() {
            return Ok(vec![output]);
        }

        let mut min_x = view.get_f64(0, &DimId::X);
        let mut min_y = view.get_f64(0, &DimId::Y);
        let mut min_z = view.get_f64(0, &DimId::Z);
        for idx in 1..view.len() {
            min_x = min_x.min(view.get_f64(idx, &DimId::X));
            min_y = min_y.min(view.get_f64(idx, &DimId::Y));
            min_z = min_z.min(view.get_f64(idx, &DimId::Z));
        }

        let mut voxels = BTreeMap::<(usize, usize, usize), (PointId, f64)>::new();
        for idx in 0..view.len() {
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            let z = view.get_f64(idx, &DimId::Z);
            let c = ((x - min_x) / self.cell) as usize;
            let r = ((y - min_y) / self.cell) as usize;
            let d = ((z - min_z) / self.cell) as usize;
            let center_x = min_x + (c as f64 + 0.5) * self.cell;
            let center_y = min_y + (r as f64 + 0.5) * self.cell;
            let center_z = min_z + (d as f64 + 0.5) * self.cell;
            let dist = squared(center_x - x) + squared(center_y - y) + squared(center_z - z);

            match voxels.get_mut(&(r, c, d)) {
                Some((kept_id, kept_dist)) if dist < *kept_dist => {
                    *kept_id = idx;
                    *kept_dist = dist;
                }
                None => {
                    voxels.insert((r, c, d), (idx, dist));
                }
                _ => {}
            }
        }

        for (_key, (idx, _dist)) in voxels {
            output.append_point(view, idx);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for VoxelCenterNearestNeighborFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

fn squared(value: f64) -> f64 {
    value * value
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
    fn keeps_point_nearest_voxel_center() {
        let view = view(&[
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 0.0),
            (2.0, 2.0, 0.0),
            (4.0, 4.0, 0.0),
        ]);
        let mut filter = VoxelCenterNearestNeighborFilter::new(2.0);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.len(), 3);
        assert_eq!(out.get_f64(0, &DimId::X), 1.0);
        assert_eq!(out.get_f64(1, &DimId::X), 2.0);
        assert_eq!(out.get_f64(2, &DimId::X), 4.0);
    }

    #[test]
    fn preserves_source_index_on_selected_points() {
        let view = view(&[(0.0, 0.0, 0.0), (1.0, 1.0, 0.0)]);
        let mut filter = VoxelCenterNearestNeighborFilter::new(2.0);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.len(), 1);
        assert_eq!(out.source_index(0), 1);
    }
}
