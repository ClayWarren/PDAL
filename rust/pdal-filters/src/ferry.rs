//! FerryFilter: Copy data from one dimension to another.

use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct FerryFilter {
    pub dims: Vec<(String, String)>,
}

impl FerryFilter {
    pub fn new(dims: Vec<(String, String)>) -> Self {
        Self { dims }
    }

    pub fn ferry_point(&self, view: &mut PointView, idx: u64) {
        for (from_name, to_name) in &self.dims {
            let from_id = parse_dim_id(from_name);
            let to_id = parse_dim_id(to_name);
            let val = view.get_f64(idx, &from_id);
            view.set_f64(idx, &to_id, val);
        }
    }
}

fn parse_dim_id(name: &str) -> DimId {
    match name {
        "X" => DimId::X,
        "Y" => DimId::Y,
        "Z" => DimId::Z,
        "Intensity" => DimId::Intensity,
        "OffsetTime" => DimId::OffsetTime,
        "Classification" => DimId::Classification,
        other => DimId::Other(other.to_string()),
    }
}

impl Filter for FerryFilter {
    fn name(&self) -> &str {
        "filters.ferry"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();

        for idx in 0..input.len() {
            out.append_point(input, idx);
            let out_idx = out.len() - 1;
            self.ferry_point(&mut out, out_idx);
        }

        Ok(vec![out])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for FerryFilter {
    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        // FerryFilter processes/modifies points inline rather than keeping/dropping,
        // so its streaming behavior is driven via the FFI pdal_stage_ferry_point.
        false
    }

    fn reset(&mut self) {}
}
