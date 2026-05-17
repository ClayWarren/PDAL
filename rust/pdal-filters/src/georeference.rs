//! `filters.georeference` -- georeferencing filter.

use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::srs::{SpatialReference, SrsTransform};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct GeoreferenceFilter {
    // This is a complex filter in PDAL; for the spike,
    // we'll implement a basic version that reprojects based on SRS.
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

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let source_srs = input
            .spatial_reference()
            .ok_or_else(|| StageError("Input view has no spatial reference".to_string()))?;

        self.ensure_transform(source_srs)?;

        let mut output = input.clone();
        output.set_spatial_reference(self.out_srs.clone());

        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }

        Ok(vec![output])
    }
}

impl Streamable for GeoreferenceFilter {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        let source_srs = match view.spatial_reference() {
            Some(s) => s,
            None => return false,
        };

        if self.ensure_transform(source_srs).is_err() {
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
