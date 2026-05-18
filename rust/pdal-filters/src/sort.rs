use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortAlgorithm {
    Normal,
    Stable,
}

pub struct SortFilter {
    pub dim_names: Vec<String>,
    pub order: SortOrder,
    pub algorithm: SortAlgorithm,
}

impl SortFilter {
    pub fn new(dim_names: Vec<String>, order: SortOrder, algorithm: SortAlgorithm) -> Self {
        SortFilter {
            dim_names,
            order,
            algorithm,
        }
    }
}

impl Filter for SortFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.sort"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut indices: Vec<u64> = (0..input.len()).collect();

        for (i, name) in self.dim_names.iter().enumerate() {
            let dim_id = DimId::from_name(name);

            let cmp = |&a: &u64, &b: &u64| {
                let val_a = input.get_f64(a, &dim_id);
                let val_b = input.get_f64(b, &dim_id);

                let order = if self.order == SortOrder::Desc {
                    val_b.partial_cmp(&val_a)
                } else {
                    val_a.partial_cmp(&val_b)
                };

                order.unwrap_or(std::cmp::Ordering::Equal)
            };

            if self.dim_names.len() > 1 {
                if i == 0 {
                    indices.sort_unstable_by(cmp);
                } else {
                    indices.sort_by(cmp);
                }
            } else {
                match self.algorithm {
                    SortAlgorithm::Stable => indices.sort_by(cmp),
                    SortAlgorithm::Normal => indices.sort_unstable_by(cmp),
                }
            }
        }

        let mut out = input.make_new();
        for idx in indices {
            out.append_point(input, idx);
        }

        Ok(vec![out])
    }
}

impl Streamable for SortFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
