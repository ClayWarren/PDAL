use crate::error::{set_last_error, string_to_c_ptr};
use crate::point_abi::pdal_bounds3d_t;
use std::ffi::c_char;

pub struct LasSummaryHandle {
    summary: pdal_io::las_summary::LasSummary,
}

#[no_mangle]
pub extern "C" fn pdal_las_summary_create() -> *mut LasSummaryHandle {
    Box::into_raw(Box::new(LasSummaryHandle {
        summary: pdal_io::las_summary::LasSummary::default(),
    }))
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_destroy(summary: *mut LasSummaryHandle) {
    if !summary.is_null() {
        drop(Box::from_raw(summary));
    }
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_clear(summary: *mut LasSummaryHandle) {
    if let Some(summary) = summary.as_mut() {
        summary.summary.clear();
    }
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_add_point(
    summary: *mut LasSummaryHandle,
    x: f64,
    y: f64,
    z: f64,
    return_number: i32,
) {
    if let Some(summary) = summary.as_mut() {
        summary.summary.add_point(x, y, z, return_number);
    }
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_total_num_points(
    summary: *const LasSummaryHandle,
) -> u64 {
    summary
        .as_ref()
        .map(|summary| summary.summary.total_num_points())
        .unwrap_or(0)
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_return_count(
    summary: *const LasSummaryHandle,
    return_number: u64,
) -> u64 {
    let Some(summary) = summary.as_ref() else {
        return 0;
    };
    summary.summary.return_count(return_number as usize)
}

/// # Safety
/// `summary` must be a pointer returned by `pdal_las_summary_create` and
/// `out_bounds` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_bounds(
    summary: *const LasSummaryHandle,
    out_bounds: *mut pdal_bounds3d_t,
) {
    if let (Some(summary), Some(out_bounds)) = (summary.as_ref(), out_bounds.as_mut()) {
        let bounds = summary.summary.bounds();
        *out_bounds = pdal_bounds3d_t {
            minx: bounds.minx,
            maxx: bounds.maxx,
            miny: bounds.miny,
            maxy: bounds.maxy,
            minz: bounds.minz,
            maxz: bounds.maxz,
        };
    }
}

#[no_mangle]
pub extern "C" fn pdal_las_base_count(format: i32) -> i32 {
    match format & 0x0f {
        0 => 20,
        1 => 28,
        2 => 26,
        3 => 34,
        6 => 30,
        7 => 36,
        8 => 38,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdal_las_point_format_supported(format: i32) -> bool {
    matches!(format, 0 | 1 | 2 | 3 | 6 | 7 | 8)
}

#[no_mangle]
pub extern "C" fn pdal_las_legacy_point_count(
    point_count: u64,
    version_minor: u8,
    point_format: i32,
) -> u32 {
    if should_mirror_las_legacy_count(point_count, version_minor, point_format) {
        point_count as u32
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pdal_las_legacy_points_by_return(
    point_count: u64,
    return_num: i32,
    version_minor: u8,
    point_format: i32,
) -> u32 {
    if !(0..5).contains(&return_num)
        || !should_mirror_las_legacy_count(point_count, version_minor, point_format)
    {
        0
    } else {
        point_count as u32
    }
}

fn should_mirror_las_legacy_count(point_count: u64, version_minor: u8, point_format: i32) -> bool {
    point_count <= u64::from(u32::MAX) && !(version_minor >= 4 && point_format >= 6)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct pdal_las_vlr_header_t {
    pub record_sig: u16,
    pub user_id: [c_char; 17],
    pub record_id: u16,
    pub data_size: u64,
    pub description: [c_char; 33],
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_vlr_header_parse(
    data: *const u8,
    data_len: u64,
    evlr: bool,
    out_header: *mut pdal_las_vlr_header_t,
) -> bool {
    let Some(out_header) = out_header.as_mut() else {
        return false;
    };
    let header_len: usize = if evlr { 60 } else { 54 };
    if data.is_null() || data_len < header_len as u64 {
        return false;
    }
    let data = std::slice::from_raw_parts(data, data_len as usize);
    let record_sig = u16::from_le_bytes([data[0], data[1]]);
    let user_id = fixed_c_string::<17>(&data[2..18]);
    let record_id = u16::from_le_bytes([data[18], data[19]]);
    let data_size = if evlr {
        u64::from_le_bytes(data[20..28].try_into().expect("slice length checked"))
    } else {
        u16::from_le_bytes([data[20], data[21]]) as u64
    };
    let description_offset = if evlr { 28 } else { 22 };
    let description = fixed_c_string::<33>(&data[description_offset..description_offset + 32]);
    *out_header = pdal_las_vlr_header_t {
        record_sig,
        user_id,
        record_id,
        data_size,
        description,
    };
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_vlr_header_write(
    header: *const pdal_las_vlr_header_t,
    evlr: bool,
    out_data: *mut u8,
    out_len: u64,
) -> bool {
    let Some(header) = header.as_ref() else {
        return false;
    };
    let header_len: usize = if evlr { 60 } else { 54 };
    if out_data.is_null() || out_len < header_len as u64 {
        return false;
    }
    if !evlr && header.data_size > u64::from(u16::MAX) {
        return false;
    }
    let out = std::slice::from_raw_parts_mut(out_data, out_len as usize);
    out[..header_len].fill(0);
    out[0..2].copy_from_slice(&header.record_sig.to_le_bytes());
    write_fixed_c_string(&header.user_id, &mut out[2..18]);
    out[18..20].copy_from_slice(&header.record_id.to_le_bytes());
    if evlr {
        out[20..28].copy_from_slice(&header.data_size.to_le_bytes());
        write_fixed_c_string(&header.description, &mut out[28..60]);
    } else {
        out[20..22].copy_from_slice(&(header.data_size as u16).to_le_bytes());
        write_fixed_c_string(&header.description, &mut out[22..54]);
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_vlr_text(data: *const u8, data_len: u64) -> *mut c_char {
    if data.is_null() && data_len != 0 {
        return string_to_c_ptr(String::new());
    }
    let data = if data_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(data, data_len as usize)
    };
    let end = data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(data.len());
    string_to_c_ptr(String::from_utf8_lossy(&data[..end]).into_owned())
}

fn fixed_c_string<const N: usize>(bytes: &[u8]) -> [c_char; N] {
    let mut out = [0 as c_char; N];
    let copy_len = bytes.len().min(N.saturating_sub(1));
    for (dst, src) in out.iter_mut().zip(bytes.iter()).take(copy_len) {
        if *src == 0 {
            break;
        }
        *dst = *src as c_char;
    }
    out
}

fn write_fixed_c_string(src: &[c_char], dst: &mut [u8]) {
    for (out, ch) in dst.iter_mut().zip(src.iter()) {
        let byte = ch.to_ne_bytes()[0];
        if byte == 0 {
            break;
        }
        *out = byte;
    }
}

pub struct LasTileHandle {
    chunk: u32,
    data: Vec<u8>,
    pos: usize,
}

#[no_mangle]
pub extern "C" fn pdal_las_tile_create(chunk: u32, size: u64) -> *mut LasTileHandle {
    let Ok(size) = usize::try_from(size) else {
        set_last_error("LAS tile size exceeds platform capacity.");
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(LasTileHandle {
        chunk,
        data: vec![0; size],
        pos: 0,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_tile_destroy(tile: *mut LasTileHandle) {
    if !tile.is_null() {
        drop(Box::from_raw(tile));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_tile_data_const(tile: *const LasTileHandle) -> *const c_char {
    tile.as_ref()
        .map(|tile| tile.data.as_ptr().cast::<c_char>())
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_tile_data(tile: *mut LasTileHandle) -> *mut c_char {
    tile.as_mut()
        .map(|tile| tile.data.as_mut_ptr().cast::<c_char>())
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_tile_size(tile: *const LasTileHandle) -> u64 {
    tile.as_ref()
        .map(|tile| tile.data.len() as u64)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_tile_pos(tile: *const LasTileHandle) -> *const c_char {
    let Some(tile) = tile.as_ref() else {
        return std::ptr::null();
    };
    if tile.pos >= tile.data.len() {
        return std::ptr::null();
    }
    tile.data[tile.pos..].as_ptr().cast::<c_char>()
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_tile_chunk(tile: *const LasTileHandle) -> u32 {
    tile.as_ref().map(|tile| tile.chunk).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_las_tile_advance(tile: *mut LasTileHandle, point_size: i32) -> bool {
    let Some(tile) = tile.as_mut() else {
        return false;
    };
    if point_size < 0 {
        return false;
    }
    tile.pos = tile.pos.saturating_add(point_size as usize);
    tile.pos < tile.data.len()
}
