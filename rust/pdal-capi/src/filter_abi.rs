use crate::error::{set_last_error, string_to_c_ptr};
use crate::point_abi::dim_id_from_name;
use crate::stage_abi::StageWrapper;
use pdal_core::options::Options;
use pdal_core::point::PointView;
use pdal_filters::approximate_coplanar::ApproximateCoplanarFilter;
use pdal_filters::assign;
use pdal_filters::chipper::ChipperFilter;
use pdal_filters::cluster::ClusterFilter;
use pdal_filters::colorinterp::ColorinterpFilter;
use pdal_filters::colorization::{BandInfo, ColorizationFilter};
use pdal_filters::crop::{CropCenter, CropFilter};
use pdal_filters::dbscan::DbscanFilter;
use pdal_filters::decimation::DecimationFilter;
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
use pdal_filters::hag_dem::HagDemFilter;
use pdal_filters::hagnn::HagNnFilter;
use pdal_filters::head::HeadFilter;
use pdal_filters::iqr::IqrFilter;
use pdal_filters::labelduplicates::LabelDuplicatesFilter;
use pdal_filters::locate::LocateFilter;
use pdal_filters::lof::LofFilter;
use pdal_filters::mad::MadFilter;
use pdal_filters::merge::MergeFilter;
use pdal_filters::miniball::MiniballFilter;
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::neighborclassifier::NeighborClassifierFilter;
use pdal_filters::nndistance::{NNDistanceFilter, NNDistanceMode};
use pdal_filters::optimal_neighborhood::OptimalNeighborhoodFilter;
use pdal_filters::outlier::OutlierFilter;
use pdal_filters::overlay::OverlayFilter;
use pdal_filters::planefit::PlaneFitFilter;
use pdal_filters::proj_pipeline::ProjPipelineFilter;
use pdal_filters::radialdensity::RadialDensityFilter;
use pdal_filters::radiusassign::{RadiusAssignFilter, RadiusAssignment};
use pdal_filters::randomize::RandomizeFilter;
use pdal_filters::range::{parse_range_limit, RangeFilter, RangeLimit};
use pdal_filters::reciprocity::ReciprocityFilter;
use pdal_filters::returns::ReturnsFilter;
use pdal_filters::sample::SampleFilter;
use pdal_filters::separatescanline::SeparateScanLineFilter;
use pdal_filters::skewnessbalancing::SkewnessBalancingFilter;
use pdal_filters::smrf::SmrfFilter;
use pdal_filters::sort::{SortAlgorithm, SortFilter, SortOrder};
use pdal_filters::sparse_surface::SparseSurfaceFilter;
use pdal_filters::splitter::SplitterFilter;
use pdal_filters::tail::TailFilter;
use pdal_filters::transformation::{
    format_transformation_matrix, parse_transformation_matrix, TransformationFilter,
};
use pdal_filters::voxel_center_nearest_neighbor::VoxelCenterNearestNeighborFilter;
use pdal_filters::voxel_centroid_nearest_neighbor::VoxelCentroidNearestNeighborFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;
use pdal_filters::zsmooth::ZsmoothFilter;
use std::ffi::{c_char, CStr};

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

/// Create a crop filter stage.
///
/// # Safety
///
/// Array pointers must either be null with a zero count or valid for their
/// matching count.
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
    let mut rust_bounds = Vec::new();
    if !bounds.is_null() {
        for idx in 0..bounds_count {
            let b = &*bounds.add(idx as usize);
            rust_bounds.push((b.minx, b.miny, b.minz, b.maxx, b.maxy, b.maxz));
        }
    }

    let mut rust_polygons = Vec::new();
    if !polygons.is_null() {
        for idx in 0..poly_count {
            let ptr = *polygons.add(idx as usize);
            if !ptr.is_null() {
                rust_polygons.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
            }
        }
    }

    let mut rust_centers = Vec::new();
    if !centers.is_null() {
        for idx in 0..center_count {
            let c = &*centers.add(idx as usize);
            if c.z.is_finite() {
                rust_centers.push(CropCenter::new_3d(c.x, c.y, c.z));
            } else {
                rust_centers.push(CropCenter::new_2d(c.x, c.y));
            }
        }
    }

    match CropFilter::new(outside, rust_bounds, rust_polygons, rust_centers, distance) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(err.0);
            std::ptr::null_mut()
        }
    }
}

/// Create an overlay filter stage.
///
/// # Safety
///
/// String pointers must be null-terminated.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_overlay(
    dim_name: *const c_char,
    datasource: *const c_char,
    column: *const c_char,
) -> *mut StageWrapper {
    if dim_name.is_null() || datasource.is_null() {
        set_last_error("null argument to pdal_stage_create_overlay");
        return std::ptr::null_mut();
    }

    let dim_name = CStr::from_ptr(dim_name).to_string_lossy();
    let datasource = CStr::from_ptr(datasource).to_string_lossy();
    let column = if column.is_null() {
        "".into()
    } else {
        CStr::from_ptr(column).to_string_lossy()
    };

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(OverlayFilter::new(&dim_name, &datasource, &column)),
    }))
}

/// Create a color interpolation filter stage.
///
/// # Safety
///
/// String pointers must be null-terminated.
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
        set_last_error("null argument to pdal_stage_create_colorinterp");
        return std::ptr::null_mut();
    }

    let dim_name = CStr::from_ptr(dim_name).to_string_lossy();
    let ramp = CStr::from_ptr(ramp).to_string_lossy();

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(ColorinterpFilter::new(
            &dim_name, &ramp, min, max, clamp, invert,
        )),
    }))
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
/// `raster_path` and every band name must be null-terminated. `bands` must
/// either be null with a zero count or valid for `count` entries.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_colorization(
    raster_path: *const c_char,
    bands: *const pdal_band_info_t,
    count: u64,
) -> *mut StageWrapper {
    if raster_path.is_null() {
        set_last_error("null argument to pdal_stage_create_colorization");
        return std::ptr::null_mut();
    }

    let raster_path = CStr::from_ptr(raster_path).to_string_lossy();
    let mut rust_bands = Vec::new();
    if !bands.is_null() {
        for idx in 0..count {
            let band = &*bands.add(idx as usize);
            if band.name.is_null() {
                set_last_error("null band name to pdal_stage_create_colorization");
                return std::ptr::null_mut();
            }
            rust_bands.push(BandInfo {
                name: CStr::from_ptr(band.name).to_string_lossy().into_owned(),
                band: band.band,
                scale: band.scale,
            });
        }
    }

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(ColorizationFilter::new(&raster_path, rust_bands)),
    }))
}

#[no_mangle]
pub extern "C" fn pdal_stage_create_h3(resolution: u64) -> *mut StageWrapper {
    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(H3Filter::new(resolution as u8)),
    }))
}

/// Create a HAG DEM filter stage.
///
/// # Safety
///
/// `raster_path` must be a null-terminated string.
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
        set_last_error("null argument to pdal_stage_create_hag_dem");
        return std::ptr::null_mut();
    }

    let raster_path = CStr::from_ptr(raster_path).to_string_lossy();
    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(HagDemFilter::new(
            &raster_path,
            band,
            zero_ground,
            min_clamp,
            max_clamp,
            nodata_height,
            ground_class,
        )),
    }))
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

/// Create a ferry filter stage from dimension specification strings.
///
/// # Safety
///
/// `specs` must be a valid array of null-terminated strings of length `count`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_ferry_specs(
    specs: *const *const std::os::raw::c_char,
    count: u64,
) -> *mut StageWrapper {
    if specs.is_null() {
        set_last_error("null argument to pdal_stage_create_ferry_specs");
        return std::ptr::null_mut();
    }
    let mut spec_strings = Vec::new();
    for i in 0..count {
        let spec_ptr = *specs.offset(i as isize);
        if spec_ptr.is_null() {
            set_last_error("null ferry dimension specification");
            return std::ptr::null_mut();
        }
        spec_strings.push(CStr::from_ptr(spec_ptr).to_string_lossy().into_owned());
    }
    match FerryFilter::parse_specs(&spec_strings) {
        Ok(dims) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(FerryFilter::new(dims)),
        })),
        Err(err) => {
            set_last_error(&err);
            std::ptr::null_mut()
        }
    }
}

/// Validate an assign filter value expression.
///
/// # Safety
///
/// `statement` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_validate_assign_statement(
    statement: *const std::os::raw::c_char,
) -> bool {
    if statement.is_null() {
        set_last_error("null assign statement");
        return false;
    }
    let statement = CStr::from_ptr(statement).to_string_lossy();
    match pdal_core::expr::AssignStatement::parse(&statement) {
        Ok(_) => true,
        Err(err) => {
            set_last_error(&err);
            false
        }
    }
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

/// Parse a PDAL range limit expression such as `Z[1:5]`.
///
/// # Safety
///
/// Output pointers must be valid when non-null. `out_dim_name` receives an
/// allocated string that must be freed with `pdal_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_range_limit_parse(
    input: *const c_char,
    out_dim_name: *mut *mut c_char,
    lower_bound: *mut f64,
    upper_bound: *mut f64,
    inclusive_lower: *mut bool,
    inclusive_upper: *mut bool,
    negate: *mut bool,
    consumed: *mut u64,
) -> *mut c_char {
    if input.is_null() {
        return string_to_c_ptr("Missing range limit.".to_string());
    }
    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_range_limit(&input) {
        Ok(parsed) => {
            if let Some(out_dim_name) = out_dim_name.as_mut() {
                *out_dim_name = string_to_c_ptr(parsed.dim_name);
            }
            if let Some(lower_bound) = lower_bound.as_mut() {
                *lower_bound = parsed.lower_bound;
            }
            if let Some(upper_bound) = upper_bound.as_mut() {
                *upper_bound = parsed.upper_bound;
            }
            if let Some(inclusive_lower) = inclusive_lower.as_mut() {
                *inclusive_lower = parsed.inclusive_lower;
            }
            if let Some(inclusive_upper) = inclusive_upper.as_mut() {
                *inclusive_upper = parsed.inclusive_upper;
            }
            if let Some(negate) = negate.as_mut() {
                *negate = parsed.negate;
            }
            if let Some(consumed) = consumed.as_mut() {
                *consumed = parsed.consumed as u64;
            }
            std::ptr::null_mut()
        }
        Err(error) => string_to_c_ptr(error),
    }
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
    let order_enum = match order_str.to_ascii_lowercase().as_str() {
        "asc" => SortOrder::Asc,
        "desc" => SortOrder::Desc,
        _ => {
            set_last_error(format!("Invalid sort order '{order_str}'."));
            return std::ptr::null_mut();
        }
    };

    let alg_str = CStr::from_ptr(algorithm).to_string_lossy();
    let alg_enum = match alg_str.to_ascii_lowercase().as_str() {
        "normal" => SortAlgorithm::Normal,
        "stable" => SortAlgorithm::Stable,
        _ => {
            set_last_error(format!("Invalid sort algorithm '{alg_str}'."));
            return std::ptr::null_mut();
        }
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
        match SampleFilter::new(options) {
            Ok(filter) => Box::into_raw(Box::new(StageWrapper {
                filter: Box::new(filter),
            })),
            Err(err) => {
                set_last_error(&err);
                std::ptr::null_mut()
            }
        }
    } else {
        std::ptr::null_mut()
    }
}

/// Create a faceraster filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_faceraster(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let filter = Box::new(FaceRasterFilter::new(options));
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
    if dim_name.eq_ignore_ascii_case("Z") {
        set_last_error("Can't use 'Z' as output dimension.");
        return std::ptr::null_mut();
    }
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

/// Create an SMRF stage.
///
/// # Safety
///
/// `returns` must either be null with a zero count or point to `count`
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_smrf(
    cell: f64,
    slope: f64,
    has_window: bool,
    window: f64,
    scalar: f64,
    threshold: f64,
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
    returns: *const *const c_char,
    count: u64,
) -> *mut StageWrapper {
    let mut rust_returns = Vec::new();
    if !returns.is_null() {
        for i in 0..count {
            let ptr = *returns.offset(i as isize);
            if ptr.is_null() {
                return std::ptr::null_mut();
            }
            rust_returns.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    let filter = Box::new(SmrfFilter::new(
        cell,
        slope,
        if has_window { Some(window) } else { None },
        scalar,
        threshold,
        ground_class,
        other_class,
        only_ground,
        rust_returns,
    ));
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
    if ground_class == low_point_class {
        set_last_error("Ground and low point class cannot be equal.");
        return std::ptr::null_mut();
    }
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

/// Create a miniball filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_miniball(knn: u64) -> *mut StageWrapper {
    let filter = Box::new(MiniballFilter::new(knn));
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
            rust_assignments.push(RadiusAssignment {
                dim_name: CStr::from_ptr(assignment.dim_name)
                    .to_string_lossy()
                    .into_owned(),
                value: assignment.value,
            });
        }
    }

    if assignment_count == 0 {
        set_last_error(
            "Empty 'update_epxression' option, must be set to apply any change on the data",
        );
        return std::ptr::null_mut();
    }

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(RadiusAssignFilter::new(
            src_domain,
            reference_domain,
            rust_assignments,
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
