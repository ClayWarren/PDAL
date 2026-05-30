//! Greedy projection triangulation backing `filters.greedyprojection`.
//!
//! Port of the C++ `GreedyProjection::filter` in
//! `filters/GreedyProjection.cpp`, which is itself a PDAL port of the PCL
//! GreedyProjectionTriangulation algorithm. The C++ wrapper feeds normals
//! (via `filters.normal`) and 3D KNN queries (via `KD3Index`) into this
//! routine through the C ABI, then writes the returned triangle indices
//! into the C++ `TriangularMesh`.
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::if_same_then_else)]
#![allow(unused_assignments)]

use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use std::f64::consts::PI;

pub struct GreedyProjectionParams {
    pub mu: f64,
    pub search_radius: f64,
    pub nnn: usize,
    pub min_angle: f64,
    pub max_angle: f64,
    pub eps_angle: f64,
    pub consistent: bool,
}

impl Default for GreedyProjectionParams {
    fn default() -> Self {
        Self {
            mu: 0.0,
            search_radius: 0.0,
            nnn: 100,
            min_angle: PI / 18.0,
            max_angle: 2.0 * PI / 3.0,
            eps_angle: PI / 4.0,
            consistent: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gp3 {
    None,
    Free,
    Fringe,
    Boundary,
    Completed,
}

const NIL: PointId = PointId::MAX;

#[derive(Clone, Copy)]
struct NnAngle {
    angle: f64,
    index: PointId,
    nn_index: i32,
    visible: bool,
}

#[derive(Clone, Copy)]
struct DoubleEdge {
    index: usize,
    first: [f64; 2],
    second: [f64; 2],
}

type V2 = [f64; 2];
type V3 = [f64; 3];

#[inline]
fn dot3(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn sub3(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn cross3(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn sqr_norm3(a: V3) -> f64 {
    dot3(a, a)
}

/// Eigen `Vector3d::unitOrthogonal()` returns a unit-length vector
/// perpendicular to the receiver. Default dummy_precision for `double` in
/// Eigen 3 is `1e-12`, so `isMuchSmallerThan(x, y)` is `x^2 <= 1e-24 * y^2`.
fn unit_orthogonal3(n: V3) -> V3 {
    let prec_sq: f64 = 1e-24;
    let (x, y, z) = (n[0], n[1], n[2]);
    let mss_xy = x * x <= prec_sq * (y * y);
    let mss_xz = x * x <= prec_sq * (z * z);
    if !mss_xy || !mss_xz {
        let invnm = 1.0 / (x * x + y * y).sqrt();
        [-y * invnm, x * invnm, 0.0]
    } else {
        let invnm = 1.0 / (y * y + z * z).sqrt();
        [0.0, -z * invnm, y * invnm]
    }
}

/// Port of `pdal::isVisible` in `filters/GreedyProjection.hpp`. Returns whether
/// point `x` is visible from the reference point `r` when the segment `s1-s2`
/// is considered. When `r == [0,0]` matches the default-arg path in C++.
fn is_visible(x: V2, s1: V2, s2: V2, r: V2) -> bool {
    let r_zero = r[0] == 0.0 && r[1] == 0.0;
    let a0 = s1[1] - s2[1];
    let b0 = s2[0] - s1[0];
    let c0 = s1[0] * s2[1] - s2[0] * s1[1];
    let mut a1 = -x[1];
    let mut b1 = x[0];
    let mut c1 = 0.0;
    if !r_zero {
        a1 += r[1];
        b1 -= r[0];
        c1 = r[0] * x[1] - x[0] * r[1];
    }
    let div = a0 * b1 - b0 * a1;
    let qx = (b0 * c1 - b1 * c0) / div;
    let qy = (a1 * c0 - a0 * c1) / div;

    let intersection_outside_xr = if r_zero {
        if x[0] > 0.0 {
            (qx <= 0.0) || (qx >= x[0])
        } else if x[0] < 0.0 {
            (qx >= 0.0) || (qx <= x[0])
        } else if x[1] > 0.0 {
            (qy <= 0.0) || (qy >= x[1])
        } else if x[1] < 0.0 {
            (qy >= 0.0) || (qy <= x[1])
        } else {
            true
        }
    } else if x[0] > r[0] {
        (qx <= r[0]) || (qx >= x[0])
    } else if x[0] < r[0] {
        (qx >= r[0]) || (qx <= x[0])
    } else if x[1] > r[1] {
        (qy <= r[1]) || (qy >= x[1])
    } else if x[1] < r[1] {
        (qy >= r[1]) || (qy <= x[1])
    } else {
        true
    };

    if intersection_outside_xr {
        true
    } else if s1[0] > s2[0] {
        (qx <= s2[0]) || (qx >= s1[0])
    } else if s1[0] < s2[0] {
        (qx >= s2[0]) || (qx <= s1[0])
    } else if s1[1] > s2[1] {
        (qy <= s2[1]) || (qy >= s1[1])
    } else if s1[1] < s2[1] {
        (qy >= s2[1]) || (qy <= s1[1])
    } else {
        false
    }
}

struct Algo<'a> {
    coords: Vec<V3>,
    normals: Vec<V3>,
    params: GreedyProjectionParams,
    nnn: usize,
    state: Vec<Gp3>,
    source: Vec<PointId>,
    ffn: Vec<PointId>,
    sfn: Vec<PointId>,
    part: Vec<PointId>,
    fringe_queue: Vec<PointId>,
    angles: Vec<NnAngle>,
    triangles: Vec<[PointId; 3]>,
    r_: PointId,
    proj_qp: V3,
    u_vec: V3,
    v_vec: V3,
    uvn_ffn: V2,
    uvn_sfn: V2,
    uvn_next_ffn: V2,
    uvn_next_sfn: V2,
    is_current_free: bool,
    current_index: PointId,
    prev_is_ffn: bool,
    prev_is_sfn: bool,
    next_is_ffn: bool,
    next_is_sfn: bool,
    changed_1st_fn: bool,
    changed_2nd_fn: bool,
    new2boundary: PointId,
    already_connected: bool,
    spatial: SpatialIndex3d<'a>,
}

mod algo;
mod algo_run;


/// Run greedy projection triangulation. `view` must have X/Y/Z plus
/// NormalX/NormalY/NormalZ populated (the C++ wrapper runs `filters.normal`
/// before calling this). Returns triangles as `(a, b, c)` index triples.
pub fn run(view: &PointView, params: GreedyProjectionParams) -> Vec<[PointId; 3]> {
    let n = view.len();
    let mut coords = Vec::with_capacity(n as usize);
    let mut normals = Vec::with_capacity(n as usize);
    for i in 0..n {
        coords.push([
            view.get_f64(i, &DimId::X),
            view.get_f64(i, &DimId::Y),
            view.get_f64(i, &DimId::Z),
        ]);
        normals.push([
            view.get_f64(i, &DimId::NormalX),
            view.get_f64(i, &DimId::NormalY),
            view.get_f64(i, &DimId::NormalZ),
        ]);
    }
    let spatial = SpatialIndex3d::new(view);
    let mut algo = Algo {
        coords,
        normals,
        params,
        nnn: 0,
        state: Vec::new(),
        source: Vec::new(),
        ffn: Vec::new(),
        sfn: Vec::new(),
        part: Vec::new(),
        fringe_queue: Vec::new(),
        angles: Vec::new(),
        triangles: Vec::new(),
        r_: 0,
        proj_qp: [0.0; 3],
        u_vec: [0.0; 3],
        v_vec: [0.0; 3],
        uvn_ffn: [0.0; 2],
        uvn_sfn: [0.0; 2],
        uvn_next_ffn: [0.0; 2],
        uvn_next_sfn: [0.0; 2],
        is_current_free: false,
        current_index: 0,
        prev_is_ffn: false,
        prev_is_sfn: false,
        next_is_ffn: false,
        next_is_sfn: false,
        changed_1st_fn: false,
        changed_2nd_fn: false,
        new2boundary: NIL,
        already_connected: false,
        spatial,
    };
    algo.run();
    algo.triangles
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn planar_view() -> PointView {
        let mut layout = PointLayout::new();
        for d in [
            DimId::X,
            DimId::Y,
            DimId::Z,
            DimId::NormalX,
            DimId::NormalY,
            DimId::NormalZ,
        ] {
            layout.register(d, DimType::F64);
        }
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);
        for y in 0..3 {
            for x in 0..3 {
                let id = view.add_point();
                view.set_f64(id, &DimId::X, x as f64);
                view.set_f64(id, &DimId::Y, y as f64);
                view.set_f64(id, &DimId::Z, 0.0);
                view.set_f64(id, &DimId::NormalX, 0.0);
                view.set_f64(id, &DimId::NormalY, 0.0);
                view.set_f64(id, &DimId::NormalZ, 1.0);
            }
        }
        view
    }

    #[test]
    fn planar_points_produce_a_valid_mesh() {
        let view = planar_view();
        let params = GreedyProjectionParams {
            mu: 2.5,
            search_radius: 2.0,
            nnn: 8,
            ..GreedyProjectionParams::default()
        };
        let triangles = run(&view, params);
        assert!(!triangles.is_empty(), "expected at least one triangle");
        let n = view.len();
        for tri in &triangles {
            for &idx in tri {
                assert!(idx < n, "triangle index out of bounds");
            }
        }
    }
}
