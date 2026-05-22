//! C ABI for writer filename-template helpers.
//!
//! Mirrors the static helpers in `pdal/Writer.cpp` so the `FlexWriter`
//! filename `#` placeholder and `#uuid#` tag behavior is owned by Rust.

use crate::error::{set_last_error, string_to_c_ptr};
use pdal_core::writer::{handle_filename_template, replace_filename_tags, FilenameTemplate};
use std::ffi::{c_char, CStr};
use std::ptr;

/// `out_pos` sentinel meaning the filename has no `#` placeholder.
///
/// This matches `std::string::npos` on the C++ side, so the wrapper can
/// return the value directly as a `std::string::size_type`.
pub const PDAL_WRITER_NO_TEMPLATE: usize = usize::MAX;

/// Validate a writer filename template and locate the `#` placeholder.
///
/// On success returns `true` and writes the placeholder byte offset to
/// `out_pos`, or [`PDAL_WRITER_NO_TEMPLATE`] when no placeholder is present.
/// On validation failure returns `false` and records the reason, retrievable
/// with `pdal_last_error`.
///
/// # Safety
/// `filename` must be a valid NUL-terminated C string. `out_pos` must be a
/// valid, writable pointer to a `usize`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_handle_filename_template(
    filename: *const c_char,
    out_pos: *mut usize,
) -> bool {
    if filename.is_null() || out_pos.is_null() {
        return false;
    }
    let filename = CStr::from_ptr(filename).to_string_lossy();
    match handle_filename_template(&filename) {
        Ok(FilenameTemplate::NoPlaceholder) => {
            *out_pos = PDAL_WRITER_NO_TEMPLATE;
            true
        }
        Ok(FilenameTemplate::Placeholder(pos)) => {
            *out_pos = pos;
            true
        }
        Err(message) => {
            set_last_error(message);
            false
        }
    }
}

/// Replace each `#uuid#` tag in `filename` with a fresh lowercase UUID.
///
/// Returns a newly allocated string (free with `pdal_string_free`), or NULL
/// on failure with the reason available from `pdal_last_error`.
///
/// # Safety
/// `filename` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_replace_tags(filename: *const c_char) -> *mut c_char {
    if filename.is_null() {
        return ptr::null_mut();
    }
    let filename = CStr::from_ptr(filename).to_string_lossy();
    match replace_filename_tags(&filename) {
        Ok(result) => string_to_c_ptr(result),
        Err(message) => {
            set_last_error(message);
            ptr::null_mut()
        }
    }
}
