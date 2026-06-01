use super::*;

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
    mad: bool,
    mad_multiplier: f64,
    k: f64,
) -> *mut StageWrapper {
    if dim_name.is_null() || ramp.is_null() {
        set_last_error("null argument to pdal_stage_create_colorinterp");
        return std::ptr::null_mut();
    }

    let dim_name = CStr::from_ptr(dim_name).to_string_lossy();
    let ramp = CStr::from_ptr(ramp).to_string_lossy();

    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(
            ColorinterpFilter::new(&dim_name, &ramp, min, max, clamp, invert)
                .with_bounds_params(mad, mad_multiplier, k),
        ),
    }))
}

/// Validate colorinterp options against a point layout.
///
/// # Safety
///
/// `layout` must be a valid pointer when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_colorinterp_validate_prepared(
    layout: *const PointLayout,
    dim_name: *const c_char,
    min: f64,
    max: f64,
) -> *mut c_char {
    if layout.is_null() || dim_name.is_null() {
        return string_to_c_ptr("Missing colorinterp layout.".to_string());
    }

    let dim_name = CStr::from_ptr(dim_name).to_string_lossy();
    match validate_prepared(&dim_name, min, max, &*layout) {
        Ok(()) => std::ptr::null_mut(),
        Err(err) => string_to_c_ptr(err),
    }
}

/// Return whether a colorinterp stage can run in streaming mode.
#[no_mangle]
pub extern "C" fn pdal_colorinterp_pipeline_streamable(min: f64, max: f64) -> bool {
    pipeline_streamable(min, max)
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

/// Create a DEM filter stage.
///
/// # Safety
///
/// `dim_name` and `raster_path` must be null-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_dem(
    dim_name: *const c_char,
    raster_path: *const c_char,
    band: i32,
    lower_bound: f64,
    upper_bound: f64,
) -> *mut StageWrapper {
    if dim_name.is_null() || raster_path.is_null() {
        set_last_error("null argument to pdal_stage_create_dem");
        return std::ptr::null_mut();
    }

    let dim_name = CStr::from_ptr(dim_name).to_string_lossy();
    let raster_path = CStr::from_ptr(raster_path).to_string_lossy();
    Box::into_raw(Box::new(StageWrapper {
        filter: Box::new(DEMFilter::new(
            &dim_name,
            &raster_path,
            band,
            lower_bound,
            upper_bound,
        )),
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
