use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

thread_local! {
    static LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("").expect("empty CString is valid"));
}

pub(crate) fn set_last_error(message: impl Into<String>) {
    let sanitized = message.into().replace('\0', "\\0");
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = CString::new(sanitized).expect("interior NULs removed");
    });
}

pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = CString::new("").expect("empty CString is valid");
    });
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "Rust panic".to_string()
    }
}

pub(crate) fn ffi_catch<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            set_last_error(panic_message(payload.as_ref()));
            fallback
        }
    }
}

pub(crate) fn string_to_c_ptr(value: String) -> *mut c_char {
    CString::new(value.replace('\0', "\\0"))
        .expect("interior NULs removed")
        .into_raw()
}

#[no_mangle]
pub extern "C" fn pdal_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn pdal_clear_error() {
    clear_last_error();
}

/// Free a string returned by this C ABI.
///
/// # Safety
///
/// `ptr` must be a string pointer returned by this library, or null. Must not
/// be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}
