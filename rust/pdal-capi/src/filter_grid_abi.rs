use pdal_core::point::PointView;
use pdal_filters::griddecimation;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Get the indices of the kept points in grid decimation.
/// Caller is responsible for freeing the returned buffer with pdal_free_u64_array.
///
/// # Safety
///
/// `view` and `output_type` must be valid.
#[no_mangle]
pub unsafe extern "C" fn pdal_grid_decimation_get_kept_indices(
    view: *const PointView,
    resolution: f64,
    output_type: *const c_char,
    out_len: *mut u64,
) -> *mut u64 {
    if view.is_null() || output_type.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let output_type_str = CStr::from_ptr(output_type).to_string_lossy();
    if let Some(pt_view) = view.as_ref() {
        let kept = griddecimation::get_kept_indices(pt_view, resolution, &output_type_str);
        *out_len = kept.len() as u64;
        let mut boxed_slice = kept.into_boxed_slice();
        let ptr = boxed_slice.as_mut_ptr();
        std::mem::forget(boxed_slice);
        ptr
    } else {
        std::ptr::null_mut()
    }
}

/// Free a u64 array allocated by Rust.
///
/// # Safety
///
/// `ptr` must be a valid pointer returned by a pdal allocator or null.
#[no_mangle]
pub unsafe extern "C" fn pdal_free_u64_array(ptr: *mut u64, len: u64) {
    if !ptr.is_null() {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len as usize));
    }
}
