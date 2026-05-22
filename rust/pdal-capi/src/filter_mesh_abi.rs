use pdal_core::point::PointView;
use pdal_filters::delaunay;

/// Compute the 2D Delaunay triangulation of a point view.
///
/// Returns a heap-allocated array of vertex indices, three per triangle, in
/// the native delaunator order; the element count is written to `out_len`.
/// Free the buffer with `pdal_free_u64_array`.
///
/// # Safety
///
/// `view` and `out_len` must be valid pointers, or `view` may be null.
#[no_mangle]
pub unsafe extern "C" fn pdal_delaunay_triangulate(
    view: *const PointView,
    out_len: *mut u64,
) -> *mut u64 {
    if view.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    if let Some(pt_view) = view.as_ref() {
        let triangles = delaunay::triangulate_xy(pt_view);
        *out_len = triangles.len() as u64;
        let mut boxed_slice = triangles.into_boxed_slice();
        let ptr = boxed_slice.as_mut_ptr();
        std::mem::forget(boxed_slice);
        ptr
    } else {
        std::ptr::null_mut()
    }
}
