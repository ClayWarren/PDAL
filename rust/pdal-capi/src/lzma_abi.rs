//! C ABI for streaming LZMA compression.

use crate::deflate_abi::emit_bytes;
use crate::error::set_last_error;
use std::io::Write;
use std::os::raw::c_char;

pub struct LzmaCompressorHandle {
    inner: Option<xz2::write::XzEncoder<Vec<u8>>>,
}

pub struct LzmaDecompressorHandle {
    inner: Option<xz2::write::XzDecoder<Vec<u8>>>,
}

unsafe fn input_slice<'a>(buf: *const c_char, len: usize) -> &'a [u8] {
    if buf.is_null() || len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(buf.cast::<u8>(), len)
    }
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_lzma_compressor_create() -> *mut LzmaCompressorHandle {
    Box::into_raw(Box::new(LzmaCompressorHandle {
        inner: Some(xz2::write::XzEncoder::new(Vec::new(), 2)),
    }))
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_lzma_compressor_update(
    handle: *mut LzmaCompressorHandle,
    buf: *const c_char,
    len: usize,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null lzma compressor handle");
        return false;
    };
    let Some(inner) = handle.inner.as_mut() else {
        set_last_error("lzma compressor already finished");
        return false;
    };
    if let Err(err) = inner.write_all(input_slice(buf, len)) {
        set_last_error(err.to_string());
        return false;
    }
    emit_bytes(std::mem::take(inner.get_mut()), out_buf, out_len);
    true
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_lzma_compressor_finish(
    handle: *mut LzmaCompressorHandle,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null lzma compressor handle");
        return false;
    };
    let Some(inner) = handle.inner.take() else {
        set_last_error("lzma compressor already finished");
        return false;
    };
    match inner.finish() {
        Ok(bytes) => {
            emit_bytes(bytes, out_buf, out_len);
            true
        }
        Err(err) => {
            set_last_error(err.to_string());
            false
        }
    }
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_lzma_compressor_destroy(handle: *mut LzmaCompressorHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_lzma_decompressor_create() -> *mut LzmaDecompressorHandle {
    Box::into_raw(Box::new(LzmaDecompressorHandle {
        inner: Some(xz2::write::XzDecoder::new(Vec::new())),
    }))
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_lzma_decompressor_update(
    handle: *mut LzmaDecompressorHandle,
    buf: *const c_char,
    len: usize,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null lzma decompressor handle");
        return false;
    };
    let Some(inner) = handle.inner.as_mut() else {
        set_last_error("lzma decompressor already finished");
        return false;
    };
    if let Err(err) = inner.write_all(input_slice(buf, len)) {
        set_last_error(err.to_string());
        return false;
    }
    emit_bytes(std::mem::take(inner.get_mut()), out_buf, out_len);
    true
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_lzma_decompressor_finish(
    handle: *mut LzmaDecompressorHandle,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null lzma decompressor handle");
        return false;
    };
    let Some(mut inner) = handle.inner.take() else {
        set_last_error("lzma decompressor already finished");
        return false;
    };
    match inner.finish() {
        Ok(bytes) => {
            emit_bytes(bytes, out_buf, out_len);
            true
        }
        Err(err) => {
            set_last_error(err.to_string());
            false
        }
    }
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_lzma_decompressor_destroy(handle: *mut LzmaDecompressorHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}
