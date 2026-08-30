#![deny(warnings)]

use pdal_capi_macros::ffi_export;
use std::sync::atomic::Ordering;

mod error {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub(crate) static CAUGHT_PANICS: AtomicUsize = AtomicUsize::new(0);

    pub(crate) fn ffi_catch<T>(fallback: T, function: impl FnOnce() -> T) -> T {
        match catch_unwind(AssertUnwindSafe(function)) {
            Ok(value) => value,
            Err(_) => {
                CAUGHT_PANICS.fetch_add(1, Ordering::Relaxed);
                fallback
            }
        }
    }
}

#[ffi_export]
extern "C" fn default_bool_fallback() -> bool {
    panic!("default bool fallback");
}

#[ffi_export]
extern "C" fn default_pointer_fallback() -> *mut u8 {
    panic!("default pointer fallback");
}

#[ffi_export(fallback = u64::MAX)]
extern "C" fn explicit_fallback() -> u64 {
    panic!("explicit fallback");
}

#[ffi_export(fallback = -1)]
extern "C" fn negative_fallback() -> i64 {
    panic!("negative fallback");
}

#[ffi_export(fallback = f64::NAN)]
extern "C" fn nan_fallback() -> f64 {
    panic!("NaN fallback");
}

#[repr(C)]
struct TestXyz {
    x: f64,
    y: f64,
    z: f64,
}

#[ffi_export(fallback = TestXyz {
    x: f64::NAN,
    y: f64::NAN,
    z: f64::NAN,
})]
extern "C" fn struct_fallback() -> TestXyz {
    panic!("struct fallback");
}

#[ffi_export]
unsafe extern "C" fn read_pointer(value: *const i32) -> i32 {
    *value
}

#[test]
fn applies_default_and_explicit_fallbacks() {
    assert!(!default_bool_fallback());
    assert!(default_pointer_fallback().is_null());
    assert_eq!(explicit_fallback(), u64::MAX);
    assert_eq!(negative_fallback(), -1);
    assert!(nan_fallback().is_nan());
    let xyz = struct_fallback();
    assert!(xyz.x.is_nan());
    assert!(xyz.y.is_nan());
    assert!(xyz.z.is_nan());
    assert_eq!(error::CAUGHT_PANICS.load(Ordering::Relaxed), 6);
}

#[test]
fn preserves_unsafe_function_bodies() {
    let value = 42;

    assert_eq!(unsafe { read_pointer(&value) }, value);
}
