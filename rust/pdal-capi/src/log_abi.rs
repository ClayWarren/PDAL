use pdal_core::log;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn pdal_log_level_string(level: i32) -> *const c_char {
    match log::level_string(level) {
        "Error" => b"Error\0".as_ptr().cast(),
        "Warning" => b"Warning\0".as_ptr().cast(),
        "Info" => b"Info\0".as_ptr().cast(),
        _ => b"Debug\0".as_ptr().cast(),
    }
}
