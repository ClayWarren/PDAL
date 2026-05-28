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
fn spatial_reference_list_matches_point_table_ordering_contract() {
    unsafe {
        let srs1_text = CString::new("EPSG:4326").unwrap();
        let srs2_text = CString::new("EPSG:32617").unwrap();
        let srs1 = pdal_spatial_reference_create(srs1_text.as_ptr());
        let srs2 = pdal_spatial_reference_create(srs2_text.as_ptr());
        let list = pdal_spatial_reference_list_create();

        assert!(pdal_spatial_reference_list_unique(list));
        assert_eq!(pdal_spatial_reference_list_size(list), 0);

        pdal_spatial_reference_list_add(list, srs1);
        pdal_spatial_reference_list_add(list, srs1);
        assert!(pdal_spatial_reference_list_unique(list));
        assert_eq!(pdal_spatial_reference_list_size(list), 1);
        let any = pdal_spatial_reference_list_any(list);
        assert_eq!(take_string(pdal_spatial_reference_text(any)), "EPSG:4326");
        pdal_spatial_reference_destroy(any);

        pdal_spatial_reference_list_add(list, srs2);
        assert!(!pdal_spatial_reference_list_unique(list));
        assert_eq!(pdal_spatial_reference_list_size(list), 2);
        let any = pdal_spatial_reference_list_any(list);
        assert_eq!(take_string(pdal_spatial_reference_text(any)), "EPSG:32617");
        pdal_spatial_reference_destroy(any);

        pdal_spatial_reference_list_add(list, srs1);
        let any = pdal_spatial_reference_list_any(list);
        assert_eq!(take_string(pdal_spatial_reference_text(any)), "EPSG:4326");
        pdal_spatial_reference_destroy(any);
        assert_eq!(pdal_spatial_reference_list_size(list), 2);

        pdal_spatial_reference_list_clear(list);
        assert!(pdal_spatial_reference_list_unique(list));
        assert_eq!(pdal_spatial_reference_list_size(list), 0);

        pdal_spatial_reference_list_destroy(list);
        pdal_spatial_reference_destroy(srs2);
        pdal_spatial_reference_destroy(srs1);
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
fn rust_stage_list_reports_core_registry_entries() {
    unsafe {
        let stages: serde_json::Value =
            serde_json::from_str(&take_string(pdal_rust_stage_list_json())).unwrap();
        let stages = stages.as_array().unwrap();

        assert!(stages.iter().any(|stage| stage == "filters.crop"));
        assert!(stages.iter().any(|stage| stage == "readers.las"));
        assert!(stages.iter().any(|stage| stage == "writers.bpf"));
    }
}

#[test]
fn stage_extensions_custom_mappings_roundtrip_through_c_abi() {
    unsafe {
        let extensions = pdal_stage_extensions_create();
        assert!(!extensions.is_null());

        let reader_stage = CString::new("readers.custom").unwrap();
        let reader_pcd = CString::new("pcd").unwrap();
        let reader_custom = CString::new("customreader").unwrap();
        let reader_values = [reader_pcd.as_ptr(), reader_custom.as_ptr()];
        pdal_stage_extensions_set(
            extensions,
            reader_stage.as_ptr(),
            reader_values.as_ptr(),
            reader_values.len() as u64,
        );

        let writer_stage = CString::new("writers.custom").unwrap();
        let writer_pcd = CString::new("pcd").unwrap();
        let writer_custom = CString::new("customwriter").unwrap();
        let writer_values = [writer_pcd.as_ptr(), writer_custom.as_ptr()];
        pdal_stage_extensions_set(
            extensions,
            writer_stage.as_ptr(),
            writer_values.as_ptr(),
            writer_values.len() as u64,
        );

        assert_eq!(
            take_string(pdal_stage_extensions_default_reader(
                extensions,
                reader_pcd.as_ptr()
            )),
            "readers.custom"
        );
        assert_eq!(
            take_string(pdal_stage_extensions_default_reader(
                extensions,
                reader_custom.as_ptr()
            )),
            "readers.custom"
        );
        assert_eq!(
            take_string(pdal_stage_extensions_default_writer(
                extensions,
                writer_pcd.as_ptr()
            )),
            "writers.custom"
        );
        assert_eq!(
            take_string(pdal_stage_extensions_default_writer(
                extensions,
                writer_custom.as_ptr()
            )),
            "writers.custom"
        );

        pdal_stage_extensions_destroy(extensions);
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

        // null version defaults to empty string
        assert_eq!(
            take_string(pdal_config_full_version_string(
                std::ptr::null(),
                sha.as_ptr()
            )),
            " (git-version: abcdef)"
        );

        // null sha defaults to empty string
        assert_eq!(
            take_string(pdal_config_full_version_string(
                version.as_ptr(),
                std::ptr::null()
            )),
            "2.10.1 (git-version: )"
        );

        // both null
        assert_eq!(
            take_string(pdal_config_full_version_string(
                std::ptr::null(),
                std::ptr::null()
            )),
            " (git-version: )"
        );
    }
}

#[test]
fn config_clear_error_clears_last_error() {
    unsafe {
        // Trigger an error first (infer reader with null should set error)
        pdal_infer_reader_driver(std::ptr::null());
        // Non-null error after inference
        pdal_clear_error();
        // After clear, the last error should be an empty string
        let last = CStr::from_ptr(pdal_last_error())
            .to_string_lossy()
            .to_string();
        assert_eq!(last, "");
    }
}

#[test]
fn config_string_free_handles_null() {
    unsafe {
        // Should not panic
        pdal_string_free(std::ptr::null_mut());
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
fn driver_inference_handles_null_filename() {
    unsafe {
        assert_eq!(take_string(pdal_infer_reader_driver(std::ptr::null())), "");
        assert_eq!(take_string(pdal_infer_writer_driver(std::ptr::null())), "");
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
fn geometry_wkt_output_roundtrips_through_c_abi() {
    unsafe {
        let polygon = CString::new("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        let mut out_wkt = std::ptr::null_mut();

        assert!(pdal_geometry_wkt_to_wkt(polygon.as_ptr(), &mut out_wkt));
        assert_eq!(take_string(out_wkt), "POLYGON ((0 0,10 0,10 10,0 10,0 0))");
        let point = CString::new("POINT (1.23456 2.34567)").unwrap();
        assert!(pdal_geometry_wkt_to_wkt_precision(
            point.as_ptr(),
            2,
            &mut out_wkt
        ));
        assert_eq!(take_string(out_wkt), "POINT (1.23 2.35)");
        assert!(!pdal_geometry_wkt_to_wkt(std::ptr::null(), &mut out_wkt));
    }
}

#[test]
fn geometry_json_is_valid_through_c_abi() {
    unsafe {
        let good = CString::new(
            r#"{ "srs": "EPSG:2991", "type": "Polygon", "coordinates": [ [ [0,0], [1,0], [1,1], [0,1], [0,0] ] ] }"#,
        )
        .unwrap();
        let mut valid = false;
        assert!(pdal_geometry_json_is_valid(good.as_ptr(), &mut valid));
        assert!(valid);

        let bad = CString::new("not json").unwrap();
        assert!(!pdal_geometry_json_is_valid(bad.as_ptr(), &mut valid));
    }
}

#[test]
fn geometry_wkt_to_json_matches_gdal_format() {
    unsafe {
        let polygon = CString::new(
            "POLYGON ((636889.412951239268295 851528.512293258565478 422.7001953125,\
             636899.14233423944097 851475.000686757150106 422.4697265625,\
             636928.33048324030824 851494.459452757611871 422.5400390625,\
             636889.412951239268295 851528.512293258565478 422.7001953125))",
        )
        .unwrap();
        let mut out_json = std::ptr::null_mut();
        assert!(pdal_geometry_wkt_to_json(
            polygon.as_ptr(),
            5,
            &mut out_json
        ));
        let json = take_string(out_json);
        assert!(json.starts_with(
            "{ \"type\": \"Polygon\", \"coordinates\": [ [ [ 636889.41295, 851528.51229, 422.7002 ]"
        ));
        assert!(json.ends_with("[ 636889.41295, 851528.51229, 422.7002 ] ] ] }"));

        assert!(!pdal_geometry_wkt_to_json(
            std::ptr::null(),
            5,
            &mut out_json
        ));
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
fn srs_user_input_to_wkt_returns_canonical_wkt_and_wkt2() {
    unsafe {
        let input = CString::new("EPSG:4326").unwrap();
        let mut wkt = std::ptr::null_mut();
        let mut wkt2 = std::ptr::null_mut();
        let mut epoch = 0.0;
        assert!(pdal_srs_user_input_to_wkt(
            input.as_ptr(),
            &mut wkt,
            &mut wkt2,
            &mut epoch
        ));
        let wkt = take_string(wkt);
        let wkt2 = take_string(wkt2);
        assert!(wkt.contains("WGS 84"));
        assert!(wkt.contains("GEOGCS["));
        assert!(wkt2.contains("WGS 84"));
        assert_eq!(epoch, 0.0);

        let bad = CString::new("not a srs").unwrap();
        let mut wkt_bad = std::ptr::null_mut();
        let mut wkt2_bad = std::ptr::null_mut();
        assert!(!pdal_srs_user_input_to_wkt(
            bad.as_ptr(),
            &mut wkt_bad,
            &mut wkt2_bad,
            &mut epoch
        ));
    }
}

#[test]
fn srs_wkt_to_proj4_returns_trimmed_string() {
    unsafe {
        let input = CString::new("EPSG:4326").unwrap();
        let mut wkt = std::ptr::null_mut();
        let mut wkt2 = std::ptr::null_mut();
        let mut epoch = 0.0;
        assert!(pdal_srs_user_input_to_wkt(
            input.as_ptr(),
            &mut wkt,
            &mut wkt2,
            &mut epoch
        ));
        let wkt_str = take_string(wkt);
        let _ = take_string(wkt2);
        let wkt_c = CString::new(wkt_str).unwrap();
        let mut proj4 = std::ptr::null_mut();
        assert!(pdal_srs_wkt_to_proj4(wkt_c.as_ptr(), &mut proj4));
        assert_eq!(take_string(proj4), "+proj=longlat +datum=WGS84 +no_defs");

        // Empty WKT yields empty PROJ4 without erroring.
        let empty = CString::new("").unwrap();
        let mut proj4 = std::ptr::null_mut();
        assert!(pdal_srs_wkt_to_proj4(empty.as_ptr(), &mut proj4));
        assert_eq!(take_string(proj4), "");
    }
}

#[test]
fn srs_wkt_to_projjson_returns_pdal_formatted_json() {
    unsafe {
        let input = CString::new("EPSG:4326").unwrap();
        let mut wkt = std::ptr::null_mut();
        let mut wkt2 = std::ptr::null_mut();
        let mut epoch = 0.0;
        assert!(pdal_srs_user_input_to_wkt(
            input.as_ptr(),
            &mut wkt,
            &mut wkt2,
            &mut epoch
        ));
        let wkt_str = take_string(wkt);
        let _ = take_string(wkt2);
        let wkt_c = CString::new(wkt_str).unwrap();
        let mut projjson = std::ptr::null_mut();

        assert!(pdal_srs_wkt_to_projjson(
            wkt_c.as_ptr(),
            epoch,
            &mut projjson
        ));
        let json = take_string(projjson);
        assert!(json.starts_with("{\n  \"type\": \"GeographicCRS\","));
        assert!(json.contains("\"name\": \"WGS 84\""));

        let empty = CString::new("").unwrap();
        let mut projjson = std::ptr::null_mut();
        assert!(pdal_srs_wkt_to_projjson(empty.as_ptr(), 0.0, &mut projjson));
        assert_eq!(take_string(projjson), "");
    }
}

#[test]
fn srs_wkt_export_helpers_route_through_c_abi() {
    unsafe {
        let input = CString::new("EPSG:32617").unwrap();
        let mut wkt = std::ptr::null_mut();
        let mut wkt2 = std::ptr::null_mut();
        let mut epoch = 0.0;
        assert!(pdal_srs_user_input_to_wkt(
            input.as_ptr(),
            &mut wkt,
            &mut wkt2,
            &mut epoch
        ));
        let wkt = CString::new(take_string(wkt)).unwrap();
        let wkt2 = CString::new(take_string(wkt2)).unwrap();

        let mut out = std::ptr::null_mut();
        assert!(pdal_srs_wkt_to_wkt1(wkt2.as_ptr(), epoch, &mut out));
        let wkt1 = take_string(out);
        assert!(wkt1.starts_with("PROJCS["));
        assert!(wkt1.contains("WGS 84 / UTM zone 17N"));

        assert!(pdal_srs_wkt_to_wkt2(wkt.as_ptr(), epoch, &mut out));
        let wkt2 = take_string(out);
        assert!(wkt2.starts_with("PROJCRS["));
        assert!(wkt2.contains("WGS 84 / UTM zone 17N"));

        assert!(pdal_srs_pretty_wkt(wkt.as_ptr(), &mut out));
        let pretty = take_string(out);
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("WGS 84 / UTM zone 17N"));

        let bad = CString::new("not wkt").unwrap();
        assert!(!pdal_srs_wkt_to_wkt1(bad.as_ptr(), 0.0, &mut out));
    }
}

#[test]
fn srs_kind_and_axis_ordering_route_through_c_abi() {
    unsafe {
        let geographic = CString::new("EPSG:4326").unwrap();
        let mut geographic_wkt = std::ptr::null_mut();
        let mut wkt2 = std::ptr::null_mut();
        let mut epoch = 0.0;
        assert!(pdal_srs_user_input_to_wkt(
            geographic.as_ptr(),
            &mut geographic_wkt,
            &mut wkt2,
            &mut epoch
        ));
        let geographic_wkt = CString::new(take_string(geographic_wkt)).unwrap();
        let _ = take_string(wkt2);

        let projected = CString::new("EPSG:32617").unwrap();
        let mut projected_wkt = std::ptr::null_mut();
        let mut wkt2 = std::ptr::null_mut();
        assert!(pdal_srs_user_input_to_wkt(
            projected.as_ptr(),
            &mut projected_wkt,
            &mut wkt2,
            &mut epoch
        ));
        let projected_wkt = CString::new(take_string(projected_wkt)).unwrap();
        let _ = take_string(wkt2);

        let mut value = false;
        assert!(pdal_srs_is_geographic(
            geographic_wkt.as_ptr(),
            0.0,
            &mut value
        ));
        assert!(value);
        assert!(pdal_srs_is_projected(
            projected_wkt.as_ptr(),
            0.0,
            &mut value
        ));
        assert!(value);
        assert!(pdal_srs_is_geocentric(
            geographic_wkt.as_ptr(),
            0.0,
            &mut value
        ));
        assert!(!value);

        let mut len = 0;
        let ordering = pdal_srs_axis_ordering(geographic_wkt.as_ptr(), 0.0, &mut len);
        assert!(!ordering.is_null());
        assert!(len >= 2);
        let values = std::slice::from_raw_parts(ordering, len as usize);
        assert!(values.iter().all(|axis| *axis > 0));
        pdal_i32_array_free(ordering, len);

        let empty = CString::new("").unwrap();
        let ordering = pdal_srs_axis_ordering(empty.as_ptr(), 0.0, &mut len);
        assert!(ordering.is_null());
        assert_eq!(len, 0);
    }
}

#[test]
fn srs_is_same_matches_equivalent_srs_through_c_abi() {
    unsafe {
        let a = CString::new("EPSG:4326").unwrap();
        let b = CString::new("+proj=longlat +datum=WGS84 +no_defs").unwrap();
        let mut wkt_a = std::ptr::null_mut();
        let mut wkt2_a = std::ptr::null_mut();
        let mut wkt_b = std::ptr::null_mut();
        let mut wkt2_b = std::ptr::null_mut();
        let mut epoch = 0.0;
        assert!(pdal_srs_user_input_to_wkt(
            a.as_ptr(),
            &mut wkt_a,
            &mut wkt2_a,
            &mut epoch
        ));
        assert!(pdal_srs_user_input_to_wkt(
            b.as_ptr(),
            &mut wkt_b,
            &mut wkt2_b,
            &mut epoch
        ));
        let wkt_a = CString::new(take_string(wkt_a)).unwrap();
        let wkt_b = CString::new(take_string(wkt_b)).unwrap();
        let _ = take_string(wkt2_a);
        let _ = take_string(wkt2_b);

        let mut same = false;
        assert!(pdal_srs_is_same(
            wkt_a.as_ptr(),
            wkt_b.as_ptr(),
            0.0,
            &mut same
        ));
        assert!(same);

        // Empty inputs report not-same without error.
        let empty = CString::new("").unwrap();
        let mut same = true;
        assert!(pdal_srs_is_same(
            empty.as_ptr(),
            wkt_b.as_ptr(),
            0.0,
            &mut same
        ));
        assert!(!same);
    }
}

#[test]
fn srs_identify_horizontal_epsg_returns_code() {
    unsafe {
        let input = CString::new("EPSG:32617").unwrap();
        let mut wkt = std::ptr::null_mut();
        let mut wkt2 = std::ptr::null_mut();
        let mut epoch = 0.0;
        assert!(pdal_srs_user_input_to_wkt(
            input.as_ptr(),
            &mut wkt,
            &mut wkt2,
            &mut epoch
        ));
        let wkt_c = CString::new(take_string(wkt)).unwrap();
        let _ = take_string(wkt2);
        let mut code = std::ptr::null_mut();
        assert!(pdal_srs_identify_horizontal_epsg(
            wkt_c.as_ptr(),
            0.0,
            &mut code
        ));
        assert_eq!(take_string(code), "32617");
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
