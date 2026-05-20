use crate::error::string_to_c_ptr;
use pdal_core::uuid;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

#[no_mangle]
pub unsafe extern "C" fn pdal_uuid_parse(input: *const c_char, out_bytes: *mut u8) -> bool {
    if input.is_null() || out_bytes.is_null() {
        return false;
    }
    let input = CStr::from_ptr(input).to_string_lossy();
    let Some(bytes) = uuid::parse_uuid(&input) else {
        return false;
    };
    ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, bytes.len());
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_uuid_unparse(bytes: *const u8) -> *mut c_char {
    if bytes.is_null() {
        return ptr::null_mut();
    }
    let bytes = std::slice::from_raw_parts(bytes, 16);
    let Ok(bytes) = <[u8; 16]>::try_from(bytes) else {
        return ptr::null_mut();
    };
    string_to_c_ptr(uuid::unparse_uuid(&bytes))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_uuid_random(out_bytes: *mut u8) -> bool {
    if out_bytes.is_null() {
        return false;
    }
    let Ok(bytes) = uuid::random_v4_uuid_bytes() else {
        return false;
    };
    ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, bytes.len());
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_uuid_is_null(bytes: *const u8) -> bool {
    if bytes.is_null() {
        return true;
    }
    let bytes = std::slice::from_raw_parts(bytes, 16);
    let Ok(bytes) = <[u8; 16]>::try_from(bytes) else {
        return true;
    };
    uuid::is_null_uuid(&bytes)
}
