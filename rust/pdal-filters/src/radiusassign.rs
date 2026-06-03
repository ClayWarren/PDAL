use pdal_core::expr::AssignStatement;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

use crate::range::RangeLimit;

pub struct RadiusAssignFilter {
    src_domain: Vec<RangeLimit>,
    reference_domain: Vec<RangeLimit>,
    assignments: Vec<AssignStatement>,
    assignments_prepared: bool,
    radius: f64,
    search_3d: bool,
    max_2d_above: f64,
    max_2d_below: f64,
}

impl RadiusAssignFilter {
    pub fn new(
        src_domain: Vec<RangeLimit>,
        reference_domain: Vec<RangeLimit>,
        assignments: Vec<AssignStatement>,
        radius: f64,
        search_3d: bool,
        max_2d_above: f64,
        max_2d_below: f64,
    ) -> Self {
        Self {
            src_domain,
            reference_domain,
            assignments,
            assignments_prepared: true,
            radius,
            search_3d,
            max_2d_above,
            max_2d_below,
        }
    }

    pub fn with_update_expressions(
        src_domain: Vec<RangeLimit>,
        reference_domain: Vec<RangeLimit>,
        expressions: &[String],
        radius: f64,
        search_3d: bool,
        max_2d_above: f64,
        max_2d_below: f64,
    ) -> Result<Self, StageError> {
        let assignments = parse_unprepared_assignments(expressions)?;
        Ok(Self {
            src_domain,
            reference_domain,
            assignments,
            assignments_prepared: false,
            radius,
            search_3d,
            max_2d_above,
            max_2d_below,
        })
    }

    fn point_passes_domain(domain: &[RangeLimit], view: &PointView, idx: u64) -> bool {
        domain.is_empty()
            || domain.iter().any(|range| {
                let dim = DimId::from_name(&range.dim_name);
                range.value_passes(view.get_f64(idx, &dim))
            })
    }

    fn reference_ids(&self, view: &PointView) -> Vec<u64> {
        (0..view.len())
            .filter(|&idx| Self::point_passes_domain(&self.reference_domain, view, idx))
            .collect()
    }

    fn has_neighbor(&self, view: &PointView, src: u64, references: &[u64]) -> bool {
        let x = view.get_f64(src, &DimId::X);
        let y = view.get_f64(src, &DimId::Y);
        let z = view.get_f64(src, &DimId::Z);
        let radius_sqr = self.radius * self.radius;

        for &candidate in references {
            let dx = view.get_f64(candidate, &DimId::X) - x;
            let dy = view.get_f64(candidate, &DimId::Y) - y;
            let dz = view.get_f64(candidate, &DimId::Z) - z;
            let distance_sqr = if self.search_3d {
                dx * dx + dy * dy + dz * dz
            } else {
                dx * dx + dy * dy
            };
            if distance_sqr >= radius_sqr {
                continue;
            }

            if !self.search_3d {
                if self.max_2d_above >= 0.0 && dz > self.max_2d_above {
                    continue;
                }
                if self.max_2d_below >= 0.0 && -dz > self.max_2d_below {
                    continue;
                }
            }

            return true;
        }

        false
    }

    fn prepare_assignments(&mut self, view: &PointView) -> Result<(), StageError> {
        if self.assignments_prepared {
            return Ok(());
        }
        for assignment in &mut self.assignments {
            assignment
                .prepare(view.layout().as_ref())
                .map_err(|err| StageError(format!("filters.radiusassign: {err}")))?;
        }
        self.assignments_prepared = true;
        Ok(())
    }
}

impl Filter for RadiusAssignFilter {
    fn name(&self) -> &str {
        "filters.radiusassign"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        if self.radius <= 0.0 {
            return Err(StageError(
                "filters.radiusassign: radius must be greater than zero.".to_string(),
            ));
        }

        self.prepare_assignments(view)?;

        let references = self.reference_ids(view);
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        for idx in 0..view.len() {
            if !Self::point_passes_domain(&self.src_domain, view, idx) {
                continue;
            }
            if !self.has_neighbor(view, idx, &references) {
                continue;
            }

            for assignment in &self.assignments {
                if !assignment.condition().eval(view, idx) {
                    continue;
                }
                let Some(dim) = assignment.ident().dim() else {
                    return Err(StageError(
                        "filters.radiusassign: invalid assignment target.".to_string(),
                    ));
                };
                output.set_f64(idx, &dim, assignment.value().eval(view, idx));
            }
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn parse_assignments(
    expressions: &[String],
    layout: &pdal_core::point::PointLayout,
) -> Result<Vec<AssignStatement>, StageError> {
    let mut assignments = parse_unprepared_assignments(expressions)?;
    for statement in &mut assignments {
        statement
            .prepare(layout)
            .map_err(|err| StageError(format!("filters.radiusassign: {err}")))?;
    }
    Ok(assignments)
}

fn parse_unprepared_assignments(
    expressions: &[String],
) -> Result<Vec<AssignStatement>, StageError> {
    if expressions.is_empty() {
        return Err(StageError(
            "Empty 'update_expression' option, must be set to apply any change on the data"
                .to_string(),
        ));
    }
    expressions
        .iter()
        .map(|expression| {
            AssignStatement::parse(expression)
                .map_err(|err| StageError(format!("filters.radiusassign: {err}")))
        })
        .collect()
}

impl Streamable for RadiusAssignFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn test_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, class) in [
            (0.0, 0.0, 0.0, 1.0),
            (0.5, 0.0, 0.0, 0.0),
            (0.0, 0.5, -2.0, 0.0),
            (10.0, 0.0, 0.0, 0.0),
        ] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
            view.set_f64(idx, &DimId::Classification, class);
        }
        view
    }

    fn class_one_domain() -> Vec<RangeLimit> {
        vec![RangeLimit {
            dim_name: "Classification".to_string(),
            lower_bound: 1.0,
            upper_bound: 1.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }]
    }

    #[test]
    fn assigns_points_inside_reference_radius() {
        let view = test_view();
        let mut filter = RadiusAssignFilter::new(
            Vec::new(),
            class_one_domain(),
            parse_assignments(
                &[String::from("Classification = 2")],
                view.layout().as_ref(),
            )
            .unwrap(),
            1.0,
            true,
            -1.0,
            -1.0,
        );
        let out = filter.run_one(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Classification), 2.0);
        assert_eq!(out.get_f64(1, &DimId::Classification), 2.0);
        assert_eq!(out.get_f64(2, &DimId::Classification), 0.0);
        assert_eq!(out.get_f64(3, &DimId::Classification), 0.0);
    }

    #[test]
    fn z_limits_apply_to_2d_searches() {
        let view = test_view();
        let mut filter = RadiusAssignFilter::new(
            Vec::new(),
            class_one_domain(),
            parse_assignments(
                &[String::from("Classification = 2")],
                view.layout().as_ref(),
            )
            .unwrap(),
            1.0,
            false,
            1.0,
            -1.0,
        );
        let out = filter.run_one(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(1, &DimId::Classification), 2.0);
        assert_eq!(out.get_f64(2, &DimId::Classification), 0.0);
    }

    #[test]
    fn update_expressions_prepare_against_input_layout() {
        let view = test_view();
        let mut filter = RadiusAssignFilter::with_update_expressions(
            Vec::new(),
            class_one_domain(),
            &[String::from("Classification = Z + 3 WHERE X < 1")],
            1.0,
            true,
            -1.0,
            -1.0,
        )
        .unwrap();
        let out = filter.run_one(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Classification), 3.0);
        assert_eq!(out.get_f64(1, &DimId::Classification), 3.0);
        assert_eq!(out.get_f64(2, &DimId::Classification), 0.0);
        assert_eq!(out.get_f64(3, &DimId::Classification), 0.0);
    }
}
