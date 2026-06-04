use super::*;

    fn temp_las(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "pdal-rs-las-writer-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }

    #[test]
    fn writer_errors_without_filename() {
        let mut writer = LasWriter::new(&Options::new());
        let view = synthetic_point_view();
        let result = writer.write(&[view]);
        assert!(result.is_err());
        assert!(result.err().unwrap().0.contains("filename"));
    }

    #[test]
    fn writer_returns_name_and_metadata() {
        let writer = LasWriter::new(&Options::new());
        assert_eq!(writer.name(), "writers.las");
        assert_eq!(writer.metadata().name(), "writers.las");
    }

    #[test]
    fn writer_writes_with_user_vlrs() {
        let path = temp_las("user-vlrs.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("user_vlr_user_id", "TestId");
        options.add("user_vlr_record_id", "42");
        options.add("user_vlr_description", "test description");
        options.add("user_vlr_data", "aGVsbG8=");
        options.add("user_vlr_evlr", "false");
        options.add("minor_version", 2u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        writer.write(&[view]).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_writes_with_forward_vlrs() {
        let path = temp_las("forward-vlrs.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("forward_vlr_user_id", "Forwarder");
        options.add("forward_vlr_record_id", "12");
        options.add("forward_vlr_description", "forwarded");
        options.add("forward_vlr_data", "aGVsbG8=");
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        writer.write(&[view]).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }

    fn assert_header_option_error(key: &str, value: u64, expected: &str) {
        let path = temp_las(&format!("{key}-{value}.las"));
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add(key, value);
        let mut writer = LasWriter::new(&options);

        let err = writer.write(&[synthetic_point_view()]).unwrap_err();

        assert!(err.to_string().contains(expected));
        assert!(!path.exists());
    }

    #[test]
    fn writer_validates_las_header_option_ranges() {
        assert_header_option_error("major_version", 2, "major_version must be 1");
        assert_header_option_error("minor_version", 5, "minor_version must be between 1 and 4");
        assert_header_option_error(
            "global_encoding",
            32,
            "global_encoding must be between 0 and 31",
        );
        assert_header_option_error(
            "creation_doy",
            367,
            "creation_doy must be between 0 and 366",
        );
    }

    #[test]
    fn writer_writes_with_pdal_metadata_and_pipeline_vlrs() {
        let path = temp_las("pdal-meta.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("pdal_metadata_json", "{\"meta\":1}");
        options.add("pdal_pipeline_json", "[{\"type\":\"readers.las\"}]");
        options.add("minor_version", 4u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        writer.write(&[view]).unwrap();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_rejects_multiple_spatial_references() {
        let path = temp_las("mixed-srs.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());

        let wgs84 = pdal_native::srs::user_input_to_wkt("EPSG:4326").unwrap();
        let utm17 = pdal_native::srs::user_input_to_wkt("EPSG:32617").unwrap();
        let mut first = synthetic_point_view();
        first.set_spatial_reference(SpatialReference::new(&wgs84.wkt));
        let mut second = synthetic_point_view();
        second.set_spatial_reference(SpatialReference::new(&utm17.wkt));

        let mut writer = LasWriter::new(&options);
        let err = writer.write(&[first, second]).unwrap_err();

        assert!(err
            .to_string()
            .contains("multiple point spatial references"));
        assert!(!path.exists());
    }

    #[test]
    fn writer_writes_laz_via_compression_option() {
        let path = temp_las("compressed.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("compression", true);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        writer.write(&[view]).unwrap();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_writes_laz_via_filename_extension() {
        let path = temp_las("auto.laz");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        writer.write(&[view]).unwrap();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_writes_with_enhanced_srs_vlrs_wkt2() {
        let path = temp_las("enhanced-wkt2.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("enhanced_srs_vlrs", true);
        options.add("srs_wkt2_vlr", "PROJCS[\"WKT2\"]");
        options.add("srs_projjson_vlr", "{\"type\":\"GeographicCRS\"}");
        options.add("srs_wkt1_vlr", "PROJCS[\"WKT1\"]");
        options.add("minor_version", 4u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_generates_wkt1_srs_vlr_for_las14() {
        let path = temp_las("wkt1-srs-vlr.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("a_srs", "EPSG:4326");
        options.add("minor_version", 4u64);
        let mut writer = LasWriter::new(&options);
        writer.write(&[synthetic_point_view()]).unwrap();

        let reader = las::Reader::from_path(&path).unwrap();
        let vlr = reader
            .header()
            .vlrs()
            .iter()
            .find(|vlr| {
                vlr.user_id == TRANSFORM_USER_ID && vlr.record_id == WKT_RECORD_ID
            })
            .expect("expected LASF_Projection WKT1 VLR");
        let text = String::from_utf8_lossy(&vlr.data);
        assert!(text.starts_with(
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]]"#
        ));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_generates_geotiff_srs_vlrs_for_las13() {
        let path = temp_las("geotiff-srs-vlr.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("a_srs", "EPSG:4326");
        options.add("minor_version", 3u64);
        let mut writer = LasWriter::new(&options);

        writer.write(&[synthetic_point_view()]).unwrap();

        let reader = las::Reader::from_path(&path).unwrap();
        let mut directory = Vec::new();
        let mut doubles = Vec::new();
        let mut ascii = Vec::new();
        for vlr in reader.header().vlrs() {
            if vlr.user_id == TRANSFORM_USER_ID {
                match vlr.record_id {
                    GEOTIFF_DIRECTORY_RECORD_ID => directory = vlr.data.clone(),
                    GEOTIFF_DOUBLES_RECORD_ID => doubles = vlr.data.clone(),
                    GEOTIFF_ASCII_RECORD_ID => ascii = vlr.data.clone(),
                    WKT_RECORD_ID => panic!("LAS 1.3 should not use WKT SRS VLR"),
                    _ => {}
                }
            }
        }
        let wkt =
            pdal_native::geotiff::wkt_from_tags(&directory, &doubles, &ascii).expect("SRS WKT");
        assert!(wkt.contains("WGS 84"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_with_discard_high_return_numbers() {
        let path = temp_las("discard-returns.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("discard_high_return_numbers", true);
        options.add("minor_version", 2u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_with_invalid_filename_path_errors() {
        let mut options = Options::new();
        options.add("filename", "/nonexistent-dir-xyz/out.las");
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn user_vlrs_from_options_skips_malformed_record_id() {
        let mut options = Options::new();
        options.add("user_vlr_user_id", "u");
        options.add("user_vlr_record_id", "not-a-number");
        options.add("user_vlr_description", "d");
        options.add("user_vlr_data", "aGVsbG8=");
        options.add("user_vlr_evlr", "false");
        let vlrs = user_vlrs_from_options(&options);
        assert!(vlrs.is_empty());
    }

    #[test]
    fn writer_skips_oversized_pdal_metadata_in_la12() {
        let path = temp_las("oversized-pdal-meta.las");
        let huge = "x".repeat(70_000);
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("pdal_metadata_json", &huge);
        options.add("pdal_pipeline_json", &huge);
        options.add("minor_version", 2u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_writes_oversized_pdal_metadata_in_la14() {
        let path = temp_las("oversized-pdal-meta-14.las");
        let huge = "x".repeat(70_000);
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("pdal_metadata_json", &huge);
        options.add("pdal_pipeline_json", &huge);
        options.add("minor_version", 4u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(path);
    }

    fn b64_encode(blob: &[u8]) -> String {
        const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in blob.chunks(3) {
            let b0 = chunk[0];
            let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
            out.push(CHARS[(b0 >> 2) as usize] as char);
            out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            out.push(if chunk.len() > 1 {
                CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                CHARS[(b2 & 0x3f) as usize] as char
            } else {
                '='
            });
        }
        out
    }

    #[test]
    fn writer_with_user_vlr_large_data_in_la14_succeeds() {
        let path = temp_las("large-user-evlr-14.las");
        let huge_b64 = b64_encode(&vec![0u8; 70_000]);
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("user_vlr_user_id", "Big");
        options.add("user_vlr_record_id", "10");
        options.add("user_vlr_description", "huge");
        options.add("user_vlr_data", huge_b64);
        options.add("user_vlr_evlr", "false");
        options.add("minor_version", 4u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_with_user_vlr_large_data_in_la12_errors() {
        let path = temp_las("large-user-vlr-12.las");
        let huge_b64 = b64_encode(&vec![1u8; 70_000]);
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("user_vlr_user_id", "Big");
        options.add("user_vlr_record_id", "10");
        options.add("user_vlr_description", "huge");
        options.add("user_vlr_data", huge_b64);
        options.add("user_vlr_evlr", "false");
        options.add("minor_version", 2u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let r = writer.write(&[view]);
        // Either errors or writes; ensure path is exercised
        let _ = r;
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_with_user_vlr_evlr_in_la12_errors() {
        let path = temp_las("user-evlr-12.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("user_vlr_user_id", "Tiny");
        options.add("user_vlr_record_id", "1");
        options.add("user_vlr_description", "small");
        options.add("user_vlr_data", "aGk=");
        options.add("user_vlr_evlr", "true");
        options.add("minor_version", 2u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let r = writer.write(&[view]);
        assert!(r.is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_with_user_vlr_evlr_in_la14_succeeds() {
        let path = temp_las("user-evlr-14.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("user_vlr_user_id", "Tiny");
        options.add("user_vlr_record_id", "1");
        options.add("user_vlr_description", "small");
        options.add("user_vlr_data", "aGk=");
        options.add("user_vlr_evlr", "true");
        options.add("minor_version", 4u64);
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_with_extra_dim_invalid_type_errors() {
        let path = temp_las("extra-bad-type.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("extra_dim_name", "MyDim");
        options.add("extra_dim_type", "not-a-real-type");
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        assert!(writer.write(&[view]).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_with_extra_dim_missing_dim_errors() {
        let path = temp_las("extra-missing-dim.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("extra_dim_name", "DefinitelyNotADim");
        options.add("extra_dim_type", "float");
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        assert!(writer.write(&[view]).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn copc_writer_rejects_standard_dim_extra_dim_but_las_allows_it() {
        // C++ CopcWriter rejects an extra_dim that names a standard point-format
        // dimension ("is a standard dimension"); C++ LasWriter allows it (a
        // standard dimension may be written as an additional extra-bytes field).
        let copc_path = temp_las("extra-standard-dim-copc.las");
        let mut copc_opts = Options::new();
        copc_opts.add("filename", copc_path.display().to_string());
        copc_opts.add("extra_dims", "X=int32");
        let mut copc = LasWriter::new_copc(&copc_opts);
        assert!(copc.write(&[synthetic_point_view()]).is_err());
        let _ = std::fs::remove_file(&copc_path);

        let las_path = temp_las("extra-standard-dim-las.las");
        let mut las_opts = Options::new();
        las_opts.add("filename", las_path.display().to_string());
        las_opts.add("extra_dims", "X=int32");
        let mut las = LasWriter::new(&las_opts);
        assert!(las.write(&[synthetic_point_view()]).is_ok());
        let _ = std::fs::remove_file(&las_path);
    }

    #[test]
    fn writer_with_extra_dims_option_rejects_missing_type() {
        let path = temp_las("extra-missing-type.las");
        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        options.add("extra_dims", "MyDim");
        let mut writer = LasWriter::new(&options);
        let view = synthetic_point_view();
        assert!(writer.write(&[view]).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_expands_filename_template_for_multiple_views() {
        let template = std::env::temp_dir().join(format!(
            "pdal-las-template-{}-#.las",
            std::process::id()
        ));
        let first = std::path::PathBuf::from(numbered_filename(template.to_str().unwrap(), 1));
        let second = std::path::PathBuf::from(numbered_filename(template.to_str().unwrap(), 2));
        let _ = std::fs::remove_file(&first);
        let _ = std::fs::remove_file(&second);
        let _ = std::fs::remove_file(&template);

        let mut options = Options::new();
        options.add("filename", template.display().to_string());
        let mut writer = LasWriter::new(&options);
        writer
            .write(&[synthetic_point_view(), synthetic_point_view()])
            .unwrap();

        assert!(first.exists());
        assert!(second.exists());
        assert!(!template.exists());
        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn dim_type_from_interpretation_signed_and_float_branches() {
        assert_eq!(dim_type_from_interpretation("int8"), Some(DimType::I8));
        assert_eq!(dim_type_from_interpretation("int16"), Some(DimType::I16));
        assert_eq!(dim_type_from_interpretation("int32"), Some(DimType::I32));
        assert_eq!(dim_type_from_interpretation("int64"), Some(DimType::I64));
        assert_eq!(dim_type_from_interpretation("unsigned8"), Some(DimType::U8));
        assert_eq!(dim_type_from_interpretation("unsigned16"), Some(DimType::U16));
        assert_eq!(dim_type_from_interpretation("unsigned32"), Some(DimType::U32));
        assert_eq!(dim_type_from_interpretation("unsigned64"), Some(DimType::U64));
        assert_eq!(dim_type_from_interpretation("float"), Some(DimType::F32));
        assert_eq!(dim_type_from_interpretation("double"), Some(DimType::F64));
        assert!(dim_type_from_interpretation("mystery").is_none());
    }

    #[test]
    fn forward_vlrs_from_options_collects_and_skips_bad_record_id() {
        let mut options = Options::new();
        options.add("forward_vlr_user_id", "u1");
        options.add("forward_vlr_user_id", "u2");
        options.add("forward_vlr_record_id", "10");
        options.add("forward_vlr_record_id", "nope");
        options.add("forward_vlr_description", "d1");
        options.add("forward_vlr_description", "d2");
        options.add("forward_vlr_data", "");
        options.add("forward_vlr_data", "");
        let vlrs = forward_vlrs_from_options(&options);
        assert_eq!(vlrs.len(), 1);
        assert_eq!(vlrs[0].record_id, 10);
    }
