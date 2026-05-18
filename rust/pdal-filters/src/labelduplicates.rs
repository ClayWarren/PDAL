use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct LabelDuplicatesFilter {
    pub dim_names: Vec<String>,
}

impl LabelDuplicatesFilter {
    pub fn new(dim_names: Vec<String>) -> Self {
        LabelDuplicatesFilter { dim_names }
    }
}

impl Filter for LabelDuplicatesFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.label_duplicates"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        if input.is_empty() {
            return Ok(vec![out]);
        }

        out.append_point(input, 0);
        let dup_dim = DimId::from_name("Duplicate");
        out.set_f64(0, &dup_dim, 0.0);

        let dims: Vec<_> = self
            .dim_names
            .iter()
            .map(|name| DimId::from_name(name))
            .collect();

        for idx in 1..input.len() {
            out.append_point(input, idx);
            let mut is_dup = true;
            for dim in &dims {
                let current = input.get_f64(idx, dim);
                let previous = input.get_f64(idx - 1, dim);
                if current != previous {
                    is_dup = false;
                    break;
                }
            }
            let out_idx = out.len() - 1;
            out.set_f64(out_idx, &dup_dim, if is_dup { 1.0 } else { 0.0 });
        }

        Ok(vec![out])
    }
}

impl Streamable for LabelDuplicatesFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
