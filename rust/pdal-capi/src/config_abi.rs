use crate::error::string_to_c_ptr;
use pdal_core::config;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const PDAL_CAPI_ABI_VERSION_MAJOR: u32 = 0;
pub const PDAL_CAPI_ABI_VERSION_MINOR: u32 = 2;
pub const PDAL_CAPI_ABI_VERSION_PATCH: u32 = 1;
pub const PDAL_CAPI_ABI_VERSION: u32 = (PDAL_CAPI_ABI_VERSION_MAJOR * 1_000_000)
    + (PDAL_CAPI_ABI_VERSION_MINOR * 1_000)
    + PDAL_CAPI_ABI_VERSION_PATCH;

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_capi_abi_version_major() -> u32 {
    PDAL_CAPI_ABI_VERSION_MAJOR
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_capi_abi_version_minor() -> u32 {
    PDAL_CAPI_ABI_VERSION_MINOR
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_capi_abi_version_patch() -> u32 {
    PDAL_CAPI_ABI_VERSION_PATCH
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_capi_abi_version() -> u32 {
    PDAL_CAPI_ABI_VERSION
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_config_version_integer(major: i32, minor: i32, patch: i32) -> i32 {
    config::version_integer(major, minor, patch)
}

#[pdal_capi_macros::ffi_export]
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
