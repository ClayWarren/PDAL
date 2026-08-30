use super::cstr_to_str;
use crate::error::{set_last_error, string_to_c_ptr};
use std::ffi::c_char;

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
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export]
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

#[pdal_capi_macros::ffi_export]
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

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_copc_key_to_string(key: *const pdal_copc_key_t) -> *mut c_char {
    let Some(key) = key.as_ref() else {
        return string_to_c_ptr(String::new());
    };
    string_to_c_ptr(format!("{}-{}-{}-{}", key.d, key.x, key.y, key.z))
}

#[pdal_capi_macros::ffi_export]
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

#[pdal_capi_macros::ffi_export]
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

#[pdal_capi_macros::ffi_export]
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
