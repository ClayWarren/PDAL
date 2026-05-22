//! Iterative Closest Point registration backing `filters.icp`.
//!
//! [`register`] aligns a `moving` point view to a `fixed` one, returning the
//! transformed moving view plus the recovered 4x4 transformation. The C++
//! `IterativeClosestPoint` filter keeps the multi-view orchestration and
//! metadata handling; only this numerical core is ported.

use crate::math::symmetric_eigen_decomposition;
use pdal_core::point::{DimId, PointView};

/// Tuning parameters for [`register`], mirroring the `filters.icp` options.
pub struct IcpParams {
    pub max_iters: i32,
    pub max_similar: i32,
    pub rotation_threshold: f64,
    pub translation_threshold: f64,
    pub mse_abs: f64,
    /// Maximum correspondence distance; `None` accepts any distance.
    pub maxdist: Option<f64>,
    /// Optional initial transform, 16 values in column-major order (matching
    /// `Eigen::Map<Matrix4d>`, which is what the C++ filter used).
    pub init: Option<[f64; 16]>,
}

/// Result of an ICP registration.
pub struct IcpResult {
    /// The moving view with transformed XYZ coordinates.
    pub view: PointView,
    /// Final transformation, 16 values in row-major order.
    pub transform: [f64; 16],
    /// Centroid of the fixed cloud used to center both clouds.
    pub centroid: [f64; 3],
    pub converged: bool,
    pub mse: f64,
}

type Mat3 = [[f64; 3]; 3];
type Mat4 = [[f64; 4]; 4];

fn read_xyz(view: &PointView) -> Vec<[f64; 3]> {
    (0..view.len())
        .map(|i| {
            [
                view.get_f64(i, &DimId::X),
                view.get_f64(i, &DimId::Y),
                view.get_f64(i, &DimId::Z),
            ]
        })
        .collect()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn sqr_dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn det3(m: &Mat3) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn mat3_apply(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn mat4_identity() -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

/// Build a 4x4 matrix from 16 column-major values (Eigen's default layout).
fn mat4_from_col_major(v: &[f64; 16]) -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (c, chunk) in v.chunks(4).enumerate() {
        for (r, &value) in chunk.iter().enumerate() {
            m[r][c] = value;
        }
    }
    m
}

fn mat4_to_row_major(m: &Mat4) -> [f64; 16] {
    let mut v = [0.0; 16];
    for r in 0..4 {
        for c in 0..4 {
            v[r * 4 + c] = m[r][c];
        }
    }
    v
}

fn mat4_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            let mut acc = 0.0;
            for k in 0..4 {
                acc += a[r][k] * b[k][c];
            }
            m[r][c] = acc;
        }
    }
    m
}

/// Apply a 4x4 transform to a 3D point (implicit homogeneous w = 1).
fn mat4_apply(m: &Mat4, p: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
        m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
        m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
    ]
}

/// Recover the rigid rotation that best maps `src` onto `dst` from the 3x3
/// cross-covariance `sigma`, via the SVD-based Umeyama/Kabsch construction.
fn rotation_from_sigma(sigma: Mat3) -> Mat3 {
    // SVD sigma = U * diag(s) * V^T, obtained from the eigendecomposition of
    // the symmetric matrix sigma^T * sigma.
    let mut ata = [[0.0; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            ata[r][c] = sigma.iter().map(|row| row[r] * row[c]).sum();
        }
    }
    // symmetric_eigen_decomposition returns ascending eigenvalues with
    // eigenvectors as columns; reorder to descending singular values.
    let (evals, evecs) = symmetric_eigen_decomposition(ata);
    let order = [2usize, 1, 0];
    let mut v: Mat3 = [[0.0; 3]; 3];
    let mut sv = [0.0; 3];
    for (k, &col) in order.iter().enumerate() {
        sv[k] = evals[col].max(0.0).sqrt();
        for r in 0..3 {
            v[r][k] = evecs[r][col];
        }
    }

    // Force V to be right-handed so the reflection fix below stays valid.
    if det3(&v) < 0.0 {
        for row in v.iter_mut() {
            row[2] = -row[2];
        }
    }

    // U columns: u_k = sigma * v_k / s_k (degenerate columns filled below).
    let mut u: Mat3 = [[0.0; 3]; 3];
    let eps = 1e-12 * sv[0].max(1.0);
    let mut degenerate = [false; 3];
    for k in 0..3 {
        let vc = [v[0][k], v[1][k], v[2][k]];
        let mapped = mat3_apply(&sigma, vc);
        if sv[k] > eps {
            let mut col = [mapped[0] / sv[k], mapped[1] / sv[k], mapped[2] / sv[k]];
            let nrm = (col[0] * col[0] + col[1] * col[1] + col[2] * col[2]).sqrt();
            if nrm > 0.0 {
                for x in &mut col {
                    *x /= nrm;
                }
            }
            for r in 0..3 {
                u[r][k] = col[r];
            }
        } else {
            degenerate[k] = true;
        }
    }
    if degenerate[2] && !degenerate[0] && !degenerate[1] {
        // A planar cloud yields one zero singular value; complete the basis.
        let u0 = [u[0][0], u[1][0], u[2][0]];
        let u1 = [u[0][1], u[1][1], u[2][1]];
        let u2 = cross(u0, u1);
        for r in 0..3 {
            u[r][2] = u2[r];
        }
    } else if degenerate[0] || degenerate[1] {
        // Rank <= 1 is ill-posed and not exercised; fall back to no rotation.
        return mat4_to_mat3_identity();
    }

    // S = diag(1, 1, sign(det sigma)) corrects an improper reflection.
    let s = [1.0, 1.0, if det3(&sigma) < 0.0 { -1.0 } else { 1.0 }];
    let mut rot = [[0.0; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            let mut acc = 0.0;
            for k in 0..3 {
                acc += u[r][k] * s[k] * v[c][k];
            }
            rot[r][c] = acc;
        }
    }
    rot
}

fn mat4_to_mat3_identity() -> Mat3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

/// Estimate the rigid transform mapping `src` onto `dst` (Umeyama, no scaling).
fn umeyama(src: &[[f64; 3]], dst: &[[f64; 3]]) -> Mat4 {
    let n = src.len();
    if n == 0 {
        return mat4_identity();
    }
    let nf = n as f64;
    let mut mean_src = [0.0; 3];
    let mut mean_dst = [0.0; 3];
    for i in 0..n {
        for k in 0..3 {
            mean_src[k] += src[i][k];
            mean_dst[k] += dst[i][k];
        }
    }
    for k in 0..3 {
        mean_src[k] /= nf;
        mean_dst[k] /= nf;
    }

    let mut sigma = [[0.0; 3]; 3];
    for i in 0..n {
        let dd = sub(dst[i], mean_dst);
        let ds = sub(src[i], mean_src);
        for r in 0..3 {
            for c in 0..3 {
                sigma[r][c] += dd[r] * ds[c];
            }
        }
    }
    for row in sigma.iter_mut() {
        for x in row.iter_mut() {
            *x /= nf;
        }
    }

    let rot = rotation_from_sigma(sigma);
    let rotated = mat3_apply(&rot, mean_src);
    let t = [
        mean_dst[0] - rotated[0],
        mean_dst[1] - rotated[1],
        mean_dst[2] - rotated[2],
    ];
    [
        [rot[0][0], rot[0][1], rot[0][2], t[0]],
        [rot[1][0], rot[1][1], rot[1][2], t[1]],
        [rot[2][0], rot[2][1], rot[2][2], t[2]],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Index of the nearest point in `cloud` to `query` (brute force).
fn nearest(cloud: &[[f64; 3]], query: [f64; 3]) -> (usize, f64) {
    let mut best = 0usize;
    let mut best_d = f64::MAX;
    for (i, p) in cloud.iter().enumerate() {
        let d = sqr_dist(query, *p);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    (best, best_d)
}

/// Register `moving` onto `fixed` with Iterative Closest Point.
pub fn register(fixed: &PointView, moving: &PointView, params: &IcpParams) -> IcpResult {
    let fixed_pts = read_xyz(fixed);
    let moving_pts = read_xyz(moving);

    // Center both clouds on the fixed cloud's centroid.
    let mut centroid = [0.0; 3];
    for p in &fixed_pts {
        for k in 0..3 {
            centroid[k] += p[k];
        }
    }
    let nf = (fixed_pts.len().max(1)) as f64;
    for c in &mut centroid {
        *c /= nf;
    }
    let fixed_demean: Vec<[f64; 3]> = fixed_pts.iter().map(|p| sub(*p, centroid)).collect();
    let moving_demean: Vec<[f64; 3]> = moving_pts.iter().map(|p| sub(*p, centroid)).collect();

    let mut final_t = match params.init {
        Some(v) => mat4_from_col_major(&v),
        None => mat4_identity(),
    };
    let sqr_maxdist = params.maxdist.map(|d| d * d).unwrap_or(f64::MAX);

    let mut converged = false;
    let mut prev_mse = 0.0;
    let mut num_similar = 0i32;

    for _ in 0..params.max_iters {
        let moving_t: Vec<[f64; 3]> = moving_demean
            .iter()
            .map(|p| mat4_apply(&final_t, *p))
            .collect();

        // Build point correspondences within the maximum distance.
        let mut a_pts: Vec<[f64; 3]> = Vec::new();
        let mut b_pts: Vec<[f64; 3]> = Vec::new();
        let mut mse = 0.0;
        for mp in &moving_t {
            let (idx, d) = nearest(&fixed_demean, *mp);
            if d < sqr_maxdist {
                a_pts.push(fixed_demean[idx]);
                b_pts.push(*mp);
                mse += d.sqrt();
            }
        }
        mse /= (b_pts.len().max(1)) as f64;

        let t = umeyama(&b_pts, &a_pts);
        final_t = mat4_mul(&final_t, &t);

        let cos_angle = 0.5 * (t[0][0] + t[1][1] + t[2][2] - 1.0);
        let translation_sqr = t[0][3] * t[0][3] + t[1][3] * t[1][3] + t[2][3] * t[2][3];

        let mut is_similar = false;
        if (mse - prev_mse).abs() < params.mse_abs {
            if num_similar >= params.max_similar {
                converged = true;
                break;
            }
            is_similar = true;
        }
        if cos_angle >= params.rotation_threshold && translation_sqr <= params.translation_threshold
        {
            if num_similar >= params.max_similar {
                converged = true;
                break;
            }
            is_similar = true;
        }
        num_similar = if is_similar { num_similar + 1 } else { 0 };
        prev_mse = mse;
    }

    // Apply the final transform to the original moving points.
    let mut out = moving.make_new();
    for i in 0..moving.len() {
        out.append_point(moving, i);
    }
    let mut transformed: Vec<[f64; 3]> = Vec::with_capacity(moving_pts.len());
    for (i, p) in moving_pts.iter().enumerate() {
        let local = mat4_apply(&final_t, sub(*p, centroid));
        let world = [
            local[0] + centroid[0],
            local[1] + centroid[1],
            local[2] + centroid[2],
        ];
        out.set_f64(i as u64, &DimId::X, world[0]);
        out.set_f64(i as u64, &DimId::Y, world[1]);
        out.set_f64(i as u64, &DimId::Z, world[2]);
        transformed.push(world);
    }

    // Final mean-squared error against the unaltered fixed cloud.
    let mut mse = 0.0;
    let mut mse_n = 0usize;
    for tp in &transformed {
        let (_, d) = nearest(&fixed_pts, *tp);
        if d < sqr_maxdist {
            mse_n += 1;
            mse += d.sqrt();
        }
    }
    mse /= (mse_n.max(1)) as f64;

    IcpResult {
        view: out,
        transform: mat4_to_row_major(&final_t),
        centroid,
        converged,
        mse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(points: &[[f64; 3]]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for p in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, p[0]);
            view.set_f64(idx, &DimId::Y, p[1]);
            view.set_f64(idx, &DimId::Z, p[2]);
        }
        view
    }

    fn default_params() -> IcpParams {
        IcpParams {
            max_iters: 100,
            max_similar: 0,
            rotation_threshold: 0.99999,
            translation_threshold: 3e-4 * 3e-4,
            mse_abs: 1e-12,
            maxdist: None,
            init: None,
        }
    }

    const CLOUD: [[f64; 3]; 6] = [
        [0.0, 0.0, 0.0],
        [4.0, 0.0, 1.0],
        [0.0, 5.0, 2.0],
        [3.0, 3.0, 0.5],
        [1.0, 4.0, 3.0],
        [5.0, 2.0, 1.5],
    ];

    #[test]
    fn identity_for_equal_clouds() {
        let fixed = view(&CLOUD);
        let moving = view(&CLOUD);
        let result = register(&fixed, &moving, &default_params());
        // Translation components should be ~0.
        for &i in &[3usize, 7, 11] {
            assert!(
                result.transform[i].abs() < 1e-6,
                "t={}",
                result.transform[i]
            );
        }
        assert!(result.converged);
    }

    #[test]
    fn recovers_pure_translation() {
        // A small shift keeps nearest-neighbor correspondences correct, so the
        // exact translation is recovered (large shifts need a denser cloud or
        // an initial guess, as the C++ tests use looser tolerances for).
        let fixed = view(&CLOUD);
        let shifted: Vec<[f64; 3]> = CLOUD
            .iter()
            .map(|p| [p[0] + 0.3, p[1] + 0.4, p[2] + 0.5])
            .collect();
        let moving = view(&shifted);
        let result = register(&fixed, &moving, &default_params());
        // The transform maps the moving cloud back onto the fixed one.
        assert!((result.transform[3] - -0.3).abs() < 1e-3);
        assert!((result.transform[7] - -0.4).abs() < 1e-3);
        assert!((result.transform[11] - -0.5).abs() < 1e-3);
        // And the transformed points coincide with the fixed cloud.
        for i in 0..result.view.len() {
            assert!((result.view.get_f64(i, &DimId::X) - CLOUD[i as usize][0]).abs() < 1e-3);
            assert!((result.view.get_f64(i, &DimId::Y) - CLOUD[i as usize][1]).abs() < 1e-3);
            assert!((result.view.get_f64(i, &DimId::Z) - CLOUD[i as usize][2]).abs() < 1e-3);
        }
    }

    #[test]
    fn recovers_rotation_about_z() {
        let angle: f64 = 0.15;
        let (c, s) = (angle.cos(), angle.sin());
        let rotated: Vec<[f64; 3]> = CLOUD
            .iter()
            .map(|p| [c * p[0] - s * p[1], s * p[0] + c * p[1], p[2]])
            .collect();
        let fixed = view(&CLOUD);
        let moving = view(&rotated);
        let result = register(&fixed, &moving, &default_params());
        // The recovered rotation should undo the applied one.
        assert!((result.transform[0] - c).abs() < 1e-3);
        assert!((result.transform[1] - s).abs() < 1e-3);
        assert!((result.transform[4] - -s).abs() < 1e-3);
        assert!((result.transform[5] - c).abs() < 1e-3);
    }
}
