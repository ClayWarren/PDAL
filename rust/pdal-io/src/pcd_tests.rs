#[cfg(test)]
mod tests {
    use super::*;
    use crate::faux::FauxReader;
    use crate::text_writer::TextWriter;
    use pdal_core::pipeline::{FilterWrapper, Pipeline, Writer};
    use pdal_filters::decimation::DecimationFilter;
    use pdal_filters::range::{RangeFilter, RangeLimit};

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn temp_path(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-pcd-{}-{name}", std::process::id()));
        let _ = fs::remove_file(&path);
        path.display().to_string()
    }

    #[test]
    fn reads_ascii_space_separated_pcd() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_space.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(0, &DimId::X), 289814.15625);
        assert_eq!(view.get_f64(0, &DimId::Y), 4320978.5);
        assert_eq!(view.get_f64(0, &DimId::Z), 170.75999450683594);
        assert_eq!(view.get_f64(9, &DimId::X), 289818.5);
    }

    #[test]
    fn streaming_ascii_chunks_match_full_read() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_space.pcd"));

        let mut full_reader = PcdReader::new(&options);
        let full = full_reader.read().unwrap().pop().unwrap();

        let mut stream_reader = PcdReader::new(&options);
        assert!(stream_reader.streamable());
        let first = stream_reader.stream_next(4).unwrap().unwrap();
        let second = stream_reader.stream_next(4).unwrap().unwrap();
        let third = stream_reader.stream_next(4).unwrap().unwrap();
        assert!(stream_reader.stream_next(4).unwrap().is_none());

        assert_eq!(first.len(), 4);
        assert_eq!(second.len(), 4);
        assert_eq!(third.len(), 2);
        assert_eq!(first.get_f64(0, &DimId::X), full.get_f64(0, &DimId::X));
        assert_eq!(second.get_f64(0, &DimId::Y), full.get_f64(4, &DimId::Y));
        assert_eq!(third.get_f64(1, &DimId::Z), full.get_f64(9, &DimId::Z));
    }

    #[test]
    fn pipeline_streams_pcd_reader_to_csv_writer() {
        let output = temp_path("stream-reader-pipeline.csv");
        let mut reader_options = Options::new();
        reader_options.add("filename", data_path("pcd/utm17_space.pcd"));
        let limits = vec![RangeLimit {
            dim_name: "X".to_string(),
            lower_bound: 289814.0,
            upper_bound: 289815.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X,Y,Z")
            .add("quote_header", false)
            .add("precision", 2);

        let mut pipeline = Pipeline::new();
        let reader = pipeline.add_reader(
            "readers.pcd",
            Box::new(PcdReader::new(&reader_options)),
            reader_options,
        );
        let filter = pipeline.add_stage(
            "filters.range",
            Box::new(FilterWrapper::new(RangeFilter::new(limits))),
            Options::new(),
        );
        let writer = pipeline.add_writer(
            "writers.text",
            Box::new(TextWriter::new(&writer_options)),
            writer_options,
        );
        pipeline.add_dependency(filter, reader).unwrap();
        pipeline.add_dependency(writer, filter).unwrap();

        assert_eq!(pipeline.execute_streaming().unwrap(), Some(2));
        let written = fs::read_to_string(&output).unwrap();
        let _ = fs::remove_file(output);
        let lines: Vec<_> = written.lines().collect();
        assert_eq!(lines[0], "X,Y,Z");
        assert_eq!(lines.len(), 3);
        assert!(lines[1].starts_with("289814.16,4320978.50,170.76"));
    }

    #[test]
    fn reads_ascii_tab_separated_pcd() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_tab.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(9, &DimId::Y), 4320980.5);
    }

    #[test]
    fn reads_binary_pcd_with_double_fields() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/autzen-utm.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 1065);
        assert!((view.get_f64(0, &DimId::X) - 494428.61).abs() < 0.01);
        assert!((view.get_f64(0, &DimId::Y) - 4877455.58).abs() < 0.01);
        assert!((view.get_f64(0, &DimId::Z) - 131.57).abs() < 0.01);
        assert!(view.get_f64(0, &DimId::GpsTime) > 0.0);
    }

    #[test]
    fn streaming_binary_chunks_match_full_read() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/autzen-utm.pcd"));

        let mut full_reader = PcdReader::new(&options);
        let full = full_reader.read().unwrap().pop().unwrap();

        let mut stream_reader = PcdReader::new(&options);
        assert!(stream_reader.streamable());
        let first = stream_reader.stream_next(500).unwrap().unwrap();
        let second = stream_reader.stream_next(500).unwrap().unwrap();
        let third = stream_reader.stream_next(500).unwrap().unwrap();
        assert!(stream_reader.stream_next(500).unwrap().is_none());

        assert_eq!(first.len(), 500);
        assert_eq!(second.len(), 500);
        assert_eq!(third.len(), 65);
        assert_eq!(first.get_f64(0, &DimId::X), full.get_f64(0, &DimId::X));
        assert_eq!(
            second.get_f64(0, &DimId::Y),
            full.get_f64(500, &DimId::Y)
        );
        assert_eq!(
            third.get_f64(64, &DimId::GpsTime),
            full.get_f64(1064, &DimId::GpsTime)
        );
    }

    #[test]
    fn comma_separated_ascii_rows_are_skipped_like_cpp_reader() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_comma.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 0);
    }

    #[test]
    fn missing_header_is_rejected() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/missingheader.pcd"));
        let mut reader = PcdReader::new(&options);

        assert!(reader.read().is_err());
    }

    #[test]
    fn writes_ascii_pcd_that_reader_roundtrips() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_space.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let output = temp_path("roundtrip.pcd");
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X,Y,Z")
            .add("precision", 2);
        let mut writer = PcdWriter::new(&writer_options);
        writer.write(std::slice::from_ref(&view)).unwrap();

        let mut read_options = Options::new();
        read_options.add("filename", &output);
        let mut roundtrip = PcdReader::new(&read_options);
        let roundtrip = roundtrip.read().unwrap().pop().unwrap();

        assert_eq!(roundtrip.len(), view.len());
        assert_eq!(roundtrip.get_f64(0, &DimId::X), view.get_f64(0, &DimId::X));
        assert_eq!(roundtrip.get_f64(9, &DimId::Z), view.get_f64(9, &DimId::Z));
    }

    #[test]
    fn streaming_ascii_writer_matches_materialized_write() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_space.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let mut first = view.clone();
        first.truncate(4);
        let mut second = view.make_new();
        for idx in 4..view.len() {
            second.append_point(&view, idx);
        }

        let materialized = temp_path("materialized-stream-compare.pcd");
        let streamed = temp_path("streamed-stream-compare.pcd");
        let mut materialized_options = Options::new();
        materialized_options
            .add("filename", &materialized)
            .add("order", "X,Y,Z")
            .add("precision", 2);
        let mut stream_options = Options::new();
        stream_options
            .add("filename", &streamed)
            .add("order", "X,Y,Z")
            .add("precision", 2);

        let mut materialized_writer = PcdWriter::new(&materialized_options);
        materialized_writer
            .write(&[first.clone(), second.clone()])
            .unwrap();
        let mut stream_writer = PcdWriter::new(&stream_options);
        assert!(stream_writer.streamable());
        stream_writer.stream_write(&first).unwrap();
        stream_writer.stream_write(&second).unwrap();
        stream_writer.stream_finish().unwrap();

        assert_eq!(fs::read(&streamed).unwrap(), fs::read(&materialized).unwrap());
        let _ = fs::remove_file(materialized);
        let _ = fs::remove_file(streamed);
    }

    #[test]
    fn streaming_ascii_writer_handles_empty_first_chunk() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_space.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let empty = view.make_new();
        let mut nonempty = view.make_new();
        nonempty.append_point(&view, 0);

        let materialized = temp_path("materialized-empty-first.pcd");
        let streamed = temp_path("streamed-empty-first.pcd");
        let mut materialized_options = Options::new();
        materialized_options
            .add("filename", &materialized)
            .add("order", "X,Y,Z")
            .add("precision", 2);
        let mut stream_options = Options::new();
        stream_options
            .add("filename", &streamed)
            .add("order", "X,Y,Z")
            .add("precision", 2);

        let mut materialized_writer = PcdWriter::new(&materialized_options);
        materialized_writer
            .write(&[empty.clone(), nonempty.clone()])
            .unwrap();
        let mut stream_writer = PcdWriter::new(&stream_options);
        stream_writer.stream_write(&empty).unwrap();
        stream_writer.stream_write(&nonempty).unwrap();
        stream_writer.stream_finish().unwrap();

        assert_eq!(fs::read(&streamed).unwrap(), fs::read(&materialized).unwrap());
        let written = fs::read_to_string(&streamed).unwrap();
        assert!(written.contains("POINTS 1\nDATA ascii\n"));
        let _ = fs::remove_file(materialized);
        let _ = fs::remove_file(streamed);
    }

    #[test]
    fn per_dimension_precision_matches_existing_writer_shape() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for values in [
            [1.0, 1.0, 1.0, 1.0],
            [
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_22,
            ],
            [3.33, 3.33, 3.33, 3.33],
        ] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, values[0]);
            view.set_f64(point, &DimId::Y, values[1]);
            view.set_f64(point, &DimId::Z, values[2]);
            view.set_f64(point, &DimId::Intensity, values[3]);
        }

        let output = temp_path("precision.pcd");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("precision", 5)
            .add("order", "X=Float:0,Y=Float:0,Z=Float:0,Intensity=Float:0");
        let mut writer = PcdWriter::new(&options);
        writer.write(&[view]).unwrap();

        let written = fs::read_to_string(output).unwrap();
        assert!(written.contains("1 1 1 1"));
        assert!(written.contains("2 2 2 2"));
        assert!(written.contains("3 3 3 3"));
    }

    #[test]
    fn writes_binary_pcd_that_reader_roundtrips_typed_fields() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for values in [
            [1.0, 1.0, 1.0, 1.0],
            [
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_22,
            ],
            [3.33, 3.33, 3.33, 3.33],
        ] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, values[0]);
            view.set_f64(point, &DimId::Y, values[1]);
            view.set_f64(point, &DimId::Z, values[2]);
            view.set_f64(point, &DimId::Intensity, values[3]);
        }

        let output = temp_path("binary.pcd");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("order", "X=Float,Y=Float,Z=Float,Intensity=Unsigned32")
            .add("compression", "binary");
        let mut writer = PcdWriter::new(&options);
        writer.write(std::slice::from_ref(&view)).unwrap();

        let mut read_options = Options::new();
        read_options.add("filename", &output);
        let mut reader = PcdReader::new(&read_options);
        let roundtrip = reader.read().unwrap().pop().unwrap();

        assert_eq!(roundtrip.len(), 3);
        assert_eq!(roundtrip.get_f64(0, &DimId::Intensity), 1.0);
        assert_eq!(roundtrip.get_f64(1, &DimId::Intensity), 2.0);
        assert_eq!(roundtrip.get_f64(2, &DimId::Intensity), 3.0);
        assert!((roundtrip.get_f64(1, &DimId::X) - 2.222_222_222_2).abs() < 0.0001);
        assert!((roundtrip.get_f64(2, &DimId::Z) - 3.33).abs() < 0.0001);
    }

    #[test]
    fn writes_compressed_pcd_that_reader_roundtrips_typed_fields() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for values in [
            [1.0, 2.0, 3.0, 42.0],
            [4.5, 5.5, 6.5, 43.0],
            [7.25, 8.25, 9.25, 44.0],
        ] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, values[0]);
            view.set_f64(point, &DimId::Y, values[1]);
            view.set_f64(point, &DimId::Z, values[2]);
            view.set_f64(point, &DimId::Intensity, values[3]);
        }

        let output = temp_path("compressed.pcd");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("order", "X=Float,Y=Float,Z=Float,Intensity=Unsigned16")
            .add("compression", "compressed");
        let mut writer = PcdWriter::new(&options);
        writer.write(std::slice::from_ref(&view)).unwrap();

        let written = fs::read(&output).unwrap();
        let marker = b"DATA binary_compressed\n";
        let marker_start = written
            .windows(marker.len())
            .position(|window| window == marker)
            .unwrap();
        let payload_start = marker_start + marker.len();
        let compressed_size = u32::from_le_bytes(
            written[payload_start..payload_start + 4]
                .try_into()
                .unwrap(),
        );
        let uncompressed_size = u32::from_le_bytes(
            written[payload_start + 4..payload_start + 8]
                .try_into()
                .unwrap(),
        );
        assert!(compressed_size > 0);
        assert_eq!(uncompressed_size, 42);

        let mut read_options = Options::new();
        read_options.add("filename", &output);
        let mut reader = PcdReader::new(&read_options);
        let roundtrip = reader.read().unwrap().pop().unwrap();

        assert_eq!(roundtrip.len(), 3);
        assert!((roundtrip.get_f64(0, &DimId::X) - 1.0).abs() < 0.0001);
        assert!((roundtrip.get_f64(1, &DimId::Y) - 5.5).abs() < 0.0001);
        assert!((roundtrip.get_f64(2, &DimId::Z) - 9.25).abs() < 0.0001);
        assert_eq!(roundtrip.get_f64(0, &DimId::Intensity), 42.0);
        assert_eq!(roundtrip.get_f64(2, &DimId::Intensity), 44.0);
    }

    #[test]
    fn reader_filter_writer_pipeline_writes_ascii_pcd() {
        let input = data_path("pcd/utm17_space.pcd");
        let output = temp_path("pipeline.pcd");

        let mut reader_options = Options::new();
        reader_options.add("filename", input);
        let mut filter_options = Options::new();
        filter_options.add("step", 2);
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X,Y,Z")
            .add("precision", 2);

        let mut pipeline = Pipeline::new();
        let reader = pipeline.add_reader(
            "readers.pcd",
            Box::new(PcdReader::new(&reader_options)),
            reader_options,
        );
        let filter = pipeline.add_stage(
            "filters.decimation",
            Box::new(FilterWrapper::new(DecimationFilter::new(&filter_options))),
            filter_options,
        );
        let writer = pipeline.add_writer(
            "writers.pcd",
            Box::new(PcdWriter::new(&writer_options)),
            writer_options,
        );
        pipeline.add_dependency(filter, reader).unwrap();
        pipeline.add_dependency(writer, filter).unwrap();

        assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
        let written = fs::read_to_string(output).unwrap();
        assert!(written.contains("POINTS 5\nDATA ascii\n"));
    }

    #[test]
    fn pipeline_streams_to_ascii_pcd_writer() {
        let output = temp_path("stream-pipeline.pcd");

        let mut reader_options = Options::new();
        reader_options
            .add("count", "12")
            .add("mode", "ramp")
            .add("bounds", "([0,11],[0,11],[0,11])");
        let limits = vec![RangeLimit {
            dim_name: "X".to_string(),
            lower_bound: 0.0,
            upper_bound: 5.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X,Y,Z")
            .add("precision", 0);

        let mut pipeline = Pipeline::new();
        let reader = pipeline.add_reader(
            "readers.faux",
            Box::new(FauxReader::new(&reader_options).unwrap()),
            reader_options,
        );
        let filter = pipeline.add_stage(
            "filters.range",
            Box::new(FilterWrapper::new(RangeFilter::new(limits))),
            Options::new(),
        );
        let writer = pipeline.add_writer(
            "writers.pcd",
            Box::new(PcdWriter::new(&writer_options)),
            writer_options,
        );
        pipeline.add_dependency(filter, reader).unwrap();
        pipeline.add_dependency(writer, filter).unwrap();

        assert_eq!(pipeline.execute_streaming().unwrap(), Some(6));
        let written = fs::read_to_string(&output).unwrap();
        assert!(written.contains("POINTS 6\nDATA ascii\n"));
        assert!(written.contains("0 0 0 "));
        assert!(written.contains("5 5 5 "));
        let _ = fs::remove_file(output);
    }

    // --- Pure helper unit tests ---

    #[test]
    fn parse_field_type_all_variants() {
        assert_eq!(parse_field_type("I").unwrap(), FieldType::Signed);
        assert_eq!(parse_field_type("U").unwrap(), FieldType::Unsigned);
        assert_eq!(parse_field_type("F").unwrap(), FieldType::Float);
        assert!(parse_field_type("X").is_err());
    }

    #[test]
    fn canonical_dim_name_known_names() {
        assert_eq!(canonical_dim_name("x"), "X");
        assert_eq!(canonical_dim_name("y"), "Y");
        assert_eq!(canonical_dim_name("z"), "Z");
        assert_eq!(canonical_dim_name("intensity"), "Intensity");
        assert_eq!(canonical_dim_name("gpstime"), "GpsTime");
        assert_eq!(canonical_dim_name("rgb"), "rgb");
        assert_eq!(canonical_dim_name("unknown"), "unknown");
    }

    #[test]
    fn dim_type_maps_correctly() {
        let float4 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Float, count: 1, size: 4, precision: 2 };
        assert_eq!(dim_type(&float4), DimType::F32);
        let float8 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 1, size: 8, precision: 2 };
        assert_eq!(dim_type(&float8), DimType::F64);
        let signed2 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Signed, count: 1, size: 2, precision: 2 };
        assert_eq!(dim_type(&signed2), DimType::I16);
        let unsigned1 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Unsigned, count: 1, size: 1, precision: 2 };
        assert_eq!(dim_type(&unsigned1), DimType::U8);
        let x_float4 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 1, size: 4, precision: 2 };
        assert_eq!(dim_type(&x_float4), DimType::F64);
    }

    #[test]
    fn default_field_for_xyz_uses_float4() {
        let field = default_field(DimId::X, 6);
        assert_eq!(field.ty, FieldType::Float);
        assert_eq!(field.size, 4);
    }

    #[test]
    fn default_field_for_other_uses_float8() {
        let field = default_field(DimId::Intensity, 6);
        assert_eq!(field.ty, FieldType::Float);
        assert_eq!(field.size, 8);
    }

    #[test]
    fn apply_writer_type_unsigned32() {
        let mut field = default_field(DimId::Intensity, 6);
        apply_writer_type(&mut field, "Unsigned32").unwrap();
        assert_eq!(field.ty, FieldType::Unsigned);
        assert_eq!(field.size, 4);
    }

    #[test]
    fn apply_writer_type_invalid_is_error() {
        let mut field = default_field(DimId::X, 6);
        assert!(apply_writer_type(&mut field, "UnknownType").is_err());
    }

    #[test]
    fn parse_numbers_valid() {
        let result = parse_numbers(&["1", "2", "3"], "SIZE").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn parse_numbers_invalid_is_error() {
        assert!(parse_numbers(&["abc"], "SIZE").is_err());
    }

    #[test]
    fn parse_one_valid() {
        assert_eq!(parse_one(&["42"], "WIDTH").unwrap(), 42);
    }

    #[test]
    fn parse_one_empty_is_error() {
        assert!(parse_one(&[], "WIDTH").is_err());
    }

    #[test]
    fn read_binary_value_u8() {
        let bytes = vec![42u8, 1, 2, 3];
        let field = Field { id: DimId::X, label: "X".into(), ty: FieldType::Unsigned, count: 1, size: 1, precision: 2 };
        let mut offset = 0;
        let val = read_binary_value(&bytes, &mut offset, &field).unwrap();
        assert_eq!(val, 42.0);
        assert_eq!(offset, 1);
    }

    #[test]
    fn read_binary_value_f32() {
        let v = 3.14_f32;
        let bytes = v.to_le_bytes().to_vec();
        let field = Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 1, size: 4, precision: 2 };
        let mut offset = 0;
        let val = read_binary_value(&bytes, &mut offset, &field).unwrap();
        assert!((val - 3.14).abs() < 0.001);
        assert_eq!(offset, 4);
    }

    #[test]
    fn write_binary_value_u16() {
        let mut out = Vec::new();
        write_binary_value(&mut out, 42.0, FieldType::Unsigned, 2).unwrap();
        assert_eq!(out, vec![42, 0]);
    }

    #[test]
    fn write_binary_value_f32() {
        let mut out = Vec::new();
        write_binary_value(&mut out, 3.14, FieldType::Float, 4).unwrap();
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn binary_payload_size_computed() {
        let field1 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 2, size: 4, precision: 2 };
        let field2 = Field { id: DimId::Y, label: "Y".into(), ty: FieldType::Unsigned, count: 1, size: 2, precision: 2 };
        let header = Header {
            fields: vec![field1, field2],
            points: 10,
            data_start: 0,
            storage: "binary".to_string(),
        };
        let size = binary_payload_size(&header).unwrap();
        assert_eq!(size, 10 * (4 * 2 + 2));
    }

    #[test]
    fn data_storage_label_normalizes() {
        assert_eq!(data_storage_label("compressed"), "binary_compressed");
        assert_eq!(data_storage_label("binary_compressed"), "binary_compressed");
        assert_eq!(data_storage_label("ascii"), "ascii");
        assert_eq!(data_storage_label("binary"), "binary");
    }

    #[test]
    fn storage_value_f32_loses_precision() {
        let field = Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 1, size: 4, precision: 2 };
        assert!((storage_value(3.1415926535, &field) - 3.1415927_f64).abs() < 0.0001);
    }

    #[test]
    fn format_number_integer_unsigned() {
        let result = format_number(42.7, 0, FieldType::Unsigned, 4);
        assert_eq!(result, "42");
    }

    #[test]
    fn format_number_float() {
        let result = format_number(3.14159, 3, FieldType::Float, 8);
        assert_eq!(result, "3.142");
    }

    #[test]
    fn parse_header_minimal_ascii() {
        let header = parse_header(b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nWIDTH 3\nHEIGHT 1\nPOINTS 3\nDATA ascii\n").unwrap();
        assert_eq!(header.points, 3);
        assert_eq!(header.storage, "ascii");
    }

    #[test]
    fn parse_header_binary() {
        let header = parse_header(b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nCOUNT 1 1 1\nWIDTH 5\nHEIGHT 1\nVIEWPOINT 0 0 0 1 0 0 0\nPOINTS 5\nDATA binary\n").unwrap();
        assert_eq!(header.points, 5);
        assert_eq!(header.storage, "binary");
    }

    #[test]
    fn parse_header_missing_width_uses_height_x_points() {
        let header = parse_header(b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nHEIGHT 10\nPOINTS 10\nDATA ascii\n").unwrap();
        assert_eq!(header.fields.len(), 3);
    }

    #[test]
    fn parse_header_rejects_missing_data_marker() {
        assert!(parse_header(b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\n").is_err());
    }

    #[test]
    fn parse_header_unknown_field_counts_rejected() {
        let h = parse_header(b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4\nTYPE F F F\nWIDTH 3\nHEIGHT 1\nPOINTS 3\nDATA ascii\n");
        assert!(h.is_err());
    }

    #[test]
    fn parse_header_missing_storage_after_data_is_err() {
        let h = parse_header(b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nWIDTH 3\nHEIGHT 1\nPOINTS 3\nDATA\n");
        assert!(h.is_err());
    }

    #[test]
    fn parse_header_missing_field_names_is_err() {
        let h = parse_header(b"VERSION 0.7\nSIZE 4 4 4\nTYPE F F F\nWIDTH 3\nHEIGHT 1\nPOINTS 3\nDATA ascii\n");
        assert!(h.is_err());
    }

    #[test]
    fn parse_header_comment_and_empty_lines_skipped() {
        let header = parse_header(b"VERSION 0.7\n# comment\n\nFIELDS x y\nSIZE 4 4\nTYPE F F\nWIDTH 2\nHEIGHT 1\nPOINTS 2\nDATA ascii\n").unwrap();
        assert_eq!(header.points, 2);
        assert_eq!(header.fields.len(), 2);
    }

    #[test]
    fn parse_header_unknown_line_is_err() {
        assert!(parse_header(b"VERSION 0.7\nUNKNOWN value\nFIELDS x\nSIZE 4\nTYPE F\nWIDTH 1\nHEIGHT 1\nPOINTS 1\nDATA ascii\n").is_err());
    }

    #[test]
    fn parse_header_defaults_count_to_one_and_size_to_four() {
        let header = parse_header(b"VERSION 0.7\nFIELDS x y z\nTYPE F F F\nWIDTH 3\nHEIGHT 1\nPOINTS 3\nDATA ascii\n").unwrap();
        assert_eq!(header.fields.len(), 3);
        for f in &header.fields {
            assert_eq!(f.size, 4);
            assert_eq!(f.count, 1);
            assert_eq!(f.ty, FieldType::Float);
        }
    }

    #[test]
    fn parse_header_columns_as_fields_alias() {
        let header = parse_header(b"VERSION 0.7\nCOLUMNS x y\nSIZE 4 4\nTYPE F F\nWIDTH 2\nHEIGHT 1\nPOINTS 2\nDATA ascii\n").unwrap();
        assert_eq!(header.fields.len(), 2);
    }

    #[test]
    fn parse_header_crlf_line_endings() {
        let header = parse_header(b"VERSION 0.7\r\nFIELDS x y z\r\nSIZE 4 4 4\r\nTYPE F F F\r\nWIDTH 3\r\nHEIGHT 1\r\nPOINTS 3\r\nDATA ascii\r\n").unwrap();
        assert_eq!(header.points, 3);
    }

    #[test]
    fn parse_header_zero_points_computed_from_width_height() {
        let header = parse_header(b"VERSION 0.7\nFIELDS x y\nSIZE 4 4\nTYPE F F\nWIDTH 5\nHEIGHT 4\nDATA ascii\n").unwrap();
        assert_eq!(header.points, 20);
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = PcdReader::new(&Options::new());
        let result = reader.read();
        assert!(result.is_err());
        assert!(result.err().unwrap().0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/file.pcd");
        let mut reader = PcdReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_unknown_storage() {
        let path = temp_path("unknown-storage");
        std::fs::write(
            &path,
            b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nWIDTH 1\nHEIGHT 1\nPOINTS 1\nDATA mysterious\n",
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", path.clone());
        let mut reader = PcdReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("Unrecognized"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reader_skips_malformed_ascii_lines() {
        let path = temp_path("malformed-ascii");
        std::fs::write(
            &path,
            b"VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nWIDTH 2\nHEIGHT 1\nPOINTS 2\nDATA ascii\n1 2 3\nbad\n4 5 6\n",
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", path.clone());
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();
        assert_eq!(view.len(), 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        let reader = PcdReader::new(&Options::new());
        let metadata = reader.metadata();
        assert_eq!(metadata.name(), "readers.pcd");
    }

    #[test]
    fn writer_errors_without_filename() {
        let mut options = Options::new();
        options.add("compression", "ascii");
        let mut writer = PcdWriter::new(&options);
        let layout = PointLayout::new();
        let view = PointView::new(Rc::new(layout));
        let result = writer.write(&[view]);
        assert!(result.is_err());
    }

    #[test]
    fn writer_with_explicit_dim_order_keeps_only_specified() {
        let mut writer_options = Options::new();
        let temp = temp_path("dim-order");
        writer_options.add("filename", temp.clone());
        writer_options.add("compression", "ascii");
        writer_options.add("order", "X=Float,Y=Float");
        writer_options.add("keep_unspecified", false);
        let mut writer = PcdWriter::new(&writer_options);

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let p = view.add_point();
        view.set_f64(p, &DimId::X, 1.0);
        view.set_f64(p, &DimId::Y, 2.0);
        view.set_f64(p, &DimId::Z, 3.0);
        writer.write(&[view]).unwrap();

        let body = std::fs::read_to_string(&temp).unwrap();
        assert!(body.contains("FIELDS X Y") || body.contains("FIELDS x y"));
        assert!(!body.contains(" Z\n") && !body.contains(" z\n"));
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn writer_unknown_compression_errors() {
        let mut writer_options = Options::new();
        let temp = temp_path("bad-compression");
        writer_options.add("filename", temp.clone());
        writer_options.add("compression", "lzma-rocketboost");
        let mut writer = PcdWriter::new(&writer_options);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let view = PointView::new(Rc::new(layout));
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn test_names_and_metadata() {
        let reader = PcdReader::new(&Options::new());
        assert_eq!(reader.name(), "readers.pcd");
        
        let mut writer_opts = Options::new();
        writer_opts.add("filename", "dummy.pcd");
        let writer = PcdWriter::new(&writer_opts);
        assert_eq!(writer.name(), "writers.pcd");
        
        let metadata = writer.metadata();
        assert_eq!(metadata.name(), "writers.pcd");
    }

    #[test]
    fn test_writer_empty_views() {
        let output = temp_path("empty-views.pcd");
        let mut writer_opts = Options::new();
        writer_opts.add("filename", &output);
        let mut writer = PcdWriter::new(&writer_opts);
        writer.write(&[]).unwrap();
        let content = fs::read_to_string(&output).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_extract_dim_errors() {
        let mut writer_opts = Options::new();
        let temp = temp_path("extract-dim-errors");
        writer_opts.add("filename", temp.clone());
        
        // 1. Dimension not found
        writer_opts.add("order", "NonexistentDim");
        let mut writer = PcdWriter::new(&writer_opts);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let view = PointView::new(Rc::new(layout));
        assert!(writer.write(&[view.clone()]).is_err());
        
        // 2. Can't convert precision
        let mut writer_opts2 = Options::new();
        writer_opts2.add("filename", temp.clone());
        writer_opts2.add("order", "X=Float:abc");
        let mut writer2 = PcdWriter::new(&writer_opts2);
        assert!(writer2.write(&[view.clone()]).is_err());
        
        // 3. Extra colon parts
        let mut writer_opts3 = Options::new();
        writer_opts3.add("filename", temp.clone());
        writer_opts3.add("order", "X=Float:2:extra");
        let mut writer3 = PcdWriter::new(&writer_opts3);
        assert!(writer3.write(&[view.clone()]).is_err());
        
        // 4. Extra equals parts
        let mut writer_opts4 = Options::new();
        writer_opts4.add("filename", temp.clone());
        writer_opts4.add("order", "X=Float=Extra");
        let mut writer4 = PcdWriter::new(&writer_opts4);
        assert!(writer4.write(&[view.clone()]).is_err());
    }

    #[test]
    fn test_read_binary_value_all_types() {
        let bytes = vec![
            1u8,
            2, 0,
            3, 0, 0, 0,
            4, 0, 0, 0, 0, 0, 0, 0,
            5,
            6, 0,
            7, 0, 0, 0,
            8, 0, 0, 0, 0, 0, 0, 0,
        ];
        
        let mut offset = 0;
        
        let f_s1 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Signed, count: 1, size: 1, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_s1).unwrap(), 1.0);
        
        let f_s2 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Signed, count: 1, size: 2, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_s2).unwrap(), 2.0);
        
        let f_s4 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Signed, count: 1, size: 4, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_s4).unwrap(), 3.0);
        
        let f_s8 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Signed, count: 1, size: 8, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_s8).unwrap(), 4.0);
        
        let f_u1 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Unsigned, count: 1, size: 1, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_u1).unwrap(), 5.0);
        
        let f_u2 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Unsigned, count: 1, size: 2, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_u2).unwrap(), 6.0);
        
        let f_u4 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Unsigned, count: 1, size: 4, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_u4).unwrap(), 7.0);
        
        let f_u8 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Unsigned, count: 1, size: 8, precision: 2 };
        assert_eq!(read_binary_value(&bytes, &mut offset, &f_u8).unwrap(), 8.0);
        
        let mut f32_offset = 0;
        let f32_bytes = 1.23f32.to_le_bytes().to_vec();
        let f_f4 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 1, size: 4, precision: 2 };
        assert!((read_binary_value(&f32_bytes, &mut f32_offset, &f_f4).unwrap() - 1.23).abs() < 1e-5);
        
        let mut f64_offset = 0;
        let f64_bytes = 4.56f64.to_le_bytes().to_vec();
        let f_f8 = Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 1, size: 8, precision: 2 };
        assert!((read_binary_value(&f64_bytes, &mut f64_offset, &f_f8).unwrap() - 4.56).abs() < 1e-9);
        
        let mut short_offset = 0;
        assert!(read_binary_value(&[0u8], &mut short_offset, &f_s2).is_err());
    }

    #[test]
    fn test_write_binary_value_all_types() {
        let mut out = Vec::new();
        write_binary_value(&mut out, 1.0, FieldType::Signed, 1).unwrap();
        write_binary_value(&mut out, 2.0, FieldType::Signed, 2).unwrap();
        write_binary_value(&mut out, 3.0, FieldType::Signed, 4).unwrap();
        write_binary_value(&mut out, 4.0, FieldType::Signed, 8).unwrap();
        write_binary_value(&mut out, 5.0, FieldType::Unsigned, 1).unwrap();
        write_binary_value(&mut out, 6.0, FieldType::Unsigned, 2).unwrap();
        write_binary_value(&mut out, 7.0, FieldType::Unsigned, 4).unwrap();
        write_binary_value(&mut out, 8.0, FieldType::Unsigned, 8).unwrap();
        write_binary_value(&mut out, 1.23, FieldType::Float, 4).unwrap();
        write_binary_value(&mut out, 4.56, FieldType::Float, 8).unwrap();
        
        assert_eq!(out.len(), 1 + 2 + 4 + 8 + 1 + 2 + 4 + 8 + 4 + 8);
        assert!(write_binary_value(&mut out, 1.0, FieldType::Signed, 3).is_err());
    }

    #[test]
    fn test_apply_writer_type_all_types() {
        let mut field = default_field(DimId::Intensity, 6);
        
        apply_writer_type(&mut field, "Unsigned8").unwrap();
        assert_eq!(field.ty, FieldType::Unsigned);
        assert_eq!(field.size, 1);
        
        apply_writer_type(&mut field, "Unsigned16").unwrap();
        assert_eq!(field.size, 2);
        
        apply_writer_type(&mut field, "Unsigned32").unwrap();
        assert_eq!(field.size, 4);
        
        apply_writer_type(&mut field, "Unsigned64").unwrap();
        assert_eq!(field.size, 8);
        
        apply_writer_type(&mut field, "Signed8").unwrap();
        assert_eq!(field.ty, FieldType::Signed);
        assert_eq!(field.size, 1);
        
        apply_writer_type(&mut field, "Signed16").unwrap();
        assert_eq!(field.size, 2);
        
        apply_writer_type(&mut field, "Signed32").unwrap();
        assert_eq!(field.size, 4);
        
        apply_writer_type(&mut field, "Signed64").unwrap();
        assert_eq!(field.size, 8);
        
        apply_writer_type(&mut field, "Float").unwrap();
        assert_eq!(field.ty, FieldType::Float);
        assert_eq!(field.size, 4);
        
        apply_writer_type(&mut field, "Double").unwrap();
        assert_eq!(field.size, 8);
    }

    #[test]
    fn test_dim_type_all_variants() {
        let f_s1 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Signed, count: 1, size: 1, precision: 2 };
        assert_eq!(dim_type(&f_s1), DimType::I8);
        
        let f_s2 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Signed, count: 1, size: 2, precision: 2 };
        assert_eq!(dim_type(&f_s2), DimType::I16);
        
        let f_s4 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Signed, count: 1, size: 4, precision: 2 };
        assert_eq!(dim_type(&f_s4), DimType::I32);
        
        let f_s8 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Signed, count: 1, size: 8, precision: 2 };
        assert_eq!(dim_type(&f_s8), DimType::I64);
        
        let f_u1 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Unsigned, count: 1, size: 1, precision: 2 };
        assert_eq!(dim_type(&f_u1), DimType::U8);
        
        let f_u2 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Unsigned, count: 1, size: 2, precision: 2 };
        assert_eq!(dim_type(&f_u2), DimType::U16);
        
        let f_u4 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Unsigned, count: 1, size: 4, precision: 2 };
        assert_eq!(dim_type(&f_u4), DimType::U32);
        
        let f_u8 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Unsigned, count: 1, size: 8, precision: 2 };
        assert_eq!(dim_type(&f_u8), DimType::U64);
        
        let f_f4 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Float, count: 1, size: 4, precision: 2 };
        assert_eq!(dim_type(&f_f4), DimType::F32);
        
        let f_f8 = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Float, count: 1, size: 8, precision: 2 };
        assert_eq!(dim_type(&f_f8), DimType::F64);
        
        let f_invalid = Field { id: DimId::Intensity, label: "Intensity".into(), ty: FieldType::Signed, count: 1, size: 3, precision: 2 };
        assert_eq!(dim_type(&f_invalid), DimType::F64);
    }

    #[test]
    fn test_compressed_payload_errors() {
        let header = Header {
            fields: vec![Field { id: DimId::X, label: "X".into(), ty: FieldType::Float, count: 1, size: 4, precision: 2 }],
            points: 10,
            data_start: 0,
            storage: "binary_compressed".to_string(),
        };
        
        assert!(read_compressed_payload(&header, &[0u8; 4]).is_err());
        
        let mut size_mismatch_bytes = vec![0u8; 8];
        size_mismatch_bytes[4..8].copy_from_slice(&5u32.to_le_bytes());
        assert!(read_compressed_payload(&header, &size_mismatch_bytes).is_err());
        
        let mut too_large_bytes = vec![0u8; 8];
        too_large_bytes[0..4].copy_from_slice(&100u32.to_le_bytes());
        too_large_bytes[4..8].copy_from_slice(&40u32.to_le_bytes());
        assert!(read_compressed_payload(&header, &too_large_bytes).is_err());
    }
}
