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
}
