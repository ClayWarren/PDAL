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
    fn streaming_binary_writer_matches_materialized_write() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/autzen-utm.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let mut first = view.clone();
        first.truncate(500);
        let mut second = view.make_new();
        for idx in 500..view.len() {
            second.append_point(&view, idx);
        }

        let materialized = temp_path("materialized-binary-stream-compare.pcd");
        let streamed = temp_path("streamed-binary-stream-compare.pcd");
        let mut materialized_options = Options::new();
        materialized_options
            .add("filename", &materialized)
            .add("compression", "binary")
            .add("order", "X=Float,Y=Float,Z=Float,GpsTime=Double");
        let mut stream_options = Options::new();
        stream_options
            .add("filename", &streamed)
            .add("compression", "binary")
            .add("order", "X=Float,Y=Float,Z=Float,GpsTime=Double");

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
        assert!(reader.streamable());
        let roundtrip = reader.read().unwrap().pop().unwrap();

        assert_eq!(roundtrip.len(), 3);
        assert!((roundtrip.get_f64(0, &DimId::X) - 1.0).abs() < 0.0001);
        assert!((roundtrip.get_f64(1, &DimId::Y) - 5.5).abs() < 0.0001);
        assert!((roundtrip.get_f64(2, &DimId::Z) - 9.25).abs() < 0.0001);
        assert_eq!(roundtrip.get_f64(0, &DimId::Intensity), 42.0);
        assert_eq!(roundtrip.get_f64(2, &DimId::Intensity), 44.0);

        let mut stream_reader = PcdReader::new(&read_options);
        let first = stream_reader.stream_next(2).unwrap().unwrap();
        let second = stream_reader.stream_next(2).unwrap().unwrap();
        assert!(stream_reader.stream_next(2).unwrap().is_none());
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 1);
        assert_eq!(first.get_f64(0, &DimId::Intensity), 42.0);
        assert!((first.get_f64(1, &DimId::Y) - 5.5).abs() < 0.0001);
        assert!((second.get_f64(0, &DimId::Z) - 9.25).abs() < 0.0001);
        let _ = fs::remove_file(output);
    }

    #[test]
    fn streaming_compressed_writer_matches_materialized_write() {
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
            [10.0, 11.0, 12.0, 45.0],
        ] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, values[0]);
            view.set_f64(point, &DimId::Y, values[1]);
            view.set_f64(point, &DimId::Z, values[2]);
            view.set_f64(point, &DimId::Intensity, values[3]);
        }

        let mut first = view.make_new();
        first.append_point(&view, 0);
        first.append_point(&view, 1);
        let mut second = view.make_new();
        second.append_point(&view, 2);
        second.append_point(&view, 3);
        let materialized = temp_path("materialized-compressed-stream-compare.pcd");
        let streamed = temp_path("streamed-compressed-stream-compare.pcd");
        let mut materialized_options = Options::new();
        materialized_options
            .add("filename", &materialized)
            .add("order", "X=Float,Y=Float,Z=Float,Intensity=Unsigned16")
            .add("compression", "compressed");
        let mut stream_options = Options::new();
        stream_options
            .add("filename", &streamed)
            .add("order", "X=Float,Y=Float,Z=Float,Intensity=Unsigned16")
            .add("compression", "compressed");

        let mut materialized_writer = PcdWriter::new(&materialized_options);
        materialized_writer
            .write(std::slice::from_ref(&view))
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

    #[test]
    fn pipeline_streams_to_compressed_pcd_writer() {
        let output = temp_path("stream-compressed-pipeline.pcd");

        let mut reader_options = Options::new();
        reader_options
            .add("count", "12")
            .add("mode", "ramp")
            .add("bounds", "([0,11],[0,11],[0,11])");
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X=Float,Y=Float,Z=Float")
            .add("compression", "compressed");

        let mut pipeline = Pipeline::new();
        let reader = pipeline.add_reader(
            "readers.faux",
            Box::new(FauxReader::new(&reader_options).unwrap()),
            reader_options,
        );
        let writer = pipeline.add_writer(
            "writers.pcd",
            Box::new(PcdWriter::new(&writer_options)),
            writer_options,
        );
        pipeline.add_dependency(writer, reader).unwrap();

        assert_eq!(pipeline.execute_streaming().unwrap(), Some(12));
        let mut read_options = Options::new();
        read_options.add("filename", &output);
        let mut reader = PcdReader::new(&read_options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 12);
        assert!((view.get_f64(11, &DimId::Z) - 11.0).abs() < 0.0001);
        let _ = fs::remove_file(output);
    }

    mod pcd_helper_tests;
}
