//! `filters.projpipeline` -- transform coordinates using a PROJ pipeline.

use pdal_core::point::{DimId, PointView};
use pdal_core::srs::{SpatialReference, SrsTransform};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct ProjPipelineFilter {
    out_srs: SpatialReference,
    coord_op: String,
    _reverse: bool,
    transform: Option<SrsTransform>,
}

impl ProjPipelineFilter {
    pub fn new(out_srs_wkt: &str, coord_op: &str, reverse: bool) -> Self {
        Self {
            out_srs: SpatialReference::new(out_srs_wkt),
            coord_op: coord_op.to_string(),
            _reverse: reverse,
            transform: None,
        }
    }

    fn ensure_transform(&mut self) -> Result<(), StageError> {
        if self.transform.is_none() {
            self.transform = Some(SrsTransform::new_pipeline(&self.coord_op).map_err(StageError)?);
        }
        Ok(())
    }
}

impl Filter for ProjPipelineFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.projpipeline"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_transform()?;
        let mut output = input.make_new();
        output.set_spatial_reference(self.out_srs.clone());

        for idx in 0..input.len() {
            let mut x = input.get_f64(idx, &DimId::X);
            let mut y = input.get_f64(idx, &DimId::Y);
            let mut z = input.get_f64(idx, &DimId::Z);

            if let Some(ref xform) = self.transform {
                if xform.transform(&mut x, &mut y, &mut z) {
                    let out_idx = output.add_point();
                    output.set_f64(out_idx, &DimId::X, x);
                    output.set_f64(out_idx, &DimId::Y, y);
                    output.set_f64(out_idx, &DimId::Z, z);
                }
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for ProjPipelineFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_transform().is_err() {
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
