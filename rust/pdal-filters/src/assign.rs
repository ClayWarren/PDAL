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

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let size = view.len();
        let mut output = PointView::new(view.layout().clone());
        for i in 0..size {
            output.append_point(view, i);
            let out_idx = output.len() - 1;
            self.assign_point(&mut output, out_idx);
        }
        Ok(vec![output])
    }

    fn streamable(&self) -> bool {
        true
    }

    fn stream_chunk(&mut self, chunk: &mut PointView) -> Result<(), StageError> {
        // Same per-point assignment as `run_one`; assign keeps every point.
        for idx in 0..chunk.len() {
            self.assign_point(chunk, idx);
        }
        Ok(())
    }
}

impl Streamable for AssignFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        self.assign_point(view, idx);
        true
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(values: &[f64]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for value in values {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, *value);
            view.set_f64(id, &DimId::Classification, 1.0);
        }
        view
    }

    #[test]
    fn assigns_values_inside_range_when_condition_passes() {
        let input = view(&[0.0, 5.0, 10.0]);
        let mut filter = AssignFilter::new(
            Some(AssignCondition {
                dim_name: "X".to_string(),
                lower_bound: 0.0,
                upper_bound: 10.0,
                inclusive_lower: true,
                inclusive_upper: true,
                negate: false,
            }),
            vec![AssignRange {
                dim_name: "Classification".to_string(),
                value: 7.0,
                lower_bound: 1.0,
                upper_bound: 1.0,
                inclusive_lower: true,
                inclusive_upper: true,
                negate: false,
            }],
        );

        let output = filter.run_one(&input).unwrap().remove(0);

        assert_eq!(output.get_f64(0, &DimId::Classification), 7.0);
        assert_eq!(output.get_f64(1, &DimId::Classification), 7.0);
        assert_eq!(output.get_f64(2, &DimId::Classification), 7.0);
        assert_eq!(input.get_f64(0, &DimId::Classification), 1.0);
    }

    #[test]
    fn stream_chunk_matches_run_one() {
        let make_filter = || {
            AssignFilter::new(
                Some(AssignCondition {
                    dim_name: "X".to_string(),
                    lower_bound: 0.0,
                    upper_bound: 10.0,
                    inclusive_lower: true,
                    inclusive_upper: true,
                    negate: false,
                }),
                vec![AssignRange {
                    dim_name: "Classification".to_string(),
                    value: 7.0,
                    lower_bound: 1.0,
                    upper_bound: 1.0,
                    inclusive_lower: true,
                    inclusive_upper: true,
                    negate: false,
                }],
            )
        };

        let input = view(&[0.0, 5.0, 10.0, 15.0]);
        assert!(make_filter().streamable());

        let standard = make_filter().run_one(&input).unwrap().remove(0);

        let mut chunk = input.clone();
        make_filter().stream_chunk(&mut chunk).unwrap();

        assert_eq!(chunk.len(), standard.len());
        for i in 0..standard.len() {
            assert_eq!(
                chunk.get_f64(i, &DimId::Classification),
                standard.get_f64(i, &DimId::Classification),
                "point {i}"
            );
            assert_eq!(chunk.get_f64(i, &DimId::X), standard.get_f64(i, &DimId::X));
        }
    }

    #[test]
    fn exclusive_bounds_negation_nan_and_streaming_match_range_contract() {
        assert!(!AssignFilter::value_passes(
            1.0, 1.0, 2.0, false, true, false
        ));
        assert!(AssignFilter::value_passes(1.0, 1.0, 2.0, false, true, true));
        assert!(!AssignFilter::value_passes(
            f64::NAN,
            1.0,
            2.0,
            true,
            true,
            false
        ));
        assert!(AssignFilter::value_passes(
            f64::NAN,
            1.0,
            2.0,
            true,
            true,
            true
        ));

        let mut filter = AssignFilter::new(
            None,
            vec![AssignRange {
                dim_name: "Classification".to_string(),
                value: 9.0,
                lower_bound: 1.0,
                upper_bound: 1.0,
                inclusive_lower: true,
                inclusive_upper: true,
                negate: false,
            }],
        );
        let mut input = view(&[1.0]);

        assert!(filter.process_one(&mut input, 0));
        assert_eq!(input.get_f64(0, &DimId::Classification), 9.0);
        filter.reset();
    }
}
