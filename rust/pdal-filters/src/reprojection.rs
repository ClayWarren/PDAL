//! `filters.reprojection` -- reproject points between SRSs.
//!
//! Port of `filters/ReprojectionFilter.cpp`.

use pdal_core::point::{DimId, PointView};
use pdal_core::srs::{SpatialReference, SrsTransform};
use pdal_core::stage::{Filter, StageError, Streamable};

/// The `filters.reprojection` stage.
pub struct ReprojectionFilter {
    out_srs: SpatialReference,
    in_srs: Option<SpatialReference>,
    transform: Option<SrsTransform>,
    error_on_failure: bool,
}

impl ReprojectionFilter {
    pub fn new(out_srs: &str, in_srs: Option<String>, error_on_failure: bool) -> Self {
        Self {
            out_srs: SpatialReference::new(out_srs),
            in_srs: in_srs.map(|wkt| SpatialReference::new(&wkt)),
            transform: None,
            error_on_failure,
        }
    }

    fn ensure_transform(&mut self, source_srs: &SpatialReference) -> Result<(), StageError> {
        if self.transform.is_none() {
            let src = self.in_srs.as_ref().unwrap_or(source_srs);
            if src.is_empty() {
                return Err(StageError("Source SRS is unknown".to_string()));
            }
            self.transform = Some(SrsTransform::new(src, &self.out_srs).map_err(StageError)?);
        }
        Ok(())
    }
}

impl Filter for ReprojectionFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.reprojection"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let source_srs = input.spatial_reference();
        self.ensure_transform(source_srs)?;

        let mut output = input.clone();
        output.set_spatial_reference(self.out_srs.clone());

        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }

        Ok(vec![output])
    }
}

impl Streamable for ReprojectionFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        let source_srs = view.spatial_reference();
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
        !self.error_on_failure
    }
}
