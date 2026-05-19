use pdal_core::utils::looks_like_json;
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_is_json(value: *const c_char) -> bool {
    if value.is_null() {
        return false;
    }
    looks_like_json(&CStr::from_ptr(value).to_string_lossy())
}
