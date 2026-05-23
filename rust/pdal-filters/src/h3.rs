//! `filters.h3` -- compute H3 indexes for points.
//!
//! Port of `filters/H3Filter.cpp`.

use h3o::{LatLng, Resolution};
use pdal_core::point::{DimId, PointView};
use pdal_core::srs::{SpatialReference, SrsTransform};
use pdal_core::stage::{Filter, StageError, Streamable};

/// The `filters.h3` stage.
pub struct H3Filter {
    resolution: u8,
    transform: Option<SrsTransform>,
}

impl H3Filter {
    /// Build the filter from a resolution parameter.
    pub fn new(resolution: u8) -> Self {
        Self {
            resolution,
            transform: None,
        }
    }

    fn ensure_transform(&mut self, source_srs: &SpatialReference) -> Result<(), StageError> {
        if self.transform.is_none() {
            if source_srs.is_empty() {
                return Err(StageError(
                    "source data has no spatial reference".to_string(),
                ));
            }
            self.transform = Some(
                SrsTransform::new(source_srs, &SpatialReference::new("EPSG:4326"))
                    .map_err(StageError)?,
            );
        }
        Ok(())
    }
}

impl Filter for H3Filter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.h3"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_transform(input.spatial_reference())?;
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for H3Filter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_transform(view.spatial_reference()).is_err() {
            return false;
        }

        let mut x = view.get_f64(idx, &DimId::X);
        let mut y = view.get_f64(idx, &DimId::Y);
        let mut z = view.get_f64(idx, &DimId::Z);
        let Some(transform) = self.transform.as_ref() else {
            return false;
        };
        if !transform.transform(&mut x, &mut y, &mut z) {
            return false;
        }

        if let Ok(latlng) = LatLng::new(y.to_radians(), x.to_radians()) {
            if let Ok(res) = Resolution::try_from(self.resolution) {
                let cell = latlng.to_cell(res);
                view.set_f64(idx, &DimId::H3, u64::from(cell) as f64);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view_with_srs(srs: &str) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::H3, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        view.set_spatial_reference(SpatialReference::new(srs));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, -122.0);
        view.set_f64(idx, &DimId::Y, 47.0);
        view.set_f64(idx, &DimId::Z, 0.0);
        view
    }

    #[test]
    fn run_one_errors_when_view_has_no_srs() {
        let mut filter = H3Filter::new(8);
        let view = view_with_srs("");
        let result = filter.run_one(&view);
        assert!(result.is_err());
    }

    #[test]
    fn run_one_assigns_h3_for_wgs84_points() {
        let mut filter = H3Filter::new(8);
        let view = view_with_srs("EPSG:4326");
        let out = filter.run_one(&view).unwrap().remove(0);
        let h3 = out.get_f64(0, &DimId::H3);
        assert!(h3 > 0.0);
    }

    #[test]
    fn process_one_returns_false_without_srs() {
        let mut filter = H3Filter::new(8);
        let mut view = view_with_srs("");
        assert!(!filter.process_one(&mut view, 0));
    }

    #[test]
    fn invalid_resolution_does_not_panic() {
        // Resolution > 15 is invalid for H3
        let mut filter = H3Filter::new(99);
        let view = view_with_srs("EPSG:4326");
        let out = filter.run_one(&view).unwrap().remove(0);
        // Default H3 value should be 0 because Resolution::try_from(99) errors
        assert_eq!(out.get_f64(0, &DimId::H3), 0.0);
    }
}
