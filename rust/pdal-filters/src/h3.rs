//! `filters.h3` -- compute H3 indexes for points.
//!
//! Port of `filters/H3Filter.cpp`.

use h3o::{LatLng, Resolution};
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

/// The `filters.h3` stage.
pub struct H3Filter {
    resolution: u8,
}

impl H3Filter {
    /// Build the filter from a resolution parameter.
    pub fn new(resolution: u8) -> Self {
        Self { resolution }
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
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for H3Filter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);

        // PDAL reprojects to WGS84 before H3 indexing.
        // For the spike, we'll assume the input is already in WGS84
        // or the reprojection is handled by a previous stage.

        let lat_rad = y.to_radians();
        let lng_rad = x.to_radians();

        if let Ok(latlng) = LatLng::new(lat_rad, lng_rad) {
            if let Ok(res) = Resolution::try_from(self.resolution) {
                let cell = latlng.to_cell(res);
                view.set_f64(idx, &DimId::H3, u64::from(cell) as f64);
            }
        }
        true
    }
}
