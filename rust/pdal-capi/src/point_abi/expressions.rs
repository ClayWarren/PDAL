use super::*;
use crate::error::set_last_error;
use pdal_core::expr::ConditionalExpression;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Split a point view into points matching and not matching a where expression.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `expression` must be a valid, NUL-terminated C
/// string. `out_keep` and `out_skip` must be valid output pointers and each
/// returned non-null view must be destroyed with `pdal_point_view_destroy`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_split_where(
    view: *const PointView,
    expression: *const c_char,
    out_keep: *mut *mut PointView,
    out_skip: *mut *mut PointView,
) -> bool {
    if view.is_null() || expression.is_null() || out_keep.is_null() || out_skip.is_null() {
        set_last_error("null argument to pdal_point_view_split_where");
        return false;
    }

    let input = &*view;
    let expression = CStr::from_ptr(expression).to_string_lossy();
    let mut where_expr = match ConditionalExpression::parse(&expression) {
        Ok(expr) => expr,
        Err(err) => {
            set_last_error(format!("Invalid 'where': {err}"));
            return false;
        }
    };
    if let Err(err) = where_expr.prepare(input.layout().as_ref()) {
        set_last_error(format!("Invalid 'where': {err}"));
        return false;
    }

    let mut keep = input.make_new();
    let mut skip = input.make_new();
    for idx in 0..input.len() {
        if where_expr.eval(input, idx) {
            keep.append_point(input, idx);
        } else {
            skip.append_point(input, idx);
        }
    }

    *out_keep = Box::into_raw(Box::new(keep));
    *out_skip = Box::into_raw(Box::new(skip));
    true
}

/// Validate a conditional expression against a point layout.
///
/// # Safety
///
/// `expression` must be a valid, NUL-terminated C string. `layout` must be a
/// valid point layout pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_expression_validate_with_layout(
    expression: *const c_char,
    layout: *const PointLayout,
) -> bool {
    if expression.is_null() {
        set_last_error("null expression");
        return false;
    }
    let Some(layout) = layout.as_ref() else {
        set_last_error("null point layout");
        return false;
    };
    let expression = CStr::from_ptr(expression).to_string_lossy();
    let mut expr = match ConditionalExpression::parse(&expression) {
        Ok(expr) => expr,
        Err(err) => {
            set_last_error(format!("Invalid expression: {err}"));
            return false;
        }
    };
    if let Err(err) = expr.prepare(layout) {
        set_last_error(format!("Invalid expression: {err}"));
        return false;
    }
    true
}

/// Evaluate a conditional expression for every point in a view.
///
/// # Safety
///
/// `view` must be valid, `expression` must be a valid NUL-terminated C string,
/// and `out_len` must be a valid output pointer. The returned pointer must be
/// freed with `pdal_u8_array_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_expression_mask(
    view: *const PointView,
    expression: *const c_char,
    out_len: *mut u64,
) -> *mut u8 {
    if view.is_null() || expression.is_null() || out_len.is_null() {
        set_last_error("null argument to pdal_point_view_expression_mask");
        return std::ptr::null_mut();
    }
    let input = &*view;
    let expression = CStr::from_ptr(expression).to_string_lossy();
    let mut expr = match ConditionalExpression::parse(&expression) {
        Ok(expr) => expr,
        Err(err) => {
            set_last_error(format!("Invalid expression: {err}"));
            return std::ptr::null_mut();
        }
    };
    if let Err(err) = expr.prepare(input.layout().as_ref()) {
        set_last_error(format!("Invalid expression: {err}"));
        return std::ptr::null_mut();
    }

    let mut mask: Vec<u8> = (0..input.len())
        .map(|idx| u8::from(expr.eval(input, idx)))
        .collect();
    *out_len = mask.len() as u64;
    let ptr = mask.as_mut_ptr();
    std::mem::forget(mask);
    ptr
}
