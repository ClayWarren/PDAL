use crate::error::string_to_c_ptr;
use pdal_core::bounds::{Bounds2D, Bounds3D};
use pdal_core::point::{
    fix_dimension_name, pdal_dimension_interpretation_name as core_dimension_interpretation_name,
    pdal_dimension_type_from_base_and_size as core_dimension_type_from_base_and_size,
    pdal_dimension_type_from_name as core_dimension_type_from_name, resolve_pdal_dimension_type,
    DimId, DimType, DimensionSummary, PointLayout, PointView,
};
use pdal_core::srs::SpatialReference;
use serde_json::json;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_bounds2d_t {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_bounds3d_t {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
    pub minz: f64,
    pub maxz: f64,
}

impl From<pdal_bounds2d_t> for Bounds2D {
    fn from(value: pdal_bounds2d_t) -> Self {
        Bounds2D {
            minx: value.minx,
            maxx: value.maxx,
            miny: value.miny,
            maxy: value.maxy,
        }
    }
}

impl From<Bounds2D> for pdal_bounds2d_t {
    fn from(value: Bounds2D) -> Self {
        pdal_bounds2d_t {
            minx: value.minx,
            maxx: value.maxx,
            miny: value.miny,
            maxy: value.maxy,
        }
    }
}

impl From<pdal_bounds3d_t> for Bounds3D {
    fn from(value: pdal_bounds3d_t) -> Self {
        Bounds3D {
            minx: value.minx,
            maxx: value.maxx,
            miny: value.miny,
            maxy: value.maxy,
            minz: value.minz,
            maxz: value.maxz,
        }
    }
}

impl From<Bounds3D> for pdal_bounds3d_t {
    fn from(value: Bounds3D) -> Self {
        pdal_bounds3d_t {
            minx: value.minx,
            maxx: value.maxx,
            miny: value.miny,
            maxy: value.maxy,
            minz: value.minz,
            maxz: value.maxz,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_spatial_result_t {
    pub id: u64,
    pub sqr_dist: f64,
}

#[derive(Clone, Copy, Debug)]
struct QuadPoint {
    id: u64,
    x: f64,
    y: f64,
}

#[derive(Debug)]
pub struct QuadIndexAbi {
    points: Vec<QuadPoint>,
    bounds: pdal_bounds2d_t,
    top_level: u64,
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_clear(bounds: *mut pdal_bounds2d_t) {
    if let Some(bounds) = bounds.as_mut() {
        *bounds = Bounds2D::empty().into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_empty(bounds: *const pdal_bounds2d_t) -> bool {
    bounds
        .as_ref()
        .map(|bounds| Bounds2D::from(*bounds).is_empty())
        .unwrap_or(true)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_grow_point(bounds: *mut pdal_bounds2d_t, x: f64, y: f64) {
    if let Some(bounds) = bounds.as_mut() {
        let mut rust_bounds = Bounds2D::from(*bounds);
        rust_bounds.grow_point(x, y);
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_grow_distance(bounds: *mut pdal_bounds2d_t, distance: f64) {
    if let Some(bounds) = bounds.as_mut() {
        let mut rust_bounds = Bounds2D::from(*bounds);
        rust_bounds.grow_distance(distance);
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_grow_bounds(
    bounds: *mut pdal_bounds2d_t,
    other: *const pdal_bounds2d_t,
) {
    if let (Some(bounds), Some(other)) = (bounds.as_mut(), other.as_ref()) {
        let mut rust_bounds = Bounds2D::from(*bounds);
        rust_bounds.grow_bounds(&Bounds2D::from(*other));
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_clip(
    bounds: *mut pdal_bounds2d_t,
    other: *const pdal_bounds2d_t,
) {
    if let (Some(bounds), Some(other)) = (bounds.as_mut(), other.as_ref()) {
        let mut rust_bounds = Bounds2D::from(*bounds);
        rust_bounds.clip(&Bounds2D::from(*other));
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_contains_point(
    bounds: *const pdal_bounds2d_t,
    x: f64,
    y: f64,
) -> bool {
    bounds
        .as_ref()
        .map(|bounds| Bounds2D::from(*bounds).contains_point(x, y))
        .unwrap_or(false)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_contains_bounds(
    bounds: *const pdal_bounds2d_t,
    other: *const pdal_bounds2d_t,
) -> bool {
    match (bounds.as_ref(), other.as_ref()) {
        (Some(bounds), Some(other)) => {
            Bounds2D::from(*bounds).contains_bounds(&Bounds2D::from(*other))
        }
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_overlaps(
    bounds: *const pdal_bounds2d_t,
    other: *const pdal_bounds2d_t,
) -> bool {
    match (bounds.as_ref(), other.as_ref()) {
        (Some(bounds), Some(other)) => Bounds2D::from(*bounds).overlaps(&Bounds2D::from(*other)),
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_clear(bounds: *mut pdal_bounds3d_t) {
    if let Some(bounds) = bounds.as_mut() {
        *bounds = Bounds3D::empty().into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_empty(bounds: *const pdal_bounds3d_t) -> bool {
    bounds
        .as_ref()
        .map(|bounds| Bounds3D::from(*bounds).is_empty())
        .unwrap_or(true)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_grow_point(
    bounds: *mut pdal_bounds3d_t,
    x: f64,
    y: f64,
    z: f64,
) {
    if let Some(bounds) = bounds.as_mut() {
        let mut rust_bounds = Bounds3D::from(*bounds);
        rust_bounds.grow_point(x, y, z);
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_grow_bounds(
    bounds: *mut pdal_bounds3d_t,
    other: *const pdal_bounds3d_t,
) {
    if let (Some(bounds), Some(other)) = (bounds.as_mut(), other.as_ref()) {
        let mut rust_bounds = Bounds3D::from(*bounds);
        rust_bounds.grow_bounds(&Bounds3D::from(*other));
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_grow_distance(bounds: *mut pdal_bounds3d_t, distance: f64) {
    if let Some(bounds) = bounds.as_mut() {
        let mut rust_bounds = Bounds3D::from(*bounds);
        rust_bounds.grow_distance(distance);
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_clip(
    bounds: *mut pdal_bounds3d_t,
    other: *const pdal_bounds3d_t,
) {
    if let (Some(bounds), Some(other)) = (bounds.as_mut(), other.as_ref()) {
        let mut rust_bounds = Bounds3D::from(*bounds);
        rust_bounds.clip(&Bounds3D::from(*other));
        *bounds = rust_bounds.into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_contains_point(
    bounds: *const pdal_bounds3d_t,
    x: f64,
    y: f64,
    z: f64,
) -> bool {
    bounds
        .as_ref()
        .map(|bounds| Bounds3D::from(*bounds).contains_point(x, y, z))
        .unwrap_or(false)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_contains_bounds(
    bounds: *const pdal_bounds3d_t,
    other: *const pdal_bounds3d_t,
) -> bool {
    match (bounds.as_ref(), other.as_ref()) {
        (Some(bounds), Some(other)) => {
            Bounds3D::from(*bounds).contains_bounds(&Bounds3D::from(*other))
        }
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_overlaps(
    bounds: *const pdal_bounds3d_t,
    other: *const pdal_bounds3d_t,
) -> bool {
    match (bounds.as_ref(), other.as_ref()) {
        (Some(bounds), Some(other)) => Bounds3D::from(*bounds).overlaps(&Bounds3D::from(*other)),
        _ => false,
    }
}

/// Map a C string dimension name to the Rust `DimId` enum.
pub(crate) fn dim_id_from_name(name: &str) -> DimId {
    match name {
        "X" => DimId::X,
        "Y" => DimId::Y,
        "Z" => DimId::Z,
        "Intensity" => DimId::Intensity,
        "OffsetTime" => DimId::OffsetTime,
        "Classification" => DimId::Classification,
        "ClusterID" => DimId::ClusterID,
        "HeightAboveGround" => DimId::HeightAboveGround,
        "LocalOutlierFactor" => DimId::LocalOutlierFactor,
        "LocalReachabilityDistance" => DimId::LocalReachabilityDistance,
        "RadialDensity" => DimId::RadialDensity,
        "NNDistance" => DimId::NNDistance,
        "Reciprocity" => DimId::Reciprocity,
        "Rank" => DimId::Rank,
        "Coplanar" => DimId::Coplanar,
        "PlaneFit" => DimId::PlaneFit,
        "Eigenvalue0" => DimId::Eigenvalue0,
        "Eigenvalue1" => DimId::Eigenvalue1,
        "Eigenvalue2" => DimId::Eigenvalue2,
        "OptimalKNN" => DimId::OptimalKNN,
        "OptimalRadius" => DimId::OptimalRadius,
        "H3" => DimId::H3,
        "GpsTime" => DimId::GpsTime,
        "Red" => DimId::Red,
        "Green" => DimId::Green,
        "Blue" => DimId::Blue,
        other => DimId::Other(other.to_string()),
    }
}

/// Map an integer type id from the C side to a `DimType`.
pub(crate) fn dim_type_from_id(ty_id: i32) -> DimType {
    match ty_id {
        0 => DimType::U8,
        1 => DimType::U16,
        2 => DimType::U32,
        3 => DimType::U64,
        4 => DimType::I8,
        5 => DimType::I16,
        6 => DimType::I32,
        7 => DimType::I64,
        8 => DimType::F32,
        9 => DimType::F64,
        _ => DimType::F64,
    }
}

pub(crate) fn dim_type_to_id(ty: DimType) -> i32 {
    match ty {
        DimType::U8 => 0,
        DimType::U16 => 1,
        DimType::U32 => 2,
        DimType::U64 => 3,
        DimType::I8 => 4,
        DimType::I16 => 5,
        DimType::I32 => 6,
        DimType::I64 => 7,
        DimType::F32 => 8,
        DimType::F64 => 9,
    }
}

// ---------------------------------------------------------------------------
// PointLayout ABI
// ---------------------------------------------------------------------------

/// Create a new, empty point layout. Returns an owned pointer.
#[no_mangle]
pub extern "C" fn pdal_point_layout_create() -> *mut PointLayout {
    Box::into_raw(Box::new(PointLayout::new()))
}

/// Register a dimension in the layout.
///
/// # Safety
///
/// `layout` must be a valid pointer returned by `pdal_point_layout_create`.
/// `name` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_layout_register_dim(
    layout: *mut PointLayout,
    name: *const c_char,
    ty_id: i32,
) {
    if let (Some(layout), false) = (layout.as_mut(), name.is_null()) {
        let n = CStr::from_ptr(name).to_string_lossy();
        layout.register(dim_id_from_name(&n), dim_type_from_id(ty_id));
    }
}

#[no_mangle]
pub extern "C" fn pdal_dimension_resolve_type(type1: i32, type2: i32) -> i32 {
    if type1 < 0 || type2 < 0 {
        return 0;
    }
    resolve_pdal_dimension_type(type1 as u32, type2 as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_interpretation_name(type_id: i32) -> *mut c_char {
    if type_id < 0 {
        return string_to_c_ptr("unknown".to_string());
    }
    string_to_c_ptr(core_dimension_interpretation_name(type_id as u32).to_string())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_type_from_name(name: *const c_char) -> i32 {
    if name.is_null() {
        return 0;
    }
    let name = CStr::from_ptr(name).to_string_lossy();
    core_dimension_type_from_name(&name) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_type_from_base_and_size(
    base: *const c_char,
    size: u64,
) -> i32 {
    if base.is_null() {
        return 0;
    }
    let base = CStr::from_ptr(base).to_string_lossy();
    core_dimension_type_from_base_and_size(&base, size as usize) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_fix_name(name: *const c_char) -> *mut c_char {
    let name = if name.is_null() {
        String::new()
    } else {
        CStr::from_ptr(name).to_string_lossy().into_owned()
    };
    string_to_c_ptr(fix_dimension_name(&name))
}

/// Destroy a point layout.
///
/// # Safety
///
/// `layout` must be a valid pointer returned by `pdal_point_layout_create`,
/// or null. Must not be called twice on the same pointer. Must not be called
/// after the layout has been consumed by `pdal_point_view_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_layout_destroy(layout: *mut PointLayout) {
    if !layout.is_null() {
        drop(Box::from_raw(layout));
    }
}

// ---------------------------------------------------------------------------
// PointView ABI
// ---------------------------------------------------------------------------

/// Create a new, empty point view from the given layout.
///
/// # Safety
///
/// `layout` must be a valid pointer returned by `pdal_point_layout_create`.
/// Ownership of the layout is transferred — the caller must **not** call
/// `pdal_point_layout_destroy` on it after this call.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_create(layout: *mut PointLayout) -> *mut PointView {
    if layout.is_null() {
        return std::ptr::null_mut();
    }
    let layout_rc = Rc::new(*Box::from_raw(layout));
    Box::into_raw(Box::new(PointView::new(layout_rc)))
}

/// Add a zero-initialised point to the view. Returns its index.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_add_point(view: *mut PointView) -> u64 {
    if let Some(view) = view.as_mut() {
        view.add_point()
    } else {
        0
    }
}

/// Set a dimension value on a point, converting from `f64`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `dim_name` must be a valid, NUL-terminated C string.
/// `idx` must be less than the number of points in the view.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_set_f64(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    val: f64,
) {
    if let (Some(view), false) = (view.as_mut(), dim_name.is_null()) {
        let n = CStr::from_ptr(dim_name).to_string_lossy();
        view.set_f64(idx, &dim_id_from_name(&n), val);
    }
}

/// Get a dimension value from a point, as `f64`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `dim_name` must be a valid, NUL-terminated C string.
/// `idx` must be less than the number of points in the view.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_get_f64(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
) -> f64 {
    if let (Some(view), false) = (view.as_ref(), dim_name.is_null()) {
        let n = CStr::from_ptr(dim_name).to_string_lossy();
        view.get_f64(idx, &dim_id_from_name(&n))
    } else {
        0.0
    }
}

/// Return the number of dimensions in the view layout.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dim_count(view: *const PointView) -> u64 {
    if let Some(view) = view.as_ref() {
        view.layout().dim_count() as u64
    } else {
        0
    }
}

/// Return a newly allocated dimension name at layout index `idx`.
///
/// The caller owns the returned string and must free it with
/// `pdal_string_free`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dim_name(view: *const PointView, idx: u64) -> *mut c_char {
    if let Some(view) = view.as_ref() {
        if let Some((id, _)) = view.layout().dim_at(idx as usize) {
            return CString::new(id.name())
                .expect("dimension names do not contain NULs")
                .into_raw();
        }
    }
    std::ptr::null_mut()
}

/// Return the integer type id of the dimension at layout index `idx`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dim_type(view: *const PointView, idx: u64) -> i32 {
    if let Some(view) = view.as_ref() {
        if let Some((_, ty)) = view.layout().dim_at(idx as usize) {
            return dim_type_to_id(ty);
        }
    }
    -1
}

/// Set a point view's spatial reference.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `srs` must be null or a valid pointer returned by
/// `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_set_spatial_reference(
    view: *mut PointView,
    srs: *const SpatialReference,
) {
    if let Some(view) = view.as_mut() {
        let spatial_reference = srs.as_ref().cloned().unwrap_or_default();
        view.set_spatial_reference(spatial_reference);
    }
}

/// Return a copy of a point view's spatial reference. Caller owns the result.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_spatial_reference(
    view: *const PointView,
) -> *mut SpatialReference {
    if let Some(view) = view.as_ref() {
        Box::into_raw(Box::new(view.spatial_reference().clone()))
    } else {
        std::ptr::null_mut()
    }
}

/// Return the number of points in the view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_length(view: *mut PointView) -> u64 {
    if let Some(view) = view.as_ref() {
        view.len()
    } else {
        0
    }
}

/// Return the original source row for a point in this view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_source_index(view: *mut PointView, idx: u64) -> u64 {
    if let Some(view) = view.as_ref() {
        view.source_index(idx)
    } else {
        idx
    }
}

/// Calculate 2D bounds for X/Y dimensions.
///
/// Returns false when `view` or `out_bounds` is null, the view is empty, or X/Y
/// are not registered in the view's layout.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `out_bounds` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_calculate_bounds_2d(
    view: *const PointView,
    out_bounds: *mut pdal_bounds2d_t,
) -> bool {
    let Some(view) = view.as_ref() else {
        return false;
    };
    let Some(bounds) = view.calculate_bounds_2d() else {
        return false;
    };
    let Some(out_bounds) = out_bounds.as_mut() else {
        return false;
    };

    *out_bounds = pdal_bounds2d_t {
        minx: bounds.minx,
        maxx: bounds.maxx,
        miny: bounds.miny,
        maxy: bounds.maxy,
    };
    true
}

/// Calculate 3D bounds for X/Y/Z dimensions.
///
/// Returns false when `view` or `out_bounds` is null, the view is empty, or
/// X/Y/Z are not registered in the view's layout.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `out_bounds` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_calculate_bounds_3d(
    view: *const PointView,
    out_bounds: *mut pdal_bounds3d_t,
) -> bool {
    let Some(view) = view.as_ref() else {
        return false;
    };
    let Some(bounds) = view.calculate_bounds_3d() else {
        return false;
    };
    let Some(out_bounds) = out_bounds.as_mut() else {
        return false;
    };

    *out_bounds = pdal_bounds3d_t {
        minx: bounds.minx,
        maxx: bounds.maxx,
        miny: bounds.miny,
        maxy: bounds.maxy,
        minz: bounds.minz,
        maxz: bounds.maxz,
    };
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_knn(
    view: *const PointView,
    dim_names: *const *const c_char,
    query: *const f64,
    dim_count: u64,
    k: u64,
    stride: u64,
    out_results: *mut pdal_spatial_result_t,
    max_results: u64,
) -> u64 {
    if view.is_null()
        || dim_names.is_null()
        || query.is_null()
        || out_results.is_null()
        || dim_count == 0
        || k == 0
        || max_results == 0
    {
        return 0;
    }
    let Some(view) = view.as_ref() else {
        return 0;
    };
    let dims = spatial_dims(dim_names, dim_count);
    let query = std::slice::from_raw_parts(query, dim_count as usize);
    let stride = stride.max(1) as usize;
    let want = (k as usize).min(max_results as usize);
    let mut results = spatial_results(view, &dims, query, f64::INFINITY);
    let search_count = want.saturating_mul(stride).min(results.len());
    results.truncate(search_count);

    let mut written = 0;
    for idx in (0..results.len()).step_by(stride).take(want) {
        *out_results.add(written) = results[idx];
        written += 1;
    }
    written as u64
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_radius(
    view: *const PointView,
    dim_names: *const *const c_char,
    query: *const f64,
    dim_count: u64,
    radius: f64,
    out_len: *mut u64,
) -> *mut pdal_spatial_result_t {
    if !out_len.is_null() {
        *out_len = 0;
    }
    if view.is_null() || dim_names.is_null() || query.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let Some(view) = view.as_ref() else {
        return std::ptr::null_mut();
    };
    let dims = spatial_dims(dim_names, dim_count);
    let query = std::slice::from_raw_parts(query, dim_count as usize);
    let mut results = spatial_results(view, &dims, query, radius * radius);
    *out_len = results.len() as u64;
    let ptr = results.as_mut_ptr();
    std::mem::forget(results);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_results_free(ptr: *mut pdal_spatial_result_t, len: u64) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len as usize, len as usize));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_create(
    xs: *const f64,
    ys: *const f64,
    ids: *const u64,
    count: u64,
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
    top_level: u64,
) -> *mut QuadIndexAbi {
    if xs.is_null() || ys.is_null() || ids.is_null() {
        return std::ptr::null_mut();
    }

    let xs = std::slice::from_raw_parts(xs, count as usize);
    let ys = std::slice::from_raw_parts(ys, count as usize);
    let ids = std::slice::from_raw_parts(ids, count as usize);
    let points = (0..count as usize)
        .map(|idx| QuadPoint {
            id: ids[idx],
            x: xs[idx],
            y: ys[idx],
        })
        .collect();

    Box::into_raw(Box::new(QuadIndexAbi {
        points,
        bounds: pdal_bounds2d_t {
            minx: x_min,
            maxx: x_max,
            miny: y_min,
            maxy: y_max,
        },
        top_level,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_bounds(
    index: *const QuadIndexAbi,
    out_bounds: *mut pdal_bounds2d_t,
) {
    if let (Some(index), Some(out_bounds)) = (index.as_ref(), out_bounds.as_mut()) {
        *out_bounds = index.bounds;
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_depth(index: *const QuadIndexAbi) -> u64 {
    if index.as_ref().map_or(true, |index| index.points.is_empty()) {
        0
    } else {
        index.as_ref().unwrap().top_level
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_fills(
    index: *const QuadIndexAbi,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    let mut fills = vec![0; pdal_quad_index_depth(index) as usize + 1];
    if let Some(last) = fills.last_mut() {
        *last = index.points.len() as u64;
    }
    leak_u64s(fills, out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_by_depth(
    index: *const QuadIndexAbi,
    depth_begin: u64,
    depth_end: u64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    if depth_end != 0 && depth_begin >= depth_end {
        return leak_u64s(Vec::new(), out_len);
    }
    leak_u64s(index.points.iter().map(|point| point.id).collect(), out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_in_bounds(
    index: *const QuadIndexAbi,
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
    depth_begin: u64,
    depth_end: u64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    if depth_end != 0 && depth_begin >= depth_end {
        return leak_u64s(Vec::new(), out_len);
    }

    let minx = x_min.min(x_max);
    let maxx = x_min.max(x_max);
    let miny = y_min.min(y_max);
    let maxy = y_min.max(y_max);
    let ids = index
        .points
        .iter()
        .filter(|point| point.x >= minx && point.x < maxx && point.y >= miny && point.y < maxy)
        .map(|point| point.id)
        .collect();
    leak_u64s(ids, out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_raster_level(
    index: *const QuadIndexAbi,
    rasterize: u64,
    x_begin: *mut f64,
    x_end: *mut f64,
    x_step: *mut f64,
    y_begin: *mut f64,
    y_end: *mut f64,
    y_step: *mut f64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    let exp = 1usize.checked_shl(rasterize as u32).unwrap_or(0);
    if exp == 0 {
        return leak_u64s(Vec::new(), out_len);
    }

    let step_x = (index.bounds.maxx - index.bounds.minx) / exp as f64;
    let step_y = (index.bounds.maxy - index.bounds.miny) / exp as f64;
    let begin_x = index.bounds.minx + step_x / 2.0;
    let begin_y = index.bounds.miny + step_y / 2.0;
    if let Some(out) = x_begin.as_mut() {
        *out = begin_x;
    }
    if let Some(out) = x_end.as_mut() {
        *out = index.bounds.maxx + step_x / 2.0;
    }
    if let Some(out) = x_step.as_mut() {
        *out = step_x;
    }
    if let Some(out) = y_begin.as_mut() {
        *out = begin_y;
    }
    if let Some(out) = y_end.as_mut() {
        *out = index.bounds.maxy + step_y / 2.0;
    }
    if let Some(out) = y_step.as_mut() {
        *out = step_y;
    }

    rasterize_points(
        index,
        begin_x,
        index.bounds.maxx + step_x / 2.0,
        step_x,
        begin_y,
        index.bounds.maxy + step_y / 2.0,
        step_y,
        out_len,
    )
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_raster_bounds(
    index: *const QuadIndexAbi,
    x_begin: f64,
    x_end: f64,
    x_step: f64,
    y_begin: f64,
    y_end: f64,
    y_step: f64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    rasterize_points(
        index, x_begin, x_end, x_step, y_begin, y_end, y_step, out_len,
    )
}

#[no_mangle]
pub unsafe extern "C" fn pdal_u64_array_free(ptr: *mut u64, len: u64) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len as usize, len as usize));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_destroy(index: *mut QuadIndexAbi) {
    if !index.is_null() {
        drop(Box::from_raw(index));
    }
}

/// Return per-dimension summaries for a view as JSON. Caller must free with
/// `pdal_string_free`.
///
/// # Safety
///
/// `view` must be null or a valid pointer returned by
/// `pdal_point_view_create`, or returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dimension_summaries_json(
    view: *const PointView,
) -> *mut c_char {
    let summaries = view
        .as_ref()
        .map(|view| {
            serde_json::Value::Array(
                view.summarize_dimensions()
                    .iter()
                    .map(dimension_summary_json)
                    .collect(),
            )
        })
        .unwrap_or_else(|| json!([]));
    string_to_c_ptr(serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string()))
}

/// Destroy a point view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create` or
/// `pdal_stage_run`, or null. Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_destroy(view: *mut PointView) {
    if !view.is_null() {
        drop(Box::from_raw(view));
    }
}

fn dimension_summary_json(summary: &DimensionSummary) -> serde_json::Value {
    json!({
        "name": summary.name,
        "count": summary.count,
        "minimum": summary.minimum,
        "maximum": summary.maximum,
        "mean": summary.mean,
    })
}

unsafe fn spatial_dims(dim_names: *const *const c_char, dim_count: u64) -> Vec<DimId> {
    let mut dims = Vec::with_capacity(dim_count as usize);
    for idx in 0..dim_count {
        let ptr = *dim_names.add(idx as usize);
        if !ptr.is_null() {
            let name = CStr::from_ptr(ptr).to_string_lossy();
            dims.push(dim_id_from_name(&name));
        }
    }
    dims
}

fn spatial_results(
    view: &PointView,
    dims: &[DimId],
    query: &[f64],
    max_sqr_dist: f64,
) -> Vec<pdal_spatial_result_t> {
    let mut results = Vec::new();
    for idx in 0..view.len() {
        let mut sqr_dist = 0.0;
        for (dim_idx, dim) in dims.iter().enumerate() {
            let delta = view.get_f64(idx, dim) - query[dim_idx];
            sqr_dist += delta * delta;
        }
        if sqr_dist <= max_sqr_dist {
            results.push(pdal_spatial_result_t { id: idx, sqr_dist });
        }
    }
    results.sort_by(|a, b| {
        a.sqr_dist
            .total_cmp(&b.sqr_dist)
            .then_with(|| a.id.cmp(&b.id))
    });
    results
}

fn leak_u64s(mut values: Vec<u64>, out_len: *mut u64) -> *mut u64 {
    unsafe {
        if !out_len.is_null() {
            *out_len = values.len() as u64;
        }
    }
    let ptr = values.as_mut_ptr();
    std::mem::forget(values);
    ptr
}

fn rasterize_points(
    index: &QuadIndexAbi,
    x_begin: f64,
    x_end: f64,
    x_step: f64,
    y_begin: f64,
    y_end: f64,
    y_step: f64,
    out_len: *mut u64,
) -> *mut u64 {
    if x_step == 0.0 || y_step == 0.0 {
        return leak_u64s(Vec::new(), out_len);
    }
    let width = ((x_end - x_begin) / x_step).round().max(0.0) as usize;
    let height = ((y_end - y_begin) / y_step).round().max(0.0) as usize;
    let mut ids = vec![u64::MAX; width.saturating_mul(height)];

    for point in &index.points {
        if point.x < x_begin
            || point.y < y_begin
            || point.x >= x_end - x_step
            || point.y >= y_end - y_step
        {
            continue;
        }

        let x_offset = ((point.x - x_begin) / x_step).round();
        let y_offset = ((point.y - y_begin) / y_step).round();
        let idx = (y_offset * ((x_end - x_begin) / x_step) + x_offset).round();
        if idx >= 0.0 {
            let idx = idx as usize;
            if let Some(slot) = ids.get_mut(idx) {
                *slot = point.id;
            }
        }
    }

    leak_u64s(ids, out_len)
}
