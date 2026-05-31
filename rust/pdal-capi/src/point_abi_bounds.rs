use crate::error::string_to_c_ptr;
use pdal_core::bounds::{
    bounds2d_equal, bounds2d_to_geojson, bounds2d_to_wkt, bounds3d_equal, bounds3d_to_wkt,
    default_bounds2d, default_bounds3d, format_bounds2d, format_bounds3d, parse_bounds2d,
    parse_bounds3d, Bounds2D, Bounds3D,
};
use std::ffi::CStr;
use std::os::raw::c_char;

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

#[repr(C)]
#[derive(Debug)]
pub struct pdal_srs_bounds_parse_result_t {
    pub is_3d: bool,
    pub bounds2d: pdal_bounds2d_t,
    pub bounds3d: pdal_bounds3d_t,
    pub srs: *mut c_char,
    pub pos: u64,
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
pub unsafe extern "C" fn pdal_bounds2d_parse(
    input: *const c_char,
    pos: u64,
    out_bounds: *mut pdal_bounds2d_t,
    out_wkt: *mut *mut c_char,
    out_pos: *mut u64,
) -> *mut c_char {
    if input.is_null() || out_bounds.is_null() || out_pos.is_null() {
        return string_to_c_ptr("Invalid null bounds parse argument.".to_string());
    }

    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_bounds2d(&input, pos as usize) {
        Ok(parsed) => {
            *out_bounds = parsed.bounds.into();
            *out_pos = parsed.pos as u64;
            if let Some(out_wkt) = out_wkt.as_mut() {
                *out_wkt = string_to_c_ptr(parsed.wkt);
            }
            std::ptr::null_mut()
        }
        Err(error) => string_to_c_ptr(error),
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

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_parse(
    input: *const c_char,
    pos: u64,
    out_bounds: *mut pdal_bounds3d_t,
    out_wkt: *mut *mut c_char,
    out_pos: *mut u64,
) -> *mut c_char {
    if input.is_null() || out_bounds.is_null() || out_pos.is_null() {
        return string_to_c_ptr("Invalid null bounds parse argument.".to_string());
    }

    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_bounds3d(&input, pos as usize) {
        Ok(parsed) => {
            *out_bounds = parsed.bounds.into();
            *out_pos = parsed.pos as u64;
            if let Some(out_wkt) = out_wkt.as_mut() {
                *out_wkt = string_to_c_ptr(parsed.wkt);
            }
            std::ptr::null_mut()
        }
        Err(error) => string_to_c_ptr(error),
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_srs_bounds_parse(
    input: *const c_char,
    pos: u64,
    out_result: *mut pdal_srs_bounds_parse_result_t,
) -> *mut c_char {
    if input.is_null() || out_result.is_null() {
        return string_to_c_ptr("Invalid null SRS bounds parse argument.".to_string());
    }

    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_srs_bounds(&input, pos as usize) {
        Ok(parsed) => {
            *out_result = parsed;
            std::ptr::null_mut()
        }
        Err(error) => string_to_c_ptr(error),
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_equal(
    left: *const pdal_bounds2d_t,
    right: *const pdal_bounds2d_t,
) -> bool {
    match (left.as_ref(), right.as_ref()) {
        (Some(left), Some(right)) => {
            bounds2d_equal(&Bounds2D::from(*left), &Bounds2D::from(*right))
        }
        _ => false,
    }
}

fn parse_srs_bounds(input: &str, pos: usize) -> Result<pdal_srs_bounds_parse_result_t, String> {
    match parse_bounds3d(input, pos) {
        Ok(parsed) => Ok(finish_srs_bounds_parse(
            input,
            parsed.pos,
            parsed.wkt,
            true,
            Bounds2D {
                minx: parsed.bounds.minx,
                maxx: parsed.bounds.maxx,
                miny: parsed.bounds.miny,
                maxy: parsed.bounds.maxy,
            },
            parsed.bounds,
        )?),
        Err(_) => {
            let parsed = parse_bounds2d(input, 0)?;
            Ok(finish_srs_bounds_parse(
                input,
                parsed.pos,
                parsed.wkt,
                false,
                parsed.bounds,
                Bounds3D {
                    minx: parsed.bounds.minx,
                    maxx: parsed.bounds.maxx,
                    miny: parsed.bounds.miny,
                    maxy: parsed.bounds.maxy,
                    minz: Bounds3D::empty().minz,
                    maxz: Bounds3D::empty().maxz,
                },
            )?)
        }
    }
}

fn finish_srs_bounds_parse(
    input: &str,
    mut pos: usize,
    mut srs: String,
    is_3d: bool,
    bounds2d: Bounds2D,
    bounds3d: Bounds3D,
) -> Result<pdal_srs_bounds_parse_result_t, String> {
    pos = skip_ascii_whitespace(input, pos);
    if pos != input.len() {
        if input.as_bytes().get(pos) != Some(&b'/') {
            return Err("Invalid character following valid bounds box.".to_string());
        }
        pos += 1;
        pos = skip_ascii_whitespace(input, pos);
        srs = input[pos..].trim().to_string();
        pos = input.len();
    }

    Ok(pdal_srs_bounds_parse_result_t {
        is_3d,
        bounds2d: bounds2d.into(),
        bounds3d: bounds3d.into(),
        srs: string_to_c_ptr(srs),
        pos: pos as u64,
    })
}

fn skip_ascii_whitespace(input: &str, mut pos: usize) -> usize {
    while input
        .as_bytes()
        .get(pos)
        .is_some_and(u8::is_ascii_whitespace)
    {
        pos += 1;
    }
    pos
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_equal(
    left: *const pdal_bounds3d_t,
    right: *const pdal_bounds3d_t,
) -> bool {
    match (left.as_ref(), right.as_ref()) {
        (Some(left), Some(right)) => {
            bounds3d_equal(&Bounds3D::from(*left), &Bounds3D::from(*right))
        }
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_default(out_bounds: *mut pdal_bounds2d_t) {
    if let Some(out_bounds) = out_bounds.as_mut() {
        *out_bounds = default_bounds2d().into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_default(out_bounds: *mut pdal_bounds3d_t) {
    if let Some(out_bounds) = out_bounds.as_mut() {
        *out_bounds = default_bounds3d().into();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_format(
    bounds: *const pdal_bounds2d_t,
    precision: u32,
) -> *mut c_char {
    bounds
        .as_ref()
        .map(|bounds| string_to_c_ptr(format_bounds2d(&Bounds2D::from(*bounds), precision)))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_format(
    bounds: *const pdal_bounds3d_t,
    precision: u32,
) -> *mut c_char {
    bounds
        .as_ref()
        .map(|bounds| string_to_c_ptr(format_bounds3d(&Bounds3D::from(*bounds), precision)))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_to_wkt(
    bounds: *const pdal_bounds2d_t,
    precision: u32,
) -> *mut c_char {
    bounds
        .as_ref()
        .map(|bounds| string_to_c_ptr(bounds2d_to_wkt(&Bounds2D::from(*bounds), precision)))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds3d_to_wkt(
    bounds: *const pdal_bounds3d_t,
    precision: u32,
) -> *mut c_char {
    bounds
        .as_ref()
        .map(|bounds| string_to_c_ptr(bounds3d_to_wkt(&Bounds3D::from(*bounds), precision)))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_bounds2d_to_geojson(
    bounds: *const pdal_bounds2d_t,
    precision: u32,
) -> *mut c_char {
    bounds
        .as_ref()
        .map(|bounds| string_to_c_ptr(bounds2d_to_geojson(&Bounds2D::from(*bounds), precision)))
        .unwrap_or(std::ptr::null_mut())
}
