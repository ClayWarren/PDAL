use crate::error::{clear_last_error, set_last_error};
use crate::stage_abi::StageWrapper;
use pdal_filters::expression::ExpressionFilter;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Create a `filters.expressionstats` stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C-string.
/// `exprs` must be a valid pointer to a C-array of `count` C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_expressionstats(
    dim_name: *const c_char,
    exprs: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    clear_last_error();
    if dim_name.is_null() || (count > 0 && exprs.is_null()) {
        set_last_error("null expressionstats input");
        return std::ptr::null_mut();
    }
    let dim_name = CStr::from_ptr(dim_name).to_string_lossy().into_owned();
    let mut sources = Vec::with_capacity(count as usize);
    for i in 0..count {
        let ptr = *exprs.offset(i as isize);
        if ptr.is_null() {
            set_last_error("null expression string");
            return std::ptr::null_mut();
        }
        sources.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    match pdal_filters::expressionstats::ExpressionStatsFilter::new(&dim_name, &sources) {
        Ok(f) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(f),
        })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a `filters.mongo` stage from a JSON expression string.
///
/// Returns null and sets the last error if `expr` is null or invalid JSON.
///
/// # Safety
///
/// `expr` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_mongoexpression(
    expr: *const c_char,
) -> *mut StageWrapper {
    clear_last_error();
    if expr.is_null() {
        set_last_error("null expression string");
        return std::ptr::null_mut();
    }
    let json_str = CStr::from_ptr(expr).to_string_lossy();
    match pdal_filters::mongo::MongoExpressionFilter::new(&json_str) {
        Ok(f) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(f),
        })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a `filters.expression` stage from a list of expression strings.
///
/// Returns null and sets the last error if `exprs` is null, contains a null
/// entry, or any expression fails to parse.
///
/// # Safety
///
/// `exprs` must be a valid pointer to a C-array of `count` C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_expression(
    exprs: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    clear_last_error();
    if exprs.is_null() {
        set_last_error("null expression array");
        return std::ptr::null_mut();
    }
    let mut sources = Vec::with_capacity(count as usize);
    for i in 0..count {
        let ptr = *exprs.offset(i as isize);
        if ptr.is_null() {
            set_last_error("null expression string");
            return std::ptr::null_mut();
        }
        sources.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    match ExpressionFilter::new(&sources) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}
