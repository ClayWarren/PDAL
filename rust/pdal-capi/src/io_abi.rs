use crate::error::{clear_last_error, set_last_error};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::point::PointView;
use std::ffi::{c_char, CStr};
use std::path::Path;

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
        let reader = Box::new(pdal_io::faux::FauxReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
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
        let reader = Box::new(pdal_io::las::LasReader::new(options));
        Box::into_raw(Box::new(ReaderHandle { reader }))
    } else {
        std::ptr::null_mut()
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
        let writer = Box::new(pdal_io::ply::PlyWriter::new(options));
        Box::into_raw(Box::new(WriterHandle { writer }))
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
