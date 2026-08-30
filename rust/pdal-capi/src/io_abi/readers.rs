use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::point::PointView;
use std::ffi::{c_char, CStr};
use std::path::Path;

#[repr(C)]
pub struct PointlessLasResult {
    pub point_count: u64,
    pub filename: *mut c_char,
}

/// Create a local pointless LAS copy from a remote/local LAS path.
///
/// # Safety
/// `filename` must be null or a valid NUL-terminated C string.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_pointless_las_create(
    filename: *const c_char,
) -> *mut PointlessLasResult {
    let filename = if filename.is_null() {
        ""
    } else {
        match CStr::from_ptr(filename).to_str() {
            Ok(value) => value,
            Err(err) => {
                set_last_error(err.to_string());
                return std::ptr::null_mut();
            }
        }
    };
    match pdal_io::pointless_las::create(filename) {
        Ok(result) => Box::into_raw(Box::new(PointlessLasResult {
            point_count: result.point_count,
            filename: string_to_c_ptr(result.path.display().to_string()),
        })),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

/// Destroy a result returned by `pdal_pointless_las_create`.
///
/// # Safety
/// `result` must be null or a pointer returned by `pdal_pointless_las_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_pointless_las_destroy(result: *mut PointlessLasResult) {
    if !result.is_null() {
        let result = Box::from_raw(result);
        crate::error::pdal_string_free(result.filename);
    }
}

// ---------------------------------------------------------------------------
// Reader C ABI
// ---------------------------------------------------------------------------

/// Opaque reader handle.
pub struct ReaderHandle {
    pub(crate) reader: Box<dyn pdal_core::pipeline::Reader>,
}

/// Create a FauxReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_faux(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        match pdal_io::faux::FauxReader::new(options) {
            Ok(reader) => Box::into_raw(Box::new(ReaderHandle {
                reader: Box::new(reader),
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

/// Create a TextReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_text(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::text::TextReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PcdReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_pcd(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::pcd::PcdReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PtsReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_pts(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::pts::PtsReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PtxReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_ptx(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ptx::PtxReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an Ilvis2Reader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_ilvis2(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ilvis2::Ilvis2Reader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an ObjReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_obj(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::obj::ObjReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PlyReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_ply(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ply::PlyReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a QfitReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_qfit(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::qfit::QfitReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SbetReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_sbet(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::sbet::SbetReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SmrmsgReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_smrmsg(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::smrmsg::SmrmsgReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an OptechReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_optech(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::optech::OptechReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a TerrasolidReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_terrasolid(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::terrasolid::TerrasolidReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a TindexReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_tindex(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::tindex::TindexReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an FbiReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_fbi(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::fbi::FbiReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

#[repr(C)]
pub struct pdal_fbi_header_info_t {
    pub version: u32,
    pub header_size: u32,
    pub point_count: u64,
    pub xyz_position: u64,
}

/// Read the FBI header summary needed by the C++ compatibility wrapper.
///
/// # Safety
/// `filename` must be a valid NUL-terminated string and `out_info` must point
/// to writable storage.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_fbi_header_info(
    filename: *const c_char,
    out_info: *mut pdal_fbi_header_info_t,
) -> i32 {
    if filename.is_null() || out_info.is_null() {
        set_last_error("pdal_fbi_header_info received null input.");
        return -1;
    }

    let path = CStr::from_ptr(filename).to_string_lossy().into_owned();
    match pdal_io::fbi::header_info(Path::new(&path)) {
        Ok(info) => {
            *out_info = pdal_fbi_header_info_t {
                version: info.version,
                header_size: info.hdr_size,
                point_count: info.fast_cnt,
                xyz_position: info.pos_xyz,
            };
            0
        }
        Err(err) => {
            set_last_error(err.to_string());
            -1
        }
    }
}

/// Create a BpfReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_bpf(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::bpf::BpfReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a GdalReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_gdal(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::gdal_reader::GdalReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a LasReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_las(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::las::LasReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Return true when the LAS file at `path` contains a COPC VLR signature.
///
/// # Safety
/// `path` must be a valid null-terminated string.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_las_detect_copc(path: *const c_char) -> bool {
    if path.is_null() {
        return false;
    }
    let path = CStr::from_ptr(path).to_string_lossy();
    pdal_io::las::detect_copc(Path::new(path.as_ref()))
}

/// Create a LazReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_laz(ops: *const Options) -> *mut ReaderHandle {
    pdal_reader_create_las(ops)
}

/// Create an SpzReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_spz(ops: *const Options) -> *mut ReaderHandle {
    #[cfg(feature = "spz")]
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::spz::SpzReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
    #[cfg(not(feature = "spz"))]
    {
        let _ = ops;
        crate::error::set_last_error("readers.spz is not enabled in this Rust C ABI build.");
        std::ptr::null_mut()
    }
}

/// Create a StacReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_stac(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::stac::StacReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Return true when `filename` can be parsed as a supported STAC JSON object.
///
/// # Safety
/// `filename` must be null or a valid NUL-terminated string.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_stac_type_supported(filename: *const c_char) -> bool {
    clear_last_error();
    let filename = if filename.is_null() {
        ""
    } else {
        match CStr::from_ptr(filename).to_str() {
            Ok(value) => value,
            Err(err) => {
                set_last_error(err.to_string());
                return false;
            }
        }
    };
    match pdal_io::source::read_to_string(filename)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| {
            json.get("type")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        }) {
        Some(stac_type) => {
            matches!(
                stac_type.as_str(),
                "Feature" | "Catalog" | "Collection" | "FeatureCollection"
            )
        }
        None => false,
    }
}

/// Create a CopcReader full-file read slice from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_copc(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::copc::CopcReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Compute a hierarchy-driven COPC preview: writes the bounds-and-resolution-
/// limited point count and dataset-coordinate bbox into the supplied outputs.
/// Returns 0 on success, -1 on error (last error is set via the standard
/// `pdal_last_error()` channel).
///
/// `out_bounds` receives `[min_x, min_y, min_z, max_x, max_y, max_z]`.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
/// `out_point_count` and `out_bounds` must point to writable storage with
/// space for `u64` and 6 `f64` respectively.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_copc_preview(
    ops: *const Options,
    out_point_count: *mut u64,
    out_bounds: *mut f64,
) -> i32 {
    let Some(options) = ops.as_ref() else {
        crate::error::set_last_error("pdal_copc_preview: options pointer is null");
        return -1;
    };
    let reader = pdal_io::copc::CopcReader::new(options);
    match reader.preview() {
        Ok(preview) => {
            if !out_point_count.is_null() {
                *out_point_count = preview.point_count;
            }
            if !out_bounds.is_null() {
                let b = preview.bounds;
                let slots = std::slice::from_raw_parts_mut(out_bounds, 6);
                slots[0] = b.min_x;
                slots[1] = b.min_y;
                slots[2] = b.min_z;
                slots[3] = b.max_x;
                slots[4] = b.max_y;
                slots[5] = b.max_z;
            }
            0
        }
        Err(err) => {
            crate::error::set_last_error(&err.0);
            -1
        }
    }
}

/// Create an EptReader local LASzip read slice from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_create_ept(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ept::EptReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Return local STAC preview metadata as JSON.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_stac_preview_json(ops: *const Options) -> *mut c_char {
    let Some(options) = ops.as_ref() else {
        set_last_error("Missing STAC preview options.");
        return std::ptr::null_mut();
    };
    match pdal_io::stac::StacReader::new(options).preview() {
        Ok(preview) => string_to_c_ptr(
            serde_json::json!({
                "point_count": preview.point_count,
                "catalog_ids": preview.catalog_ids,
                "collection_ids": preview.collection_ids,
                "item_ids": preview.item_ids,
            })
            .to_string(),
        ),
        Err(err) => {
            set_last_error(err.0);
            std::ptr::null_mut()
        }
    }
}

/// Read the first point view produced by a reader.
///
/// # Safety
/// `reader` must be a valid pointer returned by `pdal_reader_create_*`.
/// The returned view must be freed with `pdal_point_view_destroy`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_read_first(reader: *mut ReaderHandle) -> *mut PointView {
    let Some(reader) = reader.as_mut() else {
        set_last_error("null reader");
        return std::ptr::null_mut();
    };

    match reader.reader.read() {
        Ok(mut views) => {
            clear_last_error();
            views
                .drain(..)
                .next()
                .map(|view| Box::into_raw(Box::new(view)))
                .unwrap_or(std::ptr::null_mut())
        }
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Return a reader's metadata tree. Caller owns the returned pointer.
///
/// # Safety
/// `reader` must be a valid pointer returned by `pdal_reader_create_*`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_metadata(reader: *const ReaderHandle) -> *mut MetadataNode {
    let Some(reader) = reader.as_ref() else {
        set_last_error("null reader");
        return std::ptr::null_mut();
    };

    clear_last_error();
    Box::into_raw(Box::new(reader.reader.metadata()))
}

/// Read an ILVIS2 XML metadata sidecar file.
///
/// # Safety
/// `filename` must be a valid NUL-terminated string.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_ilvis2_metadata_read(filename: *const c_char) -> *mut MetadataNode {
    if filename.is_null() {
        set_last_error("null metadata filename");
        return std::ptr::null_mut();
    }

    let path = match CStr::from_ptr(filename).to_str() {
        Ok(path) => path,
        Err(err) => {
            set_last_error(format!("invalid metadata filename: {err}"));
            return std::ptr::null_mut();
        }
    };

    match pdal_io::ilvis2_metadata::read_metadata_file(path) {
        Ok(metadata) => {
            clear_last_error();
            Box::into_raw(Box::new(metadata))
        }
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Destroy a reader handle.
///
/// # Safety
/// `reader` must be a valid pointer returned by `pdal_reader_create_*`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_reader_destroy(reader: *mut ReaderHandle) {
    if !reader.is_null() {
        drop(Box::from_raw(reader));
    }
}
