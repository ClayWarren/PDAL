use crate::error::{ffi_catch, string_to_c_ptr};
use std::os::raw::c_char;

/// Return native dependency diagnostics as JSON.
///
/// Caller owns the returned string and must free it with `pdal_string_free`.
#[no_mangle]
pub extern "C" fn pdal_native_dependencies_json() -> *mut c_char {
    ffi_catch(std::ptr::null_mut(), || {
        let dependencies: Vec<_> = pdal_native::built_dependencies()
            .into_iter()
            .map(|dependency| {
                serde_json::json!({
                    "name": dependency.name,
                    "version": dependency.version,
                })
            })
            .collect();
        string_to_c_ptr(serde_json::to_string(&dependencies).unwrap())
    })
}
