use crate::error::{set_last_error, string_to_c_ptr};
use pdal_core::pipeline_reader::parse_pipeline_descriptors;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Parse a pipeline JSON document into a descriptor array (see module docs).
///
/// On success returns a newly-allocated JSON string (free with
/// `pdal_string_free`). On error returns null and sets the last error to the
/// C++-compatible message.
///
/// # Safety
/// `json` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_reader_parse_json(json: *const c_char) -> *mut c_char {
    if json.is_null() {
        set_last_error("Pipeline: null pipeline JSON.");
        return std::ptr::null_mut();
    }
    let json = CStr::from_ptr(json).to_string_lossy().into_owned();
    match parse_pipeline_descriptors(&json) {
        Ok(descriptors) => string_to_c_ptr(descriptors.to_string()),
        Err(message) => {
            set_last_error(message);
            std::ptr::null_mut()
        }
    }
}
