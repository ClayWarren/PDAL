use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::rc::Rc;

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
