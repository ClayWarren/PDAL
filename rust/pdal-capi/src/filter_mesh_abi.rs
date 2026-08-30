use pdal_core::point::PointView;
use pdal_filters::delaunay;
use pdal_filters::greedyprojection;

/// Compute the 2D Delaunay triangulation of a point view.
///
/// Returns a heap-allocated array of vertex indices, three per triangle, in
/// the native delaunator order; the element count is written to `out_len`.
/// Free the buffer with `pdal_free_u64_array`.
///
/// # Safety
///
/// `view` and `out_len` must be valid pointers, or `view` may be null.
#[pdal_capi_macros::ffi_export]
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

/// Compute the greedy projection triangulation of a point view.
///
/// Returns a heap-allocated array of vertex indices, three per triangle;
/// the element count is written to `out_len`.
/// Free the buffer with `pdal_free_u64_array`.
///
/// # Safety
///
/// `view` and `out_len` must be valid pointers, or `view` may be null.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_greedyprojection_triangulate(
    view: *const PointView,
    mu: f64,
    search_radius: f64,
    nnn: u64,
    min_angle: f64,
    max_angle: f64,
    eps_angle: f64,
    consistent: bool,
    out_len: *mut u64,
) -> *mut u64 {
    if view.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    if let Some(pt_view) = view.as_ref() {
        let params = greedyprojection::GreedyProjectionParams {
            mu,
            search_radius,
            nnn: nnn as usize,
            min_angle,
            max_angle,
            eps_angle,
            consistent,
        };
        let triangles = greedyprojection::run(pt_view, params);

        let mut flat_triangles = Vec::with_capacity(triangles.len() * 3);
        for tri in triangles {
            flat_triangles.push(tri[0]);
            flat_triangles.push(tri[1]);
            flat_triangles.push(tri[2]);
        }

        *out_len = flat_triangles.len() as u64;
        let mut boxed_slice = flat_triangles.into_boxed_slice();
        let ptr = boxed_slice.as_mut_ptr();
        std::mem::forget(boxed_slice);
        ptr
    } else {
        std::ptr::null_mut()
    }
}
