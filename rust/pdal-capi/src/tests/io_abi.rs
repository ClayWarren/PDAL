use super::*;
use std::ffi::c_void;

mod las;

#[test]
fn copc_info_abi_decodes_little_endian_payload() {
    let mut data = Vec::new();
    for value in [1.0, 2.0, 3.0, 4.0, 5.0] {
        data.extend_from_slice(&f64::to_le_bytes(value));
    }
    data.extend_from_slice(&1234_u64.to_le_bytes());
    data.extend_from_slice(&5678_u64.to_le_bytes());
    data.extend_from_slice(&10.0_f64.to_le_bytes());
    data.extend_from_slice(&20.0_f64.to_le_bytes());
    for value in 0..11 {
        data.extend_from_slice(&f64::to_le_bytes(value as f64 + 100.0));
    }

    unsafe {
        let mut info = pdal_copc_info_t {
            center_x: 0.0,
            center_y: 0.0,
            center_z: 0.0,
            halfsize: 0.0,
            spacing: 0.0,
            root_hier_offset: 0,
            root_hier_size: 0,
            gpstime_minimum: 0.0,
            gpstime_maximum: 0.0,
            reserved: [0.0; 11],
        };

        assert!(pdal_copc_info_parse(
            data.as_ptr(),
            data.len() as u64,
            &mut info
        ));
        assert_eq!(info.center_x, 1.0);
        assert_eq!(info.center_y, 2.0);
        assert_eq!(info.center_z, 3.0);
        assert_eq!(info.halfsize, 4.0);
        assert_eq!(info.spacing, 5.0);
        assert_eq!(info.root_hier_offset, 1234);
        assert_eq!(info.root_hier_size, 5678);
        assert_eq!(info.gpstime_minimum, 10.0);
        assert_eq!(info.gpstime_maximum, 20.0);
        assert_eq!(info.reserved[0], 100.0);
        assert_eq!(info.reserved[10], 110.0);

        assert!(!pdal_copc_info_parse(data.as_ptr(), 8, &mut info));
        assert!(!pdal_copc_info_parse(
            std::ptr::null(),
            data.len() as u64,
            &mut info
        ));
        assert!(!pdal_copc_info_parse(
            data.as_ptr(),
            data.len() as u64,
            std::ptr::null_mut()
        ));
    }
}

#[test]
fn copc_hierarchy_abi_decodes_entries() {
    let mut data = Vec::new();
    for (level, x, y, z, offset, byte_size, point_count) in [
        (0_i32, 0_i32, 0_i32, 0_i32, 128_u64, 256_i32, -1_i32),
        (1_i32, 1_i32, 0_i32, 1_i32, 1024_u64, 512_i32, 42_i32),
    ] {
        data.extend_from_slice(&level.to_le_bytes());
        data.extend_from_slice(&x.to_le_bytes());
        data.extend_from_slice(&y.to_le_bytes());
        data.extend_from_slice(&z.to_le_bytes());
        data.extend_from_slice(&offset.to_le_bytes());
        data.extend_from_slice(&byte_size.to_le_bytes());
        data.extend_from_slice(&point_count.to_le_bytes());
    }

    unsafe {
        let mut entries = std::ptr::null_mut();
        let mut count = 0;
        assert!(pdal_copc_hierarchy_parse(
            data.as_ptr(),
            data.len() as u64,
            &mut entries,
            &mut count
        ));
        assert_eq!(count, 2);
        assert_eq!((*entries).d, 0);
        assert_eq!((*entries.add(1)).x, 1);
        assert_eq!((*entries.add(1)).z, 1);
        assert_eq!((*entries.add(1)).offset, 1024);
        assert_eq!((*entries.add(1)).byte_size, 512);
        assert_eq!((*entries.add(1)).point_count, 42);
        pdal_copc_entries_free(entries, count);
        pdal_copc_entries_free(std::ptr::null_mut(), 0);

        assert!(!pdal_copc_hierarchy_parse(
            data.as_ptr(),
            data.len() as u64 - 1,
            &mut entries,
            &mut count
        ));
        assert!(!pdal_copc_hierarchy_parse(
            data.as_ptr(),
            data.len() as u64,
            std::ptr::null_mut(),
            &mut count
        ));
    }
}

#[test]
fn copc_key_abi_parses_children_bounds_and_hashes() {
    unsafe {
        let value = CString::new("2-3-1-0").unwrap();
        let mut key = pdal_copc_key_t {
            d: 0,
            x: 0,
            y: 0,
            z: 0,
        };

        assert!(pdal_copc_key_parse(value.as_ptr(), &mut key));
        assert_eq!((key.d, key.x, key.y, key.z), (2, 3, 1, 0));

        let s = pdal_copc_key_to_string(&key);
        assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "2-3-1-0");
        pdal_string_free(s);

        let mut child = key;
        assert!(pdal_copc_key_child(&key, 5, &mut child));
        assert_eq!((child.d, child.x, child.y, child.z), (3, 7, 2, 1));

        let root = pdal_copc_bounds3d_t {
            minx: 0.0,
            maxx: 8.0,
            miny: 10.0,
            maxy: 18.0,
            minz: 20.0,
            maxz: 28.0,
        };
        let mut bounds = root;
        assert!(pdal_copc_key_bounds(&key, &root, &mut bounds));
        assert_eq!(bounds.minx, 6.0);
        assert_eq!(bounds.maxx, 8.0);
        assert_eq!(bounds.miny, 12.0);
        assert_eq!(bounds.maxy, 14.0);
        assert_eq!(bounds.minz, 20.0);
        assert_eq!(bounds.maxz, 22.0);

        assert_ne!(pdal_copc_key_hash(&key), pdal_copc_key_hash(&child));
        let bad = CString::new("2-3-1").unwrap();
        assert!(!pdal_copc_key_parse(bad.as_ptr(), &mut key));
        assert!(!pdal_copc_key_child(&key, 0, std::ptr::null_mut()));
        assert!(!pdal_copc_key_bounds(&key, std::ptr::null(), &mut bounds));
    }
}

#[test]
fn ept_key_abi_parses_strings_and_bisects_bounds() {
    unsafe {
        let value = CString::new("2-3-4-5").unwrap();
        let mut key = pdal_ept_key_t {
            d: 0,
            x: 0,
            y: 0,
            z: 0,
            bounds: pdal_bounds3d_t {
                minx: 0.0,
                maxx: 8.0,
                miny: 0.0,
                maxy: 8.0,
                minz: 0.0,
                maxz: 8.0,
            },
        };

        assert!(pdal_ept_key_parse(value.as_ptr(), &mut key));
        assert_eq!((key.d, key.x, key.y, key.z), (2, 3, 4, 5));

        let s = pdal_ept_key_to_string(&key);
        assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "2-3-4-5");
        pdal_string_free(s);

        key.bounds = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 8.0,
            miny: 0.0,
            maxy: 8.0,
            minz: 0.0,
            maxz: 8.0,
        };
        let mut child = key;
        assert!(pdal_ept_key_bisect(&key, 5, &mut child));
        assert_eq!((child.d, child.x, child.y, child.z), (3, 7, 8, 11));
        assert_eq!(child.bounds.minx, 4.0);
        assert_eq!(child.bounds.maxx, 8.0);
        assert_eq!(child.bounds.miny, 0.0);
        assert_eq!(child.bounds.maxy, 4.0);
        assert_eq!(child.bounds.minz, 4.0);
        assert_eq!(child.bounds.maxz, 8.0);

        let bad = CString::new("1-2-3").unwrap();
        assert!(!pdal_ept_key_parse(bad.as_ptr(), &mut key));
        assert!(!pdal_ept_key_parse(std::ptr::null(), &mut key));
        assert!(!pdal_ept_key_bisect(&key, 0, std::ptr::null_mut()));
    }
}

#[test]
fn reader_and_writer_constructors_cover_supported_driver_handles() {
    unsafe {
        let options = pdal_options_create();
        let filename = CString::new("dummy").unwrap();
        let key = CString::new("filename").unwrap();
        pdal_options_add_str(options, key.as_ptr(), filename.as_ptr());

        let readers = [
            pdal_reader_create_faux(options),
            pdal_reader_create_text(options),
            pdal_reader_create_pcd(options),
            pdal_reader_create_pts(options),
            pdal_reader_create_ptx(options),
            pdal_reader_create_ilvis2(options),
            pdal_reader_create_obj(options),
            pdal_reader_create_ply(options),
            pdal_reader_create_qfit(options),
            pdal_reader_create_sbet(options),
            pdal_reader_create_smrmsg(options),
            pdal_reader_create_optech(options),
            pdal_reader_create_terrasolid(options),
            pdal_reader_create_tindex(options),
            pdal_reader_create_fbi(options),
            pdal_reader_create_bpf(options),
            pdal_reader_create_gdal(options),
            pdal_reader_create_las(options),
            pdal_reader_create_laz(options),
            pdal_reader_create_spz(options),
            pdal_reader_create_stac(options),
            pdal_reader_create_copc(options),
            pdal_reader_create_ept(options),
        ];
        for reader in readers {
            assert!(!reader.is_null());
            pdal_reader_destroy(reader);
        }

        let writers = [
            pdal_writer_create_null(std::ptr::null()),
            pdal_writer_create_fbi(options),
            pdal_writer_create_bpf(options),
            pdal_writer_create_text(options),
            pdal_writer_create_pcd(options),
            pdal_writer_create_ply(options),
            pdal_writer_create_gltf(options),
            pdal_writer_create_sbet(options),
            pdal_writer_create_las(options),
            pdal_writer_create_laz(options),
            pdal_writer_create_spz(options),
            pdal_writer_create_ogr(options),
            pdal_writer_create_gdal(options),
            pdal_writer_create_raster(options),
        ];
        for writer in writers {
            assert!(!writer.is_null());
            pdal_writer_destroy(writer);
        }

        assert!(pdal_reader_create_faux(std::ptr::null()).is_null());
        assert!(pdal_writer_create_fbi(std::ptr::null()).is_null());
        assert!(pdal_reader_read_first(std::ptr::null_mut()).is_null());
        assert!(pdal_reader_metadata(std::ptr::null()).is_null());
        assert!(!pdal_writer_write_view(
            std::ptr::null_mut(),
            std::ptr::null()
        ));
        assert!(!pdal_writer_write_views(
            std::ptr::null_mut(),
            std::ptr::null(),
            0
        ));

        pdal_reader_destroy(std::ptr::null_mut());
        pdal_writer_destroy(std::ptr::null_mut());
        pdal_options_destroy(options);
    }
}

#[test]
fn reader_read_first_returns_point_view_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        for (key, value) in [
            ("mode", "ramp"),
            ("count", "3"),
            ("minx", "10"),
            ("maxx", "12"),
            ("miny", "20"),
            ("maxy", "22"),
            ("minz", "30"),
            ("maxz", "32"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_faux(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 3);

        let x = CString::new("X").unwrap();
        let y = CString::new("Y").unwrap();
        let z = CString::new("Z").unwrap();
        assert_eq!(pdal_point_view_get_f64(view, 0, x.as_ptr()), 10.0);
        assert_eq!(pdal_point_view_get_f64(view, 1, y.as_ptr()), 21.0);
        assert_eq!(pdal_point_view_get_f64(view, 2, z.as_ptr()), 32.0);

        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[repr(C)]
struct MemoryPoint {
    x: f64,
    y: f64,
    z: f64,
    intensity: u16,
}

unsafe extern "C" fn memory_incrementer(point_id: u64, user_data: *mut c_void) -> *const u8 {
    let points = &*(user_data as *const Vec<MemoryPoint>);
    points
        .get(point_id as usize)
        .map(|point| point as *const MemoryPoint as *const u8)
        .unwrap_or(std::ptr::null())
}

#[test]
fn memoryview_shape_parse_rejects_malformed_values() {
    unsafe {
        let valid = CString::new("1, 2, 3").unwrap();
        let mut depth = 0;
        let mut rows = 0;
        let mut columns = 0;
        assert!(
            pdal_memoryview_shape_parse(valid.as_ptr(), &mut depth, &mut rows, &mut columns)
                .is_null()
        );
        assert_eq!((depth, rows, columns), (1, 2, 3));

        let too_short = CString::new("1, 2").unwrap();
        let err =
            pdal_memoryview_shape_parse(too_short.as_ptr(), &mut depth, &mut rows, &mut columns);
        assert!(!err.is_null());
        let message = CStr::from_ptr(err).to_string_lossy();
        assert!(message.contains("three integers"));
        pdal_string_free(err);
    }
}

#[test]
fn memoryview_reader_materializes_callback_memory_through_c_abi() {
    let points = vec![
        MemoryPoint {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            intensity: 10,
        },
        MemoryPoint {
            x: 4.0,
            y: 5.0,
            z: 6.0,
            intensity: 20,
        },
    ];
    let x = CString::new("X").unwrap();
    let y = CString::new("Y").unwrap();
    let z = CString::new("Z").unwrap();
    let intensity = CString::new("Intensity").unwrap();
    let fields = [
        pdal_memoryview_field_t {
            name: x.as_ptr(),
            type_id: 0x408,
            offset: std::mem::offset_of!(MemoryPoint, x) as u64,
        },
        pdal_memoryview_field_t {
            name: y.as_ptr(),
            type_id: 0x408,
            offset: std::mem::offset_of!(MemoryPoint, y) as u64,
        },
        pdal_memoryview_field_t {
            name: z.as_ptr(),
            type_id: 0x408,
            offset: std::mem::offset_of!(MemoryPoint, z) as u64,
        },
        pdal_memoryview_field_t {
            name: intensity.as_ptr(),
            type_id: 0x202,
            offset: std::mem::offset_of!(MemoryPoint, intensity) as u64,
        },
    ];

    unsafe {
        let view = pdal_memoryview_read(
            fields.as_ptr(),
            fields.len() as u64,
            Some(memory_incrementer),
            &points as *const Vec<MemoryPoint> as *mut c_void,
            0,
            0,
            0,
            false,
        );
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 2);
        assert_eq!(pdal_point_view_get_f64(view, 1, x.as_ptr()), 4.0);
        assert_eq!(pdal_point_view_get_f64(view, 1, intensity.as_ptr()), 20.0);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn memoryview_reader_synthesizes_row_major_shape_coordinates() {
    let values = vec![
        MemoryPoint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            intensity: 10,
        },
        MemoryPoint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            intensity: 20,
        },
        MemoryPoint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            intensity: 30,
        },
        MemoryPoint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            intensity: 40,
        },
    ];
    let intensity = CString::new("Intensity").unwrap();
    let x = CString::new("X").unwrap();
    let y = CString::new("Y").unwrap();
    let z = CString::new("Z").unwrap();
    let fields = [pdal_memoryview_field_t {
        name: intensity.as_ptr(),
        type_id: 0x202,
        offset: std::mem::offset_of!(MemoryPoint, intensity) as u64,
    }];

    unsafe {
        let view = pdal_memoryview_read(
            fields.as_ptr(),
            fields.len() as u64,
            Some(memory_incrementer),
            &values as *const Vec<MemoryPoint> as *mut c_void,
            1,
            2,
            2,
            false,
        );
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 4);
        assert_eq!(pdal_point_view_get_f64(view, 0, x.as_ptr()), 0.0);
        assert_eq!(pdal_point_view_get_f64(view, 1, x.as_ptr()), 1.0);
        assert_eq!(pdal_point_view_get_f64(view, 2, y.as_ptr()), 1.0);
        assert_eq!(pdal_point_view_get_f64(view, 3, z.as_ptr()), 0.0);
        assert_eq!(pdal_point_view_get_f64(view, 3, intensity.as_ptr()), 40.0);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn reader_metadata_returns_reader_metadata_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        for (key, value) in [
            ("filename", data_path("ilvis2/ILVIS2_TEST_FILE.TXT")),
            ("metadata", data_path("ilvis2/ILVIS2_TEST_FILE.TXT.xml")),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_ilvis2(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(!view.is_null());

        let metadata = pdal_reader_metadata(reader);
        assert!(!metadata.is_null());
        let granule = CString::new("GranuleUR").unwrap();
        assert_eq!(
            pdal_metadata_node_child_named_count(metadata, granule.as_ptr()),
            1
        );
        let child = pdal_metadata_node_child_named(metadata, granule.as_ptr(), 0);
        assert_eq!(
            take_string(pdal_metadata_node_value(child)),
            "SC:ILVIS2.001:51203496"
        );

        pdal_metadata_node_destroy(child);
        pdal_metadata_node_destroy(metadata);
        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn qfit_reader_returns_points_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        for (key, value) in [
            ("filename", data_path("qfit/10-word.qi")),
            ("flip_coordinates", "false".to_string()),
            ("scale_z", "0.001".to_string()),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_qfit(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 2000);

        let x = CString::new("X").unwrap();
        let z = CString::new("Z").unwrap();
        assert_eq!(pdal_point_view_get_f64(view, 0, x.as_ptr()), 221.826822);
        assert_eq!(pdal_point_view_get_f64(view, 2, z.as_ptr()), 32.0);

        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn terrasolid_reader_returns_points_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        {
            let (key, value) = ("filename", data_path("terrasolid/20020715-time-color.bin"));
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_terrasolid(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 1000);

        let x = CString::new("X").unwrap();
        let intensity = CString::new("Intensity").unwrap();
        let point_source_id = CString::new("PointSourceId").unwrap();
        let alpha = CString::new("Alpha").unwrap();
        assert_eq!(pdal_point_view_get_f64(view, 0, x.as_ptr()), 363127.94);
        assert_eq!(pdal_point_view_get_f64(view, 0, intensity.as_ptr()), 1840.0);
        assert_eq!(
            pdal_point_view_get_f64(view, 0, point_source_id.as_ptr()),
            27207.0
        );
        assert_eq!(pdal_point_view_get_f64(view, 0, alpha.as_ptr()), 0.0);

        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn fbi_reader_returns_points_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        {
            let (key, value) = ("filename", data_path("fbi/1.2-with-color.fbi"));
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_fbi(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 1065);

        let x = CString::new("X").unwrap();
        let intensity = CString::new("Intensity").unwrap();
        let classification = CString::new("Classification").unwrap();
        assert!((pdal_point_view_get_f64(view, 0, x.as_ptr()) - 635618.98).abs() < 1e-4);
        assert_eq!(
            pdal_point_view_get_f64(view, 0, intensity.as_ptr()),
            55040.0
        );
        assert_eq!(
            pdal_point_view_get_f64(view, 0, classification.as_ptr()),
            20.0
        );

        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn fbi_header_info_returns_summary_through_c_abi() {
    unsafe {
        let filename = CString::new(data_path("fbi/1.2-with-color.fbi")).unwrap();
        let mut info = pdal_fbi_header_info_t {
            version: 0,
            header_size: 0,
            point_count: 0,
            xyz_position: 0,
        };

        assert_eq!(pdal_fbi_header_info(filename.as_ptr(), &mut info), 0);
        assert_eq!(info.version, 1);
        assert_eq!(info.header_size, 1808);
        assert_eq!(info.point_count, 1065);
        assert_eq!(info.xyz_position, 1808);

        assert_eq!(pdal_fbi_header_info(std::ptr::null(), &mut info), -1);
        assert_eq!(
            pdal_fbi_header_info(filename.as_ptr(), std::ptr::null_mut()),
            -1
        );
    }
}

#[test]
fn gdal_reader_honors_header_dimensions_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        for (key, value) in [
            ("filename", data_path("gdal/autzen-height.tif")),
            ("header", "Intensity,Userdata,Z".to_string()),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_gdal(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(
            !view.is_null(),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        assert_eq!(pdal_point_view_length(view), 735 * 973);

        let x = CString::new("X").unwrap();
        let y = CString::new("Y").unwrap();
        let intensity = CString::new("Intensity").unwrap();
        let userdata = CString::new("Userdata").unwrap();
        let z = CString::new("Z").unwrap();
        assert_eq!(pdal_point_view_get_f64(view, 120000, x.as_ptr()), 195.5);
        assert_eq!(pdal_point_view_get_f64(view, 120000, y.as_ptr()), 163.5);
        assert_eq!(
            pdal_point_view_get_f64(view, 120000, intensity.as_ptr()),
            255.0
        );
        assert_eq!(
            pdal_point_view_get_f64(view, 120000, userdata.as_ptr()),
            213.0
        );
        assert_eq!(pdal_point_view_get_f64(view, 120000, z.as_ptr()), 0.0);

        let metadata = pdal_reader_metadata(reader);
        assert!(!metadata.is_null());
        let raster = CString::new("raster").unwrap();
        assert_eq!(
            pdal_metadata_node_child_named_count(metadata, raster.as_ptr()),
            1
        );

        pdal_metadata_node_destroy(metadata);
        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn las_reader_returns_points_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        {
            let (key, value) = ("filename", data_path("las/simple.las"));
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_las(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(
            !view.is_null(),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        assert_eq!(pdal_point_view_length(view), 1065);

        let x = CString::new("X").unwrap();
        let y = CString::new("Y").unwrap();
        let intensity = CString::new("Intensity").unwrap();
        assert!(pdal_point_view_get_f64(view, 0, x.as_ptr()).is_finite());
        assert!(pdal_point_view_get_f64(view, 0, y.as_ptr()).is_finite());
        assert!(pdal_point_view_get_f64(view, 0, intensity.as_ptr()).is_finite());

        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn las_reader_preserves_legacy_synthetic_flag_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        {
            let (key, value) = ("filename", data_path("las/synthetic_test.las"));
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_las(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(
            !view.is_null(),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        assert_eq!(pdal_point_view_length(view), 1);

        let classification = CString::new("Classification").unwrap();
        let synthetic = CString::new("Synthetic").unwrap();
        assert_eq!(
            pdal_point_view_get_f64(view, 0, classification.as_ptr()),
            0.0
        );
        assert_eq!(pdal_point_view_get_f64(view, 0, synthetic.as_ptr()), 1.0);

        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn las_reader_honors_geotiff_srs_vlr_order_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        {
            let (filename_key, filename) = ("filename", data_path("las/utm17.las"));
            let (order_key, order) = ("srs_vlr_order", "geotiff");
            let filename_key = CString::new(filename_key).unwrap();
            let filename = CString::new(filename).unwrap();
            let order_key = CString::new(order_key).unwrap();
            let order = CString::new(order).unwrap();
            pdal_options_add_str(options, filename_key.as_ptr(), filename.as_ptr());
            pdal_options_add_str(options, order_key.as_ptr(), order.as_ptr());
        }

        let reader = pdal_reader_create_las(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(
            !view.is_null(),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );

        let srs = pdal_point_view_spatial_reference(view);
        assert!(!srs.is_null());
        let text = take_string(pdal_spatial_reference_text(srs));
        assert!(
            text.contains("32617"),
            "expected geotiff order to resolve EPSG:32617, got {text}"
        );

        pdal_spatial_reference_destroy(srs);
        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn las_reader_preserves_extrabytes_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        {
            let (key, value) = ("filename", data_path("las/extrabytes.las"));
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_las(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(
            !view.is_null(),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );

        let flags0 = CString::new("Flags0").unwrap();
        let return_number = CString::new("ReturnNumber").unwrap();
        assert_eq!(
            pdal_point_view_get_f64(view, 0, flags0.as_ptr()),
            pdal_point_view_get_f64(view, 0, return_number.as_ptr())
        );
        assert_eq!(pdal_point_view_get_f64(view, 0, flags0.as_ptr()), 1.0);

        let mut saw_flags0 = false;
        for idx in 0..pdal_point_view_dim_count(view) {
            let name = take_string(pdal_point_view_dim_name(view, idx));
            if name == "Flags0" {
                saw_flags0 = true;
            }
        }
        assert!(saw_flags0);

        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn spz_reader_returns_points_through_c_abi() {
    unsafe {
        let options = pdal_options_create();
        {
            let (key, value) = ("filename", data_path("spz/fourth_st.spz"));
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_spz(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_length(view), 131_199);

        let x = CString::new("X").unwrap();
        let rot0 = CString::new("rot_0").unwrap();
        let color2 = CString::new("f_dc_2").unwrap();
        assert!(pdal_point_view_get_f64(view, 0, x.as_ptr()).is_finite());
        assert!(pdal_point_view_get_f64(view, 0, rot0.as_ptr()).is_finite());
        assert!(pdal_point_view_get_f64(view, 0, color2.as_ptr()).is_finite());

        let metadata = pdal_reader_metadata(reader);
        assert!(!metadata.is_null());
        let orientation = CString::new("coordinate_orientation").unwrap();
        assert_eq!(
            pdal_metadata_node_child_named_count(metadata, orientation.as_ptr()),
            1
        );

        pdal_metadata_node_destroy(metadata);
        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn stac_reader_returns_local_asset_through_c_abi() {
    unsafe {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "pdal-capi-stac-reader-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        std::fs::copy(
            data_path("ply/simple_text.ply"),
            dir.join("simple_text.ply"),
        )
        .unwrap();
        let item = dir.join("item.json");
        std::fs::write(
            &item,
            r#"{
  "type": "Feature",
  "assets": {
    "data": {"href": "simple_text.ply", "type": "application/octet-stream"}
  }
}"#,
        )
        .unwrap();

        let options = pdal_options_create();
        {
            let key = CString::new("filename").unwrap();
            let value = CString::new(item.display().to_string()).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let reader = pdal_reader_create_stac(options);
        assert!(!reader.is_null());
        let view = pdal_reader_read_first(reader);
        assert!(
            !view.is_null(),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        assert_eq!(pdal_point_view_length(view), 3);

        let x = CString::new("X").unwrap();
        assert_eq!(pdal_point_view_get_f64(view, 0, x.as_ptr()), -1.0);

        let _ = std::fs::remove_dir_all(&dir);
        pdal_point_view_destroy(view);
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
    }
}

#[test]
fn test_io_abi_nulls_and_errors() {
    unsafe {
        // --- 1. Reader null & error paths ---
        assert!(!pdal_las_detect_copc(std::ptr::null()));
        assert!(pdal_reader_read_first(std::ptr::null_mut()).is_null());
        assert!(pdal_reader_metadata(std::ptr::null()).is_null());

        // --- 2. MemoryView shape parse null ---
        let err_shape = pdal_memoryview_shape_parse(
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert!(!err_shape.is_null());
        take_string(err_shape);

        // --- 3. MemoryView read nulls & errors ---
        assert!(pdal_memoryview_read(
            std::ptr::null(),
            5,
            None,
            std::ptr::null_mut(),
            0,
            0,
            0,
            false
        )
        .is_null());
        let fields = [pdal_memoryview_field_t {
            name: std::ptr::null(),
            type_id: 0x408,
            offset: 0,
        }];
        assert!(pdal_memoryview_read(
            fields.as_ptr(),
            1,
            Some(memory_incrementer),
            std::ptr::null_mut(),
            0,
            0,
            0,
            false
        )
        .is_null());

        let name = CString::new("X").unwrap();
        let fields_bad_type = [pdal_memoryview_field_t {
            name: name.as_ptr(),
            type_id: 0x999,
            offset: 0,
        }];
        assert!(pdal_memoryview_read(
            fields_bad_type.as_ptr(),
            1,
            Some(memory_incrementer),
            std::ptr::null_mut(),
            0,
            0,
            0,
            false
        )
        .is_null());

        // --- 4. ILVIS2 metadata nulls & errors ---
        assert!(pdal_ilvis2_metadata_read(std::ptr::null()).is_null());
        let invalid_filename = [0xff, 0xff, 0]; // Invalid UTF-8
        assert!(pdal_ilvis2_metadata_read(invalid_filename.as_ptr() as *const c_char).is_null());
        let bad_path = CString::new("no/such/file.xml").unwrap();
        assert!(pdal_ilvis2_metadata_read(bad_path.as_ptr()).is_null());

        // --- 5. Writer nulls & errors ---
        assert!(!pdal_writer_write_view(
            std::ptr::null_mut(),
            std::ptr::null()
        ));
        assert!(!pdal_writer_write_views(
            std::ptr::null_mut(),
            std::ptr::null(),
            0
        ));
        let opt = pdal_options_create();
        let writer = pdal_writer_create_null(opt);
        assert!(!pdal_writer_write_views(writer, std::ptr::null(), 5));
        let views = [std::ptr::null()];
        assert!(!pdal_writer_write_views(writer, views.as_ptr(), 1));
        pdal_writer_destroy(writer);
        pdal_options_destroy(opt);

        // --- 6. EPT Preview nulls & errors ---
        assert!(pdal_ept_reader_preview_create(std::ptr::null()).is_null());
        assert!(
            pdal_ept_reader_preview_create(invalid_filename.as_ptr() as *const c_char).is_null()
        );
        assert!(pdal_ept_reader_preview_create(bad_path.as_ptr()).is_null());

        assert_eq!(pdal_ept_reader_preview_point_count(std::ptr::null()), 0);
        let mut minx = 0.0;
        let mut miny = 0.0;
        let mut minz = 0.0;
        let mut maxx = 0.0;
        let mut maxy = 0.0;
        let mut maxz = 0.0;
        assert!(!pdal_ept_reader_preview_bounds(
            std::ptr::null(),
            &mut minx,
            &mut miny,
            &mut minz,
            &mut maxx,
            &mut maxy,
            &mut maxz
        ));
        assert!(pdal_ept_reader_preview_srs_wkt(std::ptr::null()).is_null());
        assert_eq!(pdal_ept_reader_preview_dim_count(std::ptr::null()), 0);
        assert!(pdal_ept_reader_preview_dim_name(std::ptr::null(), 0).is_null());
        pdal_ept_reader_preview_destroy(std::ptr::null_mut());

        // EptPreview index out of bounds
        let preview_path = data_path("ept/lone-star-laszip/ept.json");
        let preview_path_c = CString::new(preview_path).unwrap();
        let preview = pdal_ept_reader_preview_create(preview_path_c.as_ptr());
        assert!(!preview.is_null());
        assert!(pdal_ept_reader_preview_dim_name(preview, 9999).is_null());
        pdal_ept_reader_preview_destroy(preview);
    }
}

mod ept_preview;
mod gaps;
mod writers;
