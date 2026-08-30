//! C ABI for oriented bounding box intersection.
//!
//! Mirrors `pdal::i3s::Obb::intersect` from `io/private/esri/Obb.cpp`. Each box
//! is passed as a center `[x, y, z]`, half-extents `[hx, hy, hz]`, and a
//! pre-normalized quaternion `[x, y, z, w]`.

use pdal_core::obb::{obb_intersect, Obb};
use std::slice;

unsafe fn read3(ptr: *const f64) -> [f64; 3] {
    let s = slice::from_raw_parts(ptr, 3);
    [s[0], s[1], s[2]]
}

unsafe fn read4(ptr: *const f64) -> [f64; 4] {
    let s = slice::from_raw_parts(ptr, 4);
    [s[0], s[1], s[2], s[3]]
}

/// Test whether two oriented bounding boxes intersect.
///
/// Returns `false` if any pointer is null.
///
/// # Safety
/// `center_*` and `half_*` must each point to 3 `f64` values, and `quat_*` to
/// 4 `f64` values.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_obb_intersect(
    center_a: *const f64,
    half_a: *const f64,
    quat_a: *const f64,
    center_b: *const f64,
    half_b: *const f64,
    quat_b: *const f64,
) -> bool {
    if center_a.is_null()
        || half_a.is_null()
        || quat_a.is_null()
        || center_b.is_null()
        || half_b.is_null()
        || quat_b.is_null()
    {
        return false;
    }
    let a = Obb {
        center: read3(center_a),
        half: read3(half_a),
        quat: read4(quat_a),
    };
    let b = Obb {
        center: read3(center_b),
        half: read3(half_b),
        quat: read4(quat_b),
    };
    obb_intersect(&a, &b)
}
