//! `filters.dem` -- filter points about an elevation surface.

use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct DEMFilter {
    dim_name: String,
    raster_path: String,
    band: i32,
    lower_bound: f64,
    upper_bound: f64,
    raster: Option<Raster>,
}

impl DEMFilter {
    pub fn new(
        dim_name: &str,
        raster_path: &str,
        band: i32,
        lower_bound: f64,
        upper_bound: f64,
    ) -> Self {
        Self {
            dim_name: dim_name.to_string(),
            raster_path: raster_path.to_string(),
            band,
            lower_bound,
            upper_bound,
            raster: None,
        }
    }

    fn ensure_raster(&mut self) -> Result<(), StageError> {
        if self.raster.is_none() {
            gdal::register_drivers();
            self.raster = Some(Raster::open(&self.raster_path).map_err(StageError)?);
        }
        Ok(())
    }
}

impl Filter for DEMFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.dem"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_raster()?;
        let mut scratch = input.clone();
        let mut output = input.make_new();
        for idx in 0..input.len() {
            if self.process_one(&mut scratch, idx) {
                output.append_point(input, idx);
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for DEMFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_raster().is_err() {
            return false;
        }

        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let dim = DimId::from_name(&self.dim_name);
        let z = view.get_f64(idx, &dim);

        let mut data = vec![0.0; self.band as usize];
        if let Some(ref r) = self.raster {
            if r.read_at(x, y, &mut data).is_ok() {
                let v = data[self.band as usize - 1];
                let lb = v - self.lower_bound;
                let ub = v + self.upper_bound;
                return z >= lb && z <= ub;
            }
        }
        false
    }

    fn reset(&mut self) {
        self.raster = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::gdal::Raster;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn write_dem(path: &str) {
        gdal::register_drivers();
        let mut raster =
            Raster::create_float64(path, "GTiff", 1, 1, 1, [0.0, 1.0, 0.0, 1.0, 0.0, -1.0], "")
                .unwrap();
        raster
            .write_band_f64(1, 1, 1, &[100.0], -9999.0, "Z")
            .unwrap();
    }

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, *x);
            view.set_f64(id, &DimId::Y, *y);
            view.set_f64(id, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn keeps_points_inside_dem_relative_bounds() {
        let temp = tempfile::NamedTempFile::with_suffix(".tif").unwrap();
        write_dem(temp.path().to_str().unwrap());
        let mut filter = DEMFilter::new("Z", temp.path().to_str().unwrap(), 1, 1.0, 10.0);

        let output = filter
            .run_one(&view(&[(0.5, 0.5, 105.0), (0.5, 0.5, 98.0)]))
            .unwrap()
            .remove(0);

        assert_eq!(output.len(), 1);
        assert_eq!(output.get_f64(0, &DimId::Z), 105.0);
    }

    #[test]
    fn rejects_out_of_bounds_or_missing_rasters_and_resets() {
        let temp = tempfile::NamedTempFile::with_suffix(".tif").unwrap();
        write_dem(temp.path().to_str().unwrap());
        let mut filter = DEMFilter::new("Z", temp.path().to_str().unwrap(), 1, 1.0, 10.0);
        let mut input = view(&[(10.0, 10.0, 105.0)]);

        assert!(!filter.process_one(&mut input, 0));
        filter.reset();
        assert!(DEMFilter::new("Z", "/no/such/dem.tif", 1, 1.0, 10.0)
            .run_one(&input)
            .is_err());
    }
}
