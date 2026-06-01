use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use pdal_core::options::Options;
use pdal_core::point::PointView;
use std::ffi::{c_char, CStr};

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

/// Create a CopcWriter from options.
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
