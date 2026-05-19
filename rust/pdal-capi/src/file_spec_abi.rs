use crate::error::string_to_c_ptr;
use pdal_core::file_spec::parse_file_spec_json;
use serde_json::json;
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn pdal_file_spec_parse_json(input: *const c_char) -> *mut c_char {
    let input = if input.is_null() {
        "null".to_string()
    } else {
        CStr::from_ptr(input).to_string_lossy().into_owned()
    };

    let output = match parse_file_spec_json(&input) {
        Ok(spec) => json!({
            "ok": true,
            "path": spec.path,
            "headers": spec.headers,
            "query": spec.query,
        }),
        Err(error) => json!({
            "ok": false,
            "error": error,
        }),
    };
    string_to_c_ptr(output.to_string())
}
