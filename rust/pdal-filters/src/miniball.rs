//! `filters.miniball` -- local minimal enclosing sphere score.

use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

const EPS: f64 = 1e-9;

pub struct MiniballFilter {
    knn: usize,
}

impl MiniballFilter {
    pub fn new(knn: u64) -> Self {
        Self {
            knn: knn.max(1) as usize,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Point3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Copy, Debug)]
struct Sphere {
    center: Point3,
    radius: f64,
}

impl Filter for MiniballFilter {
    fn name(&self) -> &str {
        "filters.miniball"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        for idx in 0..input.len() {
            out.append_point(input, idx);
        }
        let points: Vec<Point3> = (0..input.len())
            .map(|idx| Point3 {
                x: input.get_f64(idx, &DimId::X),
                y: input.get_f64(idx, &DimId::Y),
                z: input.get_f64(idx, &DimId::Z),
            })
            .collect();

        for idx in 0..points.len() {
            let neighbors = nearest_neighbors(&points, idx, self.knn);
            let sphere = smallest_sphere(&neighbors);
            let d = distance(points[idx], sphere.center);
            let score = if d == 0.0 && sphere.radius == 0.0 {
                0.0
            } else {
                d / (d + 2.0 * sphere.radius / 3.0_f64.sqrt())
            };
            out.set_f64(idx as u64, &DimId::Other("Miniball".to_string()), score);
        }

        Ok(vec![out])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::Other("Miniball".to_string()), DimType::F64)]
    }
}

impl Streamable for MiniballFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

fn nearest_neighbors(points: &[Point3], query: usize, knn: usize) -> Vec<Point3> {
    let mut candidates: Vec<(f64, Point3)> = points
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != query)
        .map(|(_, &point)| (distance_squared(points[query], point), point))
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    candidates
        .into_iter()
        .take(knn)
        .map(|(_, point)| point)
        .collect()
}

fn smallest_sphere(points: &[Point3]) -> Sphere {
    if points.is_empty() {
        return Sphere {
            center: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            radius: 0.0,
        };
    }

    let single = Sphere {
        center: points[0],
        radius: 0.0,
    };
    if contains_all(single, points) {
        return single;
    }
    let mut best: Option<Sphere> = None;

    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let candidate = sphere_from_2(points[i], points[j]);
            keep_best(candidate, points, &mut best);
        }
    }

    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                if let Some(candidate) = sphere_from_3(points[i], points[j], points[k]) {
                    keep_best(candidate, points, &mut best);
                }
            }
        }
    }

    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                for l in k + 1..points.len() {
                    if let Some(candidate) =
                        sphere_from_4(points[i], points[j], points[k], points[l])
                    {
                        keep_best(candidate, points, &mut best);
                    }
                }
            }
        }
    }

    best.unwrap_or(single)
}

fn keep_best(candidate: Sphere, points: &[Point3], best: &mut Option<Sphere>) {
    if contains_all(candidate, points)
        && best
            .as_ref()
            .is_none_or(|current| candidate.radius < current.radius)
    {
        *best = Some(candidate);
    }
}

fn sphere_from_2(a: Point3, b: Point3) -> Sphere {
    let center = Point3 {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
        z: (a.z + b.z) / 2.0,
    };
    Sphere {
        center,
        radius: distance(center, a),
    }
}

fn sphere_from_3(a: Point3, b: Point3, c: Point3) -> Option<Sphere> {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ab_x_ac = cross(ab, ac);
    let denom = 2.0 * dot(ab_x_ac, ab_x_ac);
    if denom.abs() < EPS {
        return None;
    }
    let term1 = scale(cross(ab_x_ac, ab), dot(ac, ac));
    let term2 = scale(cross(ac, ab_x_ac), dot(ab, ab));
    let center = add(a, scale(add(term1, term2), 1.0 / denom));
    Some(Sphere {
        center,
        radius: distance(center, a),
    })
}

fn sphere_from_4(a: Point3, b: Point3, c: Point3, d: Point3) -> Option<Sphere> {
    let rows = [
        [2.0 * (b.x - a.x), 2.0 * (b.y - a.y), 2.0 * (b.z - a.z)],
        [2.0 * (c.x - a.x), 2.0 * (c.y - a.y), 2.0 * (c.z - a.z)],
        [2.0 * (d.x - a.x), 2.0 * (d.y - a.y), 2.0 * (d.z - a.z)],
    ];
    let rhs = [
        dot(b, b) - dot(a, a),
        dot(c, c) - dot(a, a),
        dot(d, d) - dot(a, a),
    ];
    let center = solve_3x3(rows, rhs)?;
    Some(Sphere {
        center,
        radius: distance(center, a),
    })
}

fn solve_3x3(mut a: [[f64; 3]; 3], mut b: [f64; 3]) -> Option<Point3> {
    for col in 0..3 {
        let pivot = (col..3).max_by(|&r1, &r2| a[r1][col].abs().total_cmp(&a[r2][col].abs()))?;
        if a[pivot][col].abs() < EPS {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let divisor = a[col][col];
        for item in &mut a[col][col..] {
            *item /= divisor;
        }
        b[col] /= divisor;
        for row in 0..3 {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            let pivot_row = a[col];
            for (target, pivot) in a[row][col..].iter_mut().zip(pivot_row[col..].iter()) {
                *target -= factor * *pivot;
            }
            b[row] -= factor * b[col];
        }
    }
    Some(Point3 {
        x: b[0],
        y: b[1],
        z: b[2],
    })
}

fn contains_all(sphere: Sphere, points: &[Point3]) -> bool {
    points
        .iter()
        .all(|&point| distance(point, sphere.center) <= sphere.radius + EPS)
}

fn distance(a: Point3, b: Point3) -> f64 {
    distance_squared(a, b).sqrt()
}

fn distance_squared(a: Point3, b: Point3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

fn add(a: Point3, b: Point3) -> Point3 {
    Point3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

fn sub(a: Point3, b: Point3) -> Point3 {
    Point3 {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    }
}

fn scale(a: Point3, value: f64) -> Point3 {
    Point3 {
        x: a.x * value,
        y: a.y * value,
        z: a.z * value,
    }
}

fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross(a: Point3, b: Point3) -> Point3 {
    Point3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};

    #[test]
    fn cardinal_neighbors_give_zero_center_score() {
        let mut view = fixture(false);
        let mut filter = MiniballFilter::new(6);
        let out = filter.run_one(&view).unwrap().remove(0);

        assert_eq!(out.get_f64(0, &DimId::Other("Miniball".to_string())), 0.0);

        view.set_f64(0, &DimId::Z, 1.0);
        let out = filter.run_one(&view).unwrap().remove(0);
        assert!((out.get_f64(0, &DimId::Other("Miniball".to_string())) - 0.464101615).abs() < 1e-6);
    }

    fn fixture(raise_first: bool) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Other("Miniball".to_string()), DimType::F64);
        let mut view = PointView::new(std::rc::Rc::new(layout));
        for (x, y, z) in [
            (0.0, 0.0, if raise_first { 1.0 } else { 0.0 }),
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
        view
    }
}
