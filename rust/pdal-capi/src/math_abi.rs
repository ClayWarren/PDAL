//! C ABI for raster math helpers from `pdal/private/MathUtils`.
//!
//! Exposes the numerical gradient and diamond morphology routines, reusing the
//! Rust implementations that already back the `filters.smrf` port.

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
