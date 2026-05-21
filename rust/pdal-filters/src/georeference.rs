//! `filters.georeference` -- georeferencing filter.

use pdal_core::point::{DimId, PointView};
use pdal_core::srs::{SpatialReference, SrsTransform};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct GeoreferenceFilter {
    out_srs: SpatialReference,
    transform: Option<SrsTransform>,
}

impl GeoreferenceFilter {
    pub fn new(out_srs_wkt: &str) -> Self {
        Self {
            out_srs: SpatialReference::new(out_srs_wkt),
            transform: None,
        }
    }

    fn ensure_transform(&mut self, source_srs: &SpatialReference) -> Result<(), StageError> {
        if self.transform.is_none() {
            self.transform =
                Some(SrsTransform::new(source_srs, &self.out_srs).map_err(StageError)?);
        }
        Ok(())
    }
}

impl Filter for GeoreferenceFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.georeference"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let source_srs = input.spatial_reference().clone();
        if source_srs.is_empty() {
            return Err(StageError(
                "Input view has no spatial reference".to_string(),
            ));
        }
        self.ensure_transform(&source_srs)?;

        let mut output = input.clone();
        output.set_spatial_reference(self.out_srs.clone());

        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }

        Ok(vec![output])
    }
}

impl Streamable for GeoreferenceFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        let source_srs = view.spatial_reference().clone();
        if source_srs.is_empty() {
            return false;
        }

        if self.ensure_transform(&source_srs).is_err() {
            return false;
        }

        let mut x = view.get_f64(idx, &DimId::X);
        let mut y = view.get_f64(idx, &DimId::Y);
        let mut z = view.get_f64(idx, &DimId::Z);

        if let Some(ref xform) = self.transform {
            if xform.transform(&mut x, &mut y, &mut z) {
                view.set_f64(idx, &DimId::X, x);
                view.set_f64(idx, &DimId::Y, y);
                view.set_f64(idx, &DimId::Z, z);
                return true;
            }
        }
        false
    }

    fn reset(&mut self) {
        self.transform = None;
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
        let mut view = PointView::new(Rc::new(layout));
        view.set_spatial_reference(SpatialReference::new(srs));
        let id = view.add_point();
        view.set_f64(id, &DimId::X, -116.0);
        view.set_f64(id, &DimId::Y, 32.0);
        view.set_f64(id, &DimId::Z, 10.0);
        view
    }

    #[test]
    fn transforms_coordinates_and_updates_output_srs() {
        let input = view_with_srs("EPSG:4326");
        let mut filter = GeoreferenceFilter::new("EPSG:3857");

        let output = filter.run_one(&input).unwrap().remove(0);

        assert_eq!(output.spatial_reference().wkt(), "EPSG:3857");
        assert!((output.get_f64(0, &DimId::X) - input.get_f64(0, &DimId::X)).abs() > 1.0);
        assert!((output.get_f64(0, &DimId::Y) - input.get_f64(0, &DimId::Y)).abs() > 1.0);
        assert_eq!(output.get_f64(0, &DimId::Z), 10.0);
    }

    #[test]
    fn rejects_empty_srs_and_resets_cached_transform() {
        let mut filter = GeoreferenceFilter::new("EPSG:3857");
        let mut input = view_with_srs("");

        assert!(filter.run_one(&input).is_err());
        assert!(!filter.process_one(&mut input, 0));
        filter.reset();
    }
}
