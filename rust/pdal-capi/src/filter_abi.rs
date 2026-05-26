use crate::error::{set_last_error, string_to_c_ptr};
use crate::point_abi::dim_id_from_name;
use crate::stage_abi::StageWrapper;
use pdal_core::georeference::{validate_coordinate_system, validate_transform_beam_layout};
use pdal_core::options::Options;
use pdal_core::point::PointLayout;
use pdal_core::point::PointView;
use pdal_core::point::{DimId, DimType};
use pdal_filters::approximate_coplanar::ApproximateCoplanarFilter;
use pdal_filters::assign;
use pdal_filters::chipper::ChipperFilter;
use pdal_filters::cluster::ClusterFilter;
use pdal_filters::colorinterp::{pipeline_streamable, validate_prepared, ColorinterpFilter};
use pdal_filters::colorization::{BandInfo, ColorizationFilter};
use pdal_filters::covariancefeatures::{CovarianceFeaturesFilter, Mode as CovarianceMode};
use pdal_filters::crop::{CropCenter, CropFilter};
use pdal_filters::csf::CsfFilter;
use pdal_filters::dbscan::DbscanFilter;
use pdal_filters::decimation::DecimationFilter;
use pdal_filters::dem::DEMFilter;
use pdal_filters::divider;
use pdal_filters::eigenvalues::EigenvaluesFilter;
use pdal_filters::elm::ElmFilter;
use pdal_filters::estimate_rank::EstimateRankFilter;
use pdal_filters::faceraster::FaceRasterFilter;
use pdal_filters::farthestpointsampling::FarthestPointSamplingFilter;
use pdal_filters::ferry::FerryFilter;
use pdal_filters::geom_distance::GeomDistanceFilter;
use pdal_filters::gpstimeconvert::GpsTimeConvert;
use pdal_filters::groupby::GroupByFilter;
use pdal_filters::h3::H3Filter;
use pdal_filters::hag_delaunay::HagDelaunayFilter;
use pdal_filters::hag_dem::HagDemFilter;
use pdal_filters::hagnn::HagNnFilter;
use pdal_filters::head::HeadFilter;
use pdal_filters::hexbin::HexBinFilter;
use pdal_filters::iqr::IqrFilter;
use pdal_filters::labelduplicates::LabelDuplicatesFilter;
use pdal_filters::litree::LiTreeFilter;
use pdal_filters::lloydkmeans::LloydKMeansFilter;
use pdal_filters::locate::LocateFilter;
use pdal_filters::lof::LofFilter;
use pdal_filters::m3c2::{M3C2Filter, NormalOrientation as M3C2NormalOrientation};
use pdal_filters::mad::MadFilter;
use pdal_filters::merge::MergeFilter;
use pdal_filters::miniball::MiniballFilter;
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::neighborclassifier::NeighborClassifierFilter;
use pdal_filters::nndistance::{NNDistanceFilter, NNDistanceMode};
use pdal_filters::normal::NormalFilter;
use pdal_filters::optimal_neighborhood::OptimalNeighborhoodFilter;
use pdal_filters::outlier::OutlierFilter;
use pdal_filters::overlay::OverlayFilter;
use pdal_filters::planefit::PlaneFitFilter;
use pdal_filters::pmf::PmfFilter;
use pdal_filters::proj_pipeline::ProjPipelineFilter;
use pdal_filters::radialdensity::RadialDensityFilter;
use pdal_filters::radiusassign::{parse_assignments, RadiusAssignFilter};
use pdal_filters::randomize::RandomizeFilter;
use pdal_filters::range::{parse_range_limit, RangeFilter, RangeLimit};
use pdal_filters::reciprocity::ReciprocityFilter;
use pdal_filters::relaxation_dart_throwing::RelaxationDartThrowingFilter;
use pdal_filters::returns::ReturnsFilter;
use pdal_filters::sample::SampleFilter;
use pdal_filters::separatescanline::SeparateScanLineFilter;
use pdal_filters::skewnessbalancing::SkewnessBalancingFilter;
use pdal_filters::smrf::SmrfFilter;
use pdal_filters::sort::{SortAlgorithm, SortFilter, SortOrder};
use pdal_filters::sparse_surface::SparseSurfaceFilter;
use pdal_filters::splitter::SplitterFilter;
use pdal_filters::straighten::StraightenFilter;
use pdal_filters::supervoxel::SupervoxelFilter;
use pdal_filters::tail::TailFilter;
use pdal_filters::transformation::{
    format_transformation_matrix, parse_transformation_matrix, TransformationFilter,
};
use pdal_filters::voxel_center_nearest_neighbor::VoxelCenterNearestNeighborFilter;
use pdal_filters::voxel_centroid_nearest_neighbor::VoxelCentroidNearestNeighborFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;
use pdal_filters::zsmooth::ZsmoothFilter;
use std::ffi::{c_char, CStr};

unsafe fn c_string_array(values: *const *const c_char, count: u64) -> Result<Vec<String>, ()> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if values.is_null() {
        return Err(());
    }

    let mut out = Vec::new();
    for idx in 0..count {
        let ptr = *values.offset(idx as isize);
        if ptr.is_null() {
            return Err(());
        }
        out.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    Ok(out)
}

/// Create a geomdistance filter stage.
///
/// `wkt` is the candidate geometry; `ring` demotes polygons to their boundary
/// so distances measure against the polygon's edge.
///
/// # Safety
///
/// `wkt` and `dim_name` must be valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_geomdistance(
    wkt: *const c_char,
    dim_name: *const c_char,
    ring: bool,
) -> *mut StageWrapper {
    if wkt.is_null() || dim_name.is_null() {
        set_last_error("null argument to pdal_stage_create_geomdistance");
        return std::ptr::null_mut();
    }
    let wkt = CStr::from_ptr(wkt).to_string_lossy().into_owned();
    let dim_name = CStr::from_ptr(dim_name).to_string_lossy().into_owned();
    match GeomDistanceFilter::new(&wkt, &dim_name, ring) {
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
/// `out_srs` and `coord_op` must be valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_projpipeline(
    out_srs: *const c_char,
    coord_op: *const c_char,
    reverse: bool,
) -> *mut StageWrapper {
    if out_srs.is_null() || coord_op.is_null() {
        set_last_error("null argument to pdal_stage_create_projpipeline");
        return std::ptr::null_mut();
    }

    let out_srs = CStr::from_ptr(out_srs).to_string_lossy();
    let coord_op = CStr::from_ptr(coord_op).to_string_lossy();
    let filter = Box::new(ProjPipelineFilter::new(&out_srs, &coord_op, reverse));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a groupby filter stage.
///
/// # Safety
///
/// `dim_name` must be a valid NUL-terminated C string.
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

/// Parse a 4x4 transformation matrix from whitespace-separated text.
///
/// # Safety
///
/// `out_matrix` must point to at least 16 float64 values when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_transformation_matrix_parse(
    input: *const c_char,
    out_matrix: *mut f64,
) -> *mut c_char {
    if input.is_null() || out_matrix.is_null() {
        return string_to_c_ptr("Missing transformation matrix.".to_string());
    }

    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_transformation_matrix(&input) {
        Ok(matrix) => {
            std::ptr::copy_nonoverlapping(matrix.as_ptr(), out_matrix, 16);
            std::ptr::null_mut()
        }
        Err(err) => string_to_c_ptr(err),
    }
}

/// Format a 4x4 transformation matrix for PDAL option streams.
///
/// # Safety
///
/// `matrix` must point to at least 16 float64 values when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_transformation_matrix_format(matrix: *const f64) -> *mut c_char {
    if matrix.is_null() {
        return string_to_c_ptr(String::new());
    }

    let mut values = [0.0f64; 16];
    std::ptr::copy_nonoverlapping(matrix, values.as_mut_ptr(), 16);
    string_to_c_ptr(format_transformation_matrix(&values))
}

/// Validate a georeference filter coordinate system option.
///
/// # Safety
///
/// `coordinate_system` must be a null-terminated string when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_georeference_validate_coordinate_system(
    coordinate_system: *const c_char,
) -> *mut c_char {
    if coordinate_system.is_null() {
        return string_to_c_ptr("Missing coordinate system.".to_string());
    }

    let coordinate_system = CStr::from_ptr(coordinate_system).to_string_lossy();
    match validate_coordinate_system(&coordinate_system) {
        Ok(()) => std::ptr::null_mut(),
        Err(err) => string_to_c_ptr(err),
    }
}

/// Validate georeference beam-dimension requirements against a layout.
///
/// # Safety
///
/// `layout` must be a valid pointer when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_georeference_validate_transform_beam(
    layout: *const PointLayout,
    transform_beam: bool,
) -> *mut c_char {
    if layout.is_null() {
        return string_to_c_ptr("Missing point layout.".to_string());
    }

    match validate_transform_beam_layout(&*layout, transform_beam) {
        Ok(()) => std::ptr::null_mut(),
        Err(err) => string_to_c_ptr(err),
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
    if size_mode_enum == divider::DividerSizeMode::Capacity && size == 0 {
        set_last_error("Option 'capacity' must be greater than 0.");
        return std::ptr::null_mut();
    }
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

/// Create a radiusassign filter stage.
///
/// # Safety
///
/// Range and assignment pointers must be valid for their matching counts, or
/// null with a zero count.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_radiusassign(
    src_limits: *const pdal_range_limit_t,
    src_count: u64,
    reference_limits: *const pdal_range_limit_t,
    reference_count: u64,
    assignments: *const pdal_assign_range_t,
    assignment_count: u64,
    radius: f64,
    search_3d: bool,
    max_2d_above: f64,
    max_2d_below: f64,
) -> *mut StageWrapper {
    unsafe fn collect_limits(
        limits: *const pdal_range_limit_t,
        count: u64,
    ) -> Option<Vec<RangeLimit>> {
        let mut out = Vec::new();
        if limits.is_null() {
            return (count == 0).then_some(out);
        }
        for idx in 0..count {
            let limit = &*limits.add(idx as usize);
            if limit.dim_name.is_null() {
                return None;
            }
            out.push(RangeLimit {
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
        Some(out)
    }

    let Some(src_domain) = collect_limits(src_limits, src_count) else {
        set_last_error("invalid source domain passed to pdal_stage_create_radiusassign");
        return std::ptr::null_mut();
    };
    let Some(reference_domain) = collect_limits(reference_limits, reference_count) else {
        set_last_error("invalid reference domain passed to pdal_stage_create_radiusassign");
        return std::ptr::null_mut();
    };

    let mut rust_assignments = Vec::new();
    if !assignments.is_null() {
        for idx in 0..assignment_count {
            let assignment = &*assignments.add(idx as usize);
            if assignment.dim_name.is_null() {
                set_last_error("invalid assignment passed to pdal_stage_create_radiusassign");
                return std::ptr::null_mut();
            }
            let dim_name = CStr::from_ptr(assignment.dim_name).to_string_lossy();
            rust_assignments.push(format!("{dim_name} = {}", assignment.value));
        }
    }

    let mut layout = PointLayout::default();
    for expression in &rust_assignments {
        if let Some((dim, _)) = expression.split_once('=') {
            layout.register(DimId::from_name(dim.trim()), DimType::F64);
        }
    }
    let assignments = match parse_assignments(&rust_assignments, &layout) {
        Ok(assignments) => assignments,
        Err(err) => {
            set_last_error(&err.0);
            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(RadiusAssignFilter::new(
            src_domain,
            reference_domain,
            assignments,
            radius,
            search_3d,
            max_2d_above,
            max_2d_below,
        )),
    }))
}

/// Create a radiusassign filter stage from parsed update expressions.
///
/// # Safety
///
/// Range pointers and assignment string pointers must be valid for their
/// matching counts, or null with a zero count.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_radiusassign_expr(
    src_limits: *const pdal_range_limit_t,
    src_count: u64,
    reference_limits: *const pdal_range_limit_t,
    reference_count: u64,
    assignment_exprs: *const *const c_char,
    assignment_count: u64,
    radius: f64,
    search_3d: bool,
    max_2d_above: f64,
    max_2d_below: f64,
    view: *const PointView,
) -> *mut StageWrapper {
    let Some(view) = view.as_ref() else {
        set_last_error("null view passed to pdal_stage_create_radiusassign_expr");
        return std::ptr::null_mut();
    };

    unsafe fn collect_limits(
        limits: *const pdal_range_limit_t,
        count: u64,
    ) -> Option<Vec<RangeLimit>> {
        let mut out = Vec::new();
        if limits.is_null() {
            return (count == 0).then_some(out);
        }

        for idx in 0..count {
            let limit = &*limits.add(idx as usize);
            if limit.dim_name.is_null() {
                return None;
            }
            out.push(RangeLimit {
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
        Some(out)
    }

    let Some(src_domain) = collect_limits(src_limits, src_count) else {
        set_last_error("invalid source domain passed to pdal_stage_create_radiusassign_expr");
        return std::ptr::null_mut();
    };
    let Some(reference_domain) = collect_limits(reference_limits, reference_count) else {
        set_last_error("invalid reference domain passed to pdal_stage_create_radiusassign_expr");
        return std::ptr::null_mut();
    };

    let Ok(expressions) = c_string_array(assignment_exprs, assignment_count) else {
        set_last_error(
            "invalid assignment expressions passed to pdal_stage_create_radiusassign_expr",
        );
        return std::ptr::null_mut();
    };
    let assignments = match parse_assignments(&expressions, view.layout().as_ref()) {
        Ok(assignments) => assignments,
        Err(err) => {
            set_last_error(&err.0);
            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(RadiusAssignFilter::new(
            src_domain,
            reference_domain,
            assignments,
            radius,
            search_3d,
            max_2d_above,
            max_2d_below,
        )),
    }))
}

/// Create a neighborclassifier filter stage.
///
/// # Safety
///
/// `domain` must be valid for `domain_count` entries, or null with a zero
/// count. `dim_name` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_neighborclassifier(
    domain: *const pdal_range_limit_t,
    domain_count: u64,
    k: u64,
    dim_name: *const c_char,
) -> *mut StageWrapper {
    let mut rust_domain = Vec::new();
    if domain.is_null() {
        if domain_count != 0 {
            set_last_error("invalid domain passed to pdal_stage_create_neighborclassifier");
            return std::ptr::null_mut();
        }
    } else {
        for idx in 0..domain_count {
            let limit = &*domain.add(idx as usize);
            if limit.dim_name.is_null() {
                set_last_error("invalid domain passed to pdal_stage_create_neighborclassifier");
                return std::ptr::null_mut();
            }
            rust_domain.push(RangeLimit {
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
    }

    let dim_name = if dim_name.is_null() {
        "Classification".to_string()
    } else {
        CStr::from_ptr(dim_name).to_string_lossy().into_owned()
    };

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(NeighborClassifierFilter::new(
            rust_domain,
            k as usize,
            dim_name,
        )),
    }))
}

/// Create a CSF guard-stage for option validation and empty input behavior.
///
/// The full cloth simulation algorithm remains in C++; this Rust stage covers
/// the pre-processing semantics currently exercised by the C++ CSF unit tests.
///
/// # Safety
/// `ignored_dims` must be null with a zero count or point to `count`
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_csf(
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
    ignored_dims: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    let ignored_dims = match c_string_array(ignored_dims, count) {
        Ok(dims) => dims.into_iter().map(|dim| DimId::from_name(&dim)).collect(),
        Err(()) => {
            set_last_error("invalid ignored dimension list passed to pdal_stage_create_csf");
            return std::ptr::null_mut();
        }
    };

    match CsfFilter::new(ground_class, other_class, only_ground, ignored_dims) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Classify a flat XYZ point buffer with the CSF cloth-simulation algorithm
/// and write a per-point ground mask into `out_ground` (1 == ground, 0 ==
/// non-ground). The caller is responsible for ensuring `xyz` has length
/// `count * 3` (interleaved x,y,z,x,y,z,...) and `out_ground` has length
/// `count`. Returns 0 on success, -1 on error.
///
/// # Safety
/// `xyz` must point to a readable buffer of `count * 3` `f64` values, and
/// `out_ground` must point to a writable buffer of `count` `u8` values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pdal_filter_csf_classify(
    xyz: *const f64,
    count: u64,
    smooth: bool,
    time_step: f64,
    class_threshold: f64,
    height_threshold: f64,
    cloth_resolution: f64,
    rigidness: i32,
    iterations: i32,
    out_ground: *mut u8,
) -> i32 {
    use pdal_filters::csf_algorithm::{classify_ground, CsfParams, CsfPoint};

    if count == 0 {
        return 0;
    }
    if xyz.is_null() || out_ground.is_null() {
        set_last_error("pdal_filter_csf_classify: null buffer");
        return -1;
    }
    if cloth_resolution <= 0.0 {
        set_last_error("filters.csf: cloth_resolution must be positive");
        return -1;
    }
    if iterations <= 0 {
        set_last_error("filters.csf: iterations must be positive");
        return -1;
    }
    if rigidness < 0 {
        set_last_error("filters.csf: rigidness must be non-negative");
        return -1;
    }
    let count = count as usize;
    let xyz_slice = std::slice::from_raw_parts(xyz, count * 3);
    let points: Vec<CsfPoint> = (0..count)
        .map(|i| CsfPoint {
            x: xyz_slice[i * 3],
            y: xyz_slice[i * 3 + 1],
            z: xyz_slice[i * 3 + 2],
        })
        .collect();
    let params = CsfParams {
        smooth,
        time_step,
        class_threshold,
        height_threshold,
        cloth_resolution,
        rigidness,
        iterations,
    };
    let result = classify_ground(&points, &params);
    let out = std::slice::from_raw_parts_mut(out_ground, count);
    for byte in out.iter_mut() {
        *byte = 0;
    }
    for &gi in &result.ground_indices {
        if gi < count {
            out[gi] = 1;
        }
    }
    0
}

/// Validate input-normal dimensions for `filters.poisson`. Returns 0 if the
/// input layout is acceptable (either all three NormalX/Y/Z present, or none
/// present so the filter can register them), and -1 with an error string when
/// only some normals are present.
#[no_mangle]
pub extern "C" fn pdal_filter_poisson_validate_normals(
    has_normal_x: bool,
    has_normal_y: bool,
    has_normal_z: bool,
) -> i32 {
    let any = has_normal_x || has_normal_y || has_normal_z;
    let all = has_normal_x && has_normal_y && has_normal_z;
    if any && !all {
        set_last_error(
            "If normals are provided as part of the input dataset, all of X, Y and Z must be provided.",
        );
        return -1;
    }
    0
}

/// Whether `filters.poisson` must register NormalX/Y/Z given the input layout.
/// True when none of the three normal dimensions are present.
#[no_mangle]
pub extern "C" fn pdal_filter_poisson_needs_normal_dims(
    has_normal_x: bool,
    has_normal_y: bool,
    has_normal_z: bool,
) -> bool {
    !(has_normal_x || has_normal_y || has_normal_z)
}

/// Validate `filters.greedyprojection` options. The C++ filter requires
/// `multiplier > 0` and `radius > 0`. Returns 0 on success, -1 with an error
/// string otherwise.
#[no_mangle]
pub extern "C" fn pdal_filter_greedyprojection_validate_options(
    multiplier: f64,
    radius: f64,
) -> i32 {
    if multiplier <= 0.0 || multiplier.is_nan() {
        set_last_error("Option 'multiplier' must be greater than 0.");
        return -1;
    }
    if radius <= 0.0 || radius.is_nan() {
        set_last_error("Option 'radius' must be greater than 0.");
        return -1;
    }
    0
}

#[path = "filter_abi_geo.rs"]
mod filter_abi_geo;
pub use filter_abi_geo::*;

#[path = "filter_abi_basic.rs"]
mod filter_abi_basic;
pub use filter_abi_basic::*;

#[path = "filter_abi_spatial_stats.rs"]
mod filter_abi_spatial_stats;
pub use filter_abi_spatial_stats::*;
