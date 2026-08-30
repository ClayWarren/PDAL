//! C ABI for point segmentation helpers.
//!
//! Mirrors `pdal::Segmentation::extractClusters` and
//! `pdal::Segmentation::segmentReturns` from `filters/private/Segmentation.cpp`.

use pdal_core::segmentation::{extract_clusters, segment_returns};
use std::slice;

unsafe fn vec_into_raw(values: Vec<u64>) -> *mut u64 {
    if values.is_empty() {
        return std::ptr::null_mut();
    }
    let mut boxed = values.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ptr
}

/// Extract clusters of points by Euclidean region growing.
///
/// `xyz` is `count` interleaved `[x, y, z]` triples. On success returns `true`
/// and writes the cluster sizes and a flat list of point indices; both output
/// arrays are released with `pdal_u64_array_free`. Returns `false` if any
/// required pointer is null.
///
/// # Safety
/// `xyz` must be valid for `count * 3` `f64` values; the output pointers must
/// be writable.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_segmentation_extract_clusters(
    xyz: *const f64,
    count: usize,
    min_points: u64,
    max_points: u64,
    tolerance: f64,
    is_3d: bool,
    out_cluster_sizes: *mut *mut u64,
    out_cluster_count: *mut u64,
    out_point_ids: *mut *mut u64,
    out_point_count: *mut u64,
) -> bool {
    if xyz.is_null()
        || out_cluster_sizes.is_null()
        || out_cluster_count.is_null()
        || out_point_ids.is_null()
        || out_point_count.is_null()
    {
        return false;
    }

    let coords = slice::from_raw_parts(xyz, count.saturating_mul(3));
    let clusters = extract_clusters(coords, count, min_points, max_points, tolerance, is_3d);

    let sizes: Vec<u64> = clusters.iter().map(|c| c.len() as u64).collect();
    let ids: Vec<u64> = clusters.iter().flatten().map(|&id| id as u64).collect();

    *out_cluster_count = sizes.len() as u64;
    *out_point_count = ids.len() as u64;
    *out_cluster_sizes = vec_into_raw(sizes);
    *out_point_ids = vec_into_raw(ids);
    true
}

/// Classify points into the "first" output of `segmentReturns`.
///
/// `out_to_first[i]` is set to 1 when point `i` belongs to the first output.
///
/// # Safety
/// `return_number`, `number_of_returns`, and `out_to_first` must each be valid
/// for `count` elements.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_segmentation_segment_returns(
    return_number: *const u8,
    number_of_returns: *const u8,
    count: usize,
    want_first: bool,
    want_intermediate: bool,
    want_last: bool,
    want_only: bool,
    out_to_first: *mut u8,
) {
    if return_number.is_null() || number_of_returns.is_null() || out_to_first.is_null() {
        return;
    }
    let rn = slice::from_raw_parts(return_number, count);
    let nr = slice::from_raw_parts(number_of_returns, count);
    let result = segment_returns(rn, nr, want_first, want_intermediate, want_last, want_only);
    let out = slice::from_raw_parts_mut(out_to_first, count);
    for (slot, &keep) in out.iter_mut().zip(result.iter()) {
        *slot = u8::from(keep);
    }
}
