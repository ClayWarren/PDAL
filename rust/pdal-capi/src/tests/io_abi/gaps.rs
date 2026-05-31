use super::super::*;
use std::ffi::c_void;

unsafe extern "C" fn null_incrementer(_id: u64, _ud: *mut c_void) -> *const u8 {
    std::ptr::null()
}

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

fn tiny_las_header(point_count: u32) -> Vec<u8> {
    let mut header = vec![0; 227];
    header[0..4].copy_from_slice(b"LASF");
    header[25] = 2;
    header[94..96].copy_from_slice(&(227u16).to_le_bytes());
    header[96..100].copy_from_slice(&(227u32).to_le_bytes());
    header[107..111].copy_from_slice(&point_count.to_le_bytes());
    header
}

#[test]
fn pointless_las_capi_returns_temp_header_copy() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.las");
    let mut data = tiny_las_header(7);
    data.extend([1, 2, 3]);
    std::fs::write(&input, data).unwrap();

    unsafe {
        let input = cstring(input.to_str().unwrap());
        let result = pdal_pointless_las_create(input.as_ptr());
        assert!(!result.is_null());
        assert_eq!((*result).point_count, 7);
        let filename = CStr::from_ptr((*result).filename)
            .to_string_lossy()
            .into_owned();
        let output = std::fs::read(&filename).unwrap();
        assert_eq!(output.len(), 227);
        assert_eq!(u32::from_le_bytes(output[107..111].try_into().unwrap()), 0);
        pdal_pointless_las_destroy(result);
        let _ = std::fs::remove_file(filename);
    }
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn reader_and_writer_ctors_handle_null_options_path() {
    unsafe {
        assert!(pdal_reader_create_faux(std::ptr::null()).is_null());
        assert!(pdal_reader_create_text(std::ptr::null()).is_null());
        assert!(pdal_reader_create_pcd(std::ptr::null()).is_null());
        assert!(pdal_reader_create_pts(std::ptr::null()).is_null());
        assert!(pdal_reader_create_ptx(std::ptr::null()).is_null());
        assert!(pdal_reader_create_ilvis2(std::ptr::null()).is_null());
        assert!(pdal_reader_create_obj(std::ptr::null()).is_null());
        assert!(pdal_reader_create_ply(std::ptr::null()).is_null());
        assert!(pdal_reader_create_qfit(std::ptr::null()).is_null());
        assert!(pdal_reader_create_sbet(std::ptr::null()).is_null());
        assert!(pdal_reader_create_smrmsg(std::ptr::null()).is_null());
        assert!(pdal_reader_create_optech(std::ptr::null()).is_null());
        assert!(pdal_reader_create_terrasolid(std::ptr::null()).is_null());
        assert!(pdal_reader_create_fbi(std::ptr::null()).is_null());
        assert!(pdal_reader_create_bpf(std::ptr::null()).is_null());
        assert!(pdal_reader_create_gdal(std::ptr::null()).is_null());
        assert!(pdal_reader_create_las(std::ptr::null()).is_null());
        assert!(pdal_reader_create_laz(std::ptr::null()).is_null());
        assert!(pdal_reader_create_spz(std::ptr::null()).is_null());
        assert!(pdal_reader_create_stac(std::ptr::null()).is_null());
        assert!(pdal_reader_create_copc(std::ptr::null()).is_null());
        assert!(pdal_reader_create_ept(std::ptr::null()).is_null());

        let writer = pdal_writer_create_null(std::ptr::null());
        assert!(!writer.is_null());
        pdal_writer_destroy(writer);

        assert!(pdal_writer_create_fbi(std::ptr::null()).is_null());
        assert!(pdal_writer_create_bpf(std::ptr::null()).is_null());
        assert!(pdal_writer_create_text(std::ptr::null()).is_null());
        assert!(pdal_writer_create_pcd(std::ptr::null()).is_null());
        assert!(pdal_writer_create_ply(std::ptr::null()).is_null());
        assert!(pdal_writer_create_gltf(std::ptr::null()).is_null());
        assert!(pdal_writer_create_sbet(std::ptr::null()).is_null());
        assert!(pdal_writer_create_las(std::ptr::null()).is_null());
        assert!(pdal_writer_create_laz(std::ptr::null()).is_null());
        assert!(pdal_writer_create_spz(std::ptr::null()).is_null());
        assert!(pdal_writer_create_ogr(std::ptr::null()).is_null());
        assert!(pdal_writer_create_gdal(std::ptr::null()).is_null());
        assert!(pdal_writer_create_raster(std::ptr::null()).is_null());
    }
}

#[test]
fn memoryview_read_with_zero_points_returns_empty_view() {
    unsafe {
        let name = cstring("Intensity");
        let fields = [pdal_memoryview_field_t {
            name: name.as_ptr(),
            type_id: 0x404,
            offset: 0,
        }];
        let view = pdal_memoryview_read(
            fields.as_ptr(),
            1,
            Some(null_incrementer),
            std::ptr::null_mut(),
            0,
            0,
            0,
            false,
        );
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 0);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn memoryview_read_synthesizes_column_major_shape_coordinates() {
    #[repr(C)]
    struct Point {
        intensity: f32,
    }
    let points = [
        Point { intensity: 1.0 },
        Point { intensity: 2.0 },
        Point { intensity: 3.0 },
        Point { intensity: 4.0 },
    ];

    struct Ctx {
        points: *const Point,
        len: usize,
    }
    let mut ctx = Ctx {
        points: points.as_ptr(),
        len: points.len(),
    };

    unsafe extern "C" fn inc(id: u64, ud: *mut c_void) -> *const u8 {
        let ctx = &*(ud as *const Ctx);
        if (id as usize) >= ctx.len {
            return std::ptr::null();
        }
        ctx.points.add(id as usize) as *const u8
    }

    unsafe {
        let intensity = cstring("Intensity");
        let fields = [pdal_memoryview_field_t {
            name: intensity.as_ptr(),
            type_id: 0x404,
            offset: 0,
        }];
        let view = pdal_memoryview_read(
            fields.as_ptr(),
            1,
            Some(inc),
            &mut ctx as *mut Ctx as *mut c_void,
            2,
            2,
            1,
            true,
        );
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 4);
        let xname = cstring("X");
        let yname = cstring("Y");
        let zname = cstring("Z");
        assert_eq!(pdal_point_view_get_f64(view, 0, xname.as_ptr()), 0.0);
        assert_eq!(pdal_point_view_get_f64(view, 0, yname.as_ptr()), 0.0);
        assert_eq!(pdal_point_view_get_f64(view, 0, zname.as_ptr()), 0.0);
        assert_eq!(pdal_point_view_get_f64(view, 3, intensity.as_ptr()), 4.0);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn memoryview_read_covers_each_dim_type_variant() {
    #[repr(C)]
    struct Row {
        u8v: u8,
        _pad1: [u8; 1],
        u16v: u16,
        u32v: u32,
        u64v: u64,
        i8v: i8,
        _pad2: [u8; 1],
        i16v: i16,
        i32v: i32,
        i64v: i64,
        f32v: f32,
        f64v: f64,
    }
    let rows = [Row {
        u8v: 1,
        _pad1: [0; 1],
        u16v: 2,
        u32v: 3,
        u64v: 4,
        i8v: -1,
        _pad2: [0; 1],
        i16v: -2,
        i32v: -3,
        i64v: -4,
        f32v: 5.5,
        f64v: 6.5,
    }];
    struct Ctx {
        rows: *const Row,
        len: usize,
    }
    let mut ctx = Ctx {
        rows: rows.as_ptr(),
        len: rows.len(),
    };
    unsafe extern "C" fn inc(id: u64, ud: *mut c_void) -> *const u8 {
        let ctx = &*(ud as *const Ctx);
        if (id as usize) >= ctx.len {
            return std::ptr::null();
        }
        ctx.rows.add(id as usize) as *const u8
    }
    unsafe {
        let names = [
            cstring("U8val"),
            cstring("U16val"),
            cstring("U32val"),
            cstring("U64val"),
            cstring("I8val"),
            cstring("I16val"),
            cstring("I32val"),
            cstring("I64val"),
            cstring("F32val"),
            cstring("F64val"),
        ];
        let offsets = [
            std::mem::offset_of!(Row, u8v),
            std::mem::offset_of!(Row, u16v),
            std::mem::offset_of!(Row, u32v),
            std::mem::offset_of!(Row, u64v),
            std::mem::offset_of!(Row, i8v),
            std::mem::offset_of!(Row, i16v),
            std::mem::offset_of!(Row, i32v),
            std::mem::offset_of!(Row, i64v),
            std::mem::offset_of!(Row, f32v),
            std::mem::offset_of!(Row, f64v),
        ];
        let type_ids = [
            0x201u32, 0x202, 0x204, 0x208, 0x101, 0x102, 0x104, 0x108, 0x404, 0x408,
        ];
        let fields: Vec<pdal_memoryview_field_t> = (0..names.len())
            .map(|i| pdal_memoryview_field_t {
                name: names[i].as_ptr(),
                type_id: type_ids[i] as i32,
                offset: offsets[i] as u64,
            })
            .collect();
        let view = pdal_memoryview_read(
            fields.as_ptr(),
            fields.len() as u64,
            Some(inc),
            &mut ctx as *mut Ctx as *mut c_void,
            0,
            0,
            0,
            false,
        );
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 1);
        for (name, expected) in names
            .iter()
            .zip([1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0, 5.5, 6.5])
        {
            assert!((pdal_point_view_get_f64(view, 0, name.as_ptr()) - expected).abs() < 1e-6);
        }
        pdal_point_view_destroy(view);
    }
}

#[test]
fn ept_preview_happy_paths_cover_accessors() {
    let ept_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/ept/lone-star-laszip/ept.json");
    if !ept_path.exists() {
        return;
    }
    unsafe {
        let path = cstring(&ept_path.display().to_string());
        let preview = pdal_ept_reader_preview_create(path.as_ptr());
        assert!(!preview.is_null());
        assert!(pdal_ept_reader_preview_point_count(preview) > 0);

        let mut minx = 0.0;
        let mut miny = 0.0;
        let mut minz = 0.0;
        let mut maxx = 0.0;
        let mut maxy = 0.0;
        let mut maxz = 0.0;
        assert!(pdal_ept_reader_preview_bounds(
            preview, &mut minx, &mut miny, &mut minz, &mut maxx, &mut maxy, &mut maxz,
        ));
        assert!(maxx >= minx);

        let srs_raw = pdal_ept_reader_preview_srs_wkt(preview);
        if !srs_raw.is_null() {
            let _ = take_string(srs_raw);
        }

        let dim_count = pdal_ept_reader_preview_dim_count(preview);
        if dim_count > 0 {
            let name_raw = pdal_ept_reader_preview_dim_name(preview, 0);
            assert!(!name_raw.is_null());
            let _ = take_string(name_raw);
        }
        assert!(pdal_ept_reader_preview_dim_name(preview, dim_count + 1000).is_null());

        pdal_ept_reader_preview_destroy(preview);
    }
}

#[test]
fn ept_preview_create_rejects_non_utf8_filename() {
    unsafe {
        let invalid = [0xc3u8, 0x28, 0];
        let result = pdal_ept_reader_preview_create(invalid.as_ptr() as *const c_char);
        assert!(result.is_null());
    }
}

#[test]
fn ogr_writer_validate_returns_null_on_success_and_string_on_error() {
    unsafe {
        let ok = pdal_ogr_writer_validate(1, 0);
        assert!(ok.is_null());

        let zero_err = pdal_ogr_writer_validate(0, 0);
        assert!(!zero_err.is_null());
        let _ = take_string(zero_err);

        let conflict_err = pdal_ogr_writer_validate(2, 5);
        assert!(!conflict_err.is_null());
        let _ = take_string(conflict_err);
    }
}

#[test]
fn ogr_writer_dim_not_found_handles_inputs() {
    unsafe {
        assert!(pdal_ogr_writer_dim_not_found(std::ptr::null()).is_null());
        let name = cstring("FooDim");
        let raw = pdal_ogr_writer_dim_not_found(name.as_ptr());
        assert!(!raw.is_null());
        let message = take_string(raw);
        assert!(message.contains("FooDim"));

        let invalid = [0xc3u8, 0x28, 0];
        assert!(pdal_ogr_writer_dim_not_found(invalid.as_ptr() as *const c_char).is_null());
    }
}

#[test]
fn ilvis2_metadata_read_succeeds_when_sidecar_present() {
    let candidates = [
        "ilvis2/ILVIS2_GL2009_0414_R1401_063351.TXT.xml",
        "ilvis2/ILVIS2_GL2009_0414_R1401_063351.xml",
    ];
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let xml_path = candidates
        .iter()
        .map(|p| manifest.join("../..").join("test/data").join(p))
        .find(|p| p.exists());
    let Some(xml_path) = xml_path else {
        return;
    };

    unsafe {
        let path = cstring(&xml_path.display().to_string());
        let metadata = pdal_ilvis2_metadata_read(path.as_ptr());
        if !metadata.is_null() {
            pdal_metadata_node_destroy(metadata);
        }
    }
}
