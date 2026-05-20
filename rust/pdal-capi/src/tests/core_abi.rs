use super::*;

#[test]
fn spatial_reference_roundtrips_through_c_abi() {
    unsafe {
        let text = CString::new("EPSG:4326").unwrap();
        let srs = pdal_spatial_reference_create_with_epoch(text.as_ptr(), 2020.0);
        assert!(!pdal_spatial_reference_empty(srs));
        assert_eq!(pdal_spatial_reference_epoch(srs), 2020.0);
        assert_eq!(take_string(pdal_spatial_reference_text(srs)), "EPSG:4326");

        pdal_spatial_reference_set_epoch(srs, 2021.5);
        assert_eq!(pdal_spatial_reference_epoch(srs), 2021.5);
        pdal_spatial_reference_destroy(srs);
    }
}

#[test]
fn spatial_reference_wgs84_zone_code_roundtrips_through_c_abi() {
    unsafe {
        assert_eq!(
            take_string(pdal_spatial_reference_wgs84_code_from_zone(17)),
            "EPSG:32617"
        );
        assert_eq!(
            take_string(pdal_spatial_reference_wgs84_code_from_zone(-17)),
            "EPSG:32717"
        );
        assert_eq!(
            take_string(pdal_spatial_reference_wgs84_code_from_zone(0)),
            ""
        );
    }
}

#[test]
fn kernel_stage_option_parser_roundtrips_through_c_abi() {
    unsafe {
        let input = CString::new("--readers.p2g.foobar=baz").unwrap();
        let mut stage = std::ptr::null_mut();
        let mut option = std::ptr::null_mut();
        let mut value = std::ptr::null_mut();

        let result = pdal_kernel_parse_stage_option(
            input.as_ptr(),
            false,
            &mut stage,
            &mut option,
            &mut value,
        );
        assert_eq!(result, 0);
        assert_eq!(take_string(stage), "readers.p2g");
        assert_eq!(take_string(option), "foobar");
        assert_eq!(take_string(value), "baz");

        let input = CString::new("--stage.tag.option=value").unwrap();
        assert_eq!(
            pdal_kernel_parse_stage_option(
                input.as_ptr(),
                false,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ),
            2
        );
        assert_eq!(
            pdal_kernel_parse_stage_option(
                input.as_ptr(),
                true,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ),
            0
        );
    }
}

#[test]
fn plugin_name_validation_roundtrips_through_c_abi() {
    unsafe {
        let path = CString::new("libpdal_plugin_reader_a_b.dylib").unwrap();
        let reader = CString::new("reader").unwrap();
        let writer = CString::new("writer").unwrap();
        let ext = CString::new(".dylib").unwrap();
        let types = [reader.as_ptr(), writer.as_ptr()];

        assert_eq!(
            take_string(pdal_plugin_valid_name(
                path.as_ptr(),
                types.as_ptr(),
                types.len() as u64,
                ext.as_ptr(),
            )),
            "readers.a_b"
        );

        let bad = CString::new("libpdal_plugin_reader_1a_b.dylib").unwrap();
        assert_eq!(
            take_string(pdal_plugin_valid_name(
                bad.as_ptr(),
                types.as_ptr(),
                types.len() as u64,
                ext.as_ptr(),
            )),
            ""
        );
    }
}

#[test]
fn file_spec_parser_roundtrips_through_c_abi() {
    unsafe {
        let input =
            CString::new(r#"{"path":"foo.laz","headers":{"h":"v"},"query":{"q":"v"}}"#).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&take_string(pdal_file_spec_parse_json(input.as_ptr()))).unwrap();

        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["path"], "foo.laz");
        assert_eq!(parsed["headers"]["h"], "v");
        assert_eq!(parsed["query"]["q"], "v");

        let input = CString::new(r#"{"query":[]}"#).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&take_string(pdal_file_spec_parse_json(input.as_ptr()))).unwrap();
        assert_eq!(parsed["ok"], false);
        assert_eq!(
            parsed["error"],
            "'filename' object must contain 'path' member."
        );
    }
}

#[test]
fn option_name_validation_roundtrips_through_c_abi() {
    unsafe {
        let valid = CString::new("foo_123_bar_baz").unwrap();
        let bad = CString::new("foo_123_bar-baz").unwrap();

        assert!(pdal_option_name_valid(valid.as_ptr()));
        assert!(!pdal_option_name_valid(bad.as_ptr()));
        assert!(!pdal_option_name_valid(std::ptr::null()));
    }
}

#[test]
fn driver_inference_roundtrips_through_c_abi() {
    unsafe {
        let reader = CString::new("foo.laz").unwrap();
        let writer = CString::new("foo.tif").unwrap();
        let unknown = CString::new("foo.unknown").unwrap();

        assert_eq!(
            take_string(pdal_infer_reader_driver(reader.as_ptr())),
            "readers.las"
        );
        assert_eq!(
            take_string(pdal_infer_writer_driver(writer.as_ptr())),
            "writers.gdal"
        );
        assert_eq!(take_string(pdal_infer_reader_driver(unknown.as_ptr())), "");
    }
}

#[test]
fn config_helpers_roundtrip_through_c_abi() {
    unsafe {
        assert_eq!(pdal_config_version_integer(2, 10, 1), 21001);

        let version = CString::new("2.10.1").unwrap();
        let sha = CString::new("abcdef123456").unwrap();
        assert_eq!(
            take_string(pdal_config_full_version_string(
                version.as_ptr(),
                sha.as_ptr()
            )),
            "2.10.1 (git-version: abcdef)"
        );
    }
}

#[test]
fn log_level_strings_roundtrip_through_c_abi() {
    unsafe {
        assert_eq!(
            CStr::from_ptr(pdal_log_level_string(0)).to_string_lossy(),
            "Error"
        );
        assert_eq!(
            CStr::from_ptr(pdal_log_level_string(1)).to_string_lossy(),
            "Warning"
        );
        assert_eq!(
            CStr::from_ptr(pdal_log_level_string(2)).to_string_lossy(),
            "Info"
        );
        assert_eq!(
            CStr::from_ptr(pdal_log_level_string(7)).to_string_lossy(),
            "Debug"
        );
    }
}

#[test]
fn ogr_spec_roundtrips_through_c_abi() {
    unsafe {
        let input = CString::new(
                r#"{"type":"OGR","datasource":"attributes.json","drivers":["GeoJSON"],"options":{"dialect":"OGRSQL"}}"#,
            )
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&take_string(pdal_ogr_spec_parse_json(input.as_ptr()))).unwrap();

        assert_eq!(json["ok"], true);
        assert_eq!(json["datasource"], "attributes.json");
        assert_eq!(json["drivers"][0], "GeoJSON");
        assert_eq!(json["dialect"], "OGRSQL");
    }
}

#[test]
fn pipeline_stage_tag_generation_roundtrips_through_c_abi() {
    unsafe {
        let stage = CString::new("readers.las").unwrap();
        assert_eq!(
            take_string(pdal_pipeline_generate_stage_tag(
                stage.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            )),
            "readers_las1"
        );

        let existing = CString::new("readers_las1").unwrap();
        let tags = [existing.as_ptr()];
        assert_eq!(
            take_string(pdal_pipeline_generate_stage_tag(
                stage.as_ptr(),
                std::ptr::null(),
                tags.as_ptr(),
                tags.len() as u64,
            )),
            "readers_las2"
        );
    }
}

#[test]
fn utility_json_detection_roundtrips_through_c_abi() {
    unsafe {
        let object = CString::new(r#" {"path":"file.laz"} "#).unwrap();
        let plain = CString::new("file.laz").unwrap();

        assert!(pdal_utils_is_json(object.as_ptr()));
        assert!(!pdal_utils_is_json(plain.as_ptr()));
        assert!(!pdal_utils_is_json(std::ptr::null()));
    }
}

#[test]
fn native_dependencies_serialize_through_c_abi() {
    unsafe {
        let json: serde_json::Value =
            serde_json::from_str(&take_string(pdal_native_dependencies_json())).unwrap();

        assert!(json
            .as_array()
            .unwrap()
            .iter()
            .any(|dependency| dependency["name"] == "GDAL"));
        assert!(json
            .as_array()
            .unwrap()
            .iter()
            .all(|dependency| dependency["version"]
                .as_str()
                .is_some_and(|v| !v.is_empty())));
    }
}

#[test]
fn geometry_predicates_roundtrip_through_c_abi() {
    unsafe {
        let polygon = CString::new("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();

        let mut valid = false;
        assert!(pdal_geometry_wkt_is_valid(polygon.as_ptr(), &mut valid));
        assert!(valid);

        let mut contains = false;
        assert!(pdal_geometry_wkt_contains_point(
            polygon.as_ptr(),
            5.0,
            5.0,
            &mut contains
        ));
        assert!(contains);

        assert!(pdal_geometry_wkt_contains_point(
            polygon.as_ptr(),
            15.0,
            5.0,
            &mut contains
        ));
        assert!(!contains);
    }
}

#[test]
fn geometry_distance_roundtrips_through_c_abi() {
    unsafe {
        let point = CString::new("POINT(0 0 0)").unwrap();
        let mut distance = 0.0;

        assert!(pdal_geometry_wkt_distance_to_point(
            point.as_ptr(),
            3.0,
            4.0,
            0.0,
            &mut distance
        ));
        assert_eq!(distance, 5.0);
    }
}

#[test]
fn xml_schema_legacy_names_roundtrip_through_c_abi() {
    unsafe {
        let point_id = CString::new("Chipper Point ID").unwrap();
        let block_id = CString::new("Unnamed field 513").unwrap();
        let unchanged = CString::new("Intensity").unwrap();

        assert_eq!(
            take_string(pdal_xml_schema_remap_old_name(point_id.as_ptr())),
            "Chipper:PointID"
        );
        assert_eq!(
            take_string(pdal_xml_schema_remap_old_name(block_id.as_ptr())),
            "Chipper:BlockID"
        );
        assert_eq!(
            take_string(pdal_xml_schema_remap_old_name(unchanged.as_ptr())),
            "Intensity"
        );
    }
}
