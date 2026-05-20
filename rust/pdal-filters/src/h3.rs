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
