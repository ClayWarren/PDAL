use crate::error::{ffi_catch, set_last_error};
use std::convert::TryFrom;
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

fn allocation_size(block_pt_cnt: u64, dim_size: u64) -> Option<usize> {
    let point_count = usize::try_from(block_pt_cnt).ok()?;
    let byte_size = usize::try_from(dim_size).ok()?;
    point_count.checked_mul(byte_size)
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_column_storage_create(block_pt_cnt: u64) -> *mut pdal_column_storage_t {
    ffi_catch(std::ptr::null_mut(), || {
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
    })
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_column_storage_destroy(handle: *mut pdal_column_storage_t) {
    ffi_catch((), || {
        if !handle.is_null() {
            drop(Box::from_raw(handle));
        }
    });
}

/// Set the per-dimension byte sizes. Must be called after all dimensions are
/// known (i.e. after the layout is finalized) and before [`pdal_column_storage_add_point`].
/// Calling this resets storage to zero points.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_column_storage_set_dimensions(
    handle: *mut pdal_column_storage_t,
    dim_sizes: *const u64,
    dim_count: u64,
) {
    ffi_catch((), || {
        let Some(storage) = handle.as_mut() else {
            return;
        };
        let Ok(count) = usize::try_from(dim_count) else {
            set_last_error("pdal_column_storage_set_dimensions: dimension count overflow");
            return;
        };
        if count > 0 && dim_sizes.is_null() {
            set_last_error(
                "pdal_column_storage_set_dimensions: null dimension sizes with nonzero count",
            );
            return;
        }
        let sizes: Vec<u64> = if count == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(dim_sizes, count).to_vec()
        };
        storage.dim_sizes = sizes;
        storage.blocks = (0..count).map(|_| Vec::new()).collect();
        storage.num_points = 0;
    });
}

/// Allocate the next point slot, expanding per-dimension blocks lazily.
/// Returns the new point id, or `u64::MAX` on error.
#[pdal_capi_macros::ffi_export(fallback = u64::MAX)]
pub unsafe extern "C" fn pdal_column_storage_add_point(handle: *mut pdal_column_storage_t) -> u64 {
    ffi_catch(u64::MAX, || {
        let Some(storage) = handle.as_mut() else {
            set_last_error("pdal_column_storage_add_point: null handle".to_string());
            return u64::MAX;
        };
        if storage.num_points % storage.block_pt_cnt == 0 {
            let mut new_blocks = Vec::with_capacity(storage.dim_sizes.len());
            for dim_size in &storage.dim_sizes {
                let Some(size) = allocation_size(storage.block_pt_cnt, *dim_size) else {
                    set_last_error("pdal_column_storage_add_point: block size overflow");
                    return u64::MAX;
                };
                new_blocks.push(vec![0u8; size].into_boxed_slice());
            }
            for (block_list, block) in storage.blocks.iter_mut().zip(new_blocks) {
                block_list.push(block);
            }
        }
        let id = storage.num_points;
        let Some(next) = storage.num_points.checked_add(1) else {
            set_last_error("pdal_column_storage_add_point: point count overflow");
            return u64::MAX;
        };
        storage.num_points = next;
        id
    })
}

/// Return a stable pointer to the byte slot for `(dim_order, idx)`.
/// `dim_size` is the dimension's byte size; the storage must have been set up
/// with the same per-dimension size via [`pdal_column_storage_set_dimensions`].
/// Returns null if the slot does not exist yet.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_column_storage_dim_slot(
    handle: *mut pdal_column_storage_t,
    dim_order: u64,
    dim_size: u64,
    idx: u64,
) -> *mut c_void {
    ffi_catch(std::ptr::null_mut(), || {
        let Some(storage) = handle.as_mut() else {
            return std::ptr::null_mut();
        };
        let Ok(order) = usize::try_from(dim_order) else {
            return std::ptr::null_mut();
        };
        if order >= storage.blocks.len() || idx >= storage.num_points {
            return std::ptr::null_mut();
        }
        if storage.dim_sizes[order] != dim_size {
            set_last_error("pdal_column_storage_dim_slot: dimension size mismatch");
            return std::ptr::null_mut();
        }
        let block_pt_cnt = storage.block_pt_cnt;
        let Ok(block_idx) = usize::try_from(idx / block_pt_cnt) else {
            return std::ptr::null_mut();
        };
        let block_list = &mut storage.blocks[order];
        if block_idx >= block_list.len() {
            return std::ptr::null_mut();
        }
        let Some(offset) = allocation_size(idx % block_pt_cnt, dim_size) else {
            set_last_error("pdal_column_storage_dim_slot: slot offset overflow");
            return std::ptr::null_mut();
        };
        let Ok(slot_size) = usize::try_from(dim_size) else {
            set_last_error("pdal_column_storage_dim_slot: dimension size overflow");
            return std::ptr::null_mut();
        };
        let buf = &mut block_list[block_idx];
        if offset
            .checked_add(slot_size)
            .is_none_or(|end| end > buf.len())
        {
            set_last_error("pdal_column_storage_dim_slot: slot exceeds allocated block");
            return std::ptr::null_mut();
        }
        buf.as_mut_ptr().add(offset) as *mut c_void
    })
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_column_storage_num_points(
    handle: *const pdal_column_storage_t,
) -> u64 {
    ffi_catch(0, || handle.as_ref().map(|s| s.num_points).unwrap_or(0))
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

    #[test]
    fn allocation_overflow_is_reported_without_panicking() {
        unsafe {
            let storage = pdal_column_storage_create(u64::MAX);
            assert!(!storage.is_null());
            let sizes = [u64::MAX];
            pdal_column_storage_set_dimensions(storage, sizes.as_ptr(), 1);

            assert_eq!(pdal_column_storage_add_point(storage), u64::MAX);
            let message = std::ffi::CStr::from_ptr(crate::pdal_last_error());
            assert_eq!(
                message.to_string_lossy(),
                "pdal_column_storage_add_point: block size overflow"
            );
            pdal_column_storage_destroy(storage);
        }
    }

    #[test]
    fn invalid_slots_are_rejected() {
        unsafe {
            let storage = pdal_column_storage_create(4);
            assert!(!storage.is_null());
            let sizes = [8];
            pdal_column_storage_set_dimensions(storage, sizes.as_ptr(), 1);
            assert_eq!(pdal_column_storage_add_point(storage), 0);

            assert!(pdal_column_storage_dim_slot(storage, 0, 4, 0).is_null());
            assert!(pdal_column_storage_dim_slot(storage, 0, 8, 1).is_null());
            assert!(pdal_column_storage_dim_slot(storage, 1, 8, 0).is_null());

            pdal_column_storage_destroy(storage);
        }
    }
}
