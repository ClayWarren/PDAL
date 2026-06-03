use pdal_core::expr::ConditionalExpression;
use pdal_core::point::PointView;
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerMode {
    Partition,
    RoundRobin,
    Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerSizeMode {
    Count,
    Capacity,
}

pub struct DividerFilter {
    pub mode: DividerMode,
    pub size_mode: DividerSizeMode,
    pub size: u64,
    pub evals: Vec<bool>,
    expression: Option<ConditionalExpression>,
    prepared: bool,
}

impl DividerFilter {
    pub fn new(mode: DividerMode, size_mode: DividerSizeMode, size: u64, evals: Vec<bool>) -> Self {
        Self {
            mode,
            size_mode,
            size,
            evals,
            expression: None,
            prepared: false,
        }
    }

    pub fn new_expression(
        size_mode: DividerSizeMode,
        size: u64,
        expression: &str,
    ) -> Result<Self, StageError> {
        let expression = ConditionalExpression::parse(expression)
            .map_err(|err| StageError(format!("Invalid divider expression: {err}")))?;
        Ok(Self {
            mode: DividerMode::Expression,
            size_mode,
            size,
            evals: Vec::new(),
            expression: Some(expression),
            prepared: false,
        })
    }

    fn ensure_prepared(&mut self, view: &PointView) -> Result<(), StageError> {
        if !self.prepared {
            if let Some(expression) = &mut self.expression {
                expression
                    .prepare(view.layout().as_ref())
                    .map_err(StageError)?;
            }
            self.prepared = true;
        }
        Ok(())
    }
}

impl Filter for DividerFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.divider"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let size = view.len();
        if size == 0 {
            return Ok(Vec::new());
        }

        let mut target_size = self.size;
        if self.size_mode == DividerSizeMode::Capacity {
            if target_size == 0 {
                target_size = 1;
            }
            target_size = ((size as i64 - 1) / target_size as i64) as u64 + 1;
        }
        if target_size == 0 {
            target_size = 1;
        }

        let mut views = Vec::new();

        match self.mode {
            DividerMode::Partition => {
                let limit = ((size as i64 - 1) / target_size as i64) as u64 + 1;
                for _ in 0..target_size {
                    views.push(PointView::new(view.layout().clone()));
                }
                let mut view_num = 0;
                for i in 0..size {
                    views[view_num].append_point(view, i);
                    if (i + 1) % limit == 0 && view_num + 1 < target_size as usize {
                        view_num += 1;
                    }
                }
            }
            DividerMode::RoundRobin => {
                for _ in 0..target_size {
                    views.push(PointView::new(view.layout().clone()));
                }
                let mut view_num = 0;
                for i in 0..size {
                    views[view_num].append_point(view, i);
                    view_num += 1;
                    if view_num == target_size as usize {
                        view_num = 0;
                    }
                }
            }
            DividerMode::Expression => {
                self.ensure_prepared(view)?;
                views.push(PointView::new(view.layout().clone()));
                let mut view_num = 0;
                for i in 0..size {
                    let passed = if let Some(expression) = &self.expression {
                        expression.eval(view, i)
                    } else if i < self.evals.len() as u64 {
                        self.evals[i as usize]
                    } else {
                        false
                    };
                    if passed {
                        views.push(PointView::new(view.layout().clone()));
                        view_num += 1;
                    }
                    views[view_num].append_point(view, i);
                }
            }
        }

        Ok(views)
    }
}

impl Streamable for DividerFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout};
    use std::rc::Rc;

    fn view(size: u64) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for i in 0..size {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, i as f64);
        }
        view
    }

    fn xs(view: &PointView) -> Vec<f64> {
        (0..view.len())
            .map(|i| view.get_f64(i, &DimId::X))
            .collect()
    }

    #[test]
    fn partition_and_capacity_modes_split_points() {
        let input = view(5);
        let mut filter = DividerFilter::new(
            DividerMode::Partition,
            DividerSizeMode::Count,
            2,
            Vec::new(),
        );

        let outputs = filter.run_one(&input).unwrap();

        assert_eq!(outputs.len(), 2);
        assert_eq!(xs(&outputs[0]), vec![0.0, 1.0, 2.0]);
        assert_eq!(xs(&outputs[1]), vec![3.0, 4.0]);

        let outputs = DividerFilter::new(
            DividerMode::Partition,
            DividerSizeMode::Capacity,
            2,
            Vec::new(),
        )
        .run_one(&input)
        .unwrap();
        assert_eq!(outputs.len(), 3);
    }

    #[test]
    fn round_robin_and_expression_modes_route_points() {
        let input = view(5);
        let outputs = DividerFilter::new(
            DividerMode::RoundRobin,
            DividerSizeMode::Count,
            2,
            Vec::new(),
        )
        .run_one(&input)
        .unwrap();

        assert_eq!(xs(&outputs[0]), vec![0.0, 2.0, 4.0]);
        assert_eq!(xs(&outputs[1]), vec![1.0, 3.0]);

        let outputs = DividerFilter::new(
            DividerMode::Expression,
            DividerSizeMode::Count,
            0,
            vec![false, true, false, true],
        )
        .run_one(&input)
        .unwrap();
        assert_eq!(outputs.len(), 3);
        assert_eq!(xs(&outputs[0]), vec![0.0]);
        assert_eq!(xs(&outputs[1]), vec![1.0, 2.0]);
        assert_eq!(xs(&outputs[2]), vec![3.0, 4.0]);

        let outputs = DividerFilter::new_expression(DividerSizeMode::Count, 0, "X >= 2 && X < 4")
            .unwrap()
            .run_one(&input)
            .unwrap();
        assert_eq!(outputs.len(), 3);
        assert_eq!(xs(&outputs[0]), vec![0.0, 1.0]);
        assert_eq!(xs(&outputs[1]), vec![2.0]);
        assert_eq!(xs(&outputs[2]), vec![3.0, 4.0]);
    }

    #[test]
    fn empty_input_and_streaming_contract() {
        let mut filter = DividerFilter::new(
            DividerMode::Partition,
            DividerSizeMode::Count,
            0,
            Vec::new(),
        );
        assert!(filter.run_one(&view(0)).unwrap().is_empty());

        let mut input = view(1);
        assert!(!filter.process_one(&mut input, 0));
        filter.reset();
    }
}
