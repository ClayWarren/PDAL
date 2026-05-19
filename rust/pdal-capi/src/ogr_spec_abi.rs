use crate::error::string_to_c_ptr;
use pdal_core::ogr_spec::parse_ogr_spec_json;
use serde_json::json;
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn pdal_ogr_spec_parse_json(input: *const c_char) -> *mut c_char {
    let input = if input.is_null() {
        "null".to_string()
    } else {
        CStr::from_ptr(input).to_string_lossy().into_owned()
    };

    let output = match parse_ogr_spec_json(&input) {
        Ok(options) => json!({
            "ok": true,
            "datasource": options.datasource,
            "drivers": options.drivers,
            "openoptions": options.open_options,
            "layer": options.layer,
            "sql": options.sql,
            "dialect": options.dialect,
            "geometry": options.geometry,
        }),
        Err(error) => json!({
            "ok": false,
            "error": error,
        }),
    };
    string_to_c_ptr(output.to_string())
}
