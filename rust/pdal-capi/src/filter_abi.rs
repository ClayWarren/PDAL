use crate::error::{clear_last_error, ffi_catch, set_last_error};
use crate::point_abi::dim_id_from_name;
use crate::stage_abi::StageWrapper;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::point::{PointLayout, PointView};
use pdal_filters::approximate_coplanar::ApproximateCoplanarFilter;
use pdal_filters::assign;
use pdal_filters::chipper::ChipperFilter;
use pdal_filters::cluster::ClusterFilter;
use pdal_filters::dbscan::DbscanFilter;
use pdal_filters::decimation::DecimationFilter;
use pdal_filters::divider;
use pdal_filters::eigenvalues::EigenvaluesFilter;
use pdal_filters::elm::ElmFilter;
use pdal_filters::estimate_rank::EstimateRankFilter;
use pdal_filters::expression::ExpressionFilter;
use pdal_filters::farthestpointsampling::FarthestPointSamplingFilter;
use pdal_filters::ferry::FerryFilter;
use pdal_filters::gpstimeconvert::GpsTimeConvert;
use pdal_filters::griddecimation;
use pdal_filters::groupby::GroupByFilter;
use pdal_filters::hagnn::HagNnFilter;
use pdal_filters::head::HeadFilter;
use pdal_filters::iqr::IqrFilter;
use pdal_filters::labelduplicates::LabelDuplicatesFilter;
use pdal_filters::locate::LocateFilter;
use pdal_filters::lof::LofFilter;
use pdal_filters::mad::MadFilter;
use pdal_filters::merge::MergeFilter;
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::nndistance::{NNDistanceFilter, NNDistanceMode};
use pdal_filters::optimal_neighborhood::OptimalNeighborhoodFilter;
use pdal_filters::outlier::OutlierFilter;
use pdal_filters::planefit::PlaneFitFilter;
use pdal_filters::radialdensity::RadialDensityFilter;
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
use pdal_filters::tail::TailFilter;
use pdal_filters::transformation::TransformationFilter;
use pdal_filters::voxel_center_nearest_neighbor::VoxelCenterNearestNeighborFilter;
use pdal_filters::voxel_centroid_nearest_neighbor::VoxelCentroidNearestNeighborFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;
use pdal_filters::zsmooth::ZsmoothFilter;
use std::ffi::CStr;
use std::os::raw::c_char;

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

/// Check if a point passes the RangeFilter limits.
///
/// # Safety
///
/// `stage` must be a valid pointer to a stage created with `pdal_stage_create_range`.
/// `view` must be a valid pointer to a `PointView`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_range_point_passes(
    stage: *mut StageWrapper,
    view: *mut PointView,
    idx: u64,
) -> bool {
    if let (Some(stage_wrapper), Some(pt_view)) = (stage.as_ref(), view.as_ref()) {
        if let Some(range) = stage_wrapper.filter.as_any().downcast_ref::<RangeFilter>() {
            return range.point_passes(pt_view, idx);
        }
    }
    false
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
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_process_one(stage: *mut StageWrapper) -> bool {
    ffi_catch(false, || {
        clear_last_error();
        if let Some(stage) = stage.as_mut() {
            // Counter-based filters (decimation, head, tail) ignore the point;
            // pass an empty view. Data-dependent filters must instead stream
            // through `pdal_stage_process_one_at`.
            let mut empty = PointView::new(std::rc::Rc::new(PointLayout::new()));
            stage.filter.process_one(&mut empty, 0)
        } else {
            set_last_error("null stage");
            false
        }
    })
}

/// Decide whether to keep point `idx` of `view` in streaming mode, passing the
/// point so the filter can inspect its data.
///
/// This is the faithful streaming entry point, mirroring PDAL's
/// `Streamable::processOne(PointRef&)`. Use it for any filter whose streaming
/// decision depends on point values (e.g. `filters.expression`).
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
/// `view` must be a valid pointer returned by `pdal_point_view_create` or
/// `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_process_one_at(
    stage: *mut StageWrapper,
    view: *mut PointView,
    idx: u64,
) -> bool {
    ffi_catch(false, || {
        clear_last_error();
        if let (Some(stage), Some(view)) = (stage.as_mut(), view.as_mut()) {
            stage.filter.process_one(view, idx)
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
            match stage.filter.run(std::slice::from_ref(input)) {
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
            match stage.filter.run(std::slice::from_ref(input)) {
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

/// Export a stage's accumulated metadata.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_metadata(stage: *const StageWrapper) -> *mut MetadataNode {
    if let Some(stage) = stage.as_ref() {
        Box::into_raw(Box::new(stage.filter.metadata()))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a `filters.expressionstats` stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C-string.
/// `exprs` must be a valid pointer to a C-array of `count` C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_expressionstats(
    dim_name: *const c_char,
    exprs: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    clear_last_error();
    if dim_name.is_null() || (count > 0 && exprs.is_null()) {
        set_last_error("null expressionstats input");
        return std::ptr::null_mut();
    }
    let dim_name = CStr::from_ptr(dim_name).to_string_lossy().into_owned();
    let mut sources = Vec::with_capacity(count as usize);
    for i in 0..count {
        let ptr = *exprs.offset(i as isize);
        if ptr.is_null() {
            set_last_error("null expression string");
            return std::ptr::null_mut();
        }
        sources.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    match pdal_filters::expressionstats::ExpressionStatsFilter::new(&dim_name, &sources) {
        Ok(f) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(f),
        })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a `filters.mongo` stage from a JSON expression string.
///
/// Returns null and sets the last error if `expr` is null or invalid JSON.
///
/// # Safety
///
/// `expr` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_mongoexpression(
    expr: *const c_char,
) -> *mut StageWrapper {
    clear_last_error();
    if expr.is_null() {
        set_last_error("null expression string");
        return std::ptr::null_mut();
    }
    let json_str = CStr::from_ptr(expr).to_string_lossy();
    match pdal_filters::mongo::MongoExpressionFilter::new(&json_str) {
        Ok(f) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(f),
        })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a `filters.expression` stage from a list of expression strings.
///
/// Returns null and sets the last error if `exprs` is null, contains a null
/// entry, or any expression fails to parse.
///
/// # Safety
///
/// `exprs` must be a valid pointer to a C-array of `count` C-strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_expression(
    exprs: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    clear_last_error();
    if exprs.is_null() {
        set_last_error("null expression array");
        return std::ptr::null_mut();
    }
    let mut sources = Vec::with_capacity(count as usize);
    for i in 0..count {
        let ptr = *exprs.offset(i as isize);
        if ptr.is_null() {
            set_last_error("null expression string");
            return std::ptr::null_mut();
        }
        sources.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    match ExpressionFilter::new(&sources) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
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
///
/// # Safety
///
/// This function is unsafe only to match the C ABI surface; it does not
/// dereference caller-provided pointers.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_radialdensity(radius: f64) -> *mut StageWrapper {
    let filter = Box::new(RadialDensityFilter::new(radius));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create an nndistance filter stage.
///
/// # Safety
///
/// `mode` must either be null or point to a valid NUL-terminated C string.
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

/// Create a LOF filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_lof(minpts: u64) -> *mut StageWrapper {
    let filter = Box::new(LofFilter::new(minpts as usize));
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

/// Create a splitter filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_splitter(
    length: f64,
    origin_x: f64,
    origin_y: f64,
    buffer: f64,
) -> *mut StageWrapper {
    let filter = Box::new(SplitterFilter::new(length, origin_x, origin_y, buffer));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a GPS time conversion filter stage.
///
/// # Safety
///
/// `ops` may be null or must point to a valid options object.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_gpstimeconvert(
    ops: *const Options,
) -> *mut StageWrapper {
    let options = ops.as_ref().cloned().unwrap_or_default();
    match GpsTimeConvert::from_options(&options) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(&err.0);
            std::ptr::null_mut()
        }
    }
}

/// Create a chipper filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_chipper(capacity: u64) -> *mut StageWrapper {
    let filter = Box::new(ChipperFilter::new(capacity));
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
