//! C ABI for streaming zlib (DEFLATE) compression.
//!
//! Mirrors the streaming `DeflateCompressor` / `DeflateDecompressor` helpers in
//! `pdal/compression/DeflateCompression.cpp`. The handles are Rust-owned; the
//! C++ wrapper drives them and forwards produced bytes to its `BlockCb`.

use crate::error::set_last_error;
use pdal_core::deflate::{AutoDecompressor, DeflateCompressor, DeflateDecompressor};
use std::os::raw::c_char;
use std::ptr;

/// Opaque Rust-owned zlib compressor.
pub struct DeflateCompressorHandle {
    inner: DeflateCompressor,
}

/// Opaque Rust-owned zlib decompressor.
pub struct DeflateDecompressorHandle {
    inner: DecompressorKind,
}

enum DecompressorKind {
    Zlib(DeflateDecompressor),
    Auto(AutoDecompressor),
}

impl DecompressorKind {
    fn update(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Self::Zlib(decompressor) => decompressor.update(input),
            Self::Auto(decompressor) => decompressor.update(input),
        }
    }

    fn finish(&mut self) -> Result<Vec<u8>, String> {
        match self {
            Self::Zlib(decompressor) => decompressor.finish(),
            Self::Auto(decompressor) => decompressor.finish(),
        }
    }
}

/// Move a byte vector into a raw buffer for the C caller.
///
/// An empty vector yields a NULL pointer with `*out_len = 0`. Any non-NULL
/// pointer must be released with `pdal_u8_array_free`.
pub(crate) unsafe fn emit_bytes(bytes: Vec<u8>, out_buf: *mut *mut u8, out_len: *mut usize) {
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    if out_buf.is_null() {
        return;
    }
    if bytes.is_empty() {
        *out_buf = ptr::null_mut();
        return;
    }
    let mut boxed = bytes.into_boxed_slice();
    let data = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    *out_buf = data;
}

unsafe fn input_slice<'a>(buf: *const c_char, len: usize) -> &'a [u8] {
    if buf.is_null() || len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(buf.cast::<u8>(), len)
    }
}

// ---------------------------------------------------------------------------
// Compressor
// ---------------------------------------------------------------------------

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_deflate_compressor_create() -> *mut DeflateCompressorHandle {
    Box::into_raw(Box::new(DeflateCompressorHandle {
        inner: DeflateCompressor::new(),
    }))
}

/// Compress `len` bytes from `buf`. On success writes the compressed output to
/// `out_buf`/`out_len` (a NULL `out_buf` with zero length when nothing was
/// produced yet) and returns `true`. On failure returns `false`.
///
/// # Safety
/// `handle` must come from `pdal_deflate_compressor_create`. `buf` must be
/// valid for `len` bytes. `out_buf`/`out_len` must be writable.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_deflate_compressor_update(
    handle: *mut DeflateCompressorHandle,
    buf: *const c_char,
    len: usize,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null deflate compressor handle");
        return false;
    };
    match handle.inner.update(input_slice(buf, len)) {
        Ok(bytes) => {
            emit_bytes(bytes, out_buf, out_len);
            true
        }
        Err(message) => {
            set_last_error(message);
            false
        }
    }
}

/// Flush and finalize the compressed stream.
///
/// # Safety
/// See [`pdal_deflate_compressor_update`].
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_deflate_compressor_finish(
    handle: *mut DeflateCompressorHandle,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null deflate compressor handle");
        return false;
    };
    match handle.inner.finish() {
        Ok(bytes) => {
            emit_bytes(bytes, out_buf, out_len);
            true
        }
        Err(message) => {
            set_last_error(message);
            false
        }
    }
}

/// Destroy a compressor handle.
///
/// # Safety
/// `handle` must come from `pdal_deflate_compressor_create` and not be reused.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_deflate_compressor_destroy(handle: *mut DeflateCompressorHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

// ---------------------------------------------------------------------------
// Decompressor
// ---------------------------------------------------------------------------

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_deflate_decompressor_create() -> *mut DeflateDecompressorHandle {
    Box::into_raw(Box::new(DeflateDecompressorHandle {
        inner: DecompressorKind::Zlib(DeflateDecompressor::new()),
    }))
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_deflate_auto_decompressor_create() -> *mut DeflateDecompressorHandle {
    Box::into_raw(Box::new(DeflateDecompressorHandle {
        inner: DecompressorKind::Auto(AutoDecompressor::new()),
    }))
}

/// Decompress `len` bytes from `buf`. Mirrors
/// [`pdal_deflate_compressor_update`] for the inflate direction.
///
/// # Safety
/// See [`pdal_deflate_compressor_update`].
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_deflate_decompressor_update(
    handle: *mut DeflateDecompressorHandle,
    buf: *const c_char,
    len: usize,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null deflate decompressor handle");
        return false;
    };
    match handle.inner.update(input_slice(buf, len)) {
        Ok(bytes) => {
            emit_bytes(bytes, out_buf, out_len);
            true
        }
        Err(message) => {
            set_last_error(message);
            false
        }
    }
}

/// Finalize the decompressed stream.
///
/// # Safety
/// See [`pdal_deflate_compressor_update`].
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_deflate_decompressor_finish(
    handle: *mut DeflateDecompressorHandle,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null deflate decompressor handle");
        return false;
    };
    match handle.inner.finish() {
        Ok(bytes) => {
            emit_bytes(bytes, out_buf, out_len);
            true
        }
        Err(message) => {
            set_last_error(message);
            false
        }
    }
}

/// Destroy a decompressor handle.
///
/// # Safety
/// `handle` must come from `pdal_deflate_decompressor_create` and not be reused.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_deflate_decompressor_destroy(handle: *mut DeflateDecompressorHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}
