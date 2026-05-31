use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use crate::point_abi::pdal_bounds3d_t;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use std::ffi::{c_char, CStr};
use std::path::Path;
use std::rc::Rc;

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
    if return_num < 0
        || return_num >= 5
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
        let byte = *ch as u8;
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_copc_info_t {
    pub center_x: f64,
    pub center_y: f64,
    pub center_z: f64,
    pub halfsize: f64,
    pub spacing: f64,
    pub root_hier_offset: u64,
    pub root_hier_size: u64,
    pub gpstime_minimum: f64,
    pub gpstime_maximum: f64,
    pub reserved: [f64; 11],
}

/// # Safety
/// `data` must point to `data_len` readable bytes and `out_info` must point to
/// writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_copc_info_parse(
    data: *const u8,
    data_len: u64,
    out_info: *mut pdal_copc_info_t,
) -> bool {
    let Some(out_info) = out_info.as_mut() else {
        return false;
    };
    if data.is_null() {
        return false;
    }
    let data = std::slice::from_raw_parts(data, data_len as usize);
    match parse_copc_info(data) {
        Some(info) => {
            *out_info = info;
            true
        }
        None => false,
    }
}

fn parse_copc_info(data: &[u8]) -> Option<pdal_copc_info_t> {
    const COPC_INFO_BYTES: usize = 160;
    if data.len() < COPC_INFO_BYTES {
        return None;
    }

    let mut offset = 0;
    let center_x = read_f64_le(data, &mut offset)?;
    let center_y = read_f64_le(data, &mut offset)?;
    let center_z = read_f64_le(data, &mut offset)?;
    let halfsize = read_f64_le(data, &mut offset)?;
    let spacing = read_f64_le(data, &mut offset)?;
    let root_hier_offset = read_u64_le(data, &mut offset)?;
    let root_hier_size = read_u64_le(data, &mut offset)?;
    let gpstime_minimum = read_f64_le(data, &mut offset)?;
    let gpstime_maximum = read_f64_le(data, &mut offset)?;
    let mut reserved = [0.0; 11];
    for value in &mut reserved {
        *value = read_f64_le(data, &mut offset)?;
    }

    Some(pdal_copc_info_t {
        center_x,
        center_y,
        center_z,
        halfsize,
        spacing,
        root_hier_offset,
        root_hier_size,
        gpstime_minimum,
        gpstime_maximum,
        reserved,
    })
}

fn read_f64_le(data: &[u8], offset: &mut usize) -> Option<f64> {
    let bytes: [u8; 8] = data.get(*offset..*offset + 8)?.try_into().ok()?;
    *offset += 8;
    Some(f64::from_le_bytes(bytes))
}

fn read_u64_le(data: &[u8], offset: &mut usize) -> Option<u64> {
    let bytes: [u8; 8] = data.get(*offset..*offset + 8)?.try_into().ok()?;
    *offset += 8;
    Some(u64::from_le_bytes(bytes))
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct pdal_copc_entry_t {
    pub d: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub offset: u64,
    pub byte_size: i32,
    pub point_count: i32,
}

/// # Safety
/// `data` must point to `data_len` readable bytes. `out_entries` and
/// `out_count` must point to writable memory. On success, free `out_entries`
/// with `pdal_copc_entries_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_copc_hierarchy_parse(
    data: *const u8,
    data_len: u64,
    out_entries: *mut *mut pdal_copc_entry_t,
    out_count: *mut u64,
) -> bool {
    let (Some(out_entries), Some(out_count)) = (out_entries.as_mut(), out_count.as_mut()) else {
        return false;
    };
    *out_entries = std::ptr::null_mut();
    *out_count = 0;
    if data.is_null() && data_len != 0 {
        return false;
    }
    let data = if data_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(data, data_len as usize)
    };
    let entries = match pdal_io::copc_hierarchy::parse_hierarchy_page(data) {
        Ok(entries) => entries,
        Err(err) => {
            set_last_error(err);
            return false;
        }
    };
    let out: Vec<pdal_copc_entry_t> = entries
        .into_iter()
        .map(|entry| pdal_copc_entry_t {
            d: entry.key.level,
            x: entry.key.x,
            y: entry.key.y,
            z: entry.key.z,
            offset: entry.offset,
            byte_size: entry.byte_size,
            point_count: entry.point_count,
        })
        .collect();
    let mut out = out.into_boxed_slice();
    *out_count = out.len() as u64;
    *out_entries = out.as_mut_ptr();
    let _ = Box::into_raw(out);
    true
}

/// # Safety
/// `entries` must be null or a pointer returned by
/// `pdal_copc_hierarchy_parse` with the same `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_copc_entries_free(entries: *mut pdal_copc_entry_t, count: u64) {
    if !entries.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            entries,
            count as usize,
        )));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct pdal_copc_key_t {
    pub d: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_copc_bounds3d_t {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
    pub minz: f64,
    pub maxz: f64,
}

#[no_mangle]
pub unsafe extern "C" fn pdal_copc_key_parse(
    value: *const c_char,
    out_key: *mut pdal_copc_key_t,
) -> bool {
    let (Some(value), Some(out_key)) = (cstr_to_str(value), out_key.as_mut()) else {
        return false;
    };
    let Some(key) = parse_copc_key(value) else {
        return false;
    };
    *out_key = key;
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_copc_key_to_string(key: *const pdal_copc_key_t) -> *mut c_char {
    let Some(key) = key.as_ref() else {
        return string_to_c_ptr(String::new());
    };
    string_to_c_ptr(format!("{}-{}-{}-{}", key.d, key.x, key.y, key.z))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_copc_key_child(
    key: *const pdal_copc_key_t,
    direction: i32,
    out_key: *mut pdal_copc_key_t,
) -> bool {
    let (Some(key), Some(out_key)) = (key.as_ref(), out_key.as_mut()) else {
        return false;
    };
    *out_key = child_copc_key(*key, direction);
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_copc_key_bounds(
    key: *const pdal_copc_key_t,
    root: *const pdal_copc_bounds3d_t,
    out_bounds: *mut pdal_copc_bounds3d_t,
) -> bool {
    let (Some(key), Some(root), Some(out_bounds)) =
        (key.as_ref(), root.as_ref(), out_bounds.as_mut())
    else {
        return false;
    };
    *out_bounds = bounds_for_copc_key(key, root);
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_copc_key_hash(key: *const pdal_copc_key_t) -> u64 {
    let Some(key) = key.as_ref() else {
        return 0;
    };
    let k1 = ((key.d as u32 as u64) << 32) | key.x as u32 as u64;
    let k2 = ((key.y as u32 as u64) << 32) | key.z as u32 as u64;
    k1 ^ k2.rotate_left(1)
}

fn parse_copc_key(value: &str) -> Option<pdal_copc_key_t> {
    let mut parts = value.split('-');
    let d = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let z = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(pdal_copc_key_t { d, x, y, z })
}

fn child_copc_key(key: pdal_copc_key_t, direction: i32) -> pdal_copc_key_t {
    pdal_copc_key_t {
        d: key.d + 1,
        x: (key.x << 1) | (direction & 0x1),
        y: (key.y << 1) | ((direction >> 1) & 0x1),
        z: (key.z << 1) | ((direction >> 2) & 0x1),
    }
}

fn bounds_for_copc_key(key: &pdal_copc_key_t, root: &pdal_copc_bounds3d_t) -> pdal_copc_bounds3d_t {
    let width = 2_f64.powi(key.d);
    let cell_width = (root.maxx - root.minx) / width;
    let max_index = width as i32 - 1;
    pdal_copc_bounds3d_t {
        minx: if key.x == 0 {
            root.minx
        } else {
            root.minx + cell_width * f64::from(key.x)
        },
        maxx: if key.x == max_index {
            root.maxx
        } else {
            root.minx + cell_width * f64::from(key.x + 1)
        },
        miny: if key.y == 0 {
            root.miny
        } else {
            root.miny + cell_width * f64::from(key.y)
        },
        maxy: if key.y == max_index {
            root.maxy
        } else {
            root.miny + cell_width * f64::from(key.y + 1)
        },
        minz: if key.z == 0 {
            root.minz
        } else {
            root.minz + cell_width * f64::from(key.z)
        },
        maxz: if key.z == max_index {
            root.maxz
        } else {
            root.minz + cell_width * f64::from(key.z + 1)
        },
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_ept_key_t {
    pub d: u32,
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub bounds: pdal_bounds3d_t,
}

#[no_mangle]
pub unsafe extern "C" fn pdal_ept_key_parse(
    value: *const c_char,
    out_key: *mut pdal_ept_key_t,
) -> bool {
    let (Some(value), Some(out_key)) = (cstr_to_str(value), out_key.as_mut()) else {
        return false;
    };
    let Some(key) = parse_ept_key(value) else {
        return false;
    };
    *out_key = key;
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_ept_key_to_string(key: *const pdal_ept_key_t) -> *mut c_char {
    let Some(key) = key.as_ref() else {
        return string_to_c_ptr(String::new());
    };
    string_to_c_ptr(format!("{}-{}-{}-{}", key.d, key.x, key.y, key.z))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_ept_key_bisect(
    key: *const pdal_ept_key_t,
    direction: u64,
    out_key: *mut pdal_ept_key_t,
) -> bool {
    let (Some(key), Some(out_key)) = (key.as_ref(), out_key.as_mut()) else {
        return false;
    };
    *out_key = bisect_ept_key(*key, direction);
    true
}

fn parse_ept_key(value: &str) -> Option<pdal_ept_key_t> {
    let mut parts = value.split('-');
    let d = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let z = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(pdal_ept_key_t {
        d,
        x,
        y,
        z,
        bounds: empty_abi_bounds3d(),
    })
}

fn bisect_ept_key(mut key: pdal_ept_key_t, direction: u64) -> pdal_ept_key_t {
    key.d += 1;
    step_ept_key_axis(
        &mut key.x,
        &mut key.bounds.minx,
        &mut key.bounds.maxx,
        direction,
        0,
    );
    step_ept_key_axis(
        &mut key.y,
        &mut key.bounds.miny,
        &mut key.bounds.maxy,
        direction,
        1,
    );
    step_ept_key_axis(
        &mut key.z,
        &mut key.bounds.minz,
        &mut key.bounds.maxz,
        direction,
        2,
    );
    key
}

fn step_ept_key_axis(id: &mut u32, min: &mut f64, max: &mut f64, direction: u64, axis: u8) {
    *id *= 2;
    let mid = *min + ((*max - *min) / 2.0);
    if (direction & (1_u64 << axis)) != 0 {
        *min = mid;
        *id += 1;
    } else {
        *max = mid;
    }
}

fn empty_abi_bounds3d() -> pdal_bounds3d_t {
    pdal_bounds3d_t {
        minx: f64::MAX,
        maxx: f64::MIN,
        miny: f64::MAX,
        maxy: f64::MIN,
        minz: f64::MAX,
        maxz: f64::MIN,
    }
}

unsafe fn cstr_to_str<'a>(value: *const c_char) -> Option<&'a str> {
    if value.is_null() {
        return None;
    }
    CStr::from_ptr(value).to_str().ok()
}

#[repr(C)]
pub struct PointlessLasResult {
    pub point_count: u64,
    pub filename: *mut c_char,
}

/// Create a local pointless LAS copy from a remote/local LAS path.
///
/// # Safety
/// `filename` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pointless_las_create(
    filename: *const c_char,
) -> *mut PointlessLasResult {
    let filename = if filename.is_null() {
        ""
    } else {
        match CStr::from_ptr(filename).to_str() {
            Ok(value) => value,
            Err(err) => {
                set_last_error(err.to_string());
                return std::ptr::null_mut();
            }
        }
    };
    match pdal_io::pointless_las::create(filename) {
        Ok(result) => Box::into_raw(Box::new(PointlessLasResult {
            point_count: result.point_count,
            filename: string_to_c_ptr(result.path.display().to_string()),
        })),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

/// Destroy a result returned by `pdal_pointless_las_create`.
///
/// # Safety
/// `result` must be null or a pointer returned by `pdal_pointless_las_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_pointless_las_destroy(result: *mut PointlessLasResult) {
    if !result.is_null() {
        let result = Box::from_raw(result);
        crate::error::pdal_string_free(result.filename);
    }
}

// ---------------------------------------------------------------------------
// Reader C ABI
// ---------------------------------------------------------------------------

/// Opaque reader handle.
pub struct ReaderHandle {
    pub(crate) reader: Box<dyn pdal_core::pipeline::Reader>,
}

/// Create a FauxReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_faux(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        match pdal_io::faux::FauxReader::new(options) {
            Ok(reader) => Box::into_raw(Box::new(ReaderHandle {
                reader: Box::new(reader),
            })),
            Err(err) => {
                set_last_error(&err);
                std::ptr::null_mut()
            }
        }
    } else {
        std::ptr::null_mut()
    }
}

/// Create a TextReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_text(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::text::TextReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PcdReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_pcd(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::pcd::PcdReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PtsReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_pts(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::pts::PtsReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PtxReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_ptx(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ptx::PtxReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an Ilvis2Reader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_ilvis2(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ilvis2::Ilvis2Reader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an ObjReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_obj(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::obj::ObjReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PlyReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_ply(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ply::PlyReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a QfitReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_qfit(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::qfit::QfitReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SbetReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_sbet(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::sbet::SbetReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SmrmsgReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_smrmsg(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::smrmsg::SmrmsgReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an OptechReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_optech(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::optech::OptechReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a TerrasolidReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_terrasolid(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::terrasolid::TerrasolidReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a TindexReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_tindex(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::tindex::TindexReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an FbiReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_fbi(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::fbi::FbiReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

#[repr(C)]
pub struct pdal_fbi_header_info_t {
    pub version: u32,
    pub header_size: u32,
    pub point_count: u64,
    pub xyz_position: u64,
}

/// Read the FBI header summary needed by the C++ compatibility wrapper.
///
/// # Safety
/// `filename` must be a valid NUL-terminated string and `out_info` must point
/// to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pdal_fbi_header_info(
    filename: *const c_char,
    out_info: *mut pdal_fbi_header_info_t,
) -> i32 {
    if filename.is_null() || out_info.is_null() {
        set_last_error("pdal_fbi_header_info received null input.");
        return -1;
    }

    let path = CStr::from_ptr(filename).to_string_lossy().into_owned();
    match pdal_io::fbi::header_info(Path::new(&path)) {
        Ok(info) => {
            *out_info = pdal_fbi_header_info_t {
                version: info.version,
                header_size: info.hdr_size,
                point_count: info.fast_cnt,
                xyz_position: info.pos_xyz,
            };
            0
        }
        Err(err) => {
            set_last_error(err.to_string());
            -1
        }
    }
}

/// Create a BpfReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_bpf(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::bpf::BpfReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a GdalReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_gdal(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::gdal_reader::GdalReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a LasReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_las(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::las::LasReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Return true when the LAS file at `path` contains a COPC VLR signature.
///
/// # Safety
/// `path` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_detect_copc(path: *const c_char) -> bool {
    if path.is_null() {
        return false;
    }
    let path = CStr::from_ptr(path).to_string_lossy();
    pdal_io::las::detect_copc(Path::new(path.as_ref()))
}

/// Create a LazReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_laz(ops: *const Options) -> *mut ReaderHandle {
    pdal_reader_create_las(ops)
}

/// Create an SpzReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_spz(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::spz::SpzReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a StacReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_stac(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::stac::StacReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a CopcReader full-file read slice from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_copc(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::copc::CopcReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Compute a hierarchy-driven COPC preview: writes the bounds-and-resolution-
/// limited point count and dataset-coordinate bbox into the supplied outputs.
/// Returns 0 on success, -1 on error (last error is set via the standard
/// `pdal_last_error()` channel).
///
/// `out_bounds` receives `[min_x, min_y, min_z, max_x, max_y, max_z]`.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `out_point_count` and `out_bounds` must point to writable storage with
/// space for `u64` and 6 `f64` respectively.
#[no_mangle]
pub unsafe extern "C" fn pdal_copc_preview(
    ops: *const Options,
    out_point_count: *mut u64,
    out_bounds: *mut f64,
) -> i32 {
    let Some(options) = ops.as_ref() else {
        crate::error::set_last_error("pdal_copc_preview: options pointer is null");
        return -1;
    };
    let reader = pdal_io::copc::CopcReader::new(options);
    match reader.preview() {
        Ok(preview) => {
            if !out_point_count.is_null() {
                *out_point_count = preview.point_count;
            }
            if !out_bounds.is_null() {
                let b = preview.bounds;
                let slots = std::slice::from_raw_parts_mut(out_bounds, 6);
                slots[0] = b.min_x;
                slots[1] = b.min_y;
                slots[2] = b.min_z;
                slots[3] = b.max_x;
                slots[4] = b.max_y;
                slots[5] = b.max_z;
            }
            0
        }
        Err(err) => {
            crate::error::set_last_error(&err.0);
            -1
        }
    }
}

/// Create an EptReader local LASzip read slice from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_ept(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ept::EptReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Validate an EPT origin option through the Rust reader implementation.
///
/// # Safety
/// `filename` and `origin` must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_validate_origin(
    filename: *const c_char,
    origin: *const c_char,
) -> bool {
    if filename.is_null() || origin.is_null() {
        set_last_error("Missing EPT origin validation input.");
        return false;
    }
    let filename = CStr::from_ptr(filename).to_string_lossy().into_owned();
    let origin = CStr::from_ptr(origin).to_string_lossy().into_owned();
    let mut options = Options::new();
    options.add("filename", filename);
    options.add("origin", origin);
    match pdal_io::ept::EptReader::new(&options).validate_origin() {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err.0);
            false
        }
    }
}

/// Validate an EPT bounds option through the Rust reader implementation.
///
/// # Safety
/// `filename` and `bounds` must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_validate_bounds(
    filename: *const c_char,
    bounds: *const c_char,
) -> bool {
    if filename.is_null() || bounds.is_null() {
        set_last_error("Missing EPT bounds validation input.");
        return false;
    }
    let filename = CStr::from_ptr(filename).to_string_lossy().into_owned();
    let bounds = CStr::from_ptr(bounds).to_string_lossy().into_owned();
    let mut options = Options::new();
    options.add("filename", filename);
    options.add("bounds", bounds);
    match pdal_io::ept::EptReader::new(&options).validate_bounds() {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err.0);
            false
        }
    }
}

/// Return local STAC preview metadata as JSON.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stac_preview_json(ops: *const Options) -> *mut c_char {
    let Some(options) = ops.as_ref() else {
        set_last_error("Missing STAC preview options.");
        return std::ptr::null_mut();
    };
    match pdal_io::stac::StacReader::new(options).preview() {
        Ok(preview) => string_to_c_ptr(
            serde_json::json!({
                "point_count": preview.point_count,
                "catalog_ids": preview.catalog_ids,
                "collection_ids": preview.collection_ids,
                "item_ids": preview.item_ids,
            })
            .to_string(),
        ),
        Err(err) => {
            set_last_error(err.0);
            std::ptr::null_mut()
        }
    }
}

/// Read the first point view produced by a reader.
///
/// # Safety
/// `reader` must be a valid pointer returned by `pdal_reader_create_*`.
/// The returned view must be freed with `pdal_point_view_destroy`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_read_first(reader: *mut ReaderHandle) -> *mut PointView {
    let Some(reader) = reader.as_mut() else {
        set_last_error("null reader");
        return std::ptr::null_mut();
    };

    match reader.reader.read() {
        Ok(mut views) => {
            clear_last_error();
            views
                .drain(..)
                .next()
                .map(|view| Box::into_raw(Box::new(view)))
                .unwrap_or(std::ptr::null_mut())
        }
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Return a reader's metadata tree. Caller owns the returned pointer.
///
/// # Safety
/// `reader` must be a valid pointer returned by `pdal_reader_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_metadata(reader: *const ReaderHandle) -> *mut MetadataNode {
    let Some(reader) = reader.as_ref() else {
        set_last_error("null reader");
        return std::ptr::null_mut();
    };

    clear_last_error();
    Box::into_raw(Box::new(reader.reader.metadata()))
}

#[repr(C)]
pub struct pdal_memoryview_field_t {
    pub name: *const c_char,
    pub type_id: i32,
    pub offset: u64,
}

fn parse_memoryview_shape(input: &str) -> Result<(u64, u64, u64), String> {
    let values: Vec<&str> = input.split(',').collect();
    if values.len() != 3 {
        return Err(
            "Shape must be specified as three integers: 'depth, rows, columns'.".to_string(),
        );
    }

    fn parse_field(label: &str, value: &str) -> Result<u64, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() || !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("Invalid {label} value in shape: '{trimmed}'."));
        }
        trimmed
            .parse()
            .map_err(|_| format!("Invalid {label} value in shape: '{trimmed}'."))
    }

    let depth = parse_field("depth", values[0])?;
    let rows = parse_field("rows", values[1])?;
    let columns = parse_field("rows", values[2])?;

    Ok((depth, rows, columns))
}

/// Parse a memory-view shape option such as `1, 2, 3`.
///
/// # Safety
///
/// Output pointers must be valid when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_memoryview_shape_parse(
    input: *const c_char,
    out_depth: *mut u64,
    out_rows: *mut u64,
    out_columns: *mut u64,
) -> *mut c_char {
    if input.is_null() {
        return string_to_c_ptr(
            "Shape must be specified as three integers: 'depth, rows, columns'.".to_string(),
        );
    }

    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_memoryview_shape(&input) {
        Ok((depth, rows, columns)) => {
            if let Some(out_depth) = out_depth.as_mut() {
                *out_depth = depth;
            }
            if let Some(out_rows) = out_rows.as_mut() {
                *out_rows = rows;
            }
            if let Some(out_columns) = out_columns.as_mut() {
                *out_columns = columns;
            }
            std::ptr::null_mut()
        }
        Err(err) => string_to_c_ptr(err),
    }
}

pub type MemoryViewIncrementer =
    Option<unsafe extern "C" fn(point_id: u64, user_data: *mut std::ffi::c_void) -> *const u8>;

/// Read a C++ memory-view callback into a Rust-owned point view.
///
/// # Safety
/// `fields` must point to `field_count` valid field descriptors. `incrementer`
/// must return either a valid point base pointer for the requested point or
/// null to end the stream.
#[no_mangle]
pub unsafe extern "C" fn pdal_memoryview_read(
    fields: *const pdal_memoryview_field_t,
    field_count: u64,
    incrementer: MemoryViewIncrementer,
    user_data: *mut std::ffi::c_void,
    depth: u64,
    rows: u64,
    columns: u64,
    column_major: bool,
) -> *mut PointView {
    clear_last_error();
    if fields.is_null() && field_count != 0 {
        set_last_error("null memoryview fields");
        return std::ptr::null_mut();
    }
    let Some(incrementer) = incrementer else {
        set_last_error("null memoryview incrementer");
        return std::ptr::null_mut();
    };

    let raw_fields = std::slice::from_raw_parts(fields, field_count as usize);
    let mut parsed_fields = Vec::with_capacity(raw_fields.len());
    let mut has_x = false;
    let mut has_y = false;
    let mut has_z = false;
    for field in raw_fields {
        if field.name.is_null() {
            set_last_error("null memoryview field name");
            return std::ptr::null_mut();
        }
        let name = CStr::from_ptr(field.name).to_string_lossy().into_owned();
        has_x |= name == "X";
        has_y |= name == "Y";
        has_z |= name == "Z";
        let Some(ty) = dim_type_from_pdal_type(field.type_id) else {
            set_last_error("unsupported memoryview field type");
            return std::ptr::null_mut();
        };
        parsed_fields.push((name, ty, field.offset as usize));
    }

    let has_shape = depth != 0 && rows != 0 && columns != 0;
    let mut layout = PointLayout::new();
    if has_shape && !(has_x && has_y && has_z) {
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
    }
    for (name, ty, _) in &parsed_fields {
        layout.register(DimId::from_name(name), *ty);
    }

    let layout = Rc::new(layout);
    let mut view = PointView::new(layout);
    let mut idx = 0;
    loop {
        let base = incrementer(idx, user_data);
        if base.is_null() {
            break;
        }
        view.add_point();
        for (name, ty, offset) in &parsed_fields {
            let value = read_memoryview_value(base.add(*offset), *ty);
            view.set_f64(idx, &DimId::from_name(name), value);
        }
        if has_shape {
            let (x, y, z) = memoryview_coordinates(idx, depth, rows, columns, column_major);
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }
        idx += 1;
    }

    Box::into_raw(Box::new(view))
}

fn dim_type_from_pdal_type(type_id: i32) -> Option<DimType> {
    match type_id as u32 {
        0x201 => Some(DimType::U8),
        0x202 => Some(DimType::U16),
        0x204 => Some(DimType::U32),
        0x208 => Some(DimType::U64),
        0x101 => Some(DimType::I8),
        0x102 => Some(DimType::I16),
        0x104 => Some(DimType::I32),
        0x108 => Some(DimType::I64),
        0x404 => Some(DimType::F32),
        0x408 => Some(DimType::F64),
        _ => None,
    }
}

unsafe fn read_memoryview_value(ptr: *const u8, ty: DimType) -> f64 {
    match ty {
        DimType::U8 => ptr.read_unaligned() as f64,
        DimType::U16 => (ptr as *const u16).read_unaligned() as f64,
        DimType::U32 => (ptr as *const u32).read_unaligned() as f64,
        DimType::U64 => (ptr as *const u64).read_unaligned() as f64,
        DimType::I8 => (ptr as *const i8).read_unaligned() as f64,
        DimType::I16 => (ptr as *const i16).read_unaligned() as f64,
        DimType::I32 => (ptr as *const i32).read_unaligned() as f64,
        DimType::I64 => (ptr as *const i64).read_unaligned() as f64,
        DimType::F32 => (ptr as *const f32).read_unaligned() as f64,
        DimType::F64 => (ptr as *const f64).read_unaligned(),
    }
}

fn memoryview_coordinates(
    idx: u64,
    depth: u64,
    rows: u64,
    columns: u64,
    column_major: bool,
) -> (f64, f64, f64) {
    let coords = if column_major {
        let x_div = depth * rows;
        let y_div = depth;
        let x_iter = depth * rows * columns;
        let y_iter = depth * rows;
        ((idx % x_iter) / x_div, (idx % y_iter) / y_div, idx % depth)
    } else {
        let y_iter = columns * rows;
        (
            idx % columns,
            (idx % y_iter) / columns,
            idx / (columns * rows) % depth,
        )
    };
    (coords.0 as f64, coords.1 as f64, coords.2 as f64)
}

/// Read an ILVIS2 XML metadata sidecar file.
///
/// # Safety
/// `filename` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn pdal_ilvis2_metadata_read(filename: *const c_char) -> *mut MetadataNode {
    if filename.is_null() {
        set_last_error("null metadata filename");
        return std::ptr::null_mut();
    }

    let path = match CStr::from_ptr(filename).to_str() {
        Ok(path) => path,
        Err(err) => {
            set_last_error(format!("invalid metadata filename: {err}"));
            return std::ptr::null_mut();
        }
    };

    match pdal_io::ilvis2_metadata::read_metadata_file(Path::new(path)) {
        Ok(metadata) => {
            clear_last_error();
            Box::into_raw(Box::new(metadata))
        }
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Destroy a reader handle.
///
/// # Safety
/// `reader` must be a valid pointer returned by `pdal_reader_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_destroy(reader: *mut ReaderHandle) {
    if !reader.is_null() {
        drop(Box::from_raw(reader));
    }
}

// ---------------------------------------------------------------------------
// Writer C ABI
// ---------------------------------------------------------------------------

/// Opaque writer handle.
pub struct WriterHandle {
    pub(crate) writer: Box<dyn pdal_core::pipeline::Writer>,
}

/// Create a NullWriter.
///
/// # Safety
/// `ops` must be a valid pointer (may be null).
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_null(ops: *const Options) -> *mut WriterHandle {
    let options = Options::new();
    let writer = Box::new(pdal_io::nullwriter::NullWriter::new(if ops.is_null() {
        &options
    } else {
        &*ops
    }));
    Box::into_raw(Box::new(WriterHandle { writer }))
}

/// Create an FbiWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_fbi(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::fbi_writer::FbiWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a BpfWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_bpf(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::bpf::BpfWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a TextWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_text(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::text_writer::TextWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PcdWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_pcd(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::pcd::PcdWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PlyWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_ply(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        match pdal_io::ply::PlyWriter::new(options) {
            Ok(writer) => Box::into_raw(Box::new(WriterHandle {
                writer: Box::new(writer),
            })),
            Err(err) => {
                set_last_error(err.to_string());
                std::ptr::null_mut()
            }
        }
    } else {
        std::ptr::null_mut()
    }
}

/// Create a GltfWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_gltf(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::gltf::GltfWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SbetWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_sbet(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::sbet_writer::SbetWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a LasWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_las(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::las_writer::LasWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a LazWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_laz(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::las_writer::LasWriter::new_laz(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a CopcWriter from options. Currently delegates to the Rust LAS/LAZ
/// writer with COPC-required defaults forced (LAS 1.4, LAZ compression, point
/// format 6 if not otherwise set). The resulting file is a LAS 1.4 LAZ that
/// the existing `LasReader` (Rust-backed) can read; explicit COPC structure
/// generation is deferred until a real Rust COPC writer lands.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_copc(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let mut opts = options.clone();
        if !opts.has("minor_version") {
            opts.add("minor_version", "4");
        }
        // Real COPC writer: builds the octree (copc info VLR + hierarchy EVLR +
        // per-node LAZ chunks) via the ported copcwriter subsystem.
        let writer = Box::new(pdal_io::copcwriter::writer::CopcWriter::new(&opts));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SpzWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_spz(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::spz::SpzWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an OgrWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_ogr(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::ogr_writer::OgrWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Opaque handle holding the result of a Rust EPT reader preview.
pub struct EptReaderPreviewHandle {
    pub(crate) preview: pdal_io::ept::EptPreview,
}

/// Read EPT preview metadata (boundsConforming, point count, srs wkt, dim
/// names) from a local `ept.json` file. Returns null on error; call
/// `pdal_last_error` for the message. Caller frees with
/// `pdal_ept_reader_preview_destroy`.
///
/// # Safety
/// `filename` must be a valid NUL-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_create(
    filename: *const c_char,
) -> *mut EptReaderPreviewHandle {
    if filename.is_null() {
        set_last_error("null filename");
        return std::ptr::null_mut();
    }
    let Ok(filename) = CStr::from_ptr(filename).to_str() else {
        set_last_error("non-UTF8 filename");
        return std::ptr::null_mut();
    };
    match pdal_io::ept::read_ept_preview(filename) {
        Ok(preview) => {
            clear_last_error();
            Box::into_raw(Box::new(EptReaderPreviewHandle { preview }))
        }
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Get the preview's point count.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_point_count(
    handle: *const EptReaderPreviewHandle,
) -> u64 {
    handle.as_ref().map_or(0, |h| h.preview.point_count)
}

/// Get the preview's bounds_conforming. Writes into `out` and returns true
/// on success.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`. `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_bounds(
    handle: *const EptReaderPreviewHandle,
    out_minx: *mut f64,
    out_miny: *mut f64,
    out_minz: *mut f64,
    out_maxx: *mut f64,
    out_maxy: *mut f64,
    out_maxz: *mut f64,
) -> bool {
    let Some(handle) = handle.as_ref() else {
        return false;
    };
    let b = &handle.preview.bounds_conforming;
    *out_minx = b.minx;
    *out_miny = b.miny;
    *out_minz = b.minz;
    *out_maxx = b.maxx;
    *out_maxy = b.maxy;
    *out_maxz = b.maxz;
    true
}

/// Get the preview's SRS WKT string. Returns an owned C string (possibly
/// empty). Caller frees with `pdal_string_free`.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_srs_wkt(
    handle: *const EptReaderPreviewHandle,
) -> *mut c_char {
    let Some(handle) = handle.as_ref() else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(handle.preview.srs_wkt.clone())
}

/// Get the number of dim names.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_dim_count(
    handle: *const EptReaderPreviewHandle,
) -> u64 {
    handle
        .as_ref()
        .map_or(0, |h| h.preview.dim_names.len() as u64)
}

/// Get a dim name by index. Returns an owned C string or null when the index
/// is out of range. Caller frees with `pdal_string_free`.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_dim_name(
    handle: *const EptReaderPreviewHandle,
    index: u64,
) -> *mut c_char {
    let Some(handle) = handle.as_ref() else {
        return std::ptr::null_mut();
    };
    let Some(name) = handle.preview.dim_names.get(index as usize) else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(name.clone())
}

/// Destroy an EPT preview handle.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`, or null.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_destroy(handle: *mut EptReaderPreviewHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Validate the OGR writer multicount/attr_dims combination on behalf of the
/// C++ wrapper. Returns null on success, otherwise an owned C string carrying
/// the unprefixed error message. Caller frees with `pdal_string_free`.
#[no_mangle]
pub extern "C" fn pdal_ogr_writer_validate(multicount: u64, attr_dim_count: u64) -> *mut c_char {
    match pdal_io::ogr_writer::validate_multicount_and_attrs(multicount, attr_dim_count) {
        Ok(()) => std::ptr::null_mut(),
        Err(message) => string_to_c_ptr(message),
    }
}

/// Format the "attr_dims dimension not found" error used by the C++ OGR
/// writer wrapper. Returns an owned C string. Caller frees with
/// `pdal_string_free`. Returns null when `name` is null or non-UTF8.
///
/// # Safety
/// `name` must be a valid C string pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pdal_ogr_writer_dim_not_found(name: *const c_char) -> *mut c_char {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(name) = CStr::from_ptr(name).to_str() else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(pdal_io::ogr_writer::format_attr_dim_not_found(name))
}

/// Create a GdalWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_gdal(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::gdal_writer::GdalWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a RasterWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_raster(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::raster_writer::RasterWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Write a point view with a writer.
///
/// # Safety
/// `writer` must be a valid pointer returned by `pdal_writer_create_*`.
/// `view` must be a valid pointer returned by `pdal_point_view_create` or
/// `pdal_reader_read_first`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_write_view(
    writer: *mut WriterHandle,
    view: *const PointView,
) -> bool {
    let (Some(writer), Some(view)) = (writer.as_mut(), view.as_ref()) else {
        set_last_error("null writer or view");
        return false;
    };

    match writer.writer.write(std::slice::from_ref(view)) {
        Ok(()) => {
            clear_last_error();
            true
        }
        Err(err) => {
            set_last_error(err.to_string());
            false
        }
    }
}

/// Write multiple point views with a writer.
///
/// # Safety
/// `writer` must be a valid pointer returned by `pdal_writer_create_*`.
/// `views` must point to `count` valid pointers returned by
/// `pdal_point_view_create` or `pdal_reader_read_first`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_write_views(
    writer: *mut WriterHandle,
    views: *const *const PointView,
    count: u64,
) -> bool {
    let Some(writer) = writer.as_mut() else {
        set_last_error("null writer");
        return false;
    };
    if views.is_null() && count != 0 {
        set_last_error("null views");
        return false;
    }

    let raw_views = std::slice::from_raw_parts(views, count as usize);
    let mut owned_views = Vec::with_capacity(raw_views.len());
    for view in raw_views {
        let Some(view) = view.as_ref() else {
            set_last_error("null view");
            return false;
        };
        owned_views.push(view.clone());
    }

    match writer.writer.write(&owned_views) {
        Ok(()) => {
            clear_last_error();
            true
        }
        Err(err) => {
            set_last_error(err.to_string());
            false
        }
    }
}

/// Destroy a writer handle.
///
/// # Safety
/// `writer` must be a valid pointer returned by `pdal_writer_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_destroy(writer: *mut WriterHandle) {
    if !writer.is_null() {
        drop(Box::from_raw(writer));
    }
}
