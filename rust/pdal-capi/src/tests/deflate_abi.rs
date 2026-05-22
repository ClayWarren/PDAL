use super::*;
use std::os::raw::c_char;
use std::ptr;

unsafe fn drain(out_buf: *mut u8, out_len: usize) -> Vec<u8> {
    if out_buf.is_null() || out_len == 0 {
        return Vec::new();
    }
    let bytes = std::slice::from_raw_parts(out_buf, out_len).to_vec();
    pdal_u8_array_free(out_buf, out_len as u64);
    bytes
}

unsafe fn compress(data: &[u8]) -> Vec<u8> {
    let handle = pdal_deflate_compressor_create();
    assert!(!handle.is_null());

    let mut out_buf: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let mut compressed = Vec::new();

    assert!(pdal_deflate_compressor_update(
        handle,
        data.as_ptr() as *const c_char,
        data.len(),
        &mut out_buf,
        &mut out_len,
    ));
    compressed.extend(drain(out_buf, out_len));

    assert!(pdal_deflate_compressor_finish(
        handle,
        &mut out_buf,
        &mut out_len
    ));
    compressed.extend(drain(out_buf, out_len));

    pdal_deflate_compressor_destroy(handle);
    compressed
}

unsafe fn decompress(data: &[u8]) -> Vec<u8> {
    let handle = pdal_deflate_decompressor_create();
    assert!(!handle.is_null());

    let mut out_buf: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let mut decoded = Vec::new();

    assert!(pdal_deflate_decompressor_update(
        handle,
        data.as_ptr() as *const c_char,
        data.len(),
        &mut out_buf,
        &mut out_len,
    ));
    decoded.extend(drain(out_buf, out_len));

    assert!(pdal_deflate_decompressor_finish(
        handle,
        &mut out_buf,
        &mut out_len
    ));
    decoded.extend(drain(out_buf, out_len));

    pdal_deflate_decompressor_destroy(handle);
    decoded
}

#[test]
fn deflate_abi_round_trips_data() {
    unsafe {
        let data: Vec<u8> = (0..50_000u32)
            .map(|i| i.wrapping_mul(2_654_435_761) as u8)
            .collect();
        let compressed = compress(&data);
        assert!(!compressed.is_empty());
        assert_eq!(decompress(&compressed), data);
    }
}

#[test]
fn deflate_abi_round_trips_empty_input() {
    unsafe {
        let compressed = compress(&[]);
        assert_eq!(decompress(&compressed), Vec::<u8>::new());
    }
}

#[test]
fn deflate_abi_rejects_null_handle_and_garbage() {
    unsafe {
        let mut out_buf: *mut u8 = ptr::null_mut();
        let mut out_len: usize = 0;

        assert!(!pdal_deflate_compressor_update(
            ptr::null_mut(),
            ptr::null(),
            0,
            &mut out_buf,
            &mut out_len,
        ));
        assert!(!pdal_deflate_decompressor_finish(
            ptr::null_mut(),
            &mut out_buf,
            &mut out_len,
        ));

        // Garbage input cannot be inflated.
        let handle = pdal_deflate_decompressor_create();
        let garbage = [0xdeu8, 0xad, 0xbe, 0xef, 0x99, 0x12];
        let update_ok = pdal_deflate_decompressor_update(
            handle,
            garbage.as_ptr() as *const c_char,
            garbage.len(),
            &mut out_buf,
            &mut out_len,
        );
        let finish_ok = pdal_deflate_decompressor_finish(handle, &mut out_buf, &mut out_len);
        assert!(!(update_ok && finish_ok));
        pdal_deflate_decompressor_destroy(handle);
    }
}
