use super::*;

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
        "" | "asc" => SortOrder::Asc,
        "desc" => SortOrder::Desc,
        _ => {
            set_last_error(format!("Invalid sort order '{order_str}'."));
            return std::ptr::null_mut();
        }
    };

    let alg_str = CStr::from_ptr(algorithm).to_string_lossy();
    let alg_enum = match alg_str.to_ascii_lowercase().as_str() {
        "" | "normal" => SortAlgorithm::Normal,
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
