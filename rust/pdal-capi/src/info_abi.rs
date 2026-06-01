use crate::error::{set_last_error, string_to_c_ptr};
use pdal_core::info::point_view_info_summary_json;
use pdal_core::point::PointView;
use std::ffi::{c_char, CStr};

/// Build a filters.info-style summary over a point view.
///
/// # Safety
/// `view` must be null or a valid pointer returned by this C ABI. `point_spec`
/// and `query_spec` must be null or valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_info_summary_json(
    view: *const PointView,
    point_spec: *const c_char,
    query_spec: *const c_char,
) -> *mut c_char {
    let Some(view) = view.as_ref() else {
        set_last_error("pdal_info_summary_json received a null point view.");
        return std::ptr::null_mut();
    };

    let point_spec = opt_cstr(point_spec);
    let query_spec = opt_cstr(query_spec);
    match point_view_info_summary_json(view, point_spec.as_deref(), query_spec.as_deref()) {
        Ok(text) => string_to_c_ptr(text),
        Err(err) => {
            set_last_error(&err);
            std::ptr::null_mut()
        }
    }
}

unsafe fn opt_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}
