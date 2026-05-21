use pdal_core::options::Options;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelMode {
    First,
    Center,
}

pub struct VoxelDownsizeFilter {
    pub cell: f64,
    pub mode: VoxelMode,
    populated_voxels: HashSet<(i32, i32, i32)>,
    origin: Option<(f64, f64, f64)>,
}

impl VoxelDownsizeFilter {
    pub fn new(ops: &Options) -> Self {
        let cell = ops.get_f64("cell", 0.001);
        let mode_str = ops.get_str("mode", "center").to_lowercase();
        let mode = match mode_str.as_str() {
            "first" => VoxelMode::First,
            _ => VoxelMode::Center,
        };
        Self {
            cell,
            mode,
            populated_voxels: HashSet::new(),
            origin: None,
        }
    }

    fn keep_point(&mut self, x: f64, y: f64, z: f64) -> Option<(f64, f64, f64)> {
        let (ox, oy, oz) = if let Some(origin) = self.origin {
            origin
        } else {
            let origin = (
                x - self.cell / 2.0,
                y - self.cell / 2.0,
                z - self.cell / 2.0,
            );
            self.origin = Some(origin);
            origin
        };

        let vx = ((x - ox) / self.cell).floor() as i32;
        let vy = ((y - oy) / self.cell).floor() as i32;
        let vz = ((z - oz) / self.cell).floor() as i32;

        let voxel = (vx, vy, vz);
        if !self.populated_voxels.insert(voxel) {
            return None;
        }

        if self.mode == VoxelMode::Center {
            Some((
                (vx as f64 + 0.5) * self.cell + ox,
                (vy as f64 + 0.5) * self.cell + oy,
                (vz as f64 + 0.5) * self.cell + oz,
            ))
        } else {
            Some((x, y, z))
        }
    }
}

impl Filter for VoxelDownsizeFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.voxeldownsize"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        self.populated_voxels.clear();
        self.origin = None;

        let mut output = PointView::new(view.layout().clone());
        let size = view.len();

        for idx in 0..size {
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            let z = view.get_f64(idx, &DimId::Z);
            if let Some((x, y, z)) = self.keep_point(x, y, z) {
                output.append_point(view, idx);
                let out_idx = output.len() - 1;
                output.set_f64(out_idx, &DimId::X, x);
                output.set_f64(out_idx, &DimId::Y, y);
                output.set_f64(out_idx, &DimId::Z, z);
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for VoxelDownsizeFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);
        if let Some((x, y, z)) = self.keep_point(x, y, z) {
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
            true
        } else {
            false
        }
    }

    fn reset(&mut self) {
        self.populated_voxels.clear();
        self.origin = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, *x);
            view.set_f64(id, &DimId::Y, *y);
            view.set_f64(id, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn first_mode_keeps_first_point_per_voxel() {
        let input = view(&[(0.0, 0.0, 0.0), (0.1, 0.1, 0.1), (1.1, 0.0, 0.0)]);
        let mut ops = Options::default();
        ops.add("cell", 1.0);
        ops.add("mode", "first");
        let mut filter = VoxelDownsizeFilter::new(&ops);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);

        assert_eq!(out.len(), 2);
        assert_eq!(out.get_f64(0, &DimId::X), 0.0);
        assert_eq!(out.get_f64(1, &DimId::X), 1.1);
    }

    #[test]
    fn center_mode_moves_kept_point_to_voxel_center_and_reset_clears_state() {
        let input = view(&[(1.0, 2.0, 3.0), (1.2, 2.2, 3.2), (2.2, 2.0, 3.0)]);
        let mut ops = Options::default();
        ops.add("cell", 1.0);
        let mut filter = VoxelDownsizeFilter::new(&ops);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);

        assert_eq!(out.len(), 2);
        assert_eq!(out.get_f64(0, &DimId::X), 1.0);
        assert_eq!(out.get_f64(0, &DimId::Y), 2.0);
        assert_eq!(out.get_f64(0, &DimId::Z), 3.0);
        assert_eq!(out.get_f64(1, &DimId::X), 2.0);

        let mut stream_view = view(&[(0.0, 0.0, 0.0)]);
        assert!(filter.process_one(&mut stream_view, 0));
        assert!(!filter.process_one(&mut stream_view, 0));
        filter.reset();
        assert!(filter.populated_voxels.is_empty());
        assert!(filter.origin.is_none());
    }
}
