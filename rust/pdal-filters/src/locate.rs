use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError};

pub struct LocateFilter {
    dim_name: String,
    minmax: String,
}

impl LocateFilter {
    pub fn new(dim_name: String, minmax: String) -> Self {
        LocateFilter { dim_name, minmax }
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

impl Filter for LocateFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.locate"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        if input.is_empty() {
            return Ok(vec![out]);
        }

        let dim_id = parse_dim_id(&self.dim_name);

        let mut min_idx = 0;
        let mut max_idx = 0;
        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;

        for idx in 0..input.len() {
            let val = input.get_f64(idx, &dim_id);
            if val > max_val {
                max_val = val;
                max_idx = idx;
            }
            if val < min_val {
                min_val = val;
                min_idx = idx;
            }
        }

        let minmax = self.minmax.to_lowercase();
        if minmax == "min" {
            out.append_point(input, min_idx);
        } else if minmax == "max" {
            out.append_point(input, max_idx);
        }

        Ok(vec![out])
    }
}

impl pdal_core::stage::Streamable for LocateFilter {
    fn process_one(&mut self, _view: &pdal_core::point::PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
