use crate::error::set_last_error;
use std::os::raw::c_void;

/// Per-dimension blocked columnar storage that backs C++ `ColumnPointTable`.
///
/// For each registered dimension (by `dim_order`) we keep a `Vec<Box<[u8]>>`,
/// where each entry is one block of `block_pt_cnt * dim_size` bytes. Once a
/// block is allocated, the pointer returned by [`pdal_column_storage_dim_slot`]
/// is stable for the rest of the storage's lifetime — the outer `Vec` only
/// stores `Box<[u8]>` headers, so growth never moves the actual buffer memory.
#[allow(non_camel_case_types)]
pub struct pdal_column_storage_t {
    block_pt_cnt: u64,
    /// One entry per dimension (indexed by `dim_order`). Each entry holds a
    /// list of blocks. Block `b` covers point ids `[b * block_pt_cnt,
    /// (b+1) * block_pt_cnt)` for that dimension.
    blocks: Vec<Vec<Box<[u8]>>>,
    /// Per-dimension byte size for each dimension currently registered.
    dim_sizes: Vec<u64>,
    /// Current number of points stored.
    num_points: u64,
}

#[no_mangle]
pub extern "C" fn pdal_column_storage_create(block_pt_cnt: u64) -> *mut pdal_column_storage_t {
    if block_pt_cnt == 0 {
        set_last_error("pdal_column_storage_create: block_pt_cnt must be > 0".to_string());
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(pdal_column_storage_t {
        block_pt_cnt,
        blocks: Vec::new(),
        dim_sizes: Vec::new(),
        num_points: 0,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_column_storage_destroy(handle: *mut pdal_column_storage_t) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Set the per-dimension byte sizes. Must be called after all dimensions are
/// known (i.e. after the layout is finalized) and before [`pdal_column_storage_add_point`].
/// Calling this resets storage to zero points.
#[no_mangle]
pub unsafe extern "C" fn pdal_column_storage_set_dimensions(
    handle: *mut pdal_column_storage_t,
    dim_sizes: *const u64,
    dim_count: u64,
) {
    let Some(storage) = handle.as_mut() else {
        return;
    };
    let count = dim_count as usize;
    let sizes: Vec<u64> = if count == 0 || dim_sizes.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(dim_sizes, count).to_vec()
    };
    storage.dim_sizes = sizes;
    storage.blocks = (0..count).map(|_| Vec::new()).collect();
    storage.num_points = 0;
}

/// Allocate the next point slot, expanding per-dimension blocks lazily.
/// Returns the new point id, or `u64::MAX` on error.
#[no_mangle]
pub unsafe extern "C" fn pdal_column_storage_add_point(handle: *mut pdal_column_storage_t) -> u64 {
    let Some(storage) = handle.as_mut() else {
        set_last_error("pdal_column_storage_add_point: null handle".to_string());
        return u64::MAX;
    };
    if storage.num_points % storage.block_pt_cnt == 0 {
        for (dim_idx, block_list) in storage.blocks.iter_mut().enumerate() {
            let size = (storage.block_pt_cnt as usize)
                .checked_mul(storage.dim_sizes[dim_idx] as usize)
                .expect("column storage block size overflow");
            let buf = vec![0u8; size].into_boxed_slice();
            block_list.push(buf);
        }
    }
    let id = storage.num_points;
    storage.num_points += 1;
    id
}

/// Return a stable pointer to the byte slot for `(dim_order, idx)`.
/// `dim_size` is the dimension's byte size; the storage must have been set up
/// with the same per-dimension size via [`pdal_column_storage_set_dimensions`].
/// Returns null if the slot does not exist yet.
#[no_mangle]
pub unsafe extern "C" fn pdal_column_storage_dim_slot(
    handle: *mut pdal_column_storage_t,
    dim_order: u64,
    dim_size: u64,
    idx: u64,
) -> *mut c_void {
    let Some(storage) = handle.as_mut() else {
        return std::ptr::null_mut();
    };
    let order = dim_order as usize;
    if order >= storage.blocks.len() {
        return std::ptr::null_mut();
    }
    let block_pt_cnt = storage.block_pt_cnt;
    let block_idx = (idx / block_pt_cnt) as usize;
    let block_list = &mut storage.blocks[order];
    if block_idx >= block_list.len() {
        return std::ptr::null_mut();
    }
    let offset = ((idx % block_pt_cnt) as usize)
        .checked_mul(dim_size as usize)
        .expect("column storage offset overflow");
    let buf = &mut block_list[block_idx];
    buf.as_mut_ptr().add(offset) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn pdal_column_storage_num_points(
    handle: *const pdal_column_storage_t,
) -> u64 {
    handle.as_ref().map(|s| s.num_points).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_boundary_stable_pointers() {
        unsafe {
            let storage = pdal_column_storage_create(16384);
            assert!(!storage.is_null());
            let sizes: [u64; 2] = [8, 4]; // f64 + f32
            pdal_column_storage_set_dimensions(storage, sizes.as_ptr(), 2);

            for _ in 0..16385 {
                let id = pdal_column_storage_add_point(storage);
                assert_ne!(id, u64::MAX);
            }

            // Slot for point 0, dim 0 — stable pointer
            let p0 = pdal_column_storage_dim_slot(storage, 0, 8, 0);
            assert!(!p0.is_null());
            *(p0 as *mut f64) = 1234.5;

            // Slot for point 16384, dim 0 — falls in second block
            let p1 = pdal_column_storage_dim_slot(storage, 0, 8, 16384);
            assert!(!p1.is_null());
            *(p1 as *mut f64) = -9876.5;

            // After more adds, the old slot pointers must still be valid.
            for _ in 0..16384 {
                pdal_column_storage_add_point(storage);
            }
            let again = pdal_column_storage_dim_slot(storage, 0, 8, 0);
            assert_eq!(again, p0);
            assert_eq!(*(p0 as *const f64), 1234.5);
            assert_eq!(*(p1 as *const f64), -9876.5);

            pdal_column_storage_destroy(storage);
        }
    }
}
