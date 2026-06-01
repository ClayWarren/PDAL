#[cfg(test)]
mod stream_tests {
    use super::*;
    use pdal_core::options::Options;
    use pdal_core::pipeline::Reader;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    #[test]
    fn streaming_binary_reader_chunks_match_full_read() {
        let mut options = Options::new();
        options.add("filename", data_path("ply/simple_binary.ply"));

        let mut full_reader = PlyReader::new(&options);
        let full = full_reader.read().unwrap().pop().unwrap();

        let mut stream_reader = PlyReader::new(&options);
        assert!(stream_reader.streamable());
        let first = stream_reader.stream_next(2).unwrap().unwrap();
        let second = stream_reader.stream_next(2).unwrap().unwrap();
        assert!(stream_reader.stream_next(2).unwrap().is_none());

        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 1);
        assert_eq!(first.get_f64(0, &DimId::X), full.get_f64(0, &DimId::X));
        assert_eq!(first.get_f64(1, &DimId::Y), full.get_f64(1, &DimId::Y));
        assert_eq!(second.get_f64(0, &DimId::Z), full.get_f64(2, &DimId::Z));
    }

    #[test]
    fn streaming_ascii_reader_chunks_match_full_read() {
        let mut options = Options::new();
        options.add("filename", data_path("ply/text_extradim.ply"));

        let mut full_reader = PlyReader::new(&options);
        let full = full_reader.read().unwrap().pop().unwrap();

        let mut stream_reader = PlyReader::new(&options);
        assert!(stream_reader.streamable());
        let chunk = stream_reader.stream_next(4).unwrap().unwrap();
        assert!(stream_reader.stream_next(4).unwrap().is_none());

        assert_eq!(chunk.len(), full.len());
        for dim in [
            DimId::X,
            DimId::Y,
            DimId::Z,
            DimId::from_name("red"),
            DimId::from_name("green"),
            DimId::from_name("blue"),
            DimId::from_name("alpha"),
            DimId::from_name("omg"),
        ] {
            assert_eq!(chunk.get_f64(0, &dim), full.get_f64(0, &dim));
        }
    }
}
