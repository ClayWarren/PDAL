use crate::error::{set_last_error, string_to_c_ptr};
use pdal_core::options::{option_name_valid, Options};
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

/// Parse a JSON object option-file body into an options set.
///
/// # Safety
///
/// `json` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_from_json_object_text(json: *const c_char) -> *mut Options {
    if json.is_null() {
        set_last_error("null JSON options text");
        return std::ptr::null_mut();
    }
    let json = CStr::from_ptr(json).to_string_lossy();
    let value: serde_json::Value = match serde_json::from_str(&json) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err.to_string());
            return std::ptr::null_mut();
        }
    };
    let Some(object) = value.as_object() else {
        set_last_error("Options JSON must be an object.");
        return std::ptr::null_mut();
    };
    match Options::from_json_object(object) {
        Ok(options) => Box::into_raw(Box::new(options)),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

/// Parse command-line option-file text into an options set.
///
/// # Safety
///
/// `text` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_from_command_line_text(text: *const c_char) -> *mut Options {
    if text.is_null() {
        set_last_error("null command-line options text");
        return std::ptr::null_mut();
    }
    let text = CStr::from_ptr(text).to_string_lossy();
    match Options::from_command_line_text(&text) {
        Ok(options) => Box::into_raw(Box::new(options)),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
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

/// Add a string option only if the key is not already set.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` and `value` must be valid, NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_add_conditional_str(
    ops: *mut Options,
    key: *const c_char,
    value: *const c_char,
) {
    if let (Some(ops), false, false) = (ops.as_mut(), key.is_null(), value.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        let v = CStr::from_ptr(value).to_string_lossy();
        ops.add_conditional(&k, v.to_string());
    }
}

/// Remove every value for an option key.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_remove(ops: *mut Options, key: *const c_char) {
    if let (Some(ops), false) = (ops.as_mut(), key.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        ops.remove(&k);
    }
}

/// Replace every value for an option key with one string value.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` and `value` must be valid, NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_replace_str(
    ops: *mut Options,
    key: *const c_char,
    value: *const c_char,
) {
    if let (Some(ops), false, false) = (ops.as_mut(), key.is_null(), value.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        let v = CStr::from_ptr(value).to_string_lossy();
        ops.replace(&k, v.to_string());
    }
}

/// Return whether an option key exists.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_has(ops: *const Options, key: *const c_char) -> bool {
    if ops.is_null() || key.is_null() {
        return false;
    }
    let ops = &*ops;
    let key = CStr::from_ptr(key).to_string_lossy();
    ops.has(&key)
}

/// Return the number of option entries, including duplicate keys.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_count(ops: *const Options) -> u64 {
    ops.as_ref().map_or(0, |ops| ops.len() as u64)
}

/// Return the option key at a stable sorted-key index.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// The returned string must be freed with `pdal_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_key(ops: *const Options, index: u64) -> *mut c_char {
    let Some(ops) = ops.as_ref() else {
        return std::ptr::null_mut();
    };
    ops.entry(index as usize)
        .map(|(key, _)| string_to_c_ptr(key.to_string()))
        .unwrap_or(std::ptr::null_mut())
}

/// Return the option value at a stable sorted-key entry index.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// The returned string must be freed with `pdal_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_entry_value(ops: *const Options, index: u64) -> *mut c_char {
    let Some(ops) = ops.as_ref() else {
        return std::ptr::null_mut();
    };
    ops.entry(index as usize)
        .map(|(_, value)| string_to_c_ptr(value.to_string()))
        .unwrap_or(std::ptr::null_mut())
}

/// Return an option value by key.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` must be a valid, NUL-terminated C string.
/// The returned string must be freed with `pdal_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_value(
    ops: *const Options,
    key: *const c_char,
) -> *mut c_char {
    if ops.is_null() || key.is_null() {
        return std::ptr::null_mut();
    }
    let ops = &*ops;
    let key = CStr::from_ptr(key).to_string_lossy();
    ops.value(&key)
        .map(|value| string_to_c_ptr(value.to_string()))
        .unwrap_or(std::ptr::null_mut())
}

/// Return `--key=value` arguments as a JSON array.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// The returned string must be freed with `pdal_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_command_line_json(ops: *const Options) -> *mut c_char {
    let Some(ops) = ops.as_ref() else {
        return std::ptr::null_mut();
    };
    match serde_json::to_string(&ops.to_command_line()) {
        Ok(json) => string_to_c_ptr(json),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return whether an option name is valid.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_option_name_valid(name: *const c_char) -> bool {
    if name.is_null() {
        return false;
    }
    let name = CStr::from_ptr(name).to_string_lossy();
    option_name_valid(&name)
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
