use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct TransformationFilter {
    pub matrix: [f64; 16],
}

impl TransformationFilter {
    pub fn new(matrix: [f64; 16]) -> Self {
        TransformationFilter { matrix }
    }

    pub fn transform_point(&self, view: &mut PointView, idx: u64) {
        let x_dim = DimId::from_name("X");
        let y_dim = DimId::from_name("Y");
        let z_dim = DimId::from_name("Z");

        let x = view.get_f64(idx, &x_dim);
        let y = view.get_f64(idx, &y_dim);
        let z = view.get_f64(idx, &z_dim);

        let s = x * self.matrix[12] + y * self.matrix[13] + z * self.matrix[14] + self.matrix[15];

        let new_x =
            (x * self.matrix[0] + y * self.matrix[1] + z * self.matrix[2] + self.matrix[3]) / s;
        let new_y =
            (x * self.matrix[4] + y * self.matrix[5] + z * self.matrix[6] + self.matrix[7]) / s;
        let new_z =
            (x * self.matrix[8] + y * self.matrix[9] + z * self.matrix[10] + self.matrix[11]) / s;

        view.set_f64(idx, &x_dim, new_x);
        view.set_f64(idx, &y_dim, new_y);
        view.set_f64(idx, &z_dim, new_z);
    }
}

impl Filter for TransformationFilter {
    fn name(&self) -> &str {
        "filters.transformation"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();

        for idx in 0..input.len() {
            out.append_point(input, idx);
            let out_idx = out.len() - 1;
            self.transform_point(&mut out, out_idx);
        }

        Ok(vec![out])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for TransformationFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

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
    fn applies_affine_transform_to_every_point() {
        let input = view(&[(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)]);
        let matrix = [
            1.0, 0.0, 0.0, 10.0, 0.0, 2.0, 0.0, 20.0, 0.0, 0.0, 3.0, 30.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mut filter = TransformationFilter::new(matrix);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);

        assert_eq!(out.len(), 2);
        assert_eq!(out.get_f64(0, &DimId::X), 11.0);
        assert_eq!(out.get_f64(0, &DimId::Y), 24.0);
        assert_eq!(out.get_f64(0, &DimId::Z), 39.0);
        assert_eq!(out.get_f64(1, &DimId::X), 14.0);
        assert_eq!(out.get_f64(1, &DimId::Y), 30.0);
        assert_eq!(out.get_f64(1, &DimId::Z), 48.0);
    }

    #[test]
    fn supports_perspective_divide_and_is_not_streamable() {
        let mut input = view(&[(2.0, 4.0, 6.0)]);
        let matrix = [
            2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0,
        ];
        let mut filter = TransformationFilter::new(matrix);

        filter.transform_point(&mut input, 0);

        assert_eq!(input.get_f64(0, &DimId::X), 2.0);
        assert_eq!(input.get_f64(0, &DimId::Y), 4.0);
        assert_eq!(input.get_f64(0, &DimId::Z), 6.0);
        assert!(!filter.process_one(&mut input, 0));
        filter.reset();
    }
}
