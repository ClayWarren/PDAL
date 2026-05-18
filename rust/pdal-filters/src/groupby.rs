use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

pub struct GroupByFilter {
    pub dim_name: String,
}

impl GroupByFilter {
    pub fn new(dim_name: String) -> Self {
        GroupByFilter { dim_name }
    }
}

impl Filter for GroupByFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.groupby"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let dim_id = DimId::from_name(&self.dim_name);

        let mut groups: BTreeMap<i64, PointView> = BTreeMap::new();

        for i in 0..input.len() {
            let val = input.get_f64(i, &dim_id) as i64;
            groups
                .entry(val)
                .or_insert_with(|| input.make_new())
                .append_point(input, i);
        }

        Ok(groups.into_values().collect())
    }
}

impl Streamable for GroupByFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        false
    }

    fn reset(&mut self) {}
}
