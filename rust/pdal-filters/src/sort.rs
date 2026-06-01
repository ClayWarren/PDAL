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
        let indices = self.sorted_order(input);

        let mut out = input.make_new();
        for idx in indices {
            out.append_point(input, idx);
        }

        Ok(vec![out])
    }

    fn run_owned(&mut self, inputs: Vec<PointView>) -> Result<Vec<PointView>, StageError> {
        // In-place equivalent of `run_one`: reorder each view's rows by the same
        // permutation instead of building a second view, then drop attachments
        // to match `make_new`'s output (which carried none). This keeps one copy
        // of the point buffer alive instead of input + output.
        let mut outputs = Vec::with_capacity(inputs.len());
        for mut view in inputs {
            let order = self.sorted_order(&view);
            view.reorder(&order);
            view.clear_attachments();
            outputs.push(view);
        }
        Ok(outputs)
    }
}

impl SortFilter {
    /// Compute the gather permutation that sorts `input`: position `i` of the
    /// result takes the point at the returned index `[i]`. Shared by the
    /// allocating `run_one` and the in-place `run_owned`.
    fn sorted_order(&self, input: &PointView) -> Vec<u64> {
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

        indices
    }
}

impl Streamable for SortFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
