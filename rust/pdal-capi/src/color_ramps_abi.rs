use crate::error::set_last_error;
use std::ffi::{c_char, CStr};

/// Return a built-in colorinterp ramp PNG by name.
///
/// The ramp data lives in `pdal_filters::colorinterp_ramps`; this entry point
/// is the C ABI bridge the C++ `ColorinterpFilter` uses to register a named
/// ramp as a `/vsimem` PNG.
///
/// # Safety
///
/// `name`, `out_data`, and `out_len` must be valid pointers. The returned
/// data pointer is borrowed static storage and must not be freed.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_colorinterp_default_ramp(
    name: *const c_char,
    out_data: *mut *const u8,
    out_len: *mut u64,
) -> bool {
    if name.is_null() || out_data.is_null() || out_len.is_null() {
        set_last_error("null argument to pdal_colorinterp_default_ramp");
        return false;
    }
    let name = CStr::from_ptr(name).to_string_lossy();
    match pdal_filters::colorinterp_ramps::ramp_png(&name) {
        Some(data) => {
            *out_data = data.as_ptr();
            *out_len = data.len() as u64;
            true
        }
        None => false,
    }
}
