//! C ABI for raster math helpers from `pdal/private/MathUtils`.
//!
//! Exposes the numerical gradient and diamond morphology routines, reusing the
//! Rust implementations that already back the `filters.smrf` port.

use pdal_core::point::{DimId, PointView};
use pdal_filters::math;

/// Compute the numerical gradient in the X direction.
///
/// `data` and `out` are column-major `rows * cols` buffers. `out` receives the
/// result and must not alias `data`.
///
/// # Safety
/// `data` and `out` must each be valid for `rows * cols` `f64` values.
#[no_mangle]
pub unsafe extern "C" fn pdal_math_grad_x(
    data: *const f64,
    rows: usize,
    cols: usize,
    out: *mut f64,
) {
    if data.is_null() || out.is_null() {
        return;
    }
    let count = rows.saturating_mul(cols);
    let input = std::slice::from_raw_parts(data, count);
    let result = math::grad_x(input, rows, cols);
    std::ptr::copy_nonoverlapping(result.as_ptr(), out, count);
}

/// Compute the numerical gradient in the Y direction.
///
/// # Safety
/// See [`pdal_math_grad_x`].
#[no_mangle]
pub unsafe extern "C" fn pdal_math_grad_y(
    data: *const f64,
    rows: usize,
    cols: usize,
    out: *mut f64,
) {
    if data.is_null() || out.is_null() {
        return;
    }
    let count = rows.saturating_mul(cols);
    let input = std::slice::from_raw_parts(data, count);
    let result = math::grad_y(input, rows, cols);
    std::ptr::copy_nonoverlapping(result.as_ptr(), out, count);
}

/// Morphologically dilate a column-major raster in place with a diamond
/// structuring element.
///
/// # Safety
/// `data` must be valid for `rows * cols` `f64` values.
#[no_mangle]
pub unsafe extern "C" fn pdal_math_dilate_diamond(
    data: *mut f64,
    rows: usize,
    cols: usize,
    iterations: i32,
) {
    if data.is_null() {
        return;
    }
    let count = rows.saturating_mul(cols);
    let raster = std::slice::from_raw_parts_mut(data, count);
    math::dilate_diamond(raster, rows, cols, iterations.max(0) as usize);
}

/// Morphologically erode a column-major raster in place with a diamond
/// structuring element.
///
/// # Safety
/// See [`pdal_math_dilate_diamond`].
#[no_mangle]
pub unsafe extern "C" fn pdal_math_erode_diamond(
    data: *mut f64,
    rows: usize,
    cols: usize,
    iterations: i32,
) {
    if data.is_null() {
        return;
    }
    let count = rows.saturating_mul(cols);
    let raster = std::slice::from_raw_parts_mut(data, count);
    math::erode_diamond(raster, rows, cols, iterations.max(0) as usize);
}

/// Compute the centroid of `count` interleaved `[x, y, z]` points.
///
/// Writes the centroid `[x, y, z]` to `out_xyz`. A zero count yields the
/// origin.
///
/// # Safety
/// `xyz` must be valid for `count * 3` `f64` values and `out_xyz` for 3.
#[no_mangle]
pub unsafe extern "C" fn pdal_math_compute_centroid(
    xyz: *const f64,
    count: usize,
    out_xyz: *mut f64,
) {
    if xyz.is_null() || out_xyz.is_null() {
        return;
    }
    let points = std::slice::from_raw_parts(xyz, count.saturating_mul(3));
    let centroid = math::compute_centroid(points, count);
    std::ptr::copy_nonoverlapping(centroid.as_ptr(), out_xyz, 3);
}

/// Copy a point view's XYZ dimensions into an interleaved row-major buffer.
///
/// Returns the required number of `f64` entries (`view.len() * 3`). If `out_xyz`
/// is null or `out_len` is too small, no values are copied.
///
/// # Safety
/// `view` must be a valid point view pointer. `out_xyz`, when non-null, must be
/// valid for `out_len` `f64` values.
#[no_mangle]
pub unsafe extern "C" fn pdal_math_point_view_to_xyz(
    view: *const PointView,
    out_xyz: *mut f64,
    out_len: usize,
) -> usize {
    let Some(view) = view.as_ref() else {
        return 0;
    };
    let required = view.len() as usize * 3;
    if out_xyz.is_null() || out_len < required {
        return required;
    }
    let out = std::slice::from_raw_parts_mut(out_xyz, required);
    for point_idx in 0..view.len() {
        let offset = point_idx as usize * 3;
        out[offset] = view.get_f64(point_idx, &DimId::X);
        out[offset + 1] = view.get_f64(point_idx, &DimId::Y);
        out[offset + 2] = view.get_f64(point_idx, &DimId::Z);
    }
    required
}
