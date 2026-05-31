//! io reader/writer C ABI, split into cohesive submodules to keep each file
//! under ~1k LOC. All items are re-exported here so `io_abi::*` (and the
//! crate-root glob in `lib.rs`) keep their existing paths.

use std::ffi::{c_char, CStr};

mod copc;
mod ept;
mod las;
mod memoryview;
mod readers;
mod writers;

pub use copc::*;
pub use ept::*;
pub use las::*;
pub use memoryview::*;
pub use readers::*;
pub use writers::*;

/// Convert a C string pointer to a borrowed `&str`, or `None` if null/invalid.
/// Shared by the COPC/EPT key parsers.
///
/// # Safety
/// `value` must be null or a valid NUL-terminated C string.
pub(crate) unsafe fn cstr_to_str<'a>(value: *const c_char) -> Option<&'a str> {
    if value.is_null() {
        return None;
    }
    CStr::from_ptr(value).to_str().ok()
}
