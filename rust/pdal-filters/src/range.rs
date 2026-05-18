//! RangeFilter: Pass only points given a dimension/range.

use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Clone)]
pub struct RangeLimit {
    pub dim_name: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
}

impl RangeLimit {
    pub fn value_passes(&self, v: f64) -> bool {
        if v.is_nan() {
            return self.negate;
        }
        let fail = (self.inclusive_lower && v < self.lower_bound)
            || (!self.inclusive_lower && v <= self.lower_bound)
            || (self.inclusive_upper && v > self.upper_bound)
            || (!self.inclusive_upper && v >= self.upper_bound);
        if self.negate {
            fail
        } else {
            !fail
        }
    }
}

pub struct RangeFilter {
    pub limits: Vec<RangeLimit>,
}

impl RangeFilter {
    pub fn new(limits: Vec<RangeLimit>) -> Self {
        Self { limits }
    }

    pub fn point_passes(&self, view: &PointView, idx: u64) -> bool {
        if self.limits.is_empty() {
            return true;
        }

        // Sort limits by dimension name to ensure contiguous grouping
        // (matches C++ std::sort on m_ranges)
        let mut sorted_limits = self.limits.clone();
        sorted_limits.sort_by(|a, b| a.dim_name.cmp(&b.dim_name));

        let mut last_dim = &sorted_limits[0].dim_name;
        let mut passes = false;

        for r in &sorted_limits {
            if &r.dim_name != last_dim {
                if !passes {
                    return false;
                }
                last_dim = &r.dim_name;
            } else if passes {
                continue;
            }

            let dim_id = parse_dim_id(&r.dim_name);
            let val = view.get_f64(idx, &dim_id);
            passes = r.value_passes(val);
        }

        passes
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

impl Filter for RangeFilter {
    fn name(&self) -> &str {
        "filters.range"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();

        for idx in 0..input.len() {
            if self.point_passes(input, idx) {
                out.append_point(input, idx);
            }
        }

        Ok(vec![out])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for RangeFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        // Stateful point-by-point filtering is supported via point_passes in the C ABI
        false
    }

    fn reset(&mut self) {}
}
