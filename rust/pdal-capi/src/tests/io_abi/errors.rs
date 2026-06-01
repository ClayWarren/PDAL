use super::*;

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
