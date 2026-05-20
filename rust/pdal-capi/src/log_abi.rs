use pdal_core::log;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn pdal_log_level_string(level: i32) -> *const c_char {
    match log::level_string(level) {
        "Error" => c"Error".as_ptr(),
        "Warning" => c"Warning".as_ptr(),
        "Info" => c"Info".as_ptr(),
        _ => c"Debug".as_ptr(),
    }
}
