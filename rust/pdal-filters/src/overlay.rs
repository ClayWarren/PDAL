//! `filters.overlay` -- assign values to a dimension based on OGR features.

use pdal_core::gdal::{self, Vector};
use pdal_core::geometry::Geometry;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct OverlayFilter {
    dim_name: String,
    datasource: String,
    _column: String,
    polygons: Vec<(Geometry, i32)>,
}

impl OverlayFilter {
    pub fn new(dim_name: &str, datasource: &str, column: &str) -> Self {
        Self {
            dim_name: dim_name.to_string(),
            datasource: datasource.to_string(),
            _column: column.to_string(),
            polygons: Vec::new(),
        }
    }

    fn ensure_polygons(&mut self) -> Result<(), StageError> {
        if self.polygons.is_empty() {
            gdal::register_drivers();
            let ds = Vector::open(&self.datasource).map_err(StageError)?;
            // PDAL handles layers and SQL queries; for the spike, we'll take the first layer
            let features = ds.get_features(0).map_err(StageError)?;
            for (wkt, val) in features {
                let geom = Geometry::from_wkt(&wkt).map_err(StageError)?;
                self.polygons.push((geom, val));
            }
        }
        Ok(())
    }
}

impl Filter for OverlayFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.overlay"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_polygons()?;

        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for OverlayFilter {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        if self.ensure_polygons().is_err() {
            return true;
        }

        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);

        for (geom, val) in &self.polygons {
            if geom.contains(x, y) {
                let dim = DimId::from_name(&self.dim_name);
                view.set_f64(idx, &dim, *val as f64);
                break;
            }
        }

        true
    }

    fn reset(&mut self) {
        self.polygons.clear();
    }
}
