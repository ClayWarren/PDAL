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
