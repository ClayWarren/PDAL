use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct AssignRange {
    pub dim_name: String,
    pub value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
}

pub struct AssignCondition {
    pub dim_name: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
}

pub struct AssignFilter {
    pub condition: Option<AssignCondition>,
    pub assignments: Vec<AssignRange>,
}

impl AssignFilter {
    pub fn new(condition: Option<AssignCondition>, assignments: Vec<AssignRange>) -> Self {
        Self {
            condition,
            assignments,
        }
    }

    pub fn value_passes(
        v: f64,
        lower: f64,
        upper: f64,
        inclusive_lower: bool,
        inclusive_upper: bool,
        negate: bool,
    ) -> bool {
        if v.is_nan() {
            return negate;
        }
        let mut fail = (inclusive_lower && v < lower)
            || (!inclusive_lower && v <= lower)
            || (inclusive_upper && v > upper)
            || (!inclusive_upper && v >= upper);
        if negate {
            fail = !fail;
        }
        !fail
    }

    pub fn assign_point(&self, view: &mut PointView, idx: u64) {
        if let Some(ref cond) = self.condition {
            let dim = DimId::from_name(&cond.dim_name);
            let val = view.get_f64(idx, &dim);
            if !Self::value_passes(
                val,
                cond.lower_bound,
                cond.upper_bound,
                cond.inclusive_lower,
                cond.inclusive_upper,
                cond.negate,
            ) {
                return;
            }
        }

        for r in &self.assignments {
            let dim = DimId::from_name(&r.dim_name);
            let val = view.get_f64(idx, &dim);
            if Self::value_passes(
                val,
                r.lower_bound,
                r.upper_bound,
                r.inclusive_lower,
                r.inclusive_upper,
                r.negate,
            ) {
                view.set_f64(idx, &dim, r.value);
            }
        }
    }
}

impl Filter for AssignFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.assign"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let size = view.len();
        let mut output = PointView::new(view.layout().clone());
        for i in 0..size {
            output.append_point(view, i);
            let out_idx = output.len() - 1;
            self.assign_point(&mut output, out_idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for AssignFilter {
    fn process_one(&mut self) -> bool {
        false
    }

    fn reset(&mut self) {}
}
