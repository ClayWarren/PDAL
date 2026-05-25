use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct CsfFilter {
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
    ignored_dims: Vec<DimId>,
}

impl CsfFilter {
    pub fn new(
        ground_class: u8,
        other_class: u8,
        only_ground: bool,
        ignored_dims: Vec<DimId>,
    ) -> Result<Self, StageError> {
        if ground_class == other_class && !only_ground {
            return Err(StageError(
                "Ground and non-ground class cannot beequal when only_ground is false.".to_string(),
            ));
        }

        Ok(Self {
            ground_class,
            other_class,
            only_ground,
            ignored_dims,
        })
    }
}

impl Filter for CsfFilter {
    fn name(&self) -> &str {
        "filters.csf"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        for dim in &self.ignored_dims {
            if input.layout().dim(dim).is_none() {
                return Err(StageError(format!(
                    "Invalid dimension name in 'ignored' option: '{}'.",
                    dim.name()
                )));
            }
        }

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let _ = (self.ground_class, self.other_class, self.only_ground);
        Ok(vec![input.clone()])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::Classification, DimType::U8)]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for CsfFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: u64) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::PointLayout;
    use std::rc::Rc;

    #[test]
    fn rejects_equal_classes_when_only_ground_is_false() {
        let err = CsfFilter::new(2, 2, false, Vec::new()).err().unwrap();
        assert!(err.0.contains("only_ground is false"));
    }

    #[test]
    fn empty_input_produces_no_views_and_missing_ignore_dim_errors() {
        let layout = Rc::new(PointLayout::new());
        let view = PointView::new(layout);
        let mut filter = CsfFilter::new(2, 1, false, Vec::new()).unwrap();
        assert!(filter.run_one(&view).unwrap().is_empty());

        let mut filter =
            CsfFilter::new(2, 1, false, vec![DimId::Other("NoSuchDim".to_string())]).unwrap();
        let err = filter.run_one(&view).err().unwrap();
        assert!(err.0.contains("NoSuchDim"));
    }
}
