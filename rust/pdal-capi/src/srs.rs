use crate::error::string_to_c_ptr;
use pdal_core::{
    metadata::MetadataNode,
    srs::{calculate_zone, wgs84_code_from_zone, SpatialReference, SpatialReferenceList},
};
use std::ffi::CStr;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// SpatialReference ABI
// ---------------------------------------------------------------------------

/// Create a spatial reference from text. Caller owns the returned pointer.
///
/// # Safety
///
/// `text` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_create(
    text: *const c_char,
) -> *mut SpatialReference {
    let text = if text.is_null() {
        String::new()
    } else {
        CStr::from_ptr(text).to_string_lossy().into_owned()
    };
    Box::into_raw(Box::new(SpatialReference::new(&text)))
}

/// Create a spatial reference from text and coordinate epoch.
///
/// # Safety
///
/// `text` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_create_with_epoch(
    text: *const c_char,
    epoch: f64,
) -> *mut SpatialReference {
    let text = if text.is_null() {
        String::new()
    } else {
        CStr::from_ptr(text).to_string_lossy().into_owned()
    };
    Box::into_raw(Box::new(SpatialReference::with_epoch(&text, epoch)))
}

/// Whether a spatial reference has no text.
///
/// # Safety
///
/// `srs` must be null or a valid pointer returned by
/// `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_empty(srs: *const SpatialReference) -> bool {
    srs.as_ref().map(|s| s.is_empty()).unwrap_or(true)
}

/// Return the spatial reference text. Caller must free with `pdal_string_free`.
///
/// # Safety
///
/// `srs` must be null or a valid pointer returned by
/// `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_text(srs: *const SpatialReference) -> *mut c_char {
    string_to_c_ptr(
        srs.as_ref()
            .map(|srs| srs.wkt().to_string())
            .unwrap_or_default(),
    )
}

/// Return the coordinate epoch.
///
/// # Safety
///
/// `srs` must be null or a valid pointer returned by
/// `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_epoch(srs: *const SpatialReference) -> f64 {
    srs.as_ref().map(|s| s.epoch()).unwrap_or(0.0)
}

/// Set the coordinate epoch.
///
/// # Safety
///
/// `srs` must be a valid pointer returned by `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_set_epoch(srs: *mut SpatialReference, epoch: f64) {
    if let Some(srs) = srs.as_mut() {
        srs.set_epoch(epoch);
    }
}

/// Convert a spatial reference to metadata. Caller owns the returned node.
///
/// # Safety
///
/// `srs` must be null or a valid pointer returned by
/// `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_to_metadata(
    srs: *const SpatialReference,
) -> *mut MetadataNode {
    let metadata = srs
        .as_ref()
        .map(SpatialReference::to_metadata)
        .unwrap_or_else(|| SpatialReference::default().to_metadata());
    Box::into_raw(Box::new(metadata))
}

/// Create an empty spatial-reference list. Caller owns the returned pointer.
#[no_mangle]
pub extern "C" fn pdal_spatial_reference_list_create() -> *mut SpatialReferenceList {
    Box::into_raw(Box::new(SpatialReferenceList::new()))
}

/// Clear all spatial references from a list.
///
/// # Safety
///
/// `list` must be null or a valid pointer returned by
/// `pdal_spatial_reference_list_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_list_clear(list: *mut SpatialReferenceList) {
    if let Some(list) = list.as_mut() {
        list.clear();
    }
}

/// Add a spatial reference to the front of the list, moving duplicates to front.
///
/// # Safety
///
/// `list` must be null or a valid pointer returned by
/// `pdal_spatial_reference_list_create`. `srs` must be null or a valid pointer
/// returned by `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_list_add(
    list: *mut SpatialReferenceList,
    srs: *const SpatialReference,
) {
    if let Some(list) = list.as_mut() {
        list.add(srs.as_ref().cloned().unwrap_or_default());
    }
}

/// Whether the list has at most one spatial reference.
///
/// # Safety
///
/// `list` must be null or a valid pointer returned by
/// `pdal_spatial_reference_list_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_list_unique(
    list: *const SpatialReferenceList,
) -> bool {
    list.as_ref().map(|list| list.is_unique()).unwrap_or(true)
}

/// Return the number of unique spatial references in the list.
///
/// # Safety
///
/// `list` must be null or a valid pointer returned by
/// `pdal_spatial_reference_list_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_list_size(
    list: *const SpatialReferenceList,
) -> u64 {
    list.as_ref().map(|list| list.len() as u64).unwrap_or(0)
}

/// Return a clone of the front spatial reference. Caller owns the result.
///
/// # Safety
///
/// `list` must be null or a valid pointer returned by
/// `pdal_spatial_reference_list_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_list_any(
    list: *const SpatialReferenceList,
) -> *mut SpatialReference {
    let srs = list
        .as_ref()
        .map(SpatialReferenceList::any)
        .unwrap_or_default();
    Box::into_raw(Box::new(srs))
}

/// Destroy a spatial-reference list.
///
/// # Safety
///
/// `list` must be a valid pointer returned by
/// `pdal_spatial_reference_list_create`, or null. Must not be called twice on
/// the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_list_destroy(list: *mut SpatialReferenceList) {
    if !list.is_null() {
        drop(Box::from_raw(list));
    }
}

#[no_mangle]
pub extern "C" fn pdal_spatial_reference_calculate_zone(lon: f64, lat: f64) -> i32 {
    calculate_zone(lon, lat)
}

#[no_mangle]
pub extern "C" fn pdal_spatial_reference_wgs84_code_from_zone(zone: i32) -> *mut c_char {
    string_to_c_ptr(wgs84_code_from_zone(zone).unwrap_or_default())
}

/// Destroy a spatial reference.
///
/// # Safety
///
/// `srs` must be a valid pointer returned by `pdal_spatial_reference_create`,
/// or null. Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_reference_destroy(srs: *mut SpatialReference) {
    if !srs.is_null() {
        drop(Box::from_raw(srs));
    }
}
