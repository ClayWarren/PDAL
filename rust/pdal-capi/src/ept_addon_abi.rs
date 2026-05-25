use crate::error::set_last_error;
use pdal_core::point::{DimType, PointView};
use pdal_io::ept_addon::validate_ept_addon_input;
use pdal_io::ept_addon_writer::{write_addon, AddonOverlap, AddonRootBounds, AddonWriteRequest};
use std::ffi::{c_char, CStr};

/// One source EPT hierarchy node passed across the C ABI. Mirrors
/// `pdal::ept::Overlap` shape from the C++ wrapper, but kept in a POD
/// representation so the wrapper can hand a flat array to Rust.
#[repr(C)]
pub struct PdalEptOverlap {
    pub depth: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub count: u64,
    pub node_id: u64,
}

/// Root-tile dataset-coordinate bbox (currently informational only — the
/// hierarchy keys carry the tree position).
#[repr(C)]
pub struct PdalEptRootBounds {
    pub minx: f64,
    pub miny: f64,
    pub minz: f64,
    pub maxx: f64,
    pub maxy: f64,
    pub maxz: f64,
}

/// # Safety
/// `reader_name` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_addon_validate_input(reader_name: *const c_char) -> bool {
    let Some(reader_name) = cstr(reader_name) else {
        return false;
    };
    match validate_ept_addon_input(&reader_name) {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err);
            false
        }
    }
}

unsafe fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        set_last_error("null EPT addon reader name");
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

/// Write one EPT addon dimension (binary chunks + hierarchy JSON +
/// `ept-addon.json` metadata). Returns 0 on success, non-zero on failure.
/// The C++ `EptAddonWriter::writeOne` path calls this once per addon.
///
/// # Safety
/// All pointers must be valid for the duration of the call. `view` is borrowed
/// (not consumed) and remains owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_addon_write(
    view: *const PointView,
    node_id_dim: *const c_char,
    point_id_dim: *const c_char,
    source_dim: *const c_char,
    addon_file: *const c_char,
    addon_type: i32,
    hierarchy_step: u64,
    root_bounds: *const PdalEptRootBounds,
    overlaps: *const PdalEptOverlap,
    overlap_count: u64,
) -> i32 {
    let Some(view) = view.as_ref() else {
        set_last_error("null EPT addon view");
        return -1;
    };
    let Some(node_id_dim) = cstr(node_id_dim) else {
        return -1;
    };
    let Some(point_id_dim) = cstr(point_id_dim) else {
        return -1;
    };
    let Some(source_dim) = cstr(source_dim) else {
        return -1;
    };
    let Some(addon_file) = cstr(addon_file) else {
        return -1;
    };
    let Some(ty) = dim_type_from_pdal(addon_type) else {
        set_last_error(format!("Unsupported EPT addon type 0x{addon_type:x}"));
        return -1;
    };
    let bounds = if let Some(b) = root_bounds.as_ref() {
        AddonRootBounds {
            minx: b.minx,
            miny: b.miny,
            minz: b.minz,
            maxx: b.maxx,
            maxy: b.maxy,
            maxz: b.maxz,
        }
    } else {
        AddonRootBounds {
            minx: 0.0,
            miny: 0.0,
            minz: 0.0,
            maxx: 0.0,
            maxy: 0.0,
            maxz: 0.0,
        }
    };
    let overlap_slice: &[PdalEptOverlap] = if overlaps.is_null() || overlap_count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(overlaps, overlap_count as usize)
    };
    let overlaps_rust: Vec<AddonOverlap> = overlap_slice
        .iter()
        .map(|o| AddonOverlap {
            depth: o.depth,
            x: o.x,
            y: o.y,
            z: o.z,
            count: o.count,
            node_id: o.node_id,
        })
        .collect();
    match write_addon(AddonWriteRequest {
        view,
        node_id_dim: &node_id_dim,
        point_id_dim: &point_id_dim,
        source_dim: &source_dim,
        addon_file: &addon_file,
        addon_type: ty,
        hierarchy_step,
        root_bounds: bounds,
        overlaps: &overlaps_rust,
    }) {
        Ok(()) => 0,
        Err(err) => {
            set_last_error(err);
            -1
        }
    }
}

fn dim_type_from_pdal(code: i32) -> Option<DimType> {
    match code as u32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn validates_reader_name() {
        unsafe {
            let ept = CString::new("readers.ept").unwrap();
            let las = CString::new("readers.las").unwrap();
            assert!(pdal_ept_addon_validate_input(ept.as_ptr()));
            assert!(!pdal_ept_addon_validate_input(las.as_ptr()));
        }
    }
}
