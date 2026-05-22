//! Oriented bounding box intersection, ported from `io/private/esri/Obb.cpp`.
//!
//! Mirrors `pdal::i3s::Obb::intersect` and its helpers exactly. An OBB is given
//! by a center, half-extents, and a (pre-normalized) `[x, y, z, w]` quaternion.

type V3 = [f64; 3];
/// Quaternion stored as `[x, y, z, w]`.
type Quat = [f64; 4];

/// An oriented bounding box.
pub struct Obb {
    pub center: V3,
    pub half: V3,
    pub quat: Quat,
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn conjugate(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

fn quat_mul(a: Quat, b: Quat) -> Quat {
    let (ax, ay, az, aw) = (a[0], a[1], a[2], a[3]);
    let (bx, by, bz, bw) = (b[0], b[1], b[2], b[3]);
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

/// Rotate vector `v` by quaternion `q` (`q * (v, 0) * q^-1`). `q` is assumed
/// to be a unit quaternion, so its inverse is its conjugate.
fn rotate(q: Quat, v: V3) -> V3 {
    let p: Quat = [v[0], v[1], v[2], 0.0];
    let r = quat_mul(quat_mul(q, p), conjugate(q));
    [r[0], r[1], r[2]]
}

impl Obb {
    /// The `pos`-th corner (0..8) in world coordinates.
    fn corner(&self, pos: usize) -> V3 {
        let v: V3 = [
            if pos & 1 != 0 {
                -self.half[0]
            } else {
                self.half[0]
            },
            if pos & 2 != 0 {
                -self.half[1]
            } else {
                self.half[1]
            },
            if pos & 4 != 0 {
                -self.half[2]
            } else {
                self.half[2]
            },
        ];
        let r = rotate(self.quat, v);
        [
            r[0] + self.center[0],
            r[1] + self.center[1],
            r[2] + self.center[2],
        ]
    }

    /// The `pos`-th edge (0..12) as a pair of corner points.
    fn segment(&self, pos: usize) -> (V3, V3) {
        const SEGS: [(usize, usize); 12] = [
            (0, 2),
            (2, 6),
            (6, 4),
            (4, 0),
            (1, 3),
            (3, 7),
            (7, 5),
            (5, 1),
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
        ];
        (self.corner(SEGS[pos].0), self.corner(SEGS[pos].1))
    }
}

/// Inclusive containment test for the axis-aligned box `[-half, half]`.
fn box3d_contains(half: V3, p: V3) -> bool {
    p[0] >= -half[0]
        && p[0] <= half[0]
        && p[1] >= -half[1]
        && p[1] <= half[1]
        && p[2] >= -half[2]
        && p[2] <= half[2]
}

/// Test whether `seg` intersects the origin-centered axis-aligned box with the
/// given half-extents. Mirrors `Obb::intersectNormalized`.
fn intersect_normalized(half: V3, seg: (V3, V3)) -> bool {
    let (p0, p1) = seg;
    let faces: [V3; 6] = [
        [half[0], 0.0, 0.0],
        [-half[0], 0.0, 0.0],
        [0.0, half[1], 0.0],
        [0.0, -half[1], 0.0],
        [0.0, 0.0, half[2]],
        [0.0, 0.0, -half[2]],
    ];
    // 2D faces as `[minx, miny, maxx, maxy]`.
    let boxes: [[f64; 4]; 3] = [
        [-half[1], -half[2], half[1], half[2]],
        [-half[0], -half[2], half[0], half[2]],
        [-half[0], -half[1], half[0], half[1]],
    ];

    for face in faces {
        let v1 = sub(face, p0);
        let v2 = sub(p1, p0);
        let num = dot(v1, face);
        let den = dot(v2, face);
        if den == 0.0 {
            return false;
        }
        let t = num / den;
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let isect: V3 = [
            t * (p1[0] - p0[0]) + p0[0],
            t * (p1[1] - p0[1]) + p0[1],
            t * (p1[2] - p0[2]) + p0[2],
        ];

        // Drop the dimension along the face normal to get a 2D point/box.
        let mut coord = [0.0f64; 2];
        let mut pos = 0usize;
        let mut box_idx = 0usize;
        for j in 0..3 {
            if face[j] != 0.0 {
                box_idx = j;
            } else {
                coord[pos] = isect[j];
                pos += 1;
            }
        }
        let b = boxes[box_idx];
        if coord[0] >= b[0] && coord[0] <= b[2] && coord[1] >= b[1] && coord[1] <= b[3] {
            return true;
        }
    }
    false
}

/// Half of the symmetric OBB intersection test: treat `a` as axis-aligned at
/// the origin and test `b` against it. Mirrors `Obb::halfIntersect`.
fn half_intersect(a: &Obb, b: &Obb) -> bool {
    // Bring `b` into `a`'s coordinate frame.
    let translated = sub(b.center, a.center);
    let moved = Obb {
        center: rotate(conjugate(a.quat), translated),
        half: b.half,
        quat: b.quat,
    };

    let mut pmin = [f64::MAX; 3];
    let mut pmax = [f64::MIN; 3];
    for i in 0..8 {
        let corner = moved.corner(i);
        if box3d_contains(a.half, corner) {
            return true;
        }
        for k in 0..3 {
            pmax[k] = pmax[k].max(corner[k]);
            pmin[k] = pmin[k].min(corner[k]);
        }
    }

    // `b` fully surrounds `a`.
    if pmax[0] >= a.half[0]
        && pmin[0] <= -a.half[0]
        && pmax[1] >= a.half[1]
        && pmin[1] <= -a.half[1]
        && pmax[2] >= a.half[2]
        && pmin[2] <= -a.half[2]
    {
        return true;
    }

    for i in 0..12 {
        if intersect_normalized(a.half, moved.segment(i)) {
            return true;
        }
    }
    false
}

/// Test whether two oriented bounding boxes intersect.
pub fn obb_intersect(a: &Obb, b: &Obb) -> bool {
    half_intersect(a, b) || half_intersect(b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Quat = [0.0, 0.0, 0.0, 1.0];

    #[test]
    fn identical_boxes_intersect() {
        let a = Obb {
            center: [0.0, 0.0, 0.0],
            half: [1.0, 1.0, 1.0],
            quat: IDENTITY,
        };
        let b = Obb {
            center: [0.0, 0.0, 0.0],
            half: [1.0, 1.0, 1.0],
            quat: IDENTITY,
        };
        assert!(obb_intersect(&a, &b));
    }

    #[test]
    fn distant_boxes_do_not_intersect() {
        let a = Obb {
            center: [0.0, 0.0, 0.0],
            half: [1.0, 1.0, 1.0],
            quat: IDENTITY,
        };
        let b = Obb {
            center: [10.0, 0.0, 0.0],
            half: [1.0, 1.0, 1.0],
            quat: IDENTITY,
        };
        assert!(!obb_intersect(&a, &b));
    }

    #[test]
    fn touching_boxes_intersect_and_test_is_symmetric() {
        let a = Obb {
            center: [0.0, 0.0, 0.0],
            half: [1.0, 1.0, 1.0],
            quat: IDENTITY,
        };
        let b = Obb {
            center: [2.0, 0.0, 0.0],
            half: [1.0, 1.0, 1.0],
            quat: IDENTITY,
        };
        assert!(obb_intersect(&a, &b));
        assert_eq!(obb_intersect(&a, &b), obb_intersect(&b, &a));
    }

    #[test]
    fn rotated_box_intersection_matches_cpp_reference() {
        // Mirrors the C++ ObbTest.obb fixture.
        let base = Obb {
            center: [0.0, 0.0, 0.0],
            half: [2.0, 1.0, 1.5],
            quat: IDENTITY,
        };
        let clip_quat: Quat = [0.0, 0.0, -0.3826834324, 0.9238795325];
        let clip_half: V3 = [2.12132034355, std::f64::consts::FRAC_1_SQRT_2, 1.0];

        let hit = |center: V3| {
            let clip = Obb {
                center,
                half: clip_half,
                quat: clip_quat,
            };
            obb_intersect(&base, &clip)
        };
        assert!(hit([2.0, 1.0, 0.0]));
        assert!(hit([2.0, 1.0, -1.0]));
        assert!(hit([2.0, 1.0, -2.5]));
        assert!(!hit([2.0, 1.0, -2.51]));
        assert!(!hit([2.0, 3.0, 0.0]));
        assert!(hit([2.0, 2.0, 0.0]));
    }
}
