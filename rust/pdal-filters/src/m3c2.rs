use crate::math;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NormalOrientation {
    Up,
    Down,
    None,
}

pub struct M3C2Filter {
    normal_radius: f64,
    cyl_radius: f64,
    cyl_half_len: f64,
    reg_error: f64,
    orientation: NormalOrientation,
    min_points: usize,
}

impl M3C2Filter {
    pub fn new(
        normal_radius: f64,
        cyl_radius: f64,
        cyl_half_len: f64,
        reg_error: f64,
        orientation: NormalOrientation,
        min_points: usize,
    ) -> Self {
        Self {
            normal_radius,
            cyl_radius,
            cyl_half_len,
            reg_error,
            orientation,
            min_points,
        }
    }

    pub fn compute(
        &self,
        v1: &PointView,
        v2: &PointView,
        cores: &PointView,
    ) -> Result<PointView, StageError> {
        let mut out = cores.clone();
        for core_id in 0..out.len() {
            let core = point(&out, core_id);
            let Some(mut normal) = find_normal(v1, core, self.normal_radius) else {
                continue;
            };
            normal = match self.orientation {
                NormalOrientation::Up => orient_up(normal),
                NormalOrientation::Down if normal[2] > 0.0 => negate(normal),
                _ => normal,
            };

            let dists1 = filter_points(v1, core, normal, self.cyl_radius, self.cyl_half_len);
            let dists2 = filter_points(v2, core, normal, self.cyl_radius, self.cyl_half_len);
            let Some(stats) = calc_stats(&dists1, &dists2, self.min_points, self.reg_error) else {
                continue;
            };
            out.set_f64(core_id, &DimId::from_name("m3c2_distance"), stats.distance);
            out.set_f64(
                core_id,
                &DimId::from_name("m3c2_uncertainty"),
                stats.uncertainty,
            );
            out.set_f64(
                core_id,
                &DimId::from_name("m3c2_significant"),
                if stats.significant { 1.0 } else { 0.0 },
            );
            out.set_f64(core_id, &DimId::from_name("m3c2_std_dev1"), stats.std_dev1);
            out.set_f64(core_id, &DimId::from_name("m3c2_std_dev2"), stats.std_dev2);
            out.set_f64(core_id, &DimId::from_name("m3c2_count1"), stats.n1 as f64);
            out.set_f64(core_id, &DimId::from_name("m3c2_count2"), stats.n2 as f64);
        }
        Ok(out)
    }
}

impl Filter for M3C2Filter {
    fn name(&self) -> &str {
        "filters.m3c2"
    }

    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        if inputs.len() < 3 {
            return Err(StageError(
                "filters.m3c2 requires first cloud, second cloud, and core points.".to_string(),
            ));
        }
        Ok(vec![self.compute(&inputs[0], &inputs[1], &inputs[2])?])
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        Ok(vec![view.clone()])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        [
            ("m3c2_distance", DimType::F64),
            ("m3c2_uncertainty", DimType::F64),
            ("m3c2_significant", DimType::U8),
            ("m3c2_std_dev1", DimType::F64),
            ("m3c2_std_dev2", DimType::F64),
            ("m3c2_count1", DimType::U16),
            ("m3c2_count2", DimType::U16),
        ]
        .into_iter()
        .map(|(name, ty)| (DimId::from_name(name), ty))
        .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for M3C2Filter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

struct Stats {
    distance: f64,
    uncertainty: f64,
    significant: bool,
    std_dev1: f64,
    std_dev2: f64,
    n1: usize,
    n2: usize,
}

fn calc_stats(pts1: &[f64], pts2: &[f64], min_points: usize, reg_error: f64) -> Option<Stats> {
    if pts1.len() < min_points || pts2.len() < min_points {
        return None;
    }
    let (mean1, var1) = mean_var(pts1);
    let (mean2, var2) = mean_var(pts2);
    let lod_var = var1 / pts1.len() as f64 + var2 / pts2.len() as f64;
    let lod = 1.96 * (lod_var.sqrt() + reg_error);
    let distance = mean2 - mean1;
    Some(Stats {
        distance,
        uncertainty: lod,
        significant: distance.abs() > lod,
        std_dev1: var1.sqrt(),
        std_dev2: var2.sqrt(),
        n1: pts1.len(),
        n2: pts2.len(),
    })
}

fn mean_var(values: &[f64]) -> (f64, f64) {
    let sum = values.iter().sum::<f64>();
    let sum2 = values.iter().map(|v| v * v).sum::<f64>();
    let mean = sum / values.len() as f64;
    (mean, sum2 / values.len() as f64 - mean * mean)
}

fn find_normal(view: &PointView, core: [f64; 3], radius: f64) -> Option<[f64; 3]> {
    let radius2 = radius * radius;
    let ids = (0..view.len())
        .filter(|&id| squared_distance(point(view, id), core) < radius2)
        .collect::<Vec<_>>();
    if ids.len() < 3 {
        return None;
    }
    let cov = math::covariance(view, &ids);
    if math::is_zero_matrix(cov) {
        return None;
    }
    let (_values, vectors) = math::symmetric_eigen_decomposition(cov);
    Some(normalize([vectors[0][0], vectors[1][0], vectors[2][0]]))
}

fn filter_points(
    view: &PointView,
    center: [f64; 3],
    normal: [f64; 3],
    radius: f64,
    half_len: f64,
) -> Vec<f64> {
    let ball_radius2 = radius * radius + half_len * half_len;
    let candidates = (0..view.len())
        .filter(|&id| squared_distance(point(view, id), center) <= ball_radius2)
        .collect::<Vec<_>>();
    let start = usize::from(!view.is_empty() && close_point(center, point(view, 0)));
    candidates
        .into_iter()
        .skip(start)
        .filter_map(|id| point_passes(point(view, id), center, normal, radius, half_len))
        .collect()
}

fn point_passes(
    pt: [f64; 3],
    center: [f64; 3],
    normal: [f64; 3],
    radius: f64,
    half_len: f64,
) -> Option<f64> {
    let rel = sub(pt, center);
    let axial = dot(rel, normal);
    let radial2 = dot(rel, rel) - axial * axial;
    if radial2 > radius * radius {
        return None;
    }
    if axial.abs() > half_len {
        return None;
    }
    Some(axial)
}

fn point(view: &PointView, id: PointId) -> [f64; 3] {
    [
        view.get_f64(id, &DimId::X),
        view.get_f64(id, &DimId::Y),
        view.get_f64(id, &DimId::Z),
    ]
}

fn orient_up(v: [f64; 3]) -> [f64; 3] {
    if v[2] < 0.0 {
        negate(v)
    } else {
        v
    }
}

fn negate(v: [f64; 3]) -> [f64; 3] {
    [-v[0], -v[1], -v[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let len = dot(v, v).sqrt();
    if len == 0.0 {
        v
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

fn squared_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}

fn close_point(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(left, right)| (left - right).abs() <= 1e-9)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    fn view(points: &[[f64; 3]]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        for (name, ty) in
            M3C2Filter::new(1.0, 1.0, 1.0, 0.0, NormalOrientation::Up, 1).output_dimensions()
        {
            layout.register(name, ty);
        }
        let mut view = PointView::new(Rc::new(layout));
        for [x, y, z] in points {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, *x);
            view.set_f64(id, &DimId::Y, *y);
            view.set_f64(id, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn computes_distance_between_parallel_planes() {
        let v1 = view(&[
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ]);
        let v2 = view(&[
            [-1.0, -1.0, 2.0],
            [1.0, -1.0, 2.0],
            [-1.0, 1.0, 2.0],
            [1.0, 1.0, 2.0],
        ]);
        let cores = view(&[[0.0, 0.0, 0.0]]);
        let filter = M3C2Filter::new(2.0, 2.0, 3.0, 0.0, NormalOrientation::Up, 1);
        let out = filter.compute(&v1, &v2, &cores).unwrap();

        assert!((out.get_f64(0, &DimId::from_name("m3c2_distance")) - 2.0).abs() < 1e-9);
        assert_eq!(out.get_f64(0, &DimId::from_name("m3c2_significant")), 1.0);
    }
}
