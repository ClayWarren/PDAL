use crate::error::string_to_c_ptr;
use pdal_core::xml_schema::remap_old_dimension_name;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Remap legacy PDAL XML schema dimension names. Caller must free with
/// `pdal_string_free`.
///
/// # Safety
///
/// `name` must be null or a valid NUL-terminated C string.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_xml_schema_remap_old_name(name: *const c_char) -> *mut c_char {
    string_to_c_ptr(remap_old_dimension_name(&c_string_lossy(name)))
}

unsafe fn c_string_lossy(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}
