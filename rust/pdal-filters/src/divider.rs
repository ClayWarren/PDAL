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
}

impl DividerFilter {
    pub fn new(mode: DividerMode, size_mode: DividerSizeMode, size: u64, evals: Vec<bool>) -> Self {
        Self {
            mode,
            size_mode,
            size,
            evals,
        }
    }
}

impl Filter for DividerFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.divider"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
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
                views.push(PointView::new(view.layout().clone()));
                let mut view_num = 0;
                for i in 0..size {
                    let passed = if i < self.evals.len() as u64 {
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
    fn process_one(&mut self) -> bool {
        false
    }

    fn reset(&mut self) {}
}
