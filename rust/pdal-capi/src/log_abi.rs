use crate::error::string_to_c_ptr;
use pdal_core::log;
use std::ffi::CStr;
use std::os::raw::c_char;

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_log_level_string(level: i32) -> *const c_char {
    match log::level_string(level) {
        "Error" => c"Error".as_ptr(),
        "Warning" => c"Warning".as_ptr(),
        "Info" => c"Info".as_ptr(),
        _ => c"Debug".as_ptr(),
    }
}

/// Format the line-leader prefix that `pdal::Log::get(level)` emits, e.g.
/// `"(PDAL Debug) "` or `"(PDAL Debug 0.123) "`.
///
/// `leader` may be null and is treated as an empty string. The caller takes
/// ownership of the returned C string and must release it via
/// `pdal_string_free`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_log_format_prefix(
    leader: *const c_char,
    level: i32,
    timing: bool,
    elapsed_seconds: f64,
) -> *mut c_char {
    let leader = if leader.is_null() {
        String::new()
    } else {
        CStr::from_ptr(leader).to_string_lossy().into_owned()
    };
    let prefix = log::format_prefix(&leader, level, timing, elapsed_seconds);
    string_to_c_ptr(prefix)
}

/// Format the "Command 'X' not recognized" message the `pdal` app emits when
/// dispatch falls through to an unknown command.
///
/// Returns an owned C string that must be released via `pdal_string_free`.
/// A null or empty `command` produces an empty quoted name.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_app_unknown_command_message(command: *const c_char) -> *mut c_char {
    let command = if command.is_null() {
        String::new()
    } else {
        CStr::from_ptr(command).to_string_lossy().into_owned()
    };
    string_to_c_ptr(format!("Command '{}' not recognized", command))
}
