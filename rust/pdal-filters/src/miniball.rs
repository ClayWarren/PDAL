use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

const EPS: f64 = 1e-9;

pub struct MiniballFilter {
    knn: usize,
}

impl MiniballFilter {
    pub fn new(knn: usize) -> Self {
        Self { knn }
    }
}

impl Filter for MiniballFilter {
    fn name(&self) -> &str {
        "filters.miniball"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let neighbors: Vec<[f64; 3]> = index
                .knn(idx, self.knn + 1)
                .into_iter()
                .map(|(id, _dist)| id)
                .filter(|id| *id != idx)
                .map(|id| xyz(view, id))
                .collect();
            if neighbors.is_empty() {
                continue;
            }
            let sphere = smallest_enclosing_sphere(&neighbors);
            let point = xyz(view, idx);
            let d = distance(point, sphere.center);
            let value = d / (d + 2.0 * sphere.radius / 3.0_f64.sqrt());
            output.set_f64(idx, &DimId::Miniball, value);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for MiniballFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

#[derive(Clone, Copy)]
struct Sphere {
    center: [f64; 3],
    radius: f64,
}

fn smallest_enclosing_sphere(points: &[[f64; 3]]) -> Sphere {
    let mut best = Sphere {
        center: points[0],
        radius: 0.0,
    };
    if contains_all(best, points) {
        return best;
    }

    best.radius = f64::INFINITY;
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            consider(&mut best, sphere_from2(points[i], points[j]), points);
        }
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                if let Some(sphere) = sphere_from3(points[i], points[j], points[k]) {
                    consider(&mut best, sphere, points);
                }
            }
        }
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                for l in k + 1..points.len() {
                    if let Some(sphere) = sphere_from4(points[i], points[j], points[k], points[l]) {
                        consider(&mut best, sphere, points);
                    }
                }
            }
        }
    }
    best
}

fn consider(best: &mut Sphere, candidate: Sphere, points: &[[f64; 3]]) {
    if candidate.radius < best.radius && contains_all(candidate, points) {
        *best = candidate;
    }
}

fn contains_all(sphere: Sphere, points: &[[f64; 3]]) -> bool {
    points
        .iter()
        .all(|point| distance(*point, sphere.center) <= sphere.radius + EPS)
}

fn sphere_from2(a: [f64; 3], b: [f64; 3]) -> Sphere {
    let center = scale(add(a, b), 0.5);
    Sphere {
        center,
        radius: distance(a, center),
    }
}

fn sphere_from3(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<Sphere> {
    let u = sub(b, a);
    let v = sub(c, a);
    let n = cross(u, v);
    let rhs = [
        (dot(b, b) - dot(a, a)) * 0.5,
        (dot(c, c) - dot(a, a)) * 0.5,
        dot(n, a),
    ];
    let center = solve3([u, v, n], rhs)?;
    Some(Sphere {
        center,
        radius: distance(center, a),
    })
}

fn sphere_from4(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> Option<Sphere> {
    let rows = [sub(b, a), sub(c, a), sub(d, a)];
    let rhs = [
        (dot(b, b) - dot(a, a)) * 0.5,
        (dot(c, c) - dot(a, a)) * 0.5,
        (dot(d, d) - dot(a, a)) * 0.5,
    ];
    let center = solve3(rows, rhs)?;
    Some(Sphere {
        center,
        radius: distance(center, a),
    })
}

fn solve3(rows: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let det = rows[0][0] * (rows[1][1] * rows[2][2] - rows[1][2] * rows[2][1])
        - rows[0][1] * (rows[1][0] * rows[2][2] - rows[1][2] * rows[2][0])
        + rows[0][2] * (rows[1][0] * rows[2][1] - rows[1][1] * rows[2][0]);
    if det.abs() < EPS {
        return None;
    }

    let det_x = rhs[0] * (rows[1][1] * rows[2][2] - rows[1][2] * rows[2][1])
        - rows[0][1] * (rhs[1] * rows[2][2] - rows[1][2] * rhs[2])
        + rows[0][2] * (rhs[1] * rows[2][1] - rows[1][1] * rhs[2]);
    let det_y = rows[0][0] * (rhs[1] * rows[2][2] - rows[1][2] * rhs[2])
        - rhs[0] * (rows[1][0] * rows[2][2] - rows[1][2] * rows[2][0])
        + rows[0][2] * (rows[1][0] * rhs[2] - rhs[1] * rows[2][0]);
    let det_z = rows[0][0] * (rows[1][1] * rhs[2] - rhs[1] * rows[2][1])
        - rows[0][1] * (rows[1][0] * rhs[2] - rhs[1] * rows[2][0])
        + rhs[0] * (rows[1][0] * rows[2][1] - rows[1][1] * rows[2][0]);

    Some([det_x / det, det_y / det, det_z / det])
}

fn xyz(view: &PointView, id: PointId) -> [f64; 3] {
    [
        view.get_f64(id, &DimId::X),
        view.get_f64(id, &DimId::Y),
        view.get_f64(id, &DimId::Z),
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(sub(a, b), sub(a, b)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn layout() -> PointLayout {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Miniball, DimType::F64);
        layout
    }

    #[test]
    fn centered_cross_has_zero_score() {
        let mut view = PointView::new(Rc::new(layout()));
        for (x, y, z) in [
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, -1.0),
            (0.0, 1.0, 0.0),
            (0.0, -1.0, 0.0),
            (1.0, 0.0, 0.0),
            (-1.0, 0.0, 0.0),
        ] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let mut filter = MiniballFilter::new(6);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Miniball), 0.0);
    }
}
