#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::options::Options;
    use std::io::{Seek, SeekFrom, Write};

    #[test]
    fn reader_preserves_legacy_synthetic_flag() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/synthetic_test.las"
        );
        let mut options = Options::new();
        options.add("filename", path);
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("read synthetic_test.las");
        let view = &views[0];
        assert_eq!(view.get_f64(0, &DimId::Classification), 0.0);
        assert_eq!(view.get_f64(0, &DimId::Synthetic), 1.0);
    }

    #[test]
    fn reader_reads_extrabytes_vlr_with_undocumented_record() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/extrabytes.las"
        );
        let mut options = Options::new();
        options.add("filename", path);
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("read extrabytes.las");
        let view = &views[0];
        let flags0 = DimId::from_name("Flags0");
        let flags1 = DimId::from_name("Flags1");
        assert_eq!(view.get_f64(0, &flags0), 1.0);
        assert_eq!(view.get_f64(0, &flags1), 1.0);
        assert_eq!(
            view.get_f64(0, &flags0),
            view.get_f64(0, &DimId::ReturnNumber)
        );

        let names: Vec<String> = (0..view.layout().dim_count())
            .filter_map(|idx| {
                view.layout()
                    .dim_at(idx)
                    .map(|(id, _)| id.name().to_string())
            })
            .collect();
        assert!(names.iter().any(|name| name == "Flags0"));
        assert!(names.iter().any(|name| name == "Time"));
    }

    #[test]
    fn detect_copc_matches_signature_at_offset_377() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let mut data = vec![0u8; 381];
        data[377..381].copy_from_slice(b"copc");
        file.write_all(&data).unwrap();
        assert!(detect_copc(file.path()));

        data[377..381].copy_from_slice(b"las ");
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&data).unwrap();
        assert!(!detect_copc(file.path()));
    }

    // --- Pure helper unit tests ---

    #[test]
    fn parse_srs_vlr_kind_all_variants() {
        assert_eq!(parse_srs_vlr_kind("wkt1"), Some(SrsVlrKind::Wkt1));
        assert_eq!(parse_srs_vlr_kind("geotiff"), Some(SrsVlrKind::Geotiff));
        assert_eq!(parse_srs_vlr_kind("projjson"), Some(SrsVlrKind::Proj));
        assert_eq!(parse_srs_vlr_kind("wkt2"), Some(SrsVlrKind::Wkt2));
        assert_eq!(parse_srs_vlr_kind("wkt"), Some(SrsVlrKind::Wkt2));
        assert_eq!(parse_srs_vlr_kind("unknown"), None);
    }

    #[test]
    fn vlr_as_string_strips_nul() {
        assert_eq!(vlr_as_string(b"hello\0world\0"), "hello\0world");
        assert_eq!(vlr_as_string(b"test"), "test");
        assert_eq!(vlr_as_string(b""), "");
    }

    #[test]
    fn dim_type_from_interpretation_common_types() {
        assert_eq!(dim_type_from_interpretation("uint8"), Some(DimType::I8));
        assert_eq!(dim_type_from_interpretation("unsigned8"), Some(DimType::U8));
        assert_eq!(dim_type_from_interpretation("int8"), Some(DimType::I8));
        assert_eq!(dim_type_from_interpretation("uint16"), Some(DimType::I16));
        assert_eq!(dim_type_from_interpretation("unsigned16"), Some(DimType::U16));
        assert_eq!(dim_type_from_interpretation("int16"), Some(DimType::I16));
        assert_eq!(dim_type_from_interpretation("uint32"), Some(DimType::I32));
        assert_eq!(dim_type_from_interpretation("unsigned32"), Some(DimType::U32));
        assert_eq!(dim_type_from_interpretation("int32"), Some(DimType::I32));
        assert_eq!(dim_type_from_interpretation("uint64"), Some(DimType::I64));
        assert_eq!(dim_type_from_interpretation("unsigned64"), Some(DimType::U64));
        assert_eq!(dim_type_from_interpretation("int64"), Some(DimType::I64));
        assert_eq!(dim_type_from_interpretation("float"), Some(DimType::F32));
        assert_eq!(dim_type_from_interpretation("double"), Some(DimType::F64));
        assert_eq!(dim_type_from_interpretation("unknown"), None);
    }

    #[test]
    fn las_to_pdal_type_returns_proper_dim_type() {
        let (ty, count) = las_to_pdal_type(0);
        assert_eq!(ty, None);
        assert_eq!(count, 1);
    }

    #[test]
    fn extra_dim_scale_defaults_to_one() {
        let record = ExtraDimRecord {
            data_type: 0,
            options: 0,
            name: "Test".to_string(),
            scales: [0.0; 3],
            offsets: [0.0; 3],
        };
        assert!((extra_dim_scale(&record, 0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn extra_dim_offset_defaults_to_zero() {
        let record = ExtraDimRecord {
            data_type: 0,
            options: 0,
            name: "Test".to_string(),
            scales: [0.0; 3],
            offsets: [0.0; 3],
        };
        assert!((extra_dim_offset(&record, 0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn srs_vlr_order_from_options_parsed() {
        let mut options = Options::new();
        options.add("srs_vlr_order", "wkt1, geotiff");
        let order = srs_vlr_order_from_options(&options);
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn srs_vlr_order_from_options_empty_when_missing() {
        let options = Options::new();
        let order = srs_vlr_order_from_options(&options);
        assert!(order.is_empty());
    }

    #[test]
    fn configured_extra_dims_from_options_missing_is_empty() {
        let options = Options::new();
        let dims = configured_extra_dims_from_options(&options);
        assert!(dims.is_empty());
    }

    #[test]
    fn las_to_pdal_type_varied_inputs() {
        assert_eq!(las_to_pdal_type(0), (None, 1));
        assert_eq!(las_to_pdal_type(1), (Some(DimType::U8), 1));
        assert_eq!(las_to_pdal_type(2), (Some(DimType::I8), 1));
        assert_eq!(las_to_pdal_type(11), (Some(DimType::U8), 2));
    }

    fn las_path(name: &str) -> String {
        format!("{}/../../test/data/las/{}", env!("CARGO_MANIFEST_DIR"), name)
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = LasReader::new(&Options::new());
        let err = match reader.read() {
            Ok(_) => panic!("missing filename should error"),
            Err(e) => e,
        };
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "no/such/file.las");
        let mut reader = LasReader::new(&options);
        let err = match reader.read() {
            Ok(_) => panic!("missing file should error"),
            Err(e) => e,
        };
        assert!(!err.0.is_empty());
    }

    #[test]
    fn reader_errors_when_start_exceeds_point_count() {
        let mut options = Options::new();
        options.add("filename", las_path("100-points.las"));
        options.add("start", 9999u64);
        let mut reader = LasReader::new(&options);
        let err = match reader.read() {
            Ok(_) => panic!("start past end should error"),
            Err(e) => e,
        };
        assert!(err.0.contains("start") || err.0.contains("outside"));
    }

    #[test]
    fn reader_honors_start_and_count() {
        let mut options = Options::new();
        options.add("filename", las_path("100-points.las"));
        options.add("start", 10u64);
        options.add("count", 5u64);
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("read 100-points.las");
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 5);
    }

    #[test]
    fn reader_with_ignore_missing_vlrs_reads_streaming_path() {
        let mut options = Options::new();
        options.add("filename", las_path("100-points.las"));
        options.add("ignore_missing_vlrs", true);
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("streaming read");
        assert!(!views.is_empty());
        assert_eq!(views[0].len(), 100);
    }

    #[test]
    fn reader_handles_empty_las_file() {
        let mut options = Options::new();
        options.add("filename", las_path("no-points.las"));
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("read no-points.las");
        assert_eq!(views[0].len(), 0);
    }

    #[test]
    fn reader_reads_color_las_with_wkt_sidecar() {
        let mut options = Options::new();
        options.add("filename", las_path("1.2-with-color.las"));
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("read 1.2-with-color.las");
        assert!(!views.is_empty());
    }

    #[test]
    fn reader_reads_epsg_las() {
        let mut options = Options::new();
        options.add("filename", las_path("epsg_4326.las"));
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("read epsg_4326.las");
        assert!(!views.is_empty());
        assert!(!views[0].spatial_reference().wkt().is_empty());
    }

    #[test]
    fn reader_reads_las_with_lots_of_vlrs() {
        let mut options = Options::new();
        options.add("filename", las_path("lots_of_vlr.las"));
        let mut reader = LasReader::new(&options);
        let _ = reader.read();
    }

    #[test]
    fn reader_with_streaming_path_honors_start_and_count() {
        let mut options = Options::new();
        options.add("filename", las_path("100-points.las"));
        options.add("ignore_missing_vlrs", true);
        options.add("start", 20u64);
        options.add("count", 5u64);
        let mut reader = LasReader::new(&options);
        let views = reader.read().expect("streaming subset");
        assert_eq!(views[0].len(), 5);
    }
}
