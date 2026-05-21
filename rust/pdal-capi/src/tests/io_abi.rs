use super::*;

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
fn writer_write_view_consumes_point_view_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-writer-write-view-{}-{}.csv",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let filename_text = filename.display().to_string();

        let options = pdal_options_create();
        for (key, value) in [
            ("filename", filename_text.as_str()),
            ("order", "X:1,Y:1,Z:1"),
            ("keep_unspecified", "false"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let layout = pdal_point_layout_create();
        for dim in ["X", "Y", "Z"] {
            let name = CString::new(dim).unwrap();
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        let view = pdal_point_view_create(layout);
        let point = pdal_point_view_add_point(view);
        for (dim, value) in [("X", 1.25), ("Y", 2.5), ("Z", 3.75)] {
            let name = CString::new(dim).unwrap();
            pdal_point_view_set_f64(view, point, name.as_ptr(), value);
        }

        let writer = pdal_writer_create_text(options);
        assert!(!writer.is_null());
        assert!(pdal_writer_write_view(writer, view));
        assert_eq!(
            std::fs::read_to_string(&filename).unwrap(),
            "\"X\",\"Y\",\"Z\"\n1.2,2.5,3.8\n"
        );

        let _ = std::fs::remove_file(&filename);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view);
        pdal_options_destroy(options);
    }
}

#[test]
fn spz_writer_writes_view_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-spz-writer-{}-{}.spz",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let filename_text = filename.display().to_string();

        let options = pdal_options_create();
        for (key, value) in [
            ("filename", filename_text.as_str()),
            ("coordinate_orientation", "RDF"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let layout = pdal_point_layout_create();
        for dim in ["X", "Y", "Z"] {
            let name = CString::new(dim).unwrap();
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        let view = pdal_point_view_create(layout);
        let point = pdal_point_view_add_point(view);
        for (dim, value) in [("X", 1.0), ("Y", 2.0), ("Z", 3.0)] {
            let name = CString::new(dim).unwrap();
            pdal_point_view_set_f64(view, point, name.as_ptr(), value);
        }

        let writer = pdal_writer_create_spz(options);
        assert!(!writer.is_null());
        assert!(pdal_writer_write_view(writer, view));
        assert!(std::fs::metadata(&filename).unwrap().len() > 0);

        let _ = std::fs::remove_file(&filename);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view);
        pdal_options_destroy(options);
    }
}

#[test]
fn sbet_writer_writes_view_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-sbet-writer-{}-{}.sbet",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let filename_text = filename.display().to_string();

        let options = pdal_options_create();
        for (key, value) in [
            ("filename", filename_text.as_str()),
            ("angles_are_degrees", "false"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let layout = pdal_point_layout_create();
        for dim in [
            "GpsTime",
            "Y",
            "X",
            "Z",
            "XVelocity",
            "YVelocity",
            "ZVelocity",
            "Roll",
            "Pitch",
            "Azimuth",
            "WanderAngle",
            "XBodyAccel",
            "YBodyAccel",
            "ZBodyAccel",
            "XBodyAngRate",
            "YBodyAngRate",
            "ZBodyAngRate",
        ] {
            let name = CString::new(dim).unwrap();
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        let view = pdal_point_view_create(layout);
        let point = pdal_point_view_add_point(view);
        for (dim, value) in [("GpsTime", 1.0), ("X", 2.0), ("Y", 3.0), ("Z", 4.0)] {
            let name = CString::new(dim).unwrap();
            pdal_point_view_set_f64(view, point, name.as_ptr(), value);
        }

        let writer = pdal_writer_create_sbet(options);
        assert!(!writer.is_null());
        assert!(pdal_writer_write_view(writer, view));
        assert_eq!(std::fs::metadata(&filename).unwrap().len(), 17 * 8);

        let _ = std::fs::remove_file(&filename);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view);
        pdal_options_destroy(options);
    }
}
