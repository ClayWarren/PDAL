use crate::error::string_to_c_ptr;
use pdal_core::config;
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn pdal_config_version_integer(major: i32, minor: i32, patch: i32) -> i32 {
    config::version_integer(major, minor, patch)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_config_full_version_string(
    version: *const c_char,
    sha: *const c_char,
) -> *mut c_char {
    let version = if version.is_null() {
        ""
    } else {
        CStr::from_ptr(version).to_str().unwrap_or("")
    };
    let sha = if sha.is_null() {
        ""
    } else {
        CStr::from_ptr(sha).to_str().unwrap_or("")
    };
    string_to_c_ptr(config::full_version_string(version, sha))
}
