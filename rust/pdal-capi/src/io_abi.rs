use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use crate::point_abi::pdal_bounds3d_t;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use std::ffi::{c_char, CStr};
use std::path::Path;
use std::rc::Rc;

pub struct LasSummaryHandle {
    summary: pdal_io::las_summary::LasSummary,
}

#[no_mangle]
pub extern "C" fn pdal_las_summary_create() -> *mut LasSummaryHandle {
    Box::into_raw(Box::new(LasSummaryHandle {
        summary: pdal_io::las_summary::LasSummary::default(),
    }))
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_destroy(summary: *mut LasSummaryHandle) {
    if !summary.is_null() {
        drop(Box::from_raw(summary));
    }
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_clear(summary: *mut LasSummaryHandle) {
    if let Some(summary) = summary.as_mut() {
        summary.summary.clear();
    }
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_add_point(
    summary: *mut LasSummaryHandle,
    x: f64,
    y: f64,
    z: f64,
    return_number: i32,
) {
    if let Some(summary) = summary.as_mut() {
        summary.summary.add_point(x, y, z, return_number);
    }
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_total_num_points(
    summary: *const LasSummaryHandle,
) -> u64 {
    summary
        .as_ref()
        .map(|summary| summary.summary.total_num_points())
        .unwrap_or(0)
}

/// # Safety
/// `summary` must be null or a pointer returned by `pdal_las_summary_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_return_count(
    summary: *const LasSummaryHandle,
    return_number: u64,
) -> u64 {
    let Some(summary) = summary.as_ref() else {
        return 0;
    };
    summary.summary.return_count(return_number as usize)
}

/// # Safety
/// `summary` must be a pointer returned by `pdal_las_summary_create` and
/// `out_bounds` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_las_summary_bounds(
    summary: *const LasSummaryHandle,
    out_bounds: *mut pdal_bounds3d_t,
) {
    if let (Some(summary), Some(out_bounds)) = (summary.as_ref(), out_bounds.as_mut()) {
        let bounds = summary.summary.bounds();
        *out_bounds = pdal_bounds3d_t {
            minx: bounds.minx,
            maxx: bounds.maxx,
            miny: bounds.miny,
            maxy: bounds.maxy,
            minz: bounds.minz,
            maxz: bounds.maxz,
        };
    }
}

#[repr(C)]
pub struct PointlessLasResult {
    pub point_count: u64,
    pub filename: *mut c_char,
}

/// Create a local pointless LAS copy from a remote/local LAS path.
///
/// # Safety
/// `filename` must be null or a valid NUL-terminated C string.
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_laz(ops: *const Options) -> *mut ReaderHandle {
    pdal_reader_create_las(ops)
}

/// Create an SpzReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_spz(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::spz::SpzReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a StacReader from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_stac(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::stac::StacReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a CopcReader full-file read slice from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_create_ept(ops: *const Options) -> *mut ReaderHandle {
    if let Some(options) = ops.as_ref() {
        let reader = Box::new(pdal_io::ept::EptReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
    }
}

/// Validate an EPT origin option through the Rust reader implementation.
///
/// # Safety
/// `filename` and `origin` must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_validate_origin(
    filename: *const c_char,
    origin: *const c_char,
) -> bool {
    if filename.is_null() || origin.is_null() {
        set_last_error("Missing EPT origin validation input.");
        return false;
    }
    let filename = CStr::from_ptr(filename).to_string_lossy().into_owned();
    let origin = CStr::from_ptr(origin).to_string_lossy().into_owned();
    let mut options = Options::new();
    options.add("filename", filename);
    options.add("origin", origin);
    match pdal_io::ept::EptReader::new(&options).validate_origin() {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err.0);
            false
        }
    }
}

/// Validate an EPT bounds option through the Rust reader implementation.
///
/// # Safety
/// `filename` and `bounds` must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_validate_bounds(
    filename: *const c_char,
    bounds: *const c_char,
) -> bool {
    if filename.is_null() || bounds.is_null() {
        set_last_error("Missing EPT bounds validation input.");
        return false;
    }
    let filename = CStr::from_ptr(filename).to_string_lossy().into_owned();
    let bounds = CStr::from_ptr(bounds).to_string_lossy().into_owned();
    let mut options = Options::new();
    options.add("filename", filename);
    options.add("bounds", bounds);
    match pdal_io::ept::EptReader::new(&options).validate_bounds() {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err.0);
            false
        }
    }
}

/// Return local STAC preview metadata as JSON.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_metadata(reader: *const ReaderHandle) -> *mut MetadataNode {
    let Some(reader) = reader.as_ref() else {
        set_last_error("null reader");
        return std::ptr::null_mut();
    };

    clear_last_error();
    Box::into_raw(Box::new(reader.reader.metadata()))
}

#[repr(C)]
pub struct pdal_memoryview_field_t {
    pub name: *const c_char,
    pub type_id: i32,
    pub offset: u64,
}

fn parse_memoryview_shape(input: &str) -> Result<(u64, u64, u64), String> {
    let values: Vec<&str> = input.split(',').collect();
    if values.len() != 3 {
        return Err(
            "Shape must be specified as three integers: 'depth, rows, columns'.".to_string(),
        );
    }

    fn parse_field(label: &str, value: &str) -> Result<u64, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() || !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("Invalid {label} value in shape: '{trimmed}'."));
        }
        trimmed
            .parse()
            .map_err(|_| format!("Invalid {label} value in shape: '{trimmed}'."))
    }

    let depth = parse_field("depth", values[0])?;
    let rows = parse_field("rows", values[1])?;
    let columns = parse_field("rows", values[2])?;

    Ok((depth, rows, columns))
}

/// Parse a memory-view shape option such as `1, 2, 3`.
///
/// # Safety
///
/// Output pointers must be valid when non-null.
#[no_mangle]
pub unsafe extern "C" fn pdal_memoryview_shape_parse(
    input: *const c_char,
    out_depth: *mut u64,
    out_rows: *mut u64,
    out_columns: *mut u64,
) -> *mut c_char {
    if input.is_null() {
        return string_to_c_ptr(
            "Shape must be specified as three integers: 'depth, rows, columns'.".to_string(),
        );
    }

    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_memoryview_shape(&input) {
        Ok((depth, rows, columns)) => {
            if let Some(out_depth) = out_depth.as_mut() {
                *out_depth = depth;
            }
            if let Some(out_rows) = out_rows.as_mut() {
                *out_rows = rows;
            }
            if let Some(out_columns) = out_columns.as_mut() {
                *out_columns = columns;
            }
            std::ptr::null_mut()
        }
        Err(err) => string_to_c_ptr(err),
    }
}

pub type MemoryViewIncrementer =
    Option<unsafe extern "C" fn(point_id: u64, user_data: *mut std::ffi::c_void) -> *const u8>;

/// Read a C++ memory-view callback into a Rust-owned point view.
///
/// # Safety
/// `fields` must point to `field_count` valid field descriptors. `incrementer`
/// must return either a valid point base pointer for the requested point or
/// null to end the stream.
#[no_mangle]
pub unsafe extern "C" fn pdal_memoryview_read(
    fields: *const pdal_memoryview_field_t,
    field_count: u64,
    incrementer: MemoryViewIncrementer,
    user_data: *mut std::ffi::c_void,
    depth: u64,
    rows: u64,
    columns: u64,
    column_major: bool,
) -> *mut PointView {
    clear_last_error();
    if fields.is_null() && field_count != 0 {
        set_last_error("null memoryview fields");
        return std::ptr::null_mut();
    }
    let Some(incrementer) = incrementer else {
        set_last_error("null memoryview incrementer");
        return std::ptr::null_mut();
    };

    let raw_fields = std::slice::from_raw_parts(fields, field_count as usize);
    let mut parsed_fields = Vec::with_capacity(raw_fields.len());
    let mut has_x = false;
    let mut has_y = false;
    let mut has_z = false;
    for field in raw_fields {
        if field.name.is_null() {
            set_last_error("null memoryview field name");
            return std::ptr::null_mut();
        }
        let name = CStr::from_ptr(field.name).to_string_lossy().into_owned();
        has_x |= name == "X";
        has_y |= name == "Y";
        has_z |= name == "Z";
        let Some(ty) = dim_type_from_pdal_type(field.type_id) else {
            set_last_error("unsupported memoryview field type");
            return std::ptr::null_mut();
        };
        parsed_fields.push((name, ty, field.offset as usize));
    }

    let has_shape = depth != 0 && rows != 0 && columns != 0;
    let mut layout = PointLayout::new();
    if has_shape && !(has_x && has_y && has_z) {
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
    }
    for (name, ty, _) in &parsed_fields {
        layout.register(DimId::from_name(name), *ty);
    }

    let layout = Rc::new(layout);
    let mut view = PointView::new(layout);
    let mut idx = 0;
    loop {
        let base = incrementer(idx, user_data);
        if base.is_null() {
            break;
        }
        view.add_point();
        for (name, ty, offset) in &parsed_fields {
            let value = read_memoryview_value(base.add(*offset), *ty);
            view.set_f64(idx, &DimId::from_name(name), value);
        }
        if has_shape {
            let (x, y, z) = memoryview_coordinates(idx, depth, rows, columns, column_major);
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }
        idx += 1;
    }

    Box::into_raw(Box::new(view))
}

fn dim_type_from_pdal_type(type_id: i32) -> Option<DimType> {
    match type_id as u32 {
        0x201 => Some(DimType::U8),
        0x202 => Some(DimType::U16),
        0x204 => Some(DimType::U32),
        0x208 => Some(DimType::U64),
        0x101 => Some(DimType::I8),
        0x102 => Some(DimType::I16),
        0x104 => Some(DimType::I32),
        0x108 => Some(DimType::I64),
        0x404 => Some(DimType::F32),
        0x408 => Some(DimType::F64),
        _ => None,
    }
}

unsafe fn read_memoryview_value(ptr: *const u8, ty: DimType) -> f64 {
    match ty {
        DimType::U8 => ptr.read_unaligned() as f64,
        DimType::U16 => (ptr as *const u16).read_unaligned() as f64,
        DimType::U32 => (ptr as *const u32).read_unaligned() as f64,
        DimType::U64 => (ptr as *const u64).read_unaligned() as f64,
        DimType::I8 => (ptr as *const i8).read_unaligned() as f64,
        DimType::I16 => (ptr as *const i16).read_unaligned() as f64,
        DimType::I32 => (ptr as *const i32).read_unaligned() as f64,
        DimType::I64 => (ptr as *const i64).read_unaligned() as f64,
        DimType::F32 => (ptr as *const f32).read_unaligned() as f64,
        DimType::F64 => (ptr as *const f64).read_unaligned(),
    }
}

fn memoryview_coordinates(
    idx: u64,
    depth: u64,
    rows: u64,
    columns: u64,
    column_major: bool,
) -> (f64, f64, f64) {
    let coords = if column_major {
        let x_div = depth * rows;
        let y_div = depth;
        let x_iter = depth * rows * columns;
        let y_iter = depth * rows;
        ((idx % x_iter) / x_div, (idx % y_iter) / y_div, idx % depth)
    } else {
        let y_iter = columns * rows;
        (
            idx % columns,
            (idx % y_iter) / columns,
            idx / (columns * rows) % depth,
        )
    };
    (coords.0 as f64, coords.1 as f64, coords.2 as f64)
}

/// Read an ILVIS2 XML metadata sidecar file.
///
/// # Safety
/// `filename` must be a valid NUL-terminated string.
#[no_mangle]
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

    match pdal_io::ilvis2_metadata::read_metadata_file(Path::new(path)) {
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
#[no_mangle]
pub unsafe extern "C" fn pdal_reader_destroy(reader: *mut ReaderHandle) {
    if !reader.is_null() {
        drop(Box::from_raw(reader));
    }
}

// ---------------------------------------------------------------------------
// Writer C ABI
// ---------------------------------------------------------------------------

/// Opaque writer handle.
pub struct WriterHandle {
    pub(crate) writer: Box<dyn pdal_core::pipeline::Writer>,
}

/// Create a NullWriter.
///
/// # Safety
/// `ops` must be a valid pointer (may be null).
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_null(ops: *const Options) -> *mut WriterHandle {
    let options = Options::new();
    let writer = Box::new(pdal_io::nullwriter::NullWriter::new(if ops.is_null() {
        &options
    } else {
        &*ops
    }));
    Box::into_raw(Box::new(WriterHandle { writer }))
}

/// Create an FbiWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_fbi(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::fbi_writer::FbiWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a BpfWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_bpf(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::bpf::BpfWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a TextWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_text(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::text_writer::TextWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PcdWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_pcd(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::pcd::PcdWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a PlyWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_ply(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        match pdal_io::ply::PlyWriter::new(options) {
            Ok(writer) => Box::into_raw(Box::new(WriterHandle {
                writer: Box::new(writer),
            })),
            Err(err) => {
                set_last_error(err.to_string());
                std::ptr::null_mut()
            }
        }
    } else {
        std::ptr::null_mut()
    }
}

/// Create a GltfWriter.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_gltf(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::gltf::GltfWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SbetWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_sbet(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::sbet_writer::SbetWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a LasWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_las(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::las_writer::LasWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a LazWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_laz(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::las_writer::LasWriter::new_laz(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a CopcWriter from options. Currently delegates to the Rust LAS/LAZ
/// writer with COPC-required defaults forced (LAS 1.4, LAZ compression, point
/// format 6 if not otherwise set). The resulting file is a LAS 1.4 LAZ that
/// the existing `LasReader` (Rust-backed) can read; explicit COPC structure
/// generation is deferred until a real Rust COPC writer lands.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_copc(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let mut opts = options.clone();
        if !opts.has("minor_version") {
            opts.add("minor_version", "4");
        }
        // Real COPC writer: builds the octree (copc info VLR + hierarchy EVLR +
        // per-node LAZ chunks) via the ported copcwriter subsystem.
        let writer = Box::new(pdal_io::copcwriter::writer::CopcWriter::new(&opts));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an SpzWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_spz(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::spz::SpzWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create an OgrWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_ogr(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::ogr_writer::OgrWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Opaque handle holding the result of a Rust EPT reader preview.
pub struct EptReaderPreviewHandle {
    pub(crate) preview: pdal_io::ept::EptPreview,
}

/// Read EPT preview metadata (boundsConforming, point count, srs wkt, dim
/// names) from a local `ept.json` file. Returns null on error; call
/// `pdal_last_error` for the message. Caller frees with
/// `pdal_ept_reader_preview_destroy`.
///
/// # Safety
/// `filename` must be a valid NUL-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_create(
    filename: *const c_char,
) -> *mut EptReaderPreviewHandle {
    if filename.is_null() {
        set_last_error("null filename");
        return std::ptr::null_mut();
    }
    let Ok(filename) = CStr::from_ptr(filename).to_str() else {
        set_last_error("non-UTF8 filename");
        return std::ptr::null_mut();
    };
    match pdal_io::ept::read_ept_preview(filename) {
        Ok(preview) => {
            clear_last_error();
            Box::into_raw(Box::new(EptReaderPreviewHandle { preview }))
        }
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Get the preview's point count.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_point_count(
    handle: *const EptReaderPreviewHandle,
) -> u64 {
    handle.as_ref().map_or(0, |h| h.preview.point_count)
}

/// Get the preview's bounds_conforming. Writes into `out` and returns true
/// on success.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`. `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_bounds(
    handle: *const EptReaderPreviewHandle,
    out_minx: *mut f64,
    out_miny: *mut f64,
    out_minz: *mut f64,
    out_maxx: *mut f64,
    out_maxy: *mut f64,
    out_maxz: *mut f64,
) -> bool {
    let Some(handle) = handle.as_ref() else {
        return false;
    };
    let b = &handle.preview.bounds_conforming;
    *out_minx = b.minx;
    *out_miny = b.miny;
    *out_minz = b.minz;
    *out_maxx = b.maxx;
    *out_maxy = b.maxy;
    *out_maxz = b.maxz;
    true
}

/// Get the preview's SRS WKT string. Returns an owned C string (possibly
/// empty). Caller frees with `pdal_string_free`.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_srs_wkt(
    handle: *const EptReaderPreviewHandle,
) -> *mut c_char {
    let Some(handle) = handle.as_ref() else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(handle.preview.srs_wkt.clone())
}

/// Get the number of dim names.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_dim_count(
    handle: *const EptReaderPreviewHandle,
) -> u64 {
    handle
        .as_ref()
        .map_or(0, |h| h.preview.dim_names.len() as u64)
}

/// Get a dim name by index. Returns an owned C string or null when the index
/// is out of range. Caller frees with `pdal_string_free`.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_dim_name(
    handle: *const EptReaderPreviewHandle,
    index: u64,
) -> *mut c_char {
    let Some(handle) = handle.as_ref() else {
        return std::ptr::null_mut();
    };
    let Some(name) = handle.preview.dim_names.get(index as usize) else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(name.clone())
}

/// Destroy an EPT preview handle.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`, or null.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_destroy(handle: *mut EptReaderPreviewHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Validate the OGR writer multicount/attr_dims combination on behalf of the
/// C++ wrapper. Returns null on success, otherwise an owned C string carrying
/// the unprefixed error message. Caller frees with `pdal_string_free`.
#[no_mangle]
pub extern "C" fn pdal_ogr_writer_validate(multicount: u64, attr_dim_count: u64) -> *mut c_char {
    match pdal_io::ogr_writer::validate_multicount_and_attrs(multicount, attr_dim_count) {
        Ok(()) => std::ptr::null_mut(),
        Err(message) => string_to_c_ptr(message),
    }
}

/// Format the "attr_dims dimension not found" error used by the C++ OGR
/// writer wrapper. Returns an owned C string. Caller frees with
/// `pdal_string_free`. Returns null when `name` is null or non-UTF8.
///
/// # Safety
/// `name` must be a valid C string pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pdal_ogr_writer_dim_not_found(name: *const c_char) -> *mut c_char {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(name) = CStr::from_ptr(name).to_str() else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(pdal_io::ogr_writer::format_attr_dim_not_found(name))
}

/// Create a GdalWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_gdal(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::gdal_writer::GdalWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Create a RasterWriter from options.
///
/// # Safety
/// `ops` must be a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_create_raster(ops: *const Options) -> *mut WriterHandle {
    if let Some(options) = ops.as_ref() {
        let writer = Box::new(pdal_io::raster_writer::RasterWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
    } else {
        std::ptr::null_mut()
    }
}

/// Write a point view with a writer.
///
/// # Safety
/// `writer` must be a valid pointer returned by `pdal_writer_create_*`.
/// `view` must be a valid pointer returned by `pdal_point_view_create` or
/// `pdal_reader_read_first`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_write_view(
    writer: *mut WriterHandle,
    view: *const PointView,
) -> bool {
    let (Some(writer), Some(view)) = (writer.as_mut(), view.as_ref()) else {
        set_last_error("null writer or view");
        return false;
    };

    match writer.writer.write(std::slice::from_ref(view)) {
        Ok(()) => {
            clear_last_error();
            true
        }
        Err(err) => {
            set_last_error(err.to_string());
            false
        }
    }
}

/// Write multiple point views with a writer.
///
/// # Safety
/// `writer` must be a valid pointer returned by `pdal_writer_create_*`.
/// `views` must point to `count` valid pointers returned by
/// `pdal_point_view_create` or `pdal_reader_read_first`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_write_views(
    writer: *mut WriterHandle,
    views: *const *const PointView,
    count: u64,
) -> bool {
    let Some(writer) = writer.as_mut() else {
        set_last_error("null writer");
        return false;
    };
    if views.is_null() && count != 0 {
        set_last_error("null views");
        return false;
    }

    let raw_views = std::slice::from_raw_parts(views, count as usize);
    let mut owned_views = Vec::with_capacity(raw_views.len());
    for view in raw_views {
        let Some(view) = view.as_ref() else {
            set_last_error("null view");
            return false;
        };
        owned_views.push(view.clone());
    }

    match writer.writer.write(&owned_views) {
        Ok(()) => {
            clear_last_error();
            true
        }
        Err(err) => {
            set_last_error(err.to_string());
            false
        }
    }
}

/// Destroy a writer handle.
///
/// # Safety
/// `writer` must be a valid pointer returned by `pdal_writer_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_writer_destroy(writer: *mut WriterHandle) {
    if !writer.is_null() {
        drop(Box::from_raw(writer));
    }
}
