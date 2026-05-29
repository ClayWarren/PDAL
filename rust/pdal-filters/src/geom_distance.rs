//! `filters.geomdistance` -- compute distance to a given geometry.

use pdal_core::geometry::Geometry;
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct GeomDistanceFilter {
    geometry: Geometry,
    dim_name: String,
}

impl GeomDistanceFilter {
    pub fn new(wkt: &str, dim_name: &str, ring: bool) -> Result<Self, StageError> {
        let mut geometry = Geometry::from_wkt(wkt).map_err(StageError)?;
        if ring {
            // Match PDAL's `ring` option: demote polygons to their boundary so
            // distance measures against the polygon's edge rather than its
            // interior.
            geometry = geometry.boundary().map_err(StageError)?;
        }
        Ok(GeomDistanceFilter {
            geometry,
            dim_name: dim_name.to_string(),
        })
    }
}

impl Filter for GeomDistanceFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.geomdistance"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::from_name(&self.dim_name), DimType::F64)]
    }
}

impl Streamable for GeomDistanceFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);

        if let Ok(dist) = self.geometry.distance(x, y, z) {
            let dim = DimId::from_name(&self.dim_name);
            view.set_f64(idx, &dim, dist);
            true
        } else {
            false
        }
    }

    fn reset(&mut self) {}
}
