//! `filters.overlay` -- assign values to a dimension based on OGR features.

use pdal_core::gdal::{self, Vector};
use pdal_core::geometry::Geometry;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct OverlayFilter {
    dim_name: String,
    datasource: String,
    column: String,
    layer_name: String,
    query: String,
    bounds_filter: Option<Geometry>,
    polygons: Vec<(Geometry, i32)>,
}

impl OverlayFilter {
    pub fn new(dim_name: &str, datasource: &str, column: &str) -> Self {
        Self::with_layer_or_query(dim_name, datasource, column, "", "")
    }

    pub fn with_layer_or_query(
        dim_name: &str,
        datasource: &str,
        column: &str,
        layer_name: &str,
        query: &str,
    ) -> Self {
        Self {
            dim_name: dim_name.to_string(),
            datasource: datasource.to_string(),
            column: column.to_string(),
            layer_name: layer_name.to_string(),
            query: query.to_string(),
            bounds_filter: None,
            polygons: Vec::new(),
        }
    }

    pub fn with_options(
        dim_name: &str,
        datasource: &str,
        column: &str,
        layer_name: &str,
        query: &str,
        bounds_wkt: &str,
    ) -> Result<Self, StageError> {
        let bounds_filter = if bounds_wkt.trim().is_empty() {
            None
        } else {
            Some(Geometry::from_wkt(bounds_wkt).map_err(StageError)?)
        };
        Ok(Self {
            dim_name: dim_name.to_string(),
            datasource: datasource.to_string(),
            column: column.to_string(),
            layer_name: layer_name.to_string(),
            query: query.to_string(),
            bounds_filter,
            polygons: Vec::new(),
        })
    }

    fn ensure_polygons(&mut self) -> Result<(), StageError> {
        if self.polygons.is_empty() {
            gdal::register_drivers();
            let ds = Vector::open(&self.datasource).map_err(StageError)?;
            let features = if !self.layer_name.is_empty() {
                ds.get_features_by_layer(&self.layer_name, &self.column)
                    .map_err(StageError)?
            } else if !self.query.is_empty() {
                ds.get_features_by_sql(&self.query, &self.column)
                    .map_err(StageError)?
            } else {
                ds.get_features(0, &self.column).map_err(StageError)?
            };
            for (wkt, val) in features {
                let geom = Geometry::from_wkt(&wkt).map_err(StageError)?;
                if let Some(bounds) = &self.bounds_filter {
                    if !geom.intersects(bounds).map_err(StageError)? {
                        continue;
                    }
                }
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

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_polygons()?;
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for OverlayFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
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
