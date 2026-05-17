//! C ABI for the PDAL Rust port spike.
//!
//! Every function in this crate is `extern "C"` and intended to be called from
//! C or C++ through the header `include/pdal_capi.h`.

use pdal_core::options::Options;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use pdal_filters::approximate_coplanar::ApproximateCoplanarFilter;
use pdal_filters::assign;
use pdal_filters::chipper::ChipperFilter;
use pdal_filters::cluster::ClusterFilter;
use pdal_filters::covariancefeatures::{CovarianceFeaturesFilter, CovarianceFeaturesMode};
use pdal_filters::dbscan::DbscanFilter;
use pdal_filters::decimation::DecimationFilter;
use pdal_filters::divider;
use pdal_filters::eigenvalues::EigenvaluesFilter;
use pdal_filters::elm::ElmFilter;
use pdal_filters::estimate_rank::EstimateRankFilter;
use pdal_filters::farthestpointsampling::FarthestPointSamplingFilter;
use pdal_filters::ferry::FerryFilter;
use pdal_filters::griddecimation;
use pdal_filters::groupby::GroupByFilter;
use pdal_filters::hagnn::HagNnFilter;
use pdal_filters::head::HeadFilter;
use pdal_filters::iqr::IqrFilter;
use pdal_filters::labelduplicates::LabelDuplicatesFilter;
use pdal_filters::litree::LiTreeFilter;
use pdal_filters::lloydkmeans::LloydKMeansFilter;
use pdal_filters::locate::LocateFilter;
use pdal_filters::lof::LofFilter;
use pdal_filters::m3c2;
use pdal_filters::mad::MadFilter;
use pdal_filters::merge::MergeFilter;
use pdal_filters::miniball::MiniballFilter;
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::neighborclassifier::NeighborClassifierFilter;
use pdal_filters::nndistance::{NNDistanceFilter, NNDistanceMode};
use pdal_filters::normal::NormalFilter;
use pdal_filters::optimal_neighborhood::OptimalNeighborhoodFilter;
use pdal_filters::outlier::OutlierFilter;
use pdal_filters::planefit::PlaneFitFilter;
use pdal_filters::radialdensity::RadialDensityFilter;
use pdal_filters::radiusassign;
use pdal_filters::randomize::RandomizeFilter;
use pdal_filters::range::{RangeFilter, RangeLimit};
use pdal_filters::reciprocity::ReciprocityFilter;
use pdal_filters::returns::ReturnsFilter;
use pdal_filters::sample::SampleFilter;
use pdal_filters::separatescanline::SeparateScanLineFilter;
use pdal_filters::skewnessbalancing::SkewnessBalancingFilter;
use pdal_filters::sort::{SortAlgorithm, SortFilter, SortOrder};
use pdal_filters::sparse_surface::SparseSurfaceFilter;
use pdal_filters::splitter::SplitterFilter;
use pdal_filters::stats;
use pdal_filters::straighten;
use pdal_filters::tail::TailFilter;
use pdal_filters::transformation::TransformationFilter;
use pdal_filters::voxel_center_nearest_neighbor::VoxelCenterNearestNeighborFilter;
use pdal_filters::voxel_centroid_nearest_neighbor::VoxelCentroidNearestNeighborFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;
use pdal_filters::zsmooth::ZsmoothFilter;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

thread_local! {
    static LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("").expect("empty CString is valid"));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a C string dimension name to the Rust `DimId` enum.
fn dim_id_from_name(name: &str) -> DimId {
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
        "NormalX" => DimId::NormalX,
        "NormalY" => DimId::NormalY,
        "NormalZ" => DimId::NormalZ,
        "Curvature" => DimId::Curvature,
        "Reciprocity" => DimId::Reciprocity,
        "Rank" => DimId::Rank,
        "Coplanar" => DimId::Coplanar,
        "PlaneFit" => DimId::PlaneFit,
        "Eigenvalue0" => DimId::Eigenvalue0,
        "Eigenvalue1" => DimId::Eigenvalue1,
        "Eigenvalue2" => DimId::Eigenvalue2,
        "OptimalKNN" => DimId::OptimalKNN,
        "OptimalRadius" => DimId::OptimalRadius,
        "Linearity" => DimId::Linearity,
        "Planarity" => DimId::Planarity,
        "Scattering" => DimId::Scattering,
        "Verticality" => DimId::Verticality,
        "Omnivariance" => DimId::Omnivariance,
        "Anisotropy" => DimId::Anisotropy,
        "Eigenentropy" => DimId::Eigenentropy,
        "EigenvalueSum" => DimId::EigenvalueSum,
        "SurfaceVariation" => DimId::SurfaceVariation,
        "DemantkeVerticality" => DimId::DemantkeVerticality,
        "Density" => DimId::Density,
        "Miniball" => DimId::Miniball,
        other => DimId::Other(other.to_string()),
    }
}

/// Map an integer type id from the C side to a `DimType`.
fn dim_type_from_id(ty_id: i32) -> DimType {
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

fn set_last_error(message: impl Into<String>) {
    let sanitized = message.into().replace('\0', "\\0");
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = CString::new(sanitized).expect("interior NULs removed");
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = CString::new("").expect("empty CString is valid");
    });
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "Rust panic".to_string()
    }
}

fn ffi_catch<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            set_last_error(panic_message(payload.as_ref()));
            fallback
        }
    }
}

#[no_mangle]
pub extern "C" fn pdal_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn pdal_clear_error() {
    clear_last_error();
}

// ---------------------------------------------------------------------------
// Options ABI
// ---------------------------------------------------------------------------

/// Create a new, empty options set. Returns an owned pointer.
#[no_mangle]
pub extern "C" fn pdal_options_create() -> *mut Options {
    Box::into_raw(Box::new(Options::new()))
}

/// Add a floating-point option.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_add_f64(ops: *mut Options, key: *const c_char, value: f64) {
    if let (Some(ops), false) = (ops.as_mut(), key.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        ops.add(&k, value.to_string());
    }
}

/// Add an unsigned 64-bit integer option.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_add_u64(ops: *mut Options, key: *const c_char, value: u64) {
    if let (Some(ops), false) = (ops.as_mut(), key.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        ops.add(&k, value.to_string());
    }
}

/// Add a string option.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `key` and `value` must be valid, NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_add_str(
    ops: *mut Options,
    key: *const c_char,
    value: *const c_char,
) {
    if let (Some(ops), false, false) = (ops.as_mut(), key.is_null(), value.is_null()) {
        let k = CStr::from_ptr(key).to_string_lossy();
        let v = CStr::from_ptr(value).to_string_lossy();
        ops.add(&k, v.to_string());
    }
}

/// Destroy an options set.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`, or null.
/// Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_options_destroy(ops: *mut Options) {
    if !ops.is_null() {
        drop(Box::from_raw(ops));
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

/// Set the spatial reference of a point view.
///
/// # Safety
///
/// `view` must be valid. `wkt` must be a valid NUL-terminated C-string.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_set_spatial_reference(
    view: *mut PointView,
    wkt: *const c_char,
) {
    if let (Some(view), false) = (view.as_mut(), wkt.is_null()) {
        let wkt_str = CStr::from_ptr(wkt).to_string_lossy();
        view.set_spatial_reference(pdal_core::srs::SpatialReference::new(&wkt_str));
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

// ---------------------------------------------------------------------------
// Stage ABI
// ---------------------------------------------------------------------------

/// Opaque wrapper around a Rust filter that implements both `Filter` and
/// `Streamable`.
pub struct StageWrapper {
    filter: Box<dyn FilterWrapper>,
}

trait FilterWrapper {
    fn process_one(&mut self, view: &mut PointView, idx: u64) -> bool;
    fn reset(&mut self);
    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError>;
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: Filter + Streamable> FilterWrapper for T {
    fn process_one(&mut self, view: &mut PointView, idx: u64) -> bool {
        Streamable::process_one(self, view, idx)
    }
    fn reset(&mut self) {
        Streamable::reset(self)
    }
    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        Filter::run(self, input)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        Filter::as_any(self)
    }
}

/// Create an expression filter stage from options.
///
/// # Safety
///
/// `sources` must be a valid pointer to a C-array of C-strings of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_expression(
    sources: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    if sources.is_null() {
        return std::ptr::null_mut();
    }
    let mut vec_sources = Vec::new();
    for i in 0..count {
        let ptr = *sources.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        vec_sources.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    match pdal_filters::expression::ExpressionFilter::new(&vec_sources) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create an expression stats filter stage from options.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C-string.
/// `sources` must be a valid pointer to a C-array of C-strings of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_expressionstats(
    dim_name: *const c_char,
    sources: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    if dim_name.is_null() || sources.is_null() {
        return std::ptr::null_mut();
    }
    let dim = CStr::from_ptr(dim_name).to_string_lossy();
    let mut vec_sources = Vec::new();
    for i in 0..count {
        let ptr = *sources.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        vec_sources.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    match pdal_filters::expression_stats::ExpressionStatsFilter::new(&dim, &vec_sources) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create an H3 filter stage.
///
/// # Safety
///
/// Always safe to call.
#[no_mangle]
pub extern "C" fn pdal_stage_create_h3(resolution: u64) -> *mut StageWrapper {
    let filter = Box::new(pdal_filters::h3::H3Filter::new(resolution as u8));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a mongoexpression filter stage.
///
/// # Safety
///
/// `json` must be a valid NUL-terminated C-string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_mongoexpression(
    json: *const c_char,
) -> *mut StageWrapper {
    if json.is_null() {
        return std::ptr::null_mut();
    }
    let json_str = CStr::from_ptr(json).to_string_lossy();
    match pdal_filters::mongo_expression::MongoExpressionFilter::new(&json_str) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a reprojection filter stage.
///
/// # Safety
///
/// `out_srs`, `in_srs` must be valid NUL-terminated C-strings (in_srs can be null).
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_reprojection(
    out_srs: *const c_char,
    in_srs: *const c_char,
    error_on_failure: bool,
) -> *mut StageWrapper {
    if out_srs.is_null() {
        return std::ptr::null_mut();
    }
    let out_srs_wkt = CStr::from_ptr(out_srs).to_string_lossy();
    let in_srs_wkt = if in_srs.is_null() {
        None
    } else {
        Some(CStr::from_ptr(in_srs).to_string_lossy().into_owned())
    };

    let filter = Box::new(pdal_filters::reprojection::ReprojectionFilter::new(
        &out_srs_wkt,
        in_srs_wkt,
        error_on_failure,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a geomdistance filter stage.
///
/// # Safety
///
/// `wkt`, `dim_name` must be valid NUL-terminated C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_geomdistance(
    wkt: *const c_char,
    dim_name: *const c_char,
) -> *mut StageWrapper {
    if wkt.is_null() || dim_name.is_null() {
        return std::ptr::null_mut();
    }
    let wkt_str = CStr::from_ptr(wkt).to_string_lossy();
    let dim_str = CStr::from_ptr(dim_name).to_string_lossy();

    match pdal_filters::geom_distance::GeomDistanceFilter::new(&wkt_str, &dim_str) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a projpipeline filter stage.
///
/// # Safety
///
/// `out_srs`, `coord_op` must be valid NUL-terminated C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_projpipeline(
    out_srs: *const c_char,
    coord_op: *const c_char,
    reverse: bool,
) -> *mut StageWrapper {
    if out_srs.is_null() || coord_op.is_null() {
        return std::ptr::null_mut();
    }
    let out_srs_wkt = CStr::from_ptr(out_srs).to_string_lossy();
    let coord_op_str = CStr::from_ptr(coord_op).to_string_lossy();

    let filter = Box::new(pdal_filters::proj_pipeline::ProjPipelineFilter::new(
        &out_srs_wkt,
        &coord_op_str,
        reverse,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a colorinterp filter stage.
///
/// # Safety
///
/// `dim_name`, `ramp` must be valid NUL-terminated C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_colorinterp(
    dim_name: *const c_char,
    ramp: *const c_char,
    min: f64,
    max: f64,
    clamp: bool,
    invert: bool,
) -> *mut StageWrapper {
    if dim_name.is_null() || ramp.is_null() {
        return std::ptr::null_mut();
    }
    let dim_str = CStr::from_ptr(dim_name).to_string_lossy();
    let ramp_str = CStr::from_ptr(ramp).to_string_lossy();

    let filter = Box::new(pdal_filters::colorinterp::ColorinterpFilter::new(
        &dim_str, &ramp_str, min, max, clamp, invert,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

#[repr(C)]
pub struct pdal_band_info_t {
    pub name: *const c_char,
    pub band: u32,
    pub scale: f64,
}

/// Create a colorization filter stage.
///
/// # Safety
///
/// `raster_path` must be a valid NUL-terminated C-string.
/// `bands` must be a valid pointer to a C-array of `pdal_band_info_t` of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_colorization(
    raster_path: *const c_char,
    bands: *const pdal_band_info_t,
    count: u64,
) -> *mut StageWrapper {
    if raster_path.is_null() || (bands.is_null() && count > 0) {
        return std::ptr::null_mut();
    }
    let raster_str = CStr::from_ptr(raster_path).to_string_lossy();
    let mut vec_bands = Vec::new();
    for i in 0..count {
        let b = &*bands.offset(i as isize);
        if b.name.is_null() {
            return std::ptr::null_mut();
        }
        vec_bands.push(pdal_filters::colorization::BandInfo {
            name: CStr::from_ptr(b.name).to_string_lossy().into_owned(),
            band: b.band,
            scale: b.scale,
        });
    }

    let filter = Box::new(pdal_filters::colorization::ColorizationFilter::new(
        &raster_str,
        vec_bands,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a dem filter stage.
///
/// # Safety
///
/// `dim_name`, `raster_path` must be valid NUL-terminated C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_dem(
    dim_name: *const c_char,
    raster_path: *const c_char,
    band: i32,
    lower_bound: f64,
    upper_bound: f64,
) -> *mut StageWrapper {
    if dim_name.is_null() || raster_path.is_null() {
        return std::ptr::null_mut();
    }
    let dim_str = CStr::from_ptr(dim_name).to_string_lossy();
    let raster_str = CStr::from_ptr(raster_path).to_string_lossy();

    let filter = Box::new(pdal_filters::dem::DEMFilter::new(
        &dim_str,
        &raster_str,
        band,
        lower_bound,
        upper_bound,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an overlay filter stage.
///
/// # Safety
///
/// `dim_name`, `datasource`, `column` must be valid NUL-terminated C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_overlay(
    dim_name: *const c_char,
    datasource: *const c_char,
    column: *const c_char,
) -> *mut StageWrapper {
    if dim_name.is_null() || datasource.is_null() || column.is_null() {
        return std::ptr::null_mut();
    }
    let dim_str = CStr::from_ptr(dim_name).to_string_lossy();
    let ds_str = CStr::from_ptr(datasource).to_string_lossy();
    let col_str = CStr::from_ptr(column).to_string_lossy();

    let filter = Box::new(pdal_filters::overlay::OverlayFilter::new(
        &dim_str, &ds_str, &col_str,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a georeference filter stage.
///
/// # Safety
///
/// `out_srs` must be a valid NUL-terminated C-string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_georeference(
    out_srs: *const c_char,
) -> *mut StageWrapper {
    if out_srs.is_null() {
        return std::ptr::null_mut();
    }
    let out_srs_wkt = CStr::from_ptr(out_srs).to_string_lossy();

    let filter = Box::new(pdal_filters::georeference::GeoreferenceFilter::new(
        &out_srs_wkt,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a decimation filter stage from options.

/// Create a decimation filter stage from options.

#[repr(C)]
pub struct pdal_box3d_t {
    pub minx: f64,
    pub miny: f64,
    pub minz: f64,
    pub maxx: f64,
    pub maxy: f64,
    pub maxz: f64,
}

#[repr(C)]
pub struct pdal_point3d_t {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Create a crop filter stage.
///
/// # Safety
///
/// `bounds` must be a valid pointer to a C-array of `pdal_box3d_t` of length `bounds_count`.
/// `polygons` must be a valid pointer to a C-array of C-strings of length `poly_count`.
/// `centers` must be a valid pointer to a C-array of `pdal_point3d_t` of length `center_count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_crop(
    outside: bool,
    bounds: *const pdal_box3d_t,
    bounds_count: u64,
    polygons: *const *const c_char,
    poly_count: u64,
    centers: *const pdal_point3d_t,
    center_count: u64,
    distance: f64,
) -> *mut StageWrapper {
    let mut vec_bounds = Vec::new();
    for i in 0..bounds_count {
        let b = &*bounds.offset(i as isize);
        vec_bounds.push((b.minx, b.miny, b.minz, b.maxx, b.maxy, b.maxz));
    }

    let mut vec_polys = Vec::new();
    for i in 0..poly_count {
        let p = *polygons.offset(i as isize);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        vec_polys.push(CStr::from_ptr(p).to_string_lossy().into_owned());
    }

    let mut vec_centers = Vec::new();
    for i in 0..center_count {
        let c = &*centers.offset(i as isize);
        vec_centers.push((c.x, c.y, c.z));
    }

    match pdal_filters::crop::CropFilter::new(outside, vec_bounds, vec_polys, vec_centers, distance)
    {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a decimation filter stage from options.

/// Create a hag_dem filter stage.
///
/// # Safety
///
/// `raster_path` must be a valid NUL-terminated C-string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_hag_dem(
    raster_path: *const c_char,
    band: i32,
    zero_ground: bool,
    min_clamp: f64,
    max_clamp: f64,
    nodata_height: f64,
    ground_class: u8,
) -> *mut StageWrapper {
    if raster_path.is_null() {
        return std::ptr::null_mut();
    }
    let raster_str = CStr::from_ptr(raster_path).to_string_lossy();

    let filter = Box::new(pdal_filters::hag_dem::HagDemFilter::new(
        &raster_str,
        band,
        zero_ground,
        min_clamp,
        max_clamp,
        nodata_height,
        ground_class,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a decimation filter stage from options.

/// Create a decimation filter stage from options.

/// Create a decimation filter stage from options.

/// Create a decimation filter stage from options.

/// Create a decimation filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_decimation(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let filter = Box::new(DecimationFilter::new(options));
        Box::into_raw(Box::new(StageWrapper { filter }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a head filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_head(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let count = options.get_u64("count", 10);
        let invert = options.get_bool("invert", false);
        let filter = Box::new(HeadFilter::new(count, invert));
        Box::into_raw(Box::new(StageWrapper { filter }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a tail filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_tail(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let count = options.get_u64("count", 10);
        let invert = options.get_bool("invert", false);
        let filter = Box::new(TailFilter::new(count, invert));
        Box::into_raw(Box::new(StageWrapper { filter }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a locate filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_locate(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let dim_name = options.get_str("dimension", "");
        let minmax = options.get_str("minmax", "max");
        let filter = Box::new(LocateFilter::new(dim_name, minmax));
        Box::into_raw(Box::new(StageWrapper { filter }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a ferry filter stage.
///
/// # Safety
///
/// `from_dims` and `to_dims` must be valid arrays of null-terminated strings of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_ferry(
    from_dims: *const *const std::os::raw::c_char,
    to_dims: *const *const std::os::raw::c_char,
    count: u64,
) -> *mut StageWrapper {
    if from_dims.is_null() || to_dims.is_null() {
        return std::ptr::null_mut();
    }
    let mut dims = Vec::new();
    for i in 0..count {
        let from_ptr = *from_dims.offset(i as isize);
        let to_ptr = *to_dims.offset(i as isize);
        if from_ptr.is_null() || to_ptr.is_null() {
            return std::ptr::null_mut();
        }
        let from_str = CStr::from_ptr(from_ptr).to_string_lossy().into_owned();
        let to_str = CStr::from_ptr(to_ptr).to_string_lossy().into_owned();
        dims.push((from_str, to_str));
    }
    let filter = Box::new(FerryFilter::new(dims));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Copy values between dimensions on a specific point in a PointView.
///
/// # Safety
///
/// `stage` must be a valid pointer to a stage created with `pdal_stage_create_ferry`.
/// `view` must be a valid pointer to a `PointView`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_ferry_point(
    stage: *mut StageWrapper,
    view: *mut PointView,
    idx: u64,
) {
    if let (Some(stage_wrapper), Some(pt_view)) = (stage.as_ref(), view.as_mut()) {
        if let Some(ferry) = stage_wrapper.filter.as_any().downcast_ref::<FerryFilter>() {
            ferry.ferry_point(pt_view, idx);
        }
    }
}

/// Create a randomize filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_randomize(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let seed = if options.has("seed") {
            Some(options.get_u64("seed", 0) as u32)
        } else {
            None
        };
        let filter = Box::new(RandomizeFilter::new(seed));
        Box::into_raw(Box::new(StageWrapper { filter }))
    } else {
        std::ptr::null_mut()
    }
}

/// Range Limit struct for FFI translation.
#[repr(C)]
pub struct pdal_range_limit_t {
    pub dim_name: *const std::os::raw::c_char,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
}

/// Create a range filter stage.
///
/// # Safety
///
/// `limits` must be a valid pointer to an array of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_range(
    limits: *const pdal_range_limit_t,
    count: u64,
) -> *mut StageWrapper {
    if limits.is_null() {
        return std::ptr::null_mut();
    }
    let mut vec_limits = Vec::new();
    for i in 0..count {
        let limit = &*limits.offset(i as isize);
        if limit.dim_name.is_null() {
            return std::ptr::null_mut();
        }
        let name = CStr::from_ptr(limit.dim_name)
            .to_string_lossy()
            .into_owned();
        vec_limits.push(RangeLimit {
            dim_name: name,
            lower_bound: limit.lower_bound,
            upper_bound: limit.upper_bound,
            inclusive_lower: limit.inclusive_lower,
            inclusive_upper: limit.inclusive_upper,
            negate: limit.negate,
        });
    }
    let filter = Box::new(RangeFilter::new(vec_limits));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Destroy a stage.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`, or null.
/// Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_destroy(stage: *mut StageWrapper) {
    if !stage.is_null() {
        drop(Box::from_raw(stage));
    }
}

/// Reset the streaming state of a stage.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_reset(stage: *mut StageWrapper) {
    if let Some(stage) = stage.as_mut() {
        stage.filter.reset();
    }
}

/// Process one point in streaming mode. Returns `true` to keep the point.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
/// `view` must be a valid pointer to a `PointView`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_process_one(
    stage: *mut StageWrapper,
    view: *mut PointView,
    idx: u64,
) -> bool {
    ffi_catch(false, || {
        clear_last_error();
        if let (Some(stage), Some(pt_view)) = (stage.as_mut(), view.as_mut()) {
            stage.filter.process_one(pt_view, idx)
        } else {
            set_last_error("null stage or view");
            false
        }
    })
}

/// Run the filter over a complete input view. Returns a new output view
/// (caller owns it), or null on error.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
/// `input` must be a valid pointer returned by `pdal_point_view_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_run(
    stage: *mut StageWrapper,
    input: *mut PointView,
) -> *mut PointView {
    ffi_catch(std::ptr::null_mut(), || {
        clear_last_error();
        if let (Some(stage), Some(input)) = (stage.as_mut(), input.as_mut()) {
            match stage.filter.run(input) {
                Ok(mut outputs) => {
                    if !outputs.is_empty() {
                        return Box::into_raw(Box::new(outputs.remove(0)));
                    }
                }
                Err(err) => set_last_error(err.to_string()),
            }
        } else {
            set_last_error("null stage or input view");
        }
        std::ptr::null_mut()
    })
}

/// Create a sort filter stage.
///
/// # Safety
///
/// `dims` must be a valid pointer to a C-array of C-strings of length `count`.
/// `order` and `algorithm` must be valid NUL-terminated C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_sort(
    dims: *const *const c_char,
    count: u64,
    order: *const c_char,
    algorithm: *const c_char,
) -> *mut StageWrapper {
    if dims.is_null() || order.is_null() || algorithm.is_null() {
        return std::ptr::null_mut();
    }
    let mut dim_names = Vec::new();
    for i in 0..count {
        let ptr = *dims.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        dim_names.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }

    let order_str = CStr::from_ptr(order).to_string_lossy();
    let order_enum = if order_str.eq_ignore_ascii_case("desc") {
        SortOrder::Desc
    } else {
        SortOrder::Asc
    };

    let alg_str = CStr::from_ptr(algorithm).to_string_lossy();
    let alg_enum = if alg_str.eq_ignore_ascii_case("stable") {
        SortAlgorithm::Stable
    } else {
        SortAlgorithm::Normal
    };

    let filter = Box::new(SortFilter::new(dim_names, order_enum, alg_enum));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a returns filter stage.
///
/// # Safety
///
/// `groups` must be a valid pointer to a C-array of C-strings of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_returns(
    groups: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    if groups.is_null() {
        return std::ptr::null_mut();
    }
    let mut vec_groups = Vec::new();
    for i in 0..count {
        let ptr = *groups.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        vec_groups.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    let filter = Box::new(ReturnsFilter::new(vec_groups));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a separatescanline filter stage.
///
/// # Safety
///
/// Safe to call with any u64 value.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_separatescanline(groupby: u64) -> *mut StageWrapper {
    let filter = Box::new(SeparateScanLineFilter::new(groupby));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Run the filter over a complete input view, returning multiple output views.
/// The returned pointers are written into the `outputs` buffer, up to `max_outputs`.
/// Returns the actual number of output views produced.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
/// `input` must be a valid pointer returned by `pdal_point_view_create`.
/// `outputs` must be a valid pointer to a buffer of size `max_outputs` pointer elements.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_run_multi(
    stage: *mut StageWrapper,
    input: *mut PointView,
    outputs: *mut *mut PointView,
    max_outputs: u64,
) -> u64 {
    ffi_catch(0, || {
        clear_last_error();
        if let (Some(stage), Some(input), false) =
            (stage.as_mut(), input.as_mut(), outputs.is_null())
        {
            match stage.filter.run(input) {
                Ok(mut results) => {
                    let count = std::cmp::min(results.len() as u64, max_outputs);
                    for i in 0..count {
                        let view = results.remove(0);
                        *outputs.offset(i as isize) = Box::into_raw(Box::new(view));
                    }
                    return count;
                }
                Err(err) => set_last_error(err.to_string()),
            }
        } else {
            set_last_error("null stage, input view, or output buffer");
        }
        0
    })
}

/// Create a groupby filter stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C-string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_groupby(dim_name: *const c_char) -> *mut StageWrapper {
    if dim_name.is_null() {
        return std::ptr::null_mut();
    }
    let name = CStr::from_ptr(dim_name).to_string_lossy().into_owned();
    let filter = Box::new(GroupByFilter::new(name));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a label_duplicates filter stage.
///
/// # Safety
///
/// `dims` must be a valid pointer to a C-array of C-strings of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_labelduplicates(
    dims: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    if dims.is_null() {
        return std::ptr::null_mut();
    }
    let mut vec_dims = Vec::new();
    for i in 0..count {
        let ptr = *dims.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        vec_dims.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    let filter = Box::new(LabelDuplicatesFilter::new(vec_dims));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a merge filter stage.
///
/// # Safety
///
/// Always safe to call.
#[no_mangle]
pub extern "C" fn pdal_stage_create_merge() -> *mut StageWrapper {
    let filter = Box::new(MergeFilter::new());
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Append a point view to the merge filter's accumulated buffer.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_merge`.
/// `view` must be a valid pointer to a `PointView`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_merge_append(stage: *mut StageWrapper, view: *mut PointView) {
    if let (Some(stage_wrapper), Some(pt_view)) = (stage.as_ref(), view.as_ref()) {
        if let Some(merge) = stage_wrapper.filter.as_any().downcast_ref::<MergeFilter>() {
            merge.merge_view(pt_view);
        }
    }
}

/// Create a mortonorder filter stage.
///
/// # Safety
///
/// Always safe to call.
#[no_mangle]
pub extern "C" fn pdal_stage_create_mortonorder(reverse: bool) -> *mut StageWrapper {
    let filter = Box::new(MortonOrderFilter::new(reverse));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a transformation filter stage.
///
/// # Safety
///
/// `matrix` must be a valid pointer to a 16-element float64 array.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_transformation(matrix: *const f64) -> *mut StageWrapper {
    if matrix.is_null() {
        return std::ptr::null_mut();
    }
    let mut mat = [0.0f64; 16];
    std::ptr::copy_nonoverlapping(matrix, mat.as_mut_ptr(), 16);
    let filter = Box::new(TransformationFilter::new(mat));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Apply transformation to a single point in the view (for streaming mode).
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_transformation`.
/// `view` must be a valid pointer to a `PointView`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_transformation_point(
    stage: *mut StageWrapper,
    view: *mut PointView,
    idx: u64,
) {
    if let (Some(stage_wrapper), Some(pt_view)) = (stage.as_ref(), view.as_mut()) {
        if let Some(xform) = stage_wrapper
            .filter
            .as_any()
            .downcast_ref::<TransformationFilter>()
        {
            xform.transform_point(pt_view, idx);
        }
    }
}

/// Create a voxeldownsize filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_voxeldownsize(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let filter = Box::new(VoxelDownsizeFilter::new(options));
        Box::into_raw(Box::new(StageWrapper { filter }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a sample filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_sample(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let filter = Box::new(SampleFilter::new(options));
        Box::into_raw(Box::new(StageWrapper { filter }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a radialdensity filter stage.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_radialdensity(radius: f64) -> *mut StageWrapper {
    let filter = Box::new(RadialDensityFilter::new(radius));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an nndistance filter stage.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_nndistance(
    k: u64,
    mode: *const c_char,
) -> *mut StageWrapper {
    let mode = if mode.is_null() {
        NNDistanceMode::Kth
    } else {
        match CStr::from_ptr(mode).to_string_lossy().as_ref() {
            "avg" => NNDistanceMode::Average,
            _ => NNDistanceMode::Kth,
        }
    };
    let filter = Box::new(NNDistanceFilter::new(k as usize, mode));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a zsmooth filter stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C-string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_zsmooth(
    radius: f64,
    position: f64,
    dim_name: *const c_char,
) -> *mut StageWrapper {
    if dim_name.is_null() {
        return std::ptr::null_mut();
    }
    let dim_name = CStr::from_ptr(dim_name).to_string_lossy().into_owned();
    let filter = Box::new(ZsmoothFilter::new(radius, position, dim_name));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an outlier filter stage.
///
/// # Safety
///
/// `method` must be a valid NUL-terminated C-string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_outlier(
    method: *const c_char,
    min_k: u64,
    radius: f64,
    mean_k: u64,
    multiplier: f64,
    class_label: u8,
) -> *mut StageWrapper {
    if method.is_null() {
        return std::ptr::null_mut();
    }
    let method = CStr::from_ptr(method).to_string_lossy().into_owned();
    let filter = Box::new(OutlierFilter::new(
        method,
        min_k as usize,
        radius,
        mean_k as usize,
        multiplier,
        class_label,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a DBSCAN filter stage.
///
/// # Safety
///
/// `dims` must be a valid pointer to a C-array of C-strings of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_dbscan(
    min_points: u64,
    eps: f64,
    dims: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    if dims.is_null() {
        return std::ptr::null_mut();
    }
    let mut dim_names = Vec::new();
    for i in 0..count {
        let ptr = *dims.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        dim_names.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    let filter = Box::new(DbscanFilter::new(min_points as usize, eps, dim_names));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a covariance features filter stage.
///
/// # Safety
///
/// `dims` must be a valid pointer to a C-array of C-strings of length `dim_count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_covariancefeatures(
    knn: u64,
    has_radius: bool,
    radius: f64,
    min_k: u64,
    stride: u64,
    mode: u8,
    optimal: bool,
    dims: *const *const c_char,
    dim_count: u64,
) -> *mut StageWrapper {
    if dims.is_null() {
        return std::ptr::null_mut();
    }
    let mut feature_dims = Vec::new();
    for i in 0..dim_count {
        let ptr = *dims.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        feature_dims.push(dim_id_from_name(&CStr::from_ptr(ptr).to_string_lossy()));
    }
    let mode = match mode {
        1 => CovarianceFeaturesMode::Sqrt,
        2 => CovarianceFeaturesMode::Normalized,
        _ => CovarianceFeaturesMode::Raw,
    };
    let filter = Box::new(CovarianceFeaturesFilter::new(
        knn as usize,
        has_radius.then_some(radius),
        min_k as usize,
        stride as usize,
        mode,
        optimal,
        feature_dims,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a LOF filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_lof(minpts: u64) -> *mut StageWrapper {
    let filter = Box::new(LofFilter::new(minpts as usize));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a miniball filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_miniball(knn: u64) -> *mut StageWrapper {
    let filter = Box::new(MiniballFilter::new(knn as usize));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an ELM filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_elm(
    cell: f64,
    class_label: u8,
    threshold: f64,
) -> *mut StageWrapper {
    let filter = Box::new(ElmFilter::new(cell, class_label, threshold));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a skewness balancing filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_skewnessbalancing(
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
) -> *mut StageWrapper {
    let filter = Box::new(SkewnessBalancingFilter::new(
        ground_class,
        other_class,
        only_ground,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a splitter filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_splitter(
    length: f64,
    x_origin: f64,
    y_origin: f64,
    buffer: f64,
) -> *mut StageWrapper {
    let filter = Box::new(SplitterFilter::new(length, x_origin, y_origin, buffer));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an IQR filter stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_iqr(
    multiplier: f64,
    dim_name: *const c_char,
) -> *mut StageWrapper {
    if dim_name.is_null() {
        return std::ptr::null_mut();
    }
    let dim = dim_id_from_name(&CStr::from_ptr(dim_name).to_string_lossy());
    let filter = Box::new(IqrFilter::new(multiplier, dim));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a MAD filter stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_mad(
    multiplier: f64,
    dim_name: *const c_char,
    mad_multiplier: f64,
) -> *mut StageWrapper {
    if dim_name.is_null() {
        return std::ptr::null_mut();
    }
    let dim = dim_id_from_name(&CStr::from_ptr(dim_name).to_string_lossy());
    let filter = Box::new(MadFilter::new(multiplier, dim, mad_multiplier));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a HAG nearest-neighbor filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_hagnn(
    count: u64,
    max_distance: f64,
    allow_extrapolation: bool,
    class_label: u8,
) -> *mut StageWrapper {
    let filter = Box::new(HagNnFilter::new(
        count as usize,
        max_distance,
        allow_extrapolation,
        class_label,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a cluster filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_cluster(
    min_points: u64,
    max_points: u64,
    tolerance: f64,
    is_3d: bool,
) -> *mut StageWrapper {
    let filter = Box::new(ClusterFilter::new(
        min_points as usize,
        max_points as usize,
        tolerance,
        is_3d,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a sparse surface filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_sparsesurface(
    radius: f64,
    ground_class: u8,
    low_point_class: u8,
) -> *mut StageWrapper {
    let filter = Box::new(SparseSurfaceFilter::new(
        radius,
        ground_class,
        low_point_class,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a Li tree filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_litree(
    min_size: u64,
    min_hag: f64,
    dummy_radius: f64,
) -> *mut StageWrapper {
    let filter = Box::new(LiTreeFilter::new(min_size as usize, min_hag, dummy_radius));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a neighbor classifier filter stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C-string and `domain` must be a
/// valid pointer to an array of length `domain_count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_neighborclassifier(
    k: u64,
    dim_name: *const c_char,
    domain: *const pdal_range_limit_t,
    domain_count: u64,
) -> *mut StageWrapper {
    if dim_name.is_null() || (domain.is_null() && domain_count != 0) {
        return std::ptr::null_mut();
    }

    let dim_name = CStr::from_ptr(dim_name).to_string_lossy().into_owned();
    let mut ranges = Vec::new();
    for i in 0..domain_count {
        let limit = &*domain.offset(i as isize);
        if limit.dim_name.is_null() {
            return std::ptr::null_mut();
        }
        ranges.push(RangeLimit {
            dim_name: CStr::from_ptr(limit.dim_name)
                .to_string_lossy()
                .into_owned(),
            lower_bound: limit.lower_bound,
            upper_bound: limit.upper_bound,
            inclusive_lower: limit.inclusive_lower,
            inclusive_upper: limit.inclusive_upper,
            negate: limit.negate,
        });
    }

    let filter = Box::new(NeighborClassifierFilter::new(k as usize, dim_name, ranges));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

unsafe fn range_limits_from_ffi(
    limits: *const pdal_range_limit_t,
    count: u64,
) -> Option<Vec<RangeLimit>> {
    if limits.is_null() && count != 0 {
        return None;
    }

    let mut ranges = Vec::new();
    for i in 0..count {
        let limit = &*limits.offset(i as isize);
        if limit.dim_name.is_null() {
            return None;
        }
        ranges.push(RangeLimit {
            dim_name: CStr::from_ptr(limit.dim_name)
                .to_string_lossy()
                .into_owned(),
            lower_bound: limit.lower_bound,
            upper_bound: limit.upper_bound,
            inclusive_lower: limit.inclusive_lower,
            inclusive_upper: limit.inclusive_upper,
            negate: limit.negate,
        });
    }
    Some(ranges)
}

/// Get the point indices that filters.radiusassign should update.
/// Caller is responsible for freeing the returned buffer with pdal_free_u64_array.
///
/// # Safety
///
/// `view` must be valid. Range pointers must either be null with zero count or
/// valid arrays of the matching count. `out_len` must be valid.
#[no_mangle]
pub unsafe extern "C" fn pdal_radiusassign_get_update_indices(
    view: *mut PointView,
    src_domain: *const pdal_range_limit_t,
    src_count: u64,
    reference_domain: *const pdal_range_limit_t,
    reference_count: u64,
    radius: f64,
    search_3d: bool,
    max_2d_above: f64,
    max_2d_below: f64,
    out_len: *mut u64,
) -> *mut u64 {
    if view.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let Some(src) = range_limits_from_ffi(src_domain, src_count) else {
        return std::ptr::null_mut();
    };
    let Some(reference) = range_limits_from_ffi(reference_domain, reference_count) else {
        return std::ptr::null_mut();
    };

    let updates = radiusassign::update_indices(
        &*view,
        &src,
        &reference,
        radius,
        search_3d,
        max_2d_above,
        max_2d_below,
    );
    *out_len = updates.len() as u64;
    let mut boxed = updates.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ptr
}

/// Create a normal filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_normal(
    knn: u64,
    has_radius: bool,
    radius: f64,
    always_up: bool,
    has_viewpoint: bool,
    viewpoint_x: f64,
    viewpoint_y: f64,
    viewpoint_z: f64,
    refine: bool,
) -> *mut StageWrapper {
    let viewpoint = has_viewpoint.then_some([viewpoint_x, viewpoint_y, viewpoint_z]);
    let filter = Box::new(NormalFilter::new(
        knn as usize,
        has_radius.then_some(radius),
        always_up,
        viewpoint,
        refine,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Transform one point using filters.straighten's affine math.
///
/// # Safety
///
/// `out_xyz` must point to at least three writable doubles.
#[no_mangle]
pub unsafe extern "C" fn pdal_straighten_transform_point(
    reverse: bool,
    x: f64,
    y: f64,
    z: f64,
    segment_x: f64,
    segment_y: f64,
    segment_z: f64,
    segment_m: f64,
    segment_azimuth: f64,
    segment_offset: f64,
    offset: f64,
    out_xyz: *mut f64,
) -> bool {
    if out_xyz.is_null() {
        return false;
    }
    let segment = [segment_x, segment_y, segment_z, segment_m, segment_azimuth];
    let transformed = if reverse {
        straighten::unstraighten_point([x, y, z], segment)
    } else {
        straighten::straighten_point([x, y, z], segment, segment_offset, offset)
    };
    std::ptr::copy_nonoverlapping(transformed.as_ptr(), out_xyz, 3);
    true
}

#[repr(C)]
pub struct pdal_m3c2_stats_t {
    distance: f64,
    uncertainty: f64,
    significant: bool,
    std_dev1: f64,
    std_dev2: f64,
    n1: u64,
    n2: u64,
}

/// Compute M3C2 distance statistics for two candidate point sets.
///
/// # Safety
///
/// Point arrays must contain `count * 3` doubles. `out_stats` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pdal_m3c2_compute_stats(
    pts1: *const f64,
    pts1_count: u64,
    skip_first1: bool,
    pts2: *const f64,
    pts2_count: u64,
    skip_first2: bool,
    center: *const f64,
    normal: *const f64,
    cyl_radius2: f64,
    cyl_half_len: f64,
    min_points: u64,
    reg_error: f64,
    out_stats: *mut pdal_m3c2_stats_t,
) -> bool {
    if pts1.is_null()
        || pts2.is_null()
        || center.is_null()
        || normal.is_null()
        || out_stats.is_null()
    {
        return false;
    }

    let pts1 = std::slice::from_raw_parts(pts1, pts1_count as usize * 3);
    let pts2 = std::slice::from_raw_parts(pts2, pts2_count as usize * 3);
    let center = std::slice::from_raw_parts(center, 3);
    let normal = std::slice::from_raw_parts(normal, 3);
    let to_points = |values: &[f64]| -> Vec<[f64; 3]> {
        values
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect()
    };

    let Some(stats) = m3c2::compute_stats(
        &to_points(pts1),
        &to_points(pts2),
        m3c2::M3c2Config {
            skip_first1,
            skip_first2,
            center: [center[0], center[1], center[2]],
            normal: [normal[0], normal[1], normal[2]],
            cyl_radius2,
            cyl_half_len,
            min_points: min_points as usize,
            reg_error,
        },
    ) else {
        return false;
    };

    *out_stats = pdal_m3c2_stats_t {
        distance: stats.distance,
        uncertainty: stats.uncertainty,
        significant: stats.significant,
        std_dev1: stats.std_dev1,
        std_dev2: stats.std_dev2,
        n1: stats.n1,
        n2: stats.n2,
    };
    true
}

/// Create a voxel center nearest neighbor filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_voxelcenternearestneighbor(cell: f64) -> *mut StageWrapper {
    let filter = Box::new(VoxelCenterNearestNeighborFilter::new(cell));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a voxel centroid nearest neighbor filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_voxelcentroidnearestneighbor(cell: f64) -> *mut StageWrapper {
    let filter = Box::new(VoxelCentroidNearestNeighborFilter::new(cell));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a reciprocity filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_reciprocity(knn: u64) -> *mut StageWrapper {
    let filter = Box::new(ReciprocityFilter::new(knn as usize));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an estimate rank filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_estimaterank(knn: u64, threshold: f64) -> *mut StageWrapper {
    let filter = Box::new(EstimateRankFilter::new(knn as usize, threshold));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an approximate coplanar filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_approximatecoplanar(
    knn: u64,
    threshold1: f64,
    threshold2: f64,
) -> *mut StageWrapper {
    let filter = Box::new(ApproximateCoplanarFilter::new(
        knn as usize,
        threshold1,
        threshold2,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a plane fit filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_planefit(knn: u64) -> *mut StageWrapper {
    let filter = Box::new(PlaneFitFilter::new(knn as usize));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an eigenvalues filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_eigenvalues(
    knn: u64,
    normalize: bool,
    stride: u64,
    has_radius: bool,
    radius: f64,
    min_k: u64,
) -> *mut StageWrapper {
    let filter = Box::new(EigenvaluesFilter::new(
        knn as usize,
        normalize,
        stride as usize,
        has_radius.then_some(radius),
        min_k as usize,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an optimal neighborhood filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_optimalneighborhood(
    min_k: u64,
    max_k: u64,
) -> *mut StageWrapper {
    let filter = Box::new(OptimalNeighborhoodFilter::new(
        min_k as usize,
        max_k as usize,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Get the indices of the kept points in grid decimation.
/// Caller is responsible for freeing the returned buffer with pdal_free_u64_array.
///
/// # Safety
///
/// `view` and `output_type` must be valid.
#[no_mangle]
pub unsafe extern "C" fn pdal_grid_decimation_get_kept_indices(
    view: *const PointView,
    resolution: f64,
    output_type: *const c_char,
    out_len: *mut u64,
) -> *mut u64 {
    if view.is_null() || output_type.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let output_type_str = CStr::from_ptr(output_type).to_string_lossy();
    if let Some(pt_view) = view.as_ref() {
        let kept = griddecimation::get_kept_indices(pt_view, resolution, &output_type_str);
        *out_len = kept.len() as u64;
        let mut boxed_slice = kept.into_boxed_slice();
        let ptr = boxed_slice.as_mut_ptr();
        std::mem::forget(boxed_slice);
        ptr
    } else {
        std::ptr::null_mut()
    }
}

/// Free a u64 array allocated by Rust.
///
/// # Safety
///
/// `ptr` must be a valid pointer returned by a pdal allocator or null.
#[no_mangle]
pub unsafe extern "C" fn pdal_free_u64_array(ptr: *mut u64, len: u64) {
    if !ptr.is_null() {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len as usize));
    }
}

/// Create a divider filter stage.
///
/// # Safety
///
/// `evals` must be a valid pointer of length `evals_count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_divider(
    mode: i32,
    size_mode: i32,
    size: u64,
    evals: *const u8,
    evals_count: u64,
) -> *mut StageWrapper {
    let mode_enum = match mode {
        1 => divider::DividerMode::RoundRobin,
        2 => divider::DividerMode::Expression,
        _ => divider::DividerMode::Partition,
    };
    let size_mode_enum = match size_mode {
        1 => divider::DividerSizeMode::Capacity,
        _ => divider::DividerSizeMode::Count,
    };
    let mut vec_evals = Vec::new();
    if !evals.is_null() {
        let slice = std::slice::from_raw_parts(evals, evals_count as usize);
        vec_evals = slice.iter().map(|&b| b != 0).collect();
    }
    let filter = Box::new(divider::DividerFilter::new(
        mode_enum,
        size_mode_enum,
        size,
        vec_evals,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a chipper filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_chipper(capacity: u64) -> *mut StageWrapper {
    let filter = Box::new(ChipperFilter::new(capacity));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a Lloyd K-means filter stage.
///
/// # Safety
///
/// `dims` must be a valid pointer to a C-array of C-strings of length `dim_count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_lloydkmeans(
    k: u64,
    maxiters: u64,
    dims: *const *const c_char,
    dim_count: u64,
) -> *mut StageWrapper {
    if dims.is_null() {
        return std::ptr::null_mut();
    }
    let mut dimensions = Vec::new();
    for i in 0..dim_count {
        let ptr = *dims.offset(i as isize);
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        dimensions.push(dim_id_from_name(&CStr::from_ptr(ptr).to_string_lossy()));
    }
    let filter = Box::new(LloydKMeansFilter::new(
        k as usize,
        dimensions,
        maxiters as usize,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a farthestpointsampling filter stage.
///
/// # Safety
///
/// Always safe.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_farthestpointsampling(count: u64) -> *mut StageWrapper {
    let filter = Box::new(FarthestPointSamplingFilter::new(count));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Assign Range representation for FFI.
#[repr(C)]
pub struct pdal_assign_range_t {
    pub dim_name: *const c_char,
    pub value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
}

/// Create an assign filter stage.
///
/// # Safety
///
/// `cond_dim` must be null-terminated or null. `assignments` must be valid pointer of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_assign(
    has_condition: bool,
    cond_dim: *const c_char,
    cond_lower: f64,
    cond_upper: f64,
    cond_inclusive_lower: bool,
    cond_inclusive_upper: bool,
    cond_negate: bool,

    assignments: *const pdal_assign_range_t,
    count: u64,
) -> *mut StageWrapper {
    let condition = if has_condition && !cond_dim.is_null() {
        let name = CStr::from_ptr(cond_dim).to_string_lossy().into_owned();
        Some(assign::AssignCondition {
            dim_name: name,
            lower_bound: cond_lower,
            upper_bound: cond_upper,
            inclusive_lower: cond_inclusive_lower,
            inclusive_upper: cond_inclusive_upper,
            negate: cond_negate,
        })
    } else {
        None
    };

    let mut vec_assignments = Vec::new();
    if !assignments.is_null() {
        for i in 0..count {
            let r = &*assignments.offset(i as isize);
            if !r.dim_name.is_null() {
                let name = CStr::from_ptr(r.dim_name).to_string_lossy().into_owned();
                vec_assignments.push(assign::AssignRange {
                    dim_name: name,
                    value: r.value,
                    lower_bound: r.lower_bound,
                    upper_bound: r.upper_bound,
                    inclusive_lower: r.inclusive_lower,
                    inclusive_upper: r.inclusive_upper,
                    negate: r.negate,
                });
            }
        }
    }

    let filter = Box::new(assign::AssignFilter::new(condition, vec_assignments));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Dim Stats representation for FFI.
#[repr(C)]
pub struct pdal_dim_stats_t {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub m1: f64,
    pub m2: f64,
    pub m3: f64,
    pub m4: f64,
    pub median: f64,
    pub mad: f64,
    pub unique_values: *mut f64,
    pub unique_counts: *mut u64,
    pub unique_len: u64,
}

/// Compute statistics on a PointView.
///
/// # Safety
///
/// Pre-allocated arrays and bounds must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdal_stats_compute(
    view: *mut PointView,
    dims: *const *const c_char,
    dims_count: u64,
    advanced: bool,
    enums: *const *const c_char,
    enums_count: u64,
    counts: *const *const c_char,
    counts_count: u64,
    globals: *const *const c_char,
    globals_count: u64,
    out_stats: *mut pdal_dim_stats_t,
) {
    if view.is_null() || dims.is_null() || out_stats.is_null() {
        return;
    }
    let pt_view = &mut *view;

    let mut enum_names = std::collections::HashSet::new();
    for i in 0..enums_count {
        let ptr = *enums.offset(i as isize);
        if !ptr.is_null() {
            enum_names.insert(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    let mut count_names = std::collections::HashSet::new();
    for i in 0..counts_count {
        let ptr = *counts.offset(i as isize);
        if !ptr.is_null() {
            count_names.insert(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    let mut global_names = std::collections::HashSet::new();
    for i in 0..globals_count {
        let ptr = *globals.offset(i as isize);
        if !ptr.is_null() {
            global_names.insert(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    for i in 0..dims_count {
        let ptr = *dims.offset(i as isize);
        if ptr.is_null() {
            continue;
        }
        let dim_name = CStr::from_ptr(ptr).to_string_lossy().into_owned();

        let enum_type = if global_names.contains(&dim_name) {
            3
        } else if count_names.contains(&dim_name) {
            2
        } else if enum_names.contains(&dim_name) {
            1
        } else {
            0
        };

        let mut summary = stats::Summary::new(dim_name.clone(), enum_type, advanced);
        let dim_id = DimId::from_name(&dim_name);
        for pt_idx in 0..pt_view.len() {
            let val = pt_view.get_f64(pt_idx, &dim_id);
            summary.insert(val);
        }
        if enum_type == 3 {
            summary.compute_global_stats();
        }

        let mut unique_values_ptr = std::ptr::null_mut();
        let mut unique_counts_ptr = std::ptr::null_mut();
        let mut unique_len = 0;

        if enum_type == 1 || enum_type == 2 {
            let mut keys: Vec<f64> = summary
                .values
                .keys()
                .map(|&bits| f64::from_bits(bits))
                .collect();
            keys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            unique_len = keys.len() as u64;

            let mut vals = Vec::new();
            for &k in &keys {
                let bits = k.to_bits();
                vals.push(*summary.values.get(&bits).unwrap_or(&0));
            }

            let mut boxed_keys = keys.into_boxed_slice();
            unique_values_ptr = boxed_keys.as_mut_ptr();
            std::mem::forget(boxed_keys);

            let mut boxed_vals = vals.into_boxed_slice();
            unique_counts_ptr = boxed_vals.as_mut_ptr();
            std::mem::forget(boxed_vals);
        }

        *out_stats.offset(i as isize) = pdal_dim_stats_t {
            count: summary.cnt,
            min: summary.min,
            max: summary.max,
            m1: summary.m1,
            m2: summary.m2,
            m3: summary.m3,
            m4: summary.m4,
            median: summary.median,
            mad: summary.mad,
            unique_values: unique_values_ptr,
            unique_counts: unique_counts_ptr,
            unique_len,
        };
    }
}

/// Free the allocated arrays within `pdal_dim_stats_t`.
///
/// # Safety
///
/// Always safe if pointers match allocated memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_free_stats_arrays(ptr: *mut pdal_dim_stats_t, dims_count: u64) {
    if ptr.is_null() {
        return;
    }
    for i in 0..dims_count {
        let stats = &*ptr.offset(i as isize);
        if !stats.unique_values.is_null() {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                stats.unique_values,
                stats.unique_len as usize,
            ));
        }
        if !stats.unique_counts.is_null() {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                stats.unique_counts,
                stats.unique_len as usize,
            ));
        }
    }
}
