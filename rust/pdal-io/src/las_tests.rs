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
}
