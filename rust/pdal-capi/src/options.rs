use pdal_core::options::Options;
use std::ffi::CStr;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Options ABI
// ---------------------------------------------------------------------------

/// Create a new, empty options set. Returns an owned pointer.
#[no_mangle]
pub extern "C" fn pdal_options_create() -> *mut Options {
    Box::into_raw(Box::new(Options::new()))
}

/// Add a floating-point option.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_add_f64(ops: *mut Options, key: *const c_char, value: f64) {
    if let (Some(ops), false) = (ops.as_mut(), key.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        ops.add(&k, value.to_string());
    }
}

/// Add an unsigned 64-bit integer option.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_add_u64(ops: *mut Options, key: *const c_char, value: u64) {
    if let (Some(ops), false) = (ops.as_mut(), key.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        ops.add(&k, value.to_string());
    }
}

/// Add a string option.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` and `value` must be valid, NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_add_str(
    ops: *mut Options,
    key: *const c_char,
    value: *const c_char,
) {
    if let (Some(ops), false, false) = (ops.as_mut(), key.is_null(), value.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        let v = CStr::from_ptr(value).to_string_lossy();
        ops.add(&k, v.to_string());
    }
}

/// Destroy an options set.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`, or null.
/// Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_destroy(ops: *mut Options) {
    if !ops.is_null() {
        drop(Box::from_raw(ops));
    }
}
