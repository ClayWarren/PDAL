#[cfg(test)]
mod tests {
    use super::*;
    use crate::las::LasReader;
    use pdal_core::pipeline::Reader;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn unique_suffix() -> String {
        format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn streaming_write_matches_materialized_write_byte_for_byte() {
        let src = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/autzen_trim.las"
        );
        let mut ropts = Options::new();
        ropts.add("filename", src);

        // The same points, materialized in one view and produced in chunks.
        let full = LasReader::new(&ropts).read().unwrap().remove(0);
        let mut stream_reader = LasReader::new(&ropts);
        let mut chunks = Vec::new();
        while let Some(c) = stream_reader.stream_next(30_000).unwrap() {
            chunks.push(c);
        }
        assert!(chunks.len() > 1, "fixture should span multiple chunks");

        let tmp = std::env::temp_dir();
        let suffix = unique_suffix();
        let materialized = tmp.join(format!("las-mat-{suffix}.las"));
        let streamed = tmp.join(format!("las-stream-{suffix}.las"));

        let mut wopts_a = Options::new();
        wopts_a.add("filename", materialized.display().to_string());
        LasWriter::new(&wopts_a).write(&[full]).unwrap();

        let mut wopts_b = Options::new();
        wopts_b.add("filename", streamed.display().to_string());
        let mut wb = LasWriter::new(&wopts_b);
        assert!(wb.streamable());
        for c in &chunks {
            wb.stream_write(c).unwrap();
        }
        wb.stream_finish().unwrap();

        let bytes_a = std::fs::read(&materialized).unwrap();
        let bytes_b = std::fs::read(&streamed).unwrap();
        let _ = std::fs::remove_file(&materialized);
        let _ = std::fs::remove_file(&streamed);

        assert_eq!(bytes_a.len(), bytes_b.len(), "LAS file sizes differ");
        assert!(
            bytes_a == bytes_b,
            "streamed LAS output is not byte-identical to materialized write"
        );
    }

    #[test]
    fn streaming_write_preserves_header_for_empty_chunks() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::U16);
        let mut chunk = PointView::new(Rc::new(layout));
        chunk.truncate(0);

        let path = std::env::temp_dir().join(format!("las-stream-empty-{}.las", unique_suffix()));
        let mut opts = Options::new();
        opts.add("filename", path.display().to_string());
        let mut writer = LasWriter::new(&opts);

        writer.stream_write(&chunk).unwrap();
        writer.stream_finish().unwrap();

        let reader = las::Reader::from_path(&path).unwrap();
        assert_eq!(reader.header().number_of_points(), 0);
        assert_eq!(reader.header().point_format().to_u8().unwrap(), 3);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pdal_header_bounds_match_scaled_roundtrip() {
        let transforms = las::Vector {
            x: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
            y: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
            z: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
        };
        let view = header_bbox_view();
        let bounds = pdal_header_bounds(&[view], &transforms);
        assert!((bounds.min.x - -136.8310).abs() < 1e-4);
        assert!((bounds.max.x - 194.1731).abs() < 1e-4);
        assert!((bounds.min.y - -165.4601).abs() < 1e-4);
        assert!((bounds.max.y - 165.5438).abs() < 1e-4);
        assert!((bounds.min.z - -20.4150).abs() < 1e-4);
        assert!((bounds.max.z - 310.5888).abs() < 1e-4);
    }

    #[test]
    fn writer_honors_global_encoding_option() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-global-encoding-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        options.add("minor_version", "3");
        options.add("dataformat_id", "3");
        options.add("global_encoding", "0");
        LasWriter::new(&options)
            .write(&[synthetic_point_view()])
            .unwrap();

        let reader = las::Reader::from_path(&temp).unwrap();
        let header = reader.header();
        assert_eq!(header.version(), las::Version::new(1, 3));
        assert!(!header.has_wkt_crs());

        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn writer_preserves_synthetic_flag_for_las10() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-synthetic-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        let mut writer = LasWriter::new(&options);
        writer.write(&[synthetic_point_view()]).unwrap();

        let mut reader = las::Reader::from_path(&temp).unwrap();
        let point = reader.points().next().unwrap().unwrap();
        assert!(point.is_synthetic);
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn quantize_scan_angle_matches_pdal_roundtrip() {
        let degrees = -16.998001098632812_f64;
        let target = (degrees / SCAN_ANGLE_SCALE_FACTOR).round() as i16;
        let quantized = quantize_scan_angle(7, degrees);
        let encoded = (f64::from(quantized) / SCAN_ANGLE_SCALE_FACTOR) as i16;
        assert_eq!(encoded, target);
        assert_eq!(target, -2833);
    }

    #[test]
    fn format7_laz_roundtrip_preserves_first_point_fields() {
        let source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/autzen_trim_7.las"
        );
        let output = std::env::temp_dir().join(format!(
            "pdal-format7-laz-{}-{}.laz",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let mut read_options = Options::new();
        read_options.add("filename", source);
        let input = LasReader::new(&read_options)
            .read()
            .expect("read source las");

        let mut write_options = Options::new();
        write_options.add("filename", output.display().to_string());
        write_options.add("dataformat_id", "7");
        write_options.add("minor_version", "4");
        write_options.add("compression", "true");
        LasWriter::new_laz(&write_options)
            .write(&input)
            .expect("write format 7 laz");

        let mut roundtrip_options = Options::new();
        roundtrip_options.add("filename", output.display().to_string());
        let output_views = LasReader::new(&roundtrip_options)
            .read()
            .expect("read written laz");

        let source_view = &input[0];
        let written_view = &output_views[0];
        assert_eq!(source_view.len(), written_view.len());

        let scan_channel = DimId::from_name("ScanChannel");
        for idx in 0..source_view.len().min(10) {
            for dim in [
                DimId::X,
                DimId::Y,
                DimId::Z,
                DimId::Intensity,
                DimId::ReturnNumber,
                DimId::NumberOfReturns,
                DimId::Classification,
                DimId::ScanAngleRank,
                DimId::GpsTime,
                DimId::Red,
                DimId::Green,
                DimId::Blue,
                scan_channel.clone(),
            ] {
                let left = source_view.get_f64(idx, &dim);
                let right = written_view.get_f64(idx, &dim);
                assert!(
                    (left - right).abs() <= 1e-9,
                    "point {idx} dim {:?}: {left} vs {right}",
                    dim
                );
            }
        }

        let _ = std::fs::remove_file(output);
    }

    fn header_bbox_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let coords = [
            (-136.8309503964847, -165.4601240504369, -20.415032985882097),
            (194.17314124182556, 165.54376758787334, 310.58878865242816),
        ];
        for (x, y, z) in coords {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, z);
        }
        view
    }

    fn synthetic_point_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        layout.register(DimId::Synthetic, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let id = view.add_point();
        view.set_f64(id, &DimId::X, 1.0);
        view.set_f64(id, &DimId::Y, 2.0);
        view.set_f64(id, &DimId::Z, 3.0);
        view.set_f64(id, &DimId::Classification, 2.0);
        view.set_f64(id, &DimId::Synthetic, 1.0);
        view
    }

    #[test]
    fn writer_discards_high_return_numbers_when_requested() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-discard-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        options.add("minor_version", "2");
        options.add("dataformat_id", "0");
        options.add("discard_high_return_numbers", "true");

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));

        let keep = view.add_point();
        view.set_f64(keep, &DimId::X, 1.0);
        view.set_f64(keep, &DimId::Y, 2.0);
        view.set_f64(keep, &DimId::Z, 3.0);
        view.set_f64(keep, &DimId::ReturnNumber, 5.0);
        view.set_f64(keep, &DimId::NumberOfReturns, 7.0);

        let drop = view.add_point();
        view.set_f64(drop, &DimId::X, 4.0);
        view.set_f64(drop, &DimId::Y, 5.0);
        view.set_f64(drop, &DimId::Z, 6.0);
        view.set_f64(drop, &DimId::ReturnNumber, 6.0);
        view.set_f64(drop, &DimId::NumberOfReturns, 7.0);

        LasWriter::new(&options)
            .write(&[view])
            .expect("write las with discard");

        let mut read_options = Options::new();
        read_options.add("filename", temp.display().to_string());
        let views = LasReader::new(&read_options)
            .read()
            .expect("read las with discard");
        assert_eq!(views[0].len(), 1);
        assert_eq!(views[0].get_f64(0, &DimId::ReturnNumber), 5.0);
        assert_eq!(views[0].get_f64(0, &DimId::NumberOfReturns), 5.0);

        let _ = std::fs::remove_file(temp);
    }

    // --- Pure helper unit tests ---

    #[test]
    fn pdal_sround_positive_and_negative() {
        assert!((pdal_sround(3.14159) - 3.0).abs() < 0.001);
        assert!((pdal_sround(-2.71828) - -3.0).abs() < 0.001);
        assert!((pdal_sround(0.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn pdal_scaled_i32_roundtrips() {
        let t = las::Transform { scale: 0.0001, offset: 0.0 };
        assert_eq!(pdal_scaled_i32(1.2345678, &t), 12346);
        assert_eq!(pdal_scaled_i32(-1.2345678, &t), -12346);
        assert_eq!(pdal_scaled_i32(0.0, &t), 0);
    }

    #[test]
    fn pdal_from_scaled_matches_original() {
        let t = las::Transform { scale: 0.0001, offset: 0.0 };
        let scaled = pdal_scaled_i32(1.2345678, &t);
        let back = pdal_from_scaled(scaled, &t);
        assert!((back - 1.2346).abs() < 0.0001);
    }

    #[test]
    fn max_return_count_v1_3() {
        assert_eq!(max_return_count(3), 5);
    }

    #[test]
    fn max_return_count_v1_4() {
        assert_eq!(max_return_count(4), 15);
    }

    #[test]
    fn pdal_to_las_type_all_formats() {
        assert_eq!(pdal_to_las_type(DimType::U8), 1);
        assert_eq!(pdal_to_las_type(DimType::I8), 2);
        assert_eq!(pdal_to_las_type(DimType::U16), 3);
        assert_eq!(pdal_to_las_type(DimType::I16), 4);
        assert_eq!(pdal_to_las_type(DimType::U32), 5);
        assert_eq!(pdal_to_las_type(DimType::I32), 6);
        assert_eq!(pdal_to_las_type(DimType::U64), 7);
        assert_eq!(pdal_to_las_type(DimType::I64), 8);
        assert_eq!(pdal_to_las_type(DimType::F32), 9);
        assert_eq!(pdal_to_las_type(DimType::F64), 10);
    }

    #[test]
    fn las_inverse_ceil_and_floor() {
        let t = las::Transform { scale: 0.0001, offset: 0.0 };
        assert!((las_inverse_ceil(3.14159, &t) - 31416.0).abs() < 0.0001);
        assert!((las_inverse_ceil(-3.14159, &t) - -31415.0).abs() < 0.0001);
        assert!((las_inverse_floor(3.14159, &t) - 31415.0).abs() < 0.0001);
        assert!((las_inverse_floor(-3.14159, &t) - -31416.0).abs() < 0.0001);
    }

    #[test]
    fn pdrf_dims_maps_correctly() {
        let dims = pdrf_dims(0);
        assert!(dims.contains(&DimId::X));
        assert!(dims.contains(&DimId::Intensity));
        assert!(!dims.contains(&DimId::GpsTime));

        let dims = pdrf_dims(1);
        assert!(dims.contains(&DimId::GpsTime));

        let dims_with6 = pdrf_dims(6);
        assert!(dims_with6.contains(&DimId::GpsTime));
        assert!(dims_with6.contains(&DimId::from_name("ScanChannel")));

        let dims_with8 = pdrf_dims(8);
        assert!(dims_with8.contains(&DimId::Infrared));
    }

    #[test]
    fn scan_angle_f32_for_i16_roundtrips() {
        let val = scan_angle_f32_for_i16(-2833);
        assert!((f64::from(val) - -16.998).abs() < 0.001);
    }

    #[test]
    fn write_pdal_val_roundtrip_u8() {
        let mut out = Vec::new();
        write_pdal_val(&mut out as &mut dyn std::io::Write, 42.0, DimType::U8).unwrap();
        assert_eq!(out, vec![42u8]);
    }

    #[test]
    fn numeric_option_f64_reads_value() {
        let mut opts = Options::new();
        opts.add("scale_x", "0.001");
        assert!((numeric_option_f64(&opts, "scale_x").unwrap() - 0.001).abs() < 1e-9);
    }

    #[test]
    fn numeric_option_u8_reads_value() {
        let mut opts = Options::new();
        opts.add("test_opt", "42");
        assert_eq!(numeric_option_u8(&opts, "test_opt").unwrap(), 42);
    }

    #[test]
    fn numeric_option_u16_reads_value() {
        let mut opts = Options::new();
        opts.add("test_opt", "1000");
        assert_eq!(numeric_option_u16(&opts, "test_opt").unwrap(), 1000);
    }

    #[test]
    fn numeric_option_u32_reads_value() {
        let mut opts = Options::new();
        opts.add("test_opt", "100000");
        assert_eq!(numeric_option_u32(&opts, "test_opt").unwrap(), 100000);
    }

    #[test]
    fn numeric_option_i32_reads_value() {
        let mut opts = Options::new();
        opts.add("test_opt", "-42");
        assert_eq!(numeric_option_i32(&opts, "test_opt").unwrap(), -42);
    }

    #[test]
    fn string_option_reads_value() {
        let mut opts = Options::new();
        opts.add("test_opt", "hello");
        assert_eq!(string_option(&opts, "test_opt").unwrap(), "hello");
    }

    #[test]
    fn binary_option_reads_base64() {
        let mut opts = Options::new();
        opts.add("test_opt", "AA==");
        assert_eq!(binary_option(&opts, "test_opt").unwrap(), vec![0u8]);
    }

    #[test]
    fn quantize_scan_angle_format_pre_6_rounds_to_integer() {
        let result = quantize_scan_angle(0, -16.998001098632812_f64);
        assert!((f64::from(result) - -17.0).abs() < 0.001);
    }

    #[test]
    fn quantize_scan_angle_format_6_scales_correctly() {
        let result = quantize_scan_angle(6, 0.0);
        assert!((f64::from(result) - 0.0).abs() < 0.001);
    }

    #[test]
    fn min_xyz_empty_returns_none() {
        let result = min_xyz(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn min_xyz_finds_min_across_points() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let a = view.add_point();
        view.set_f64(a, &DimId::X, 10.0);
        view.set_f64(a, &DimId::Y, 20.0);
        view.set_f64(a, &DimId::Z, 30.0);
        let b = view.add_point();
        view.set_f64(b, &DimId::X, 5.0);
        view.set_f64(b, &DimId::Y, 15.0);
        view.set_f64(b, &DimId::Z, 25.0);
        let result = min_xyz(&[view]).unwrap();
        assert!((result[0] - 5.0).abs() < 1e-9);
        assert!((result[1] - 15.0).abs() < 1e-9);
        assert!((result[2] - 25.0).abs() < 1e-9);
    }

    #[test]
    fn configured_extra_dims_parses_names_and_types() {
        let mut opts = Options::new();
        opts.add("extra_dim_name", "Intensity");
        opts.add("extra_dim_type", "uint8");
        opts.add("extra_dim_name", "Amplitude");
        opts.add("extra_dim_type", "float");
        let dims = configured_extra_dims_from_options(&opts);
        assert_eq!(dims.len(), 2);
        assert_eq!(dims[0].name, "Intensity");
        assert_eq!(dims[0].type_name, "uint8");
        assert_eq!(dims[1].name, "Amplitude");
        assert_eq!(dims[1].type_name, "float");
    }

    #[test]
    fn configured_extra_dims_parses_pdal_extra_dims_option() {
        let mut opts = Options::new();
        opts.add("extra_dims", "Q=int32, S=double");
        let dims = configured_extra_dims_from_options(&opts);
        assert_eq!(dims.len(), 2);
        assert_eq!(dims[0].name, "Q");
        assert_eq!(dims[0].type_name, "int32");
        assert_eq!(dims[1].name, "S");
        assert_eq!(dims[1].type_name, "double");
    }

    #[test]
    fn configured_extra_dims_preserves_all_marker() {
        let mut opts = Options::new();
        opts.add("extra_dims", "all");
        let dims = configured_extra_dims_from_options(&opts);
        assert_eq!(dims.len(), 1);
        assert_eq!(dims[0].name, "all");
    }

    #[test]
    fn configured_extra_dims_uses_min_of_lengths() {
        let mut opts = Options::new();
        opts.add("extra_dim_name", "A");
        opts.add("extra_dim_name", "B");
        opts.add("extra_dim_type", "uint8");
        let dims = configured_extra_dims_from_options(&opts);
        assert_eq!(dims.len(), 1);
        assert_eq!(dims[0].name, "A");
    }

    #[test]
    fn configured_extra_dims_missing_is_empty() {
        let opts = Options::new();
        let dims = configured_extra_dims_from_options(&opts);
        assert!(dims.is_empty());
    }

    #[test]
    fn dim_type_from_interpretation_writer_unsigned_matches_correctly() {
        assert_eq!(dim_type_from_interpretation("unsigned8"), Some(DimType::U8));
        assert_eq!(dim_type_from_interpretation("unsigned16"), Some(DimType::U16));
        assert_eq!(dim_type_from_interpretation("unsigned32"), Some(DimType::U32));
        assert_eq!(dim_type_from_interpretation("unsigned64"), Some(DimType::U64));
    }

    #[test]
    fn dim_type_from_interpretation_writer_uint_contains_int() {
        assert_eq!(dim_type_from_interpretation("uint8"), Some(DimType::I8));
        assert_eq!(dim_type_from_interpretation("uint16"), Some(DimType::I16));
        assert_eq!(dim_type_from_interpretation("uint32"), Some(DimType::I32));
        assert_eq!(dim_type_from_interpretation("uint64"), Some(DimType::I64));
    }

    #[test]
    fn dim_type_from_interpretation_writer_signed() {
        assert_eq!(dim_type_from_interpretation("int8"), Some(DimType::I8));
        assert_eq!(dim_type_from_interpretation("int16"), Some(DimType::I16));
        assert_eq!(dim_type_from_interpretation("int32"), Some(DimType::I32));
        assert_eq!(dim_type_from_interpretation("int64"), Some(DimType::I64));
    }

    #[test]
    fn dim_type_from_interpretation_writer_float_types() {
        assert_eq!(dim_type_from_interpretation("float"), Some(DimType::F32));
        assert_eq!(dim_type_from_interpretation("double"), Some(DimType::F64));
    }

    #[test]
    fn dim_type_from_interpretation_writer_unknown_returns_none() {
        assert_eq!(dim_type_from_interpretation("complex128"), None);
        assert_eq!(dim_type_from_interpretation(""), None);
    }

    #[test]
    fn extra_dims_from_views_finds_non_standard_dims() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let extra_id = DimId::from_name("MyExtraDim");
        layout.register(extra_id.clone(), DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, 1.0);
        view.set_f64(idx, &DimId::Y, 2.0);
        view.set_f64(idx, &DimId::Z, 3.0);
        view.set_f64(idx, &extra_id, 42.0);
        let dims = extra_dims_from_views(&[view], 0);
        assert_eq!(dims.len(), 1);
        assert!(dims.iter().any(|d| d.id.name() == "MyExtraDim"));
        assert_eq!(dims[0].size, 8);
    }

    #[test]
    fn extra_dims_from_views_empty_views_returns_empty() {
        let dims = extra_dims_from_views(&[], 0);
        assert!(dims.is_empty());
    }

    #[test]
    fn extra_dims_from_views_no_non_standard_dims_returns_empty() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let view = PointView::new(Rc::new(layout));
        let dims = extra_dims_from_views(&[view], 0);
        assert!(dims.is_empty());
    }

    #[test]
    fn user_vlrs_from_options_parses_single_vlr() {
        let mut opts = Options::new();
        opts.add("user_vlr_user_id", "testid");
        opts.add("user_vlr_record_id", "1234");
        opts.add("user_vlr_description", "desc");
        opts.add("user_vlr_data", "AA==");
        opts.add("user_vlr_evlr", "true");
        let vlrs = user_vlrs_from_options(&opts);
        assert_eq!(vlrs.len(), 1);
        assert_eq!(vlrs[0].user_id, "testid");
        assert_eq!(vlrs[0].record_id, 1234);
        assert_eq!(vlrs[0].description, "desc");
        assert_eq!(vlrs[0].data, vec![0u8]);
        assert!(vlrs[0].write_as_evlr);
    }

    #[test]
    fn user_vlrs_from_options_evlr_truthy_values() {
        let mut opts = Options::new();
        opts.add("user_vlr_user_id", "u");
        opts.add("user_vlr_record_id", "1");
        opts.add("user_vlr_description", "d");
        opts.add("user_vlr_data", "AA==");
        opts.add("user_vlr_evlr", "yes");
        let vlrs = user_vlrs_from_options(&opts);
        assert!(vlrs[0].write_as_evlr);
    }

    #[test]
    fn user_vlrs_from_options_evlr_falsey_returns_false() {
        let mut opts = Options::new();
        opts.add("user_vlr_user_id", "u");
        opts.add("user_vlr_record_id", "1");
        opts.add("user_vlr_description", "d");
        opts.add("user_vlr_data", "AA==");
        opts.add("user_vlr_evlr", "false");
        let vlrs = user_vlrs_from_options(&opts);
        assert!(!vlrs[0].write_as_evlr);
    }

    #[test]
    fn user_vlrs_from_options_invalid_record_id_skips() {
        let mut opts = Options::new();
        opts.add("user_vlr_user_id", "u");
        opts.add("user_vlr_record_id", "not_a_number");
        opts.add("user_vlr_description", "d");
        opts.add("user_vlr_data", "AA==");
        opts.add("user_vlr_evlr", "false");
        let vlrs = user_vlrs_from_options(&opts);
        assert!(vlrs.is_empty());
    }

    #[test]
    fn user_vlrs_from_options_empty_when_no_keys() {
        let opts = Options::new();
        let vlrs = user_vlrs_from_options(&opts);
        assert!(vlrs.is_empty());
    }

    #[test]
    fn forward_vlrs_from_options_parses_single_vlr() {
        let mut opts = Options::new();
        opts.add("forward_vlr_user_id", "fwdid");
        opts.add("forward_vlr_record_id", "5678");
        opts.add("forward_vlr_description", "fwddesc");
        opts.add("forward_vlr_data", "AQID");
        let vlrs = forward_vlrs_from_options(&opts);
        assert_eq!(vlrs.len(), 1);
        assert_eq!(vlrs[0].user_id, "fwdid");
        assert_eq!(vlrs[0].record_id, 5678);
        assert_eq!(vlrs[0].description, "fwddesc");
        assert_eq!(vlrs[0].data, vec![1, 2, 3]);
    }

    #[test]
    fn forward_vlrs_from_options_invalid_record_id_skips() {
        let mut opts = Options::new();
        opts.add("forward_vlr_user_id", "u");
        opts.add("forward_vlr_record_id", "bad");
        opts.add("forward_vlr_description", "d");
        opts.add("forward_vlr_data", "AA==");
        let vlrs = forward_vlrs_from_options(&opts);
        assert!(vlrs.is_empty());
    }

    #[test]
    fn forward_vlrs_from_options_empty_when_no_keys() {
        let opts = Options::new();
        let vlrs = forward_vlrs_from_options(&opts);
        assert!(vlrs.is_empty());
    }

    #[test]
    fn quantize_coord_roundtrips_through_scaled() {
        let t = las::Transform { scale: 0.001, offset: 100.0 };
        let input = 123.456789;
        let quantized = quantize_coord(input, &t);
        let restored = (pdal_sround((quantized - t.offset) / t.scale) as i32 as f64) * t.scale + t.offset;
        assert!((quantized - restored).abs() < 1e-12);
    }

    #[test]
    fn quantize_coord_negative_values() {
        let t = las::Transform { scale: 0.0001, offset: 0.0 };
        let result = quantize_coord(-1.2345678, &t);
        let expected = pdal_from_scaled(pdal_scaled_i32(-1.2345678, &t), &t);
        assert!((result - expected).abs() < 1e-12);
    }

    #[test]
    fn quantize_coord_zero() {
        let t = las::Transform { scale: 0.001, offset: 0.0 };
        let result = quantize_coord(0.0, &t);
        assert!((result - 0.0).abs() < 1e-12);
    }

    #[test]
    fn dim_flag_true_when_dim_exists_and_positive() {
        let mut layout = PointLayout::new();
        layout.register(DimId::Synthetic, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::Synthetic, 1.0);
        assert!(dim_flag(&view, idx, &DimId::Synthetic));
    }

    #[test]
    fn dim_flag_false_when_dim_exists_but_zero() {
        let mut layout = PointLayout::new();
        layout.register(DimId::Synthetic, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::Synthetic, 0.0);
        assert!(!dim_flag(&view, idx, &DimId::Synthetic));
    }

    #[test]
    fn dim_flag_false_when_dim_does_not_exist() {
        let layout = PointLayout::new();
        let view = PointView::new(Rc::new(layout));
        assert!(!dim_flag(&view, 0, &DimId::Synthetic));
    }

    #[test]
    fn dim_u8_returns_value_when_dim_exists() {
        let mut layout = PointLayout::new();
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::Classification, 6.0);
        assert_eq!(dim_u8(&view, idx, &DimId::Classification, 0), 6);
    }

    #[test]
    fn dim_u8_returns_default_when_dim_missing() {
        let layout = PointLayout::new();
        let view = PointView::new(Rc::new(layout));
        assert_eq!(dim_u8(&view, 0, &DimId::Classification, 99), 99);
    }

    #[test]
    fn dim_u16_returns_value_when_dim_exists() {
        let mut layout = PointLayout::new();
        layout.register(DimId::PointSourceId, DimType::U16);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::PointSourceId, 42.0);
        assert_eq!(dim_u16(&view, idx, &DimId::PointSourceId), 42);
    }

    #[test]
    fn dim_u16_returns_zero_when_dim_missing() {
        let layout = PointLayout::new();
        let view = PointView::new(Rc::new(layout));
        assert_eq!(dim_u16(&view, 0, &DimId::PointSourceId), 0);
    }

    #[test]
    fn scan_direction_left_to_right_when_flag_positive() {
        let mut layout = PointLayout::new();
        layout.register(DimId::ScanDirectionFlag, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::ScanDirectionFlag, 1.0);
        assert_eq!(scan_direction(&view, idx), ScanDirection::LeftToRight);
    }

    #[test]
    fn scan_direction_right_to_left_when_flag_zero() {
        let mut layout = PointLayout::new();
        layout.register(DimId::ScanDirectionFlag, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        view.set_f64(idx, &DimId::ScanDirectionFlag, 0.0);
        assert_eq!(scan_direction(&view, idx), ScanDirection::RightToLeft);
    }

    #[test]
    fn classification_maps_known_values() {
        assert_eq!(classification(0), Classification::CreatedNeverClassified);
        assert_eq!(classification(1), Classification::Unclassified);
        assert_eq!(classification(2), Classification::Ground);
        assert_eq!(classification(3), Classification::LowVegetation);
        assert_eq!(classification(4), Classification::MediumVegetation);
        assert_eq!(classification(5), Classification::HighVegetation);
        assert_eq!(classification(6), Classification::Building);
        assert_eq!(classification(7), Classification::LowPoint);
        assert_eq!(classification(8), Classification::ModelKeyPoint);
        assert_eq!(classification(9), Classification::Water);
    }

    #[test]
    fn classification_reserved_for_unknown() {
        match classification(10) {
            Classification::Reserved(v) => assert_eq!(v, 10),
            _ => panic!("expected Reserved"),
        }
        match classification(255) {
            Classification::Reserved(v) => assert_eq!(v, 255),
            _ => panic!("expected Reserved"),
        }
    }

    #[test]
    fn write_extra_dim_vlr_record_writes_expected_bytes() {
        let ed = ExtraDim {
            id: DimId::from_name("TestDim"),
            ty: DimType::F64,
            size: 8,
        };
        let mut buf = Vec::new();
        write_extra_dim_vlr_record(&mut buf, &ed).unwrap();
        // reserved: u16(0) + u8(type) + u8(0) + 32 byte name + u32(0) + 72 reserved + 6*f64(0) + 32 reserved
        assert_eq!(buf.len(), 2 + 1 + 1 + 32 + 4 + 72 + 48 + 32);
        assert_eq!(buf[0..2], [0u8; 2]); // reserved u16
        assert_eq!(buf[2], 10); // pdal_to_las_type(F64) = 10
        assert_eq!(buf[3], 0); // reserved u8
        // Check name "TestDim" is written (null-padded to 32)
        let name_bytes = &buf[4..36];
        assert_eq!(&name_bytes[..7], b"TestDim");
        assert!(name_bytes[7..].iter().all(|b| *b == 0));
    }

    #[test]
    fn write_extra_dim_vlr_record_truncates_long_name() {
        let long_name = "A".repeat(40);
        let ed = ExtraDim {
            id: DimId::from_name(&long_name),
            ty: DimType::U8,
            size: 1,
        };
        let mut buf = Vec::new();
        write_extra_dim_vlr_record(&mut buf, &ed).unwrap();
        let name_bytes = &buf[4..36];
        assert!(name_bytes.iter().all(|b| *b == b'A' || *b == 0));
    }

    mod writer_cases {
        include!("las_writer_tests/writer_cases.rs");
    }
}
