use super::*;
use pdal_filters::hexer::{H3Grid, HexGrid, HexId};
use std::os::raw::{c_char, c_int};

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

/// Create a hexbin filter stage from options.
///
/// # Safety
///
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_hexbin(ops: *const Options) -> *mut StageWrapper {
    if let Some(options) = ops.as_ref() {
        let edge = if options.has("edge_length") {
            Some(options.get_f64("edge_length", 0.0))
        } else if options.has("edge_size") {
            Some(options.get_f64("edge_size", 0.0))
        } else {
            None
        };
        let density = options.get_str("density", "");
        let boundary = options.get_str("boundary", "");
        let driver = options.get_str("ogrdriver", "GeoJSON");
        let layer_name = options.get_str("lyr_name", "hexbins");
        let mut filter = HexBinFilter::with_options(
            edge,
            options.get_u64("threshold", 15) as u32,
            options.get_u64("sample_size", 5000) as usize,
            (!density.is_empty()).then_some(density),
            (!boundary.is_empty()).then_some(boundary),
            options.get_bool("output_tesselation", false),
        );
        filter.set_output_driver(driver);
        filter.set_layer_name(layer_name);
        // H3 grid mode: `h3_resolution` is only forwarded when explicitly set
        // (>= 0); its absence means auto-estimate the resolution from a sample.
        if options.get_bool("h3_grid", false) {
            let resolution = options
                .has("h3_resolution")
                .then(|| options.get_u64("h3_resolution", 0) as u8);
            filter.set_h3(resolution);
        }
        Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        }))
    } else {
        std::ptr::null_mut()
    }
}

/// Build a HexGrid WKT boundary from packed `(i, j)` hex coordinates.
///
/// # Safety
///
/// `hexes` must point to `pair_count * 2` valid `int32_t` values.
#[no_mangle]
pub unsafe extern "C" fn pdal_hexgrid_wkt(
    height: f64,
    dense_limit: c_int,
    hexes: *const c_int,
    pair_count: u64,
    precision: u64,
) -> *mut c_char {
    if hexes.is_null() && pair_count != 0 {
        set_last_error("Missing hex coordinates.");
        return std::ptr::null_mut();
    }

    let raw = std::slice::from_raw_parts(hexes, pair_count as usize * 2);
    let ids: Vec<HexId> = raw
        .chunks_exact(2)
        .map(|pair| HexId::new(pair[0], pair[1]))
        .collect();
    let mut grid = HexGrid::with_height(height, dense_limit);
    grid.set_hexes(&ids);
    match grid.find_shapes() {
        Ok(()) => {
            grid.find_parent_paths();
            grid.sort_paths();
            string_to_c_ptr(grid.to_wkt(precision as usize))
        }
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

/// Build an H3Grid WKT boundary from packed `(i, j)` H3 local coordinates.
///
/// # Safety
///
/// `hexes` must point to `pair_count * 2` valid `int32_t` values.
#[no_mangle]
pub unsafe extern "C" fn pdal_h3grid_wkt(
    resolution: u8,
    dense_limit: c_int,
    origin_lat_degrees: f64,
    origin_lng_degrees: f64,
    hexes: *const c_int,
    pair_count: u64,
    precision: u64,
) -> *mut c_char {
    if hexes.is_null() && pair_count != 0 {
        set_last_error("Missing H3 hex coordinates.");
        return std::ptr::null_mut();
    }

    let raw = std::slice::from_raw_parts(hexes, pair_count as usize * 2);
    let ids: Vec<HexId> = raw
        .chunks_exact(2)
        .map(|pair| HexId::new(pair[0], pair[1]))
        .collect();
    let origin =
        match H3Grid::origin_from_degrees(origin_lat_degrees, origin_lng_degrees, resolution) {
            Ok(origin) => origin,
            Err(err) => {
                set_last_error(err);
                return std::ptr::null_mut();
            }
        };
    let mut grid = match H3Grid::new(resolution, dense_limit, origin) {
        Ok(grid) => grid,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };
    grid.set_hexes(&ids);
    match grid.find_shapes() {
        Ok(()) => {
            grid.find_parent_paths();
            grid.sort_paths();
            string_to_c_ptr(grid.to_wkt(precision as usize))
        }
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
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

/// A dimension range (`Dim[lo:hi]`) passed across the C ABI by component,
/// mirroring the C++ `DimRange` fields. Used by the SMRF `ignore` option so the
/// C++ wrapper doesn't have to re-serialize parsed ranges to strings.
#[repr(C)]
pub struct pdal_dim_range_t {
    pub dim_name: *const c_char,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
}

/// Create an SMRF stage.
///
/// # Safety
///
/// `returns` must either be null with a zero count or point to `count`
/// NUL-terminated C strings. `ignore` must either be null with a zero count or
/// point to `ignore_count` `pdal_dim_range_t` whose `dim_name` are
/// NUL-terminated C strings.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pdal_stage_create_smrf(
    cell: f64,
    slope: f64,
    has_window: bool,
    window: f64,
    scalar: f64,
    threshold: f64,
    cut: f64,
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
    returns: *const *const c_char,
    count: u64,
    ignore: *const pdal_dim_range_t,
    ignore_count: u64,
    classbits: u8,
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

    let mut rust_ignore = Vec::new();
    if !ignore.is_null() {
        for i in 0..ignore_count {
            let range = &*ignore.offset(i as isize);
            if range.dim_name.is_null() {
                return std::ptr::null_mut();
            }
            rust_ignore.push(pdal_filters::range::RangeLimit {
                dim_name: CStr::from_ptr(range.dim_name)
                    .to_string_lossy()
                    .into_owned(),
                lower_bound: range.lower_bound,
                upper_bound: range.upper_bound,
                inclusive_lower: range.inclusive_lower,
                inclusive_upper: range.inclusive_upper,
                negate: range.negate,
            });
        }
    }

    let filter = Box::new(SmrfFilter::with_segmentation(
        cell,
        slope,
        if has_window { Some(window) } else { None },
        scalar,
        threshold,
        cut,
        ground_class,
        other_class,
        only_ground,
        rust_returns,
        rust_ignore,
        classbits,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a PMF stage.
///
/// # Safety
///
/// `returns` must either be null with a zero count or point to `count`
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_pmf(
    cell_size: f64,
    exponential: bool,
    initial_distance: f64,
    max_distance: f64,
    max_window_size: f64,
    slope: f64,
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
                set_last_error("Null return name in PMF options.");
                return std::ptr::null_mut();
            }
            rust_returns.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    match PmfFilter::new(
        cell_size,
        exponential,
        initial_distance,
        rust_returns,
        max_distance,
        max_window_size,
        slope,
        ground_class,
        other_class,
        only_ground,
    ) {
        Ok(filter) => Box::into_raw(Box::new(StageWrapper {
            filter: Box::new(filter),
        })),
        Err(err) => {
            set_last_error(&err.0);
            std::ptr::null_mut()
        }
    }
}

/// Create a Li tree filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_litree(
    min_points: u64,
    min_height: f64,
    radius: f64,
) -> *mut StageWrapper {
    let filter = Box::new(LiTreeFilter::new(min_points as usize, min_height, radius));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Compute M3C2 output for two source clouds and a core-point view.
///
/// # Safety
///
/// `view1`, `view2`, and `cores` must be valid point-view pointers returned by
/// the Rust C ABI.
#[no_mangle]
pub unsafe extern "C" fn pdal_m3c2_compute(
    view1: *const PointView,
    view2: *const PointView,
    cores: *const PointView,
    normal_radius: f64,
    cyl_radius: f64,
    cyl_half_len: f64,
    reg_error: f64,
    orientation: u8,
    min_points: u64,
) -> *mut PointView {
    let Some(view1) = view1.as_ref() else {
        set_last_error("M3C2 missing first view.");
        return std::ptr::null_mut();
    };
    let Some(view2) = view2.as_ref() else {
        set_last_error("M3C2 missing second view.");
        return std::ptr::null_mut();
    };
    let Some(cores) = cores.as_ref() else {
        set_last_error("M3C2 missing core points.");
        return std::ptr::null_mut();
    };

    let orientation = match orientation {
        1 => M3C2NormalOrientation::Down,
        2 => M3C2NormalOrientation::None,
        _ => M3C2NormalOrientation::Up,
    };
    let filter = M3C2Filter::new(
        normal_radius,
        cyl_radius,
        cyl_half_len,
        reg_error,
        orientation,
        min_points as usize,
    );
    match filter.compute(view1, view2, cores) {
        Ok(out) => Box::into_raw(Box::new(out)),
        Err(err) => {
            set_last_error(&err.0);
            std::ptr::null_mut()
        }
    }
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

/// Create a HAG Delaunay filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_hag_delaunay(
    count: u64,
    allow_extrapolation: bool,
    class_label: u8,
) -> *mut StageWrapper {
    let filter = Box::new(HagDelaunayFilter::new(
        count as usize,
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

/// Create a supervoxel filter stage.
#[no_mangle]
pub extern "C" fn pdal_stage_create_supervoxel(knn: u64, resolution: f64) -> *mut StageWrapper {
    let filter = Box::new(SupervoxelFilter::new(knn as usize, resolution));
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

/// Create a covariance-features filter stage.
///
/// # Safety
/// `dims` must be a valid array of `dim_count` NUL-terminated C strings, or
/// null.
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
    let mut tokens: Vec<String> = Vec::new();
    if !dims.is_null() {
        for i in 0..dim_count as usize {
            let ptr = *dims.add(i);
            if !ptr.is_null() {
                tokens.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
            }
        }
    }
    let feature_set = tokens.join(",");
    let filter = Box::new(CovarianceFeaturesFilter::new(
        knn as usize,
        stride as usize,
        has_radius.then_some(radius),
        min_k as usize,
        CovarianceMode::from_u32(u32::from(mode)),
        optimal,
        &feature_set,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a normal-estimation filter stage.
///
/// `knn` is the effective neighbor count (including the query point).
#[no_mangle]
pub extern "C" fn pdal_stage_create_normal(
    knn: u64,
    has_radius: bool,
    radius: f64,
    has_viewpoint: bool,
    viewpoint_x: f64,
    viewpoint_y: f64,
    viewpoint_z: f64,
    always_up: bool,
    refine: bool,
) -> *mut StageWrapper {
    let filter = Box::new(NormalFilter::new(
        knn as usize,
        has_radius.then_some(radius),
        has_viewpoint.then_some([viewpoint_x, viewpoint_y, viewpoint_z]),
        always_up,
        refine,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a relaxation dart-throwing subsampling filter stage.
///
/// `count` is the target output point count. When `has_seed` is false the
/// shuffle is seeded from the wall clock, matching the C++ default.
#[no_mangle]
pub extern "C" fn pdal_stage_create_relaxationdartthrowing(
    decay: f64,
    radius: f64,
    terminal_radius: f64,
    count: u64,
    shuffle: bool,
    has_seed: bool,
    seed: u32,
) -> *mut StageWrapper {
    let filter = Box::new(RelaxationDartThrowingFilter::new(
        decay,
        radius,
        terminal_radius,
        count,
        shuffle,
        has_seed.then_some(seed),
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}

/// Create a straighten filter stage.
///
/// Returns null when the polyline WKT cannot be parsed as a `LINESTRING ZM`.
///
/// # Safety
///
/// `polyline` must be a valid NUL-terminated C string when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_straighten(
    polyline: *const c_char,
    reverse: bool,
    offset: f64,
) -> *mut StageWrapper {
    if polyline.is_null() {
        return std::ptr::null_mut();
    }
    let wkt = CStr::from_ptr(polyline).to_string_lossy();
    match StraightenFilter::new(&wkt, reverse, offset) {
        Some(f) => {
            let filter = Box::new(f);
            Box::into_raw(Box::new(StageWrapper { filter }))
        }
        None => std::ptr::null_mut(),
    }
}

/// Create a Lloyd's k-means clustering filter stage.
///
/// # Safety
///
/// `dims` must be null with `dim_count` zero, or point to `dim_count` valid
/// C strings naming the clustering dimensions.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_lloydkmeans(
    k: u64,
    maxiters: u64,
    dims: *const *const c_char,
    dim_count: u64,
) -> *mut StageWrapper {
    let mut dim_ids: Vec<DimId> = Vec::new();
    if !dims.is_null() {
        for i in 0..dim_count as usize {
            let ptr = *dims.add(i);
            if !ptr.is_null() {
                let name = CStr::from_ptr(ptr).to_string_lossy();
                dim_ids.push(DimId::from_name(&name));
            }
        }
    }
    let filter = Box::new(LloydKMeansFilter::new(
        k as usize,
        maxiters as usize,
        dim_ids,
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
