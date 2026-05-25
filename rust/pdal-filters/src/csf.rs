//! `filters.csf` -- Cloth Simulation Filter wiring.
//!
//! The full algorithm lives in [`crate::csf_algorithm`]; this module exposes
//! the filter handle that the C ABI / pipeline construct and owns the
//! point-classification step on the output view.

use crate::csf_algorithm::{classify_ground, CsfParams, CsfPoint};
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct CsfFilter {
    pub ground_class: u8,
    pub other_class: u8,
    pub only_ground: bool,
    pub ignored_dims: Vec<DimId>,
    pub params: CsfParams,
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
            params: CsfParams::default(),
        })
    }

    pub fn with_params(mut self, params: CsfParams) -> Self {
        self.params = params;
        self
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

        // Collect XYZ points.
        let n = input.len() as usize;
        let mut points: Vec<CsfPoint> = Vec::with_capacity(n);
        for idx in 0..(n as u64) {
            points.push(CsfPoint {
                x: input.get_f64(idx, &DimId::X),
                y: input.get_f64(idx, &DimId::Y),
                z: input.get_f64(idx, &DimId::Z),
            });
        }

        let result = classify_ground(&points, &self.params);

        // Build the output view. Clone the input so we preserve all other
        // dimensions, then overwrite Classification per the ground/other split.
        let mut output = input.clone();
        let mut is_ground = vec![false; n];
        for &gi in &result.ground_indices {
            if gi < n {
                is_ground[gi] = true;
            }
        }

        for (idx, &ground) in is_ground.iter().enumerate() {
            if ground {
                output.set_f64(idx as u64, &DimId::Classification, self.ground_class as f64);
            } else if !self.only_ground {
                output.set_f64(idx as u64, &DimId::Classification, self.other_class as f64);
            }
            // only_ground == true: leave non-ground points untouched.
        }
        Ok(vec![output])
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
