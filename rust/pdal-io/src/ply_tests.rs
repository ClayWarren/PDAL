#[cfg(test)]
mod tests {
    use super::*;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn read_ply(path: &str) -> PointView {
        let mut options = Options::new();
        options.add("filename", data_path(path));
        let mut reader = PlyReader::new(&options);
        let mut views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        views.pop().unwrap()
    }

    #[test]
    fn reads_ascii_text_vertices() {
        let view = read_ply("ply/simple_text.ply");
        assert_eq!(view.len(), 3);

        for (idx, (x, y, z)) in [(-1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)]
            .into_iter()
            .enumerate()
        {
            let idx = idx as u64;
            assert_eq!(view.get_f64(idx, &DimId::X), x);
            assert_eq!(view.get_f64(idx, &DimId::Y), y);
            assert_eq!(view.get_f64(idx, &DimId::Z), z);
        }
    }

    #[test]
    fn reads_extra_dimensions_and_empty_face_element() {
        let view = read_ply("ply/text_extradim.ply");
        assert_eq!(view.len(), 1);

        assert_eq!(view.get_f64(0, &DimId::X), -2.64944);
        assert_eq!(view.get_f64(0, &DimId::Y), -13.0955);
        assert_eq!(view.get_f64(0, &DimId::Z), 0.00640115);
        assert_eq!(view.get_f64(0, &DimId::from_name("red")), 63.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("green")), 200.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("blue")), 64.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("alpha")), 255.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("omg")), 1234.0);
    }

    #[test]
    fn consumes_vertex_list_properties() {
        let output = temp_path("vertex-list.ply");
        fs::write(
            &output,
            b"ply
format ascii 1.0
element vertex 2
property float x
property float y
property float z
property list uchar int neighbor_indices
end_header
1 2 3 2 0 1
4 5 6 1 0
",
        )
        .unwrap();

        let view = read_back(&output);
        assert_eq!(view.len(), 2);
        assert_eq!(view.get_f64(0, &DimId::X), 1.0);
        assert_eq!(view.get_f64(0, &DimId::Y), 2.0);
        assert_eq!(view.get_f64(0, &DimId::Z), 3.0);
        assert_eq!(view.get_f64(1, &DimId::X), 4.0);
        assert_eq!(view.get_f64(1, &DimId::Y), 5.0);
        assert_eq!(view.get_f64(1, &DimId::Z), 6.0);
    }

    #[test]
    fn reads_ascii_mesh_faces() {
        let view = read_ply("ply/mesh.ply");
        assert_eq!(view.len(), 4);

        let mesh = view.mesh().unwrap();
        assert_eq!(mesh.len(), 2);
        assert_eq!(mesh.triangles()[0].a, 0);
        assert_eq!(mesh.triangles()[0].b, 1);
        assert_eq!(mesh.triangles()[0].c, 2);
        assert_eq!(mesh.triangles()[1].a, 1);
        assert_eq!(mesh.triangles()[1].b, 2);
        assert_eq!(mesh.triangles()[1].c, 3);
    }

    #[test]
    fn reads_sized_dimensions_with_xyz_forced_to_double() {
        let view = read_ply("ply/sized_dims.ply");
        assert_eq!(view.len(), 1);

        // `x` is declared int8 but X is always stored as a double.
        assert_eq!(view.get_f64(0, &DimId::X), 1.0);
        assert_eq!(view.get_f64(0, &DimId::Y), 12346.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("j")), 12345.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("i")), 1234567890.0);
    }

    #[test]
    fn rejects_a_file_without_a_vertex_element() {
        let mut options = Options::new();
        options.add("filename", data_path("ply/no_vertex.ply"));
        assert!(PlyReader::new(&options).read().is_err());
    }

    #[test]
    fn reads_binary_little_endian_vertices() {
        let view = read_ply("ply/simple_binary.ply");
        assert_eq!(view.len(), 3);

        for (idx, (x, y, z)) in [(-1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)]
            .into_iter()
            .enumerate()
        {
            let idx = idx as u64;
            assert_eq!(view.get_f64(idx, &DimId::X), x);
            assert_eq!(view.get_f64(idx, &DimId::Y), y);
            assert_eq!(view.get_f64(idx, &DimId::Z), z);
        }
    }

    fn temp_path(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-ply-{}-{name}", std::process::id()));
        let _ = fs::remove_file(&path);
        path.display().to_string()
    }

    fn xyz_view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for &(x, y, z) in points {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, x);
            view.set_f64(point, &DimId::Y, y);
            view.set_f64(point, &DimId::Z, z);
        }
        view
    }

    fn read_back(path: &str) -> PointView {
        let mut options = Options::new();
        options.add("filename", path);
        PlyReader::new(&options).read().unwrap().pop().unwrap()
    }

    #[test]
    fn writes_ascii_ply_that_reader_round_trips() {
        let view = xyz_view(&[(-1.5, 0.0, 0.25), (0.0, 1.0, 2.0), (3.5, -4.25, 5.0)]);
        let output = temp_path("roundtrip.ply");

        let mut options = Options::new();
        options.add("filename", &output).add("precision", 6);
        PlyWriter::new(&options).unwrap().write(&[view]).unwrap();

        let back = read_back(&output);
        assert_eq!(back.len(), 3);
        assert_eq!(back.get_f64(0, &DimId::X), -1.5);
        assert_eq!(back.get_f64(0, &DimId::Z), 0.25);
        assert_eq!(back.get_f64(2, &DimId::X), 3.5);
        assert_eq!(back.get_f64(2, &DimId::Y), -4.25);
    }

    #[test]
    fn writes_ascii_mesh_faces_matching_existing_fixture() {
        let mut view = xyz_view(&[
            (1.0, 1.0, 0.0),
            (2.0, 1.0, 0.0),
            (1.0, 2.0, 0.0),
            (2.0, 2.0, 2.0),
        ]);
        let mesh = view.create_mesh();
        mesh.add(0, 1, 2);
        mesh.add(1, 2, 3);

        let output = temp_path("mesh.ply");
        let mut options = Options::new();
        options.add("filename", &output).add("faces", true);
        PlyWriter::new(&options).unwrap().write(&[view]).unwrap();

        let written = fs::read_to_string(&output).unwrap();
        let expected = fs::read_to_string(data_path("ply/mesh.ply")).unwrap();
        assert_eq!(written, expected);
    }

    #[test]
    fn writes_ascii_mesh_faces_with_precision() {
        let mut view = xyz_view(&[
            (1.0, 1.0, 0.0),
            (2.0, 1.0, 0.0),
            (1.0, 2.0, 0.0),
            (2.0, 2.0, 2.0),
        ]);
        let mesh = view.create_mesh();
        mesh.add(0, 1, 2);
        mesh.add(1, 2, 3);

        let output = temp_path("mesh-fixed.ply");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("faces", true)
            .add("precision", 3);
        PlyWriter::new(&options).unwrap().write(&[view]).unwrap();

        let written = fs::read_to_string(&output).unwrap();
        let expected = fs::read_to_string(data_path("ply/mesh_fixed.ply")).unwrap();
        assert_eq!(written, expected);
    }

    #[test]
    fn dims_option_selects_and_orders_properties() {
        let view = xyz_view(&[(1.0, 2.0, 3.0)]);
        let output = temp_path("dimorder.ply");

        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("precision", 3)
            .add("dims", "Z,X");
        PlyWriter::new(&options).unwrap().write(&[view]).unwrap();

        let written = fs::read_to_string(&output).unwrap();
        let header: Vec<&str> = written.lines().collect();
        assert!(header.contains(&"property float64 z"));
        assert!(header.contains(&"property float64 x"));
        assert!(!header.contains(&"property float64 y"));

        let back = read_back(&output);
        assert_eq!(back.get_f64(0, &DimId::X), 1.0);
        assert_eq!(back.get_f64(0, &DimId::Z), 3.0);
    }

    #[test]
    fn writes_binary_little_endian_ply_that_reader_round_trips() {
        let view = xyz_view(&[(1.0, 1.0, 1.0)]);
        let output = temp_path("binary.ply");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("storage_mode", "binary_little_endian");
        PlyWriter::new(&options).unwrap().write(&[view]).unwrap();

        let back = read_back(&output);
        assert_eq!(back.len(), 1);
        assert_eq!(back.get_f64(0, &DimId::X), 1.0);
        assert_eq!(back.get_f64(0, &DimId::Y), 1.0);
        assert_eq!(back.get_f64(0, &DimId::Z), 1.0);
    }

    #[test]
    fn writes_hash_template_as_one_file_per_view() {
        let views = [
            xyz_view(&[(1.0, 2.0, 3.0)]),
            xyz_view(&[(4.0, 5.0, 6.0), (7.0, 8.0, 9.0)]),
            xyz_view(&[(10.0, 11.0, 12.0), (13.0, 14.0, 15.0), (16.0, 17.0, 18.0)]),
        ];
        let output = temp_path("flex-#.ply");
        for idx in 1..=3 {
            let _ = fs::remove_file(output.replace('#', &idx.to_string()));
        }

        let mut options = Options::new();
        options.add("filename", &output);
        PlyWriter::new(&options).unwrap().write(&views).unwrap();

        assert!(!Path::new(&output).exists());
        for (idx, expected_len) in [1, 2, 3].into_iter().enumerate() {
            let path = output.replace('#', &(idx + 1).to_string());
            let back = read_back(&path);
            assert_eq!(back.len(), expected_len);
            assert_eq!(back.get_f64(0, &DimId::X), views[idx].get_f64(0, &DimId::X));
        }
    }

    #[test]
    fn binary_storage_rejects_precision_like_cpp_writer() {
        let output = temp_path("binary-precision.ply");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("storage_mode", "little endian")
            .add("precision", 3);

        let err = PlyWriter::new(&options).err().unwrap();
        assert!(err.0.contains("precision"));
        assert!(err.0.contains("storage_mode"));
    }

    #[test]
    fn read_binary_value_big_endian_and_integer_types() {
        use std::io::Cursor;
        let mut buf = Vec::new();
        buf.extend_from_slice(&(-3_i16).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryBigEndian, DimType::I16).unwrap();
        assert_eq!(val, -3.0);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(300_u16).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryBigEndian, DimType::U16).unwrap();
        assert_eq!(val, 300.0);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(-100_000_i32).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryBigEndian, DimType::I32).unwrap();
        assert_eq!(val, -100_000.0);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(400_000_u32).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryBigEndian, DimType::U32).unwrap();
        assert_eq!(val, 400_000.0);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(3.14_f32).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryBigEndian, DimType::F32).unwrap();
        assert!((val - 3.14).abs() < 0.001);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(std::f64::consts::PI).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryBigEndian, DimType::F64).unwrap();
        assert!((val - std::f64::consts::PI).abs() < 0.0001);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(42_i8).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryBigEndian, DimType::I8).unwrap();
        assert_eq!(val, 42.0);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(200_u8).to_be_bytes());
        let mut cursor = Cursor::new(&buf[..]);
        let val = read_binary_value(&mut cursor, PlyFormat::BinaryLittleEndian, DimType::U8).unwrap();
        assert_eq!(val, 200.0);
    }

    #[test]
    fn read_binary_value_unsupported_ascii_format_is_error() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0u8; 4]);
        let mut cursor = Cursor::new(&buf[..]);
        let err = read_binary_value(&mut cursor, PlyFormat::Ascii, DimType::F32);
        assert!(err.is_err());
    }

    #[test]
    fn ply_reader_empty_filename_is_error() {
        let opts = Options::new();
        let mut reader = PlyReader::new(&opts);
        assert!(reader.read().is_err());
    }

    #[test]
    fn ply_reader_name_is_readers_ply() {
        let opts = Options::new();
        let reader = PlyReader::new(&opts);
        assert_eq!((&reader as &dyn Reader).name(), "readers.ply");
    }

    #[test]
    fn ply_writer_empty_filename_is_error() {
        let opts = Options::new();
        let mut writer = PlyWriter::new(&opts).unwrap();
        assert!(writer.write(&[]).is_err());
    }

    #[test]
    fn ply_writer_name_is_writers_ply() {
        let mut opts = Options::new();
        opts.add("filename", "test.ply");
        let writer = PlyWriter::new(&opts).unwrap();
        assert_eq!((&writer as &dyn Writer).name(), "writers.ply");
    }

    #[test]
    fn parse_property_list_on_non_vertex_succeeds() {
        let prop =
            parse_property(&["property", "list", "uchar", "int32", "indices"], "face").unwrap();
        match prop {
            PlyProp::List {
                ref name,
                count_ty,
                list_ty,
            } => {
                assert_eq!(name, "indices");
                assert_eq!(count_ty, DimType::U8);
                assert_eq!(list_ty, DimType::I32);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_property_list_invalid_type_is_error() {
        assert!(parse_property(
            &["property", "list", "badtype", "int32", "x"],
            "face"
        )
        .is_err());
    }

    #[test]
    fn parse_property_list_missing_name_is_error() {
        assert!(parse_property(
            &["property", "list", "uchar", "int32"],
            "face"
        )
        .is_err());
    }

    #[test]
    fn parse_property_list_extra_tokens_is_error() {
        assert!(parse_property(
            &["property", "list", "uchar", "int32", "indices", "extra"],
            "face"
        )
        .is_err());
    }

    #[test]
    fn parse_property_simple_missing_name_is_error() {
        assert!(parse_property(&["property", "float32"], "vertex").is_err());
    }

    #[test]
    fn parse_property_missing_all_is_error() {
        assert!(parse_property(&["property"], "vertex").is_err());
    }

    #[test]
    fn ply_type_string_full_coverage() {
        assert_eq!(ply_type_string(DimType::I16, false), Some("short"));
        assert_eq!(ply_type_string(DimType::U16, false), Some("ushort"));
        assert_eq!(ply_type_string(DimType::I32, false), Some("int"));
        assert_eq!(ply_type_string(DimType::U32, false), Some("uint"));
        assert_eq!(ply_type_string(DimType::F32, false), Some("float"));
        assert_eq!(ply_type_string(DimType::F64, false), Some("double"));
        assert_eq!(ply_type_string(DimType::I64, true), None);
        assert_eq!(ply_type_string(DimType::U64, true), None);
    }

    #[test]
    fn format_value_binary_variants() {
        assert_eq!(format_value(-5.7, DimType::I8, None), "-6");
        assert_eq!(format_value(42.3, DimType::U16, None), "42");
        assert_eq!(format_value(-1.0, DimType::U8, None), "0");
        assert_eq!(format_value(3.1415, DimType::F32, None), "3.1414999961853027");
        assert_eq!(format_value(3.1415, DimType::F32, Some(2)), "3.14");
        assert_eq!(format_value(2.5, DimType::F64, None), "2.5");
        assert_eq!(format_value(-200.0, DimType::I64, None), "-200");
    }

    #[test]
    fn write_ply_value_ascii_success() {
        let mut out = Vec::new();
        write_ply_value(&mut out, PlyFormat::Ascii, 3.14, DimType::F64, None).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "3.14");
    }

    #[test]
    fn write_ply_value_binary_success() {
        let mut out = Vec::new();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, -3.0, DimType::I8, None)
            .unwrap();
        assert_eq!(out, vec![253u8]);

        let mut out = Vec::new();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 200.0, DimType::U8, None)
            .unwrap();
        assert_eq!(out, vec![200]);

        let mut out = Vec::new();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, -1.0, DimType::U8, None)
            .unwrap();
        assert_eq!(out, vec![0]);

        let mut out = Vec::new();
        write_ply_value(
            &mut out,
            PlyFormat::BinaryBigEndian,
            3.14,
            DimType::F32,
            None,
        )
        .unwrap();
        assert_eq!(out.len(), 4);

        let mut out = Vec::new();
        write_ply_value(
            &mut out,
            PlyFormat::BinaryBigEndian,
            std::f64::consts::PI,
            DimType::F64,
            None,
        )
        .unwrap();
        assert_eq!(out.len(), 8);
    }

    #[test]
    fn write_ply_value_unsupported_type_is_error() {
        let mut out = Vec::new();
        let err =
            write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 5.0, DimType::I64, None);
        assert!(err.is_err());
        let err =
            write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 5.0, DimType::U64, None);
        assert!(err.is_err());
    }

    #[test]
    fn write_triangle_ascii_and_binary() {
        let tri = pdal_core::point::Triangle {
            a: 0,
            b: 1,
            c: 2,
        };

        let mut out = Vec::new();
        write_triangle(&mut out, PlyFormat::Ascii, &tri, 0).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "3 0 1 2\n");

        let mut out = Vec::new();
        write_triangle(&mut out, PlyFormat::BinaryLittleEndian, &tri, 0).unwrap();
        assert_eq!(out.len(), 1 + 4 * 3);
        assert_eq!(out[0], 3);

        let mut out = Vec::new();
        write_triangle(&mut out, PlyFormat::BinaryBigEndian, &tri, 10).unwrap();
        assert_eq!(out.len(), 1 + 4 * 3);
        assert_eq!(out[0], 3);
        let a_val = u32::from_be_bytes(out[1..5].try_into().unwrap());
        assert_eq!(a_val, 10);
    }

    #[test]
    fn ply_writer_big_endian_binary_roundtrip() {
        let view = xyz_view(&[(-1.5, 0.0, 0.25), (0.0, 1.0, 2.0)]);
        let output = temp_path("bigendian.ply");
        let mut opts = Options::new();
        opts.add("filename", &output)
            .add("storage_mode", "big endian");
        PlyWriter::new(&opts).unwrap().write(&[view]).unwrap();
        let back = read_back(&output);
        assert_eq!(back.len(), 2);
        assert_eq!(back.get_f64(0, &DimId::X), -1.5);
    }

    #[test]
    fn ply_writer_rejects_precision_with_binary() {
        let output = temp_path("bigendian-precision.ply");
        let mut opts = Options::new();
        opts.add("filename", &output)
            .add("storage_mode", "big endian")
            .add("precision", 3);
        let result = PlyWriter::new(&opts);
        assert!(result.is_err());
    }

    #[test]
    fn ply_writer_unknown_dim_in_dims_is_error() {
        let view = xyz_view(&[(1.0, 2.0, 3.0)]);
        let output = temp_path("uknown-dim.ply");
        let mut opts = Options::new();
        opts.add("filename", &output)
            .add("dims", "UnknownDim");
        let mut writer = PlyWriter::new(&opts).unwrap();
        let err = writer.write(&[view]).unwrap_err();
        assert!(err.0.contains("Unknown dimension"));
    }

    #[test]
    fn ply_writer_invalid_type_in_dims_is_error() {
        let view = xyz_view(&[(1.0, 2.0, 3.0)]);
        let output = temp_path("invalid-type.ply");
        let mut opts = Options::new();
        opts.add("filename", &output)
            .add("dims", "X=BadType");
        let mut writer = PlyWriter::new(&opts).unwrap();
        let err = writer.write(&[view]).unwrap_err();
        assert!(err.0.contains("Invalid type"));
    }

    #[test]
    fn ply_writer_faces_ascii_matches_expected() {
        let mut view = xyz_view(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)]);
        let mesh = view.create_mesh();
        mesh.add(0, 1, 2);
        let output = temp_path("face-only.ply");
        let mut opts = Options::new();
        opts.add("filename", &output).add("faces", true);
        PlyWriter::new(&opts).unwrap().write(&[view]).unwrap();
        let written = fs::read_to_string(&output).unwrap();
        assert!(written.contains("element face 1"));
        assert!(written.contains("property list uint8 uint32 vertex_indices"));
        assert!(written.contains("3 0 1 2"));
    }

    #[test]
    fn ply_writer_uses_hash_template_for_multi_view() {
        let views = vec![
            xyz_view(&[(1.0, 2.0, 3.0)]),
            xyz_view(&[(4.0, 5.0, 6.0)]),
        ];
        let template = temp_path("multi-#.ply");
        for i in 1..=2 {
            let _ = fs::remove_file(template.replace('#', &i.to_string()));
        }
        let mut opts = Options::new();
        opts.add("filename", &template);
        PlyWriter::new(&opts).unwrap().write(&views).unwrap();
        for i in 1..=2 {
            let path = template.replace('#', &i.to_string());
            assert!(Path::new(&path).exists(), "missing {path}");
            let back = read_back(&path);
            assert_eq!(back.len(), 1);
        }
    }

    #[test]
    fn ply_writer_zero_views_writes_empty_vertex() {
        let output = temp_path("empty.ply");
        let mut opts = Options::new();
        opts.add("filename", &output);
        PlyWriter::new(&opts).unwrap().write(&[]).unwrap();
        let written = fs::read_to_string(&output).unwrap();
        assert!(written.contains("element vertex 0"));
    }

    #[test]
    fn ply_writer_metadata_contains_filename() {
        let mut opts = Options::new();
        opts.add("filename", "test.ply");
        let writer = PlyWriter::new(&opts).unwrap();
        let meta = writer.metadata();
        let json = format!("{:?}", meta);
        assert!(json.contains("test.ply"));
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = PlyReader::new(&Options::new());
        let result = reader.read();
        assert!(result.is_err());
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/file.ply");
        let mut reader = PlyReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        let reader = PlyReader::new(&Options::new());
        let metadata = reader.metadata();
        assert_eq!(metadata.name(), "readers.ply");
    }

    #[test]
    fn writer_errors_without_filename() {
        let mut options = Options::new();
        options.add("storage_mode", "ascii");
        let writer = PlyWriter::new(&options);
        assert!(writer.is_ok());
        let layout = PointLayout::new();
        let view = PointView::new(Rc::new(layout));
        let result = writer.unwrap().write(&[view]);
        assert!(result.is_err());
    }

    #[test]
    fn writer_errors_with_invalid_storage_mode() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.ply");
        options.add("storage_mode", "alien-mode");
        let result = PlyWriter::new(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_names_and_metadata() {
        let reader = PlyReader::new(&Options::new());
        assert_eq!(reader.name(), "readers.ply");
        assert_eq!(reader.metadata().name(), "readers.ply");
        
        let mut writer_opts = Options::new();
        writer_opts.add("filename", "dummy.ply");
        let writer = PlyWriter::new(&writer_opts).unwrap();
        assert_eq!(writer.name(), "writers.ply");
        assert_eq!(writer.metadata().name(), "writers.ply");
    }

    #[test]
    fn test_read_binary_value_all_types() {
        use std::io::Cursor;
        
        // Little Endian
        assert_eq!(read_binary_value(&mut Cursor::new(&[1u8]), PlyFormat::BinaryLittleEndian, DimType::I8).unwrap(), 1.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[2u8]), PlyFormat::BinaryLittleEndian, DimType::U8).unwrap(), 2.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[3u8, 0]), PlyFormat::BinaryLittleEndian, DimType::I16).unwrap(), 3.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[4u8, 0]), PlyFormat::BinaryLittleEndian, DimType::U16).unwrap(), 4.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[5u8, 0, 0, 0]), PlyFormat::BinaryLittleEndian, DimType::I32).unwrap(), 5.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[6u8, 0, 0, 0]), PlyFormat::BinaryLittleEndian, DimType::U32).unwrap(), 6.0);
        
        // Big Endian
        assert_eq!(read_binary_value(&mut Cursor::new(&[1u8]), PlyFormat::BinaryBigEndian, DimType::I8).unwrap(), 1.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[2u8]), PlyFormat::BinaryBigEndian, DimType::U8).unwrap(), 2.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[0, 3u8]), PlyFormat::BinaryBigEndian, DimType::I16).unwrap(), 3.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[0, 4u8]), PlyFormat::BinaryBigEndian, DimType::U16).unwrap(), 4.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[0, 0, 0, 5u8]), PlyFormat::BinaryBigEndian, DimType::I32).unwrap(), 5.0);
        assert_eq!(read_binary_value(&mut Cursor::new(&[0, 0, 0, 6u8]), PlyFormat::BinaryBigEndian, DimType::U32).unwrap(), 6.0);
        
        assert!(read_binary_value(&mut Cursor::new(&[0u8; 8]), PlyFormat::BinaryLittleEndian, DimType::I64).is_err());
    }

    #[test]
    fn test_parse_header_errors() {
        assert!(parse_header("notply\n").is_err());
        assert!(parse_header("ply\nnotformat\n").is_err());
        assert!(parse_header("ply\nformat mysterious 1.0\n").is_err());
        assert!(parse_header("ply\nformat ascii 2.0\n").is_err());
        assert!(parse_header("ply\nformat ascii 1.0\nproperty float x\n").is_err());
        assert!(parse_header("ply\nformat ascii 1.0\nelement vertex 10\nproperty float x\n").is_err());
        assert!(parse_header("ply\nformat ascii 1.0\ninvalid_keyword\n").is_err());
        assert!(parse_header("ply\nformat ascii 1.0\nelement vertex 10\nproperty list uchar int vertex_indices\n").is_err());
    }

    #[test]
    fn test_write_ply_value_all_types() {
        let mut out = Vec::new();
        
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 1.0, DimType::I8, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 2.0, DimType::U8, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 3.0, DimType::I16, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 4.0, DimType::U16, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 5.0, DimType::I32, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 6.0, DimType::U32, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 1.23, DimType::F32, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 4.56, DimType::F64, None).unwrap();
        
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 1.0, DimType::I8, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 2.0, DimType::U8, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 3.0, DimType::I16, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 4.0, DimType::U16, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 5.0, DimType::I32, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 6.0, DimType::U32, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 1.23, DimType::F32, None).unwrap();
        write_ply_value(&mut out, PlyFormat::BinaryBigEndian, 4.56, DimType::F64, None).unwrap();
        
        assert_eq!(out.len(), (1+1+2+2+4+4+4+8) * 2);
        assert!(write_ply_value(&mut out, PlyFormat::BinaryLittleEndian, 1.0, DimType::I64, None).is_err());
    }

    #[test]
    fn test_ply_type_string_and_errors() {
        assert_eq!(ply_type_string(DimType::I8, true), Some("int8"));
        assert_eq!(ply_type_string(DimType::I8, false), Some("char"));
        assert_eq!(ply_type_string(DimType::U8, true), Some("uint8"));
        assert_eq!(ply_type_string(DimType::U8, false), Some("uchar"));
        assert_eq!(ply_type_string(DimType::I16, true), Some("int16"));
        assert_eq!(ply_type_string(DimType::I16, false), Some("short"));
        assert_eq!(ply_type_string(DimType::U16, true), Some("uint16"));
        assert_eq!(ply_type_string(DimType::U16, false), Some("ushort"));
        assert_eq!(ply_type_string(DimType::I32, true), Some("int32"));
        assert_eq!(ply_type_string(DimType::I32, false), Some("int"));
        assert_eq!(ply_type_string(DimType::U32, true), Some("uint32"));
        assert_eq!(ply_type_string(DimType::U32, false), Some("uint"));
        assert_eq!(ply_type_string(DimType::F32, true), Some("float32"));
        assert_eq!(ply_type_string(DimType::F32, false), Some("float"));
        assert_eq!(ply_type_string(DimType::F64, true), Some("float64"));
        assert_eq!(ply_type_string(DimType::F64, false), Some("double"));
        assert_eq!(ply_type_string(DimType::I64, true), None);
    }

    #[test]
    fn test_read_simple_non_simple_error() {
        let prop = PlyProp::List { name: "test".into(), count_ty: DimType::U8, list_ty: DimType::I32 };
        let mut data = PlyData::Ascii("".split_whitespace());
        assert!(data.read_simple(&prop, "test_element").is_err());
    }

    #[test]
    fn test_read_value_missing_token_error() {
        let mut data = PlyData::Ascii("".split_whitespace());
        assert!(data.read_value(DimType::F64, "vertex").is_err());
        
        let mut data_invalid = PlyData::Ascii("abc".split_whitespace());
        assert!(data_invalid.read_value(DimType::F64, "vertex").is_err());
    }
}
