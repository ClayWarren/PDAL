use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

thread_local! {
    static LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("").expect("empty CString is valid"));
}

pub(crate) fn set_last_error(message: impl Into<String>) {
    let sanitized = message.into().replace('\0', "\\0");
    let Ok(message) = CString::new(sanitized) else {
        return;
    };
    LAST_ERROR.with(|slot| {
        if let Ok(mut current) = slot.try_borrow_mut() {
            *current = message;
        }
    });
}

pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|slot| {
        if let Ok(mut current) = slot.try_borrow_mut() {
            *current = CString::default();
        }
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
        .unwrap_or_default()
        .into_raw()
}

/// Copy a C string into the canonical Rust-owned string allocation domain.
///
/// This symbol is an internal bridge for the C++ CLI introspection helpers.
/// Public callers receive the resulting pointer through a `pdal_*` function
/// and release it with `pdal_string_free`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn rust_capi_string_copy(value: *const c_char) -> *mut c_char {
    ffi_catch(std::ptr::null_mut(), || {
        if value.is_null() {
            return std::ptr::null_mut();
        }
        CStr::from_ptr(value).to_owned().into_raw()
    })
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.try_borrow()
            .map(|message| message.as_ptr())
            .unwrap_or(c"".as_ptr())
    })
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_clear_error() {
    clear_last_error();
}

/// Free a string returned by this C ABI.
///
/// # Safety
///
/// `ptr` must be a string pointer returned by this library, or null. Must not
/// be called twice on the same pointer.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_catch_returns_fallback_and_records_panic() {
        clear_last_error();

        let result = ffi_catch(17, || panic!("C ABI panic probe"));

        assert_eq!(result, 17);
        let message = unsafe { CStr::from_ptr(pdal_last_error()) };
        assert_eq!(message.to_string_lossy(), "C ABI panic probe");
    }

    #[test]
    fn string_copy_preserves_non_utf8_bytes() {
        let source_bytes = [0xff, b'v', b'a', b'l', b'u', b'e', 0];
        let source = CStr::from_bytes_with_nul(&source_bytes).expect("valid C string");

        let copy = unsafe { rust_capi_string_copy(source.as_ptr()) };

        assert!(!copy.is_null());
        assert_eq!(unsafe { CStr::from_ptr(copy) }.to_bytes(), b"\xffvalue");
        unsafe { pdal_string_free(copy) };
    }
}
