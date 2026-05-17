use pdal_core::options::Options;
use pdal_core::point::{DimId, PointId, PointView};
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
}

impl Filter for VoxelDownsizeFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.voxeldownsize"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        self.populated_voxels.clear();
        self.origin = None;

        let mut output = PointView::new(view.layout().clone());
        let size = view.len();

        for idx in 0..size {
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            let z = view.get_f64(idx, &DimId::Z);

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
            let inserted = self.populated_voxels.insert(voxel);
            if inserted {
                output.append_point(view, idx);
                if self.mode == VoxelMode::Center {
                    let out_idx = output.len() - 1;
                    output.set_f64(out_idx, &DimId::X, (vx as f64 + 0.5) * self.cell + ox);
                    output.set_f64(out_idx, &DimId::Y, (vy as f64 + 0.5) * self.cell + oy);
                    output.set_f64(out_idx, &DimId::Z, (vz as f64 + 0.5) * self.cell + oz);
                }
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for VoxelDownsizeFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }

    fn reset(&mut self) {
        self.populated_voxels.clear();
        self.origin = None;
    }
}
