use super::*;

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
fn ogr_writer_writes_geojson_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-ogr-writer-{}-{}.geojson",
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
            ("ogrdriver", "GeoJSON"),
            ("attr_dims", "Intensity"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let layout = pdal_point_layout_create();
        for dim in ["X", "Y", "Z", "Intensity"] {
            let name = CString::new(dim).unwrap();
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        let view = pdal_point_view_create(layout);
        let point = pdal_point_view_add_point(view);
        for (dim, value) in [("X", 1.0), ("Y", 2.0), ("Z", 3.0), ("Intensity", 10.0)] {
            let name = CString::new(dim).unwrap();
            pdal_point_view_set_f64(view, point, name.as_ptr(), value);
        }

        let writer = pdal_writer_create_ogr(options);
        assert!(!writer.is_null());
        assert!(
            pdal_writer_write_view(writer, view),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&filename).unwrap()).unwrap();
        assert_eq!(json["type"], "FeatureCollection");
        assert_eq!(json["features"][0]["properties"]["Intensity"], 10.0);

        let _ = std::fs::remove_file(&filename);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view);
        pdal_options_destroy(options);
    }
}

#[test]
fn gdal_writer_writes_raster_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-gdal-writer-{}-{}.tif",
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
            ("gdaldriver", "GTiff"),
            ("resolution", "1"),
            ("output_type", "count"),
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
        for (x, y, z) in [(0.25, 0.25, 1.0), (1.25, 0.25, 2.0), (0.25, 1.25, 3.0)] {
            let point = pdal_point_view_add_point(view);
            for (dim, value) in [("X", x), ("Y", y), ("Z", z)] {
                let name = CString::new(dim).unwrap();
                pdal_point_view_set_f64(view, point, name.as_ptr(), value);
            }
        }

        let writer = pdal_writer_create_gdal(options);
        assert!(!writer.is_null());
        assert!(
            pdal_writer_write_view(writer, view),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        assert!(std::fs::metadata(&filename).unwrap().len() > 0);

        let _ = std::fs::remove_file(&filename);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view);
        pdal_options_destroy(options);
    }
}

#[test]
fn raster_writer_writes_attachment_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-raster-writer-{}-{}.tif",
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
            ("gdaldriver", "GTiff"),
            ("rasters", "faceraster"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let layout = pdal_point_layout_create();
        let view = pdal_point_view_create(layout);
        let name = CString::new("faceraster").unwrap();
        let limits = pdal_raster_limits_t {
            x_origin: 10.0,
            y_origin: 20.0,
            width: 2,
            height: 2,
            edge_length: 1.0,
        };
        assert!(pdal_point_view_create_raster(
            view,
            name.as_ptr(),
            &limits,
            -9999.0
        ));
        assert!(pdal_point_view_set_raster_cell(
            view,
            name.as_ptr(),
            0,
            0,
            42.0
        ));

        let writer = pdal_writer_create_raster(options);
        assert!(!writer.is_null());
        assert!(
            pdal_writer_write_view(writer, view),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        assert!(std::fs::metadata(&filename).unwrap().len() > 0);

        let _ = std::fs::remove_file(&filename);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view);
        pdal_options_destroy(options);
    }
}

#[test]
fn writer_write_views_passes_multiple_views_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-raster-writer-views-{}-{}.tif",
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
            ("gdaldriver", "GTiff"),
            ("rasters", "a,b"),
            ("data_type", "float"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let layout_a = pdal_point_layout_create();
        let view_a = pdal_point_view_create(layout_a);
        let name_a = CString::new("a").unwrap();
        let limits = pdal_raster_limits_t {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 1,
            height: 1,
            edge_length: 1.0,
        };
        assert!(pdal_point_view_create_raster(
            view_a,
            name_a.as_ptr(),
            &limits,
            -9999.0
        ));
        assert!(pdal_point_view_set_raster_cell(
            view_a,
            name_a.as_ptr(),
            0,
            0,
            10.0
        ));

        let layout_b = pdal_point_layout_create();
        let view_b = pdal_point_view_create(layout_b);
        let name_b = CString::new("b").unwrap();
        assert!(pdal_point_view_create_raster(
            view_b,
            name_b.as_ptr(),
            &limits,
            -9999.0
        ));
        assert!(pdal_point_view_set_raster_cell(
            view_b,
            name_b.as_ptr(),
            0,
            0,
            20.0
        ));

        let writer = pdal_writer_create_raster(options);
        assert!(!writer.is_null());
        let views = [view_a as *const _, view_b as *const _];
        assert!(
            pdal_writer_write_views(writer, views.as_ptr(), views.len() as u64),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );

        let raster = pdal_core::gdal::Raster::open(&filename_text).unwrap();
        assert_eq!(raster.band_count(), 2);
        assert_eq!(raster.band_type_name(1).unwrap(), "Float32");
        let mut values = [0.0];
        raster.read_band(2, 1, 1, &mut values).unwrap();
        assert_eq!(values[0], 20.0);

        let _ = std::fs::remove_file(&filename);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view_a);
        pdal_point_view_destroy(view_b);
        pdal_options_destroy(options);
    }
}

#[test]
fn las_writer_writes_view_through_c_abi() {
    unsafe {
        let mut filename = std::env::temp_dir();
        filename.push(format!(
            "pdal-capi-las-writer-{}-{}.las",
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
            ("minor_version", "2"),
            ("dataformat_id", "3"),
        ] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
        }

        let layout = pdal_point_layout_create();
        for dim in ["X", "Y", "Z", "Intensity"] {
            let name = CString::new(dim).unwrap();
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        let view = pdal_point_view_create(layout);
        for (x, y, z, intensity) in [(1.0, 2.0, 3.0, 10.0), (4.0, 5.0, 6.0, 20.0)] {
            let point = pdal_point_view_add_point(view);
            for (dim, value) in [("X", x), ("Y", y), ("Z", z), ("Intensity", intensity)] {
                let name = CString::new(dim).unwrap();
                pdal_point_view_set_f64(view, point, name.as_ptr(), value);
            }
        }

        let writer = pdal_writer_create_las(options);
        assert!(!writer.is_null());
        assert!(
            pdal_writer_write_view(writer, view),
            "{}",
            CStr::from_ptr(pdal_last_error()).to_string_lossy()
        );
        assert!(std::fs::metadata(&filename).unwrap().len() > 0);

        let read_options = pdal_options_create();
        {
            let key = CString::new("filename").unwrap();
            let value = CString::new(filename_text).unwrap();
            pdal_options_add_str(read_options, key.as_ptr(), value.as_ptr());
        }
        let reader = pdal_reader_create_las(read_options);
        assert!(!reader.is_null());
        let back = pdal_reader_read_first(reader);
        assert!(!back.is_null());
        assert_eq!(pdal_point_view_length(back), 2);
        let z = CString::new("Z").unwrap();
        assert_eq!(pdal_point_view_get_f64(back, 1, z.as_ptr()), 6.0);

        let _ = std::fs::remove_file(&filename);
        pdal_point_view_destroy(back);
        pdal_reader_destroy(reader);
        pdal_options_destroy(read_options);
        pdal_writer_destroy(writer);
        pdal_point_view_destroy(view);
        pdal_options_destroy(options);
    }
}

#[test]
fn las_detect_copc_matches_fixture_signature() {
    unsafe {
        let copc = CString::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/copc/1.2-with-color.copc.laz"
        ))
        .unwrap();
        assert!(pdal_las_detect_copc(copc.as_ptr()));

        let las = CString::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/synthetic_test.las"
        ))
        .unwrap();
        assert!(!pdal_las_detect_copc(las.as_ptr()));
        assert!(!pdal_las_detect_copc(std::ptr::null()));
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
