//! C ABI for streaming Zstandard compression.

use crate::deflate_abi::emit_bytes;
use crate::error::set_last_error;
use std::io::Write;
use std::os::raw::c_char;

pub struct ZstdCompressorHandle {
    inner: Option<zstd::stream::write::Encoder<'static, Vec<u8>>>,
}

pub struct ZstdDecompressorHandle {
    inner: Option<zstd::stream::write::Decoder<'static, Vec<u8>>>,
}

unsafe fn input_slice<'a>(buf: *const c_char, len: usize) -> &'a [u8] {
    if buf.is_null() || len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(buf.cast::<u8>(), len)
    }
}

#[no_mangle]
pub extern "C" fn pdal_zstd_compressor_create(level: i32) -> *mut ZstdCompressorHandle {
    match zstd::stream::write::Encoder::new(Vec::new(), level) {
        Ok(inner) => Box::into_raw(Box::new(ZstdCompressorHandle { inner: Some(inner) })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_zstd_compressor_update(
    handle: *mut ZstdCompressorHandle,
    buf: *const c_char,
    len: usize,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null zstd compressor handle");
        return false;
    };
    let Some(inner) = handle.inner.as_mut() else {
        set_last_error("zstd compressor already finished");
        return false;
    };
    if let Err(err) = inner.write_all(input_slice(buf, len)) {
        set_last_error(err.to_string());
        return false;
    }
    emit_bytes(std::mem::take(inner.get_mut()), out_buf, out_len);
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_zstd_compressor_finish(
    handle: *mut ZstdCompressorHandle,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null zstd compressor handle");
        return false;
    };
    let Some(inner) = handle.inner.take() else {
        set_last_error("zstd compressor already finished");
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

#[no_mangle]
pub unsafe extern "C" fn pdal_zstd_compressor_destroy(handle: *mut ZstdCompressorHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub extern "C" fn pdal_zstd_decompressor_create() -> *mut ZstdDecompressorHandle {
    match zstd::stream::write::Decoder::new(Vec::new()) {
        Ok(inner) => Box::into_raw(Box::new(ZstdDecompressorHandle { inner: Some(inner) })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_zstd_decompressor_update(
    handle: *mut ZstdDecompressorHandle,
    buf: *const c_char,
    len: usize,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null zstd decompressor handle");
        return false;
    };
    let Some(inner) = handle.inner.as_mut() else {
        set_last_error("zstd decompressor already finished");
        return false;
    };
    if let Err(err) = inner.write_all(input_slice(buf, len)) {
        set_last_error(err.to_string());
        return false;
    }
    emit_bytes(std::mem::take(inner.get_mut()), out_buf, out_len);
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_zstd_decompressor_finish(
    handle: *mut ZstdDecompressorHandle,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    let Some(handle) = handle.as_mut() else {
        set_last_error("null zstd decompressor handle");
        return false;
    };
    let Some(inner) = handle.inner.take() else {
        set_last_error("zstd decompressor already finished");
        return false;
    };
    let mut inner = inner;
    if let Err(err) = inner.flush() {
        set_last_error(err.to_string());
        return false;
    }
    emit_bytes(inner.into_inner(), out_buf, out_len);
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_zstd_decompressor_destroy(handle: *mut ZstdDecompressorHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}
