//! `filters.projpipeline` -- transform coordinates using a PROJ pipeline.

use pdal_core::point::{DimId, PointView};
use pdal_core::srs::{CoordOperationTransform, SpatialReference};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct ProjPipelineFilter {
    out_srs: SpatialReference,
    coord_op: String,
    reverse: bool,
    transform: Option<CoordOperationTransform>,
}

impl ProjPipelineFilter {
    pub fn new(out_srs_wkt: &str, coord_op: &str, reverse: bool) -> Self {
        Self {
            out_srs: SpatialReference::new(out_srs_wkt),
            coord_op: coord_op.to_string(),
            reverse,
            transform: None,
        }
    }

    fn ensure_transform(&mut self) -> Result<(), StageError> {
        if self.transform.is_none() {
            self.transform = Some(
                CoordOperationTransform::new(&self.coord_op, self.reverse).map_err(StageError)?,
            );
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

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_transform()?;
        let mut output = input.make_new();
        output.set_spatial_reference(self.out_srs.clone());

        for idx in 0..input.len() {
            let mut x = input.get_f64(idx, &DimId::X);
            let mut y = input.get_f64(idx, &DimId::Y);
            let mut z = input.get_f64(idx, &DimId::Z);

            if let Some(ref xform) = self.transform {
                if xform.transform(&mut x, &mut y, &mut z) {
                    output.append_point(input, idx);
                    let out_idx = output.len() - 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    #[test]
    fn run_preserves_non_coordinate_dimensions() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::F64);

        let mut input = PointView::new(Rc::new(layout));
        let idx = input.add_point();
        input.set_f64(idx, &DimId::X, -93.35156259);
        input.set_f64(idx, &DimId::Y, 41.577148395);
        input.set_f64(idx, &DimId::Z, 16.0);
        input.set_f64(idx, &DimId::Intensity, 42.0);

        let mut filter = ProjPipelineFilter::new(
            "",
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=deg",
            false,
        );
        let output = filter.run_one(&input).unwrap();

        assert_eq!(output[0].len(), 1);
        assert_eq!(output[0].get_f64(0, &DimId::Intensity), 42.0);
        assert_eq!(output[0].source_index(0), 0);
    }

    #[test]
    fn filter_name_is_filters_projpipeline() {
        let f = ProjPipelineFilter::new(
            "",
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=deg",
            false,
        );
        assert_eq!(f.name(), "filters.projpipeline");
    }

    #[test]
    fn ensure_transform_errors_on_invalid_pipeline() {
        let mut f = ProjPipelineFilter::new("", "not-a-real-pipeline", false);
        let r = f.ensure_transform();
        assert!(r.is_err());
    }

    #[test]
    fn run_one_errors_on_invalid_pipeline() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let input = PointView::new(Rc::new(layout));
        let mut f = ProjPipelineFilter::new("", "not-a-real-pipeline", false);
        assert!(f.run_one(&input).is_err());
    }

    #[test]
    fn streamable_process_one_returns_false_on_invalid_pipeline() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        view.add_point();
        let mut f = ProjPipelineFilter::new("", "not-a-real-pipeline", false);
        assert!(!f.process_one(&mut view, 0));
    }

    #[test]
    fn streamable_process_one_transforms_point() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let p = view.add_point();
        view.set_f64(p, &DimId::X, 1.0);
        view.set_f64(p, &DimId::Y, 2.0);
        view.set_f64(p, &DimId::Z, 3.0);
        let mut f = ProjPipelineFilter::new(
            "",
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=deg",
            false,
        );
        assert!(f.process_one(&mut view, 0));
    }

    #[test]
    fn reset_clears_transform() {
        let mut f = ProjPipelineFilter::new(
            "",
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=deg",
            false,
        );
        f.ensure_transform().unwrap();
        assert!(f.transform.is_some());
        f.reset();
        assert!(f.transform.is_none());
    }

    #[test]
    fn reverse_mode_uses_inverse_coordinate_operation() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let p = view.add_point();
        view.set_f64(p, &DimId::X, std::f64::consts::PI);
        view.set_f64(p, &DimId::Y, std::f64::consts::FRAC_PI_2);
        view.set_f64(p, &DimId::Z, 3.0);

        let mut f = ProjPipelineFilter::new(
            "",
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad",
            true,
        );
        let out = f.run_one(&view).unwrap().pop().unwrap();

        assert!((out.get_f64(0, &DimId::X) - 180.0).abs() < 1e-9);
        assert!((out.get_f64(0, &DimId::Y) - 90.0).abs() < 1e-9);
        assert_eq!(out.get_f64(0, &DimId::Z), 3.0);
    }
}
