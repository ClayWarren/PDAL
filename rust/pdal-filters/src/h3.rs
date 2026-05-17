//! `filters.h3` -- compute H3 indexes for points.
//!
//! Port of `filters/H3Filter.cpp`.

use h3o::{LatLng, Resolution};
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::srs::{SpatialReference, SrsTransform};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct H3Filter {
    resolution: u8,
    transform: Option<SrsTransform>,
}

impl H3Filter {
    pub fn new(resolution: u8) -> Self {
        Self {
            resolution,
            transform: None,
        }
    }

    fn ensure_transform(&mut self, srs: &SpatialReference) -> Result<(), StageError> {
        if self.transform.is_none() {
            let dst = SpatialReference::new("EPSG:4326");
            self.transform = Some(SrsTransform::new(srs, &dst).map_err(StageError)?);
        }
        Ok(())
    }

    fn process_point(&self, view: &mut PointView, idx: PointId) -> Result<(), StageError> {
        let mut x = view.get_f64(idx, &DimId::X);
        let mut y = view.get_f64(idx, &DimId::Y);
        let mut z = view.get_f64(idx, &DimId::Z);

        if let Some(ref xform) = self.transform {
            if xform.transform(&mut x, &mut y, &mut z) {
                let lat_rad = y.to_radians();
                let lng_rad = x.to_radians();
                let latlng =
                    LatLng::new(lat_rad, lng_rad).map_err(|e| StageError(e.to_string()))?;
                let res =
                    Resolution::try_from(self.resolution).map_err(|e| StageError(e.to_string()))?;
                let cell = latlng.to_cell(res);
                view.set_f64(idx, &DimId::H3, u64::from(cell) as f64);
            } else {
                return Err(StageError(format!(
                    "Failed to reproject point ({}, {}, {})",
                    x, y, z
                )));
            }
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

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = input.clone();

        let srs = input
            .spatial_reference()
            .ok_or_else(|| StageError("Input data has no spatial reference".to_string()))?;
        self.ensure_transform(srs)?;

        for idx in 0..output.len() {
            self.process_point(&mut output, idx)?;
        }

        Ok(vec![output])
    }
}

impl Streamable for H3Filter {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        if let Some(srs) = view.spatial_reference() {
            if self.ensure_transform(srs).is_ok() {
                return self.process_point(view, idx).is_ok();
            }
        }
        false
    }
}
