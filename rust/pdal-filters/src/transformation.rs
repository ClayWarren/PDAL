use pdal_core::point::{DimId, PointId, PointView};
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

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
