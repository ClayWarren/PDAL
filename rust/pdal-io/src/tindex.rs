use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::PointView;
use pdal_core::stage::StageError;
use std::path::{Path, PathBuf};

/// Tile-index reader for GeoJSON indexes produced by `pdal tindex create`.
pub struct TindexReader {
    filename: String,
    location_field: String,
}

impl TindexReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            location_field: options.get_str("tindex_name", "location"),
        }
    }
}

impl Reader for TindexReader {
    fn name(&self) -> &str {
        "readers.tindex"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "TindexReader requires a filename option.".to_string(),
            ));
        }

        let text = std::fs::read_to_string(&self.filename)
            .map_err(|err| StageError(format!("Can't open file '{}': {err}", self.filename)))?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
            StageError(format!(
                "TindexReader expected a GeoJSON FeatureCollection: {err}"
            ))
        })?;
        let features = json["features"].as_array().ok_or_else(|| {
            StageError("TindexReader expected a GeoJSON FeatureCollection.".to_string())
        })?;

        let mut merged: Option<PointView> = None;
        let base = Path::new(&self.filename).parent().unwrap_or(Path::new(""));
        for feature in features {
            let location = feature["properties"][self.location_field.as_str()]
                .as_str()
                .ok_or_else(|| {
                    StageError(format!(
                        "TindexReader feature is missing '{}'.",
                        self.location_field
                    ))
                })?;
            let path = resolve_location(base, location);
            let mut views = read_point_file(&path)?;
            for view in views.drain(..) {
                append_view(&mut merged, &view, &path)?;
            }
        }

        match merged {
            Some(view) => Ok(vec![view]),
            None => Ok(Vec::new()),
        }
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.tindex")
    }
}

pub(crate) fn resolve_location(base: &Path, location: &str) -> PathBuf {
    let path = Path::new(location);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

pub(crate) fn read_point_file(path: &Path) -> Result<Vec<PointView>, StageError> {
    let filename = path.to_string_lossy();
    let driver = pdal_core::driver::infer_reader_driver(&filename).ok_or_else(|| {
        StageError(format!(
            "TindexReader cannot infer a reader driver for '{}'.",
            path.display()
        ))
    })?;
    let mut options = Options::new();
    options.add("filename", filename.as_ref());
    match driver {
        "readers.bpf" => crate::bpf::BpfReader::new(&options).read(),
        "readers.fbi" => crate::fbi::FbiReader::new(&options).read(),
        "readers.gdal" => crate::gdal_reader::GdalReader::new(&options).read(),
        "readers.text" => crate::text::TextReader::new(&options).read(),
        "readers.pcd" => crate::pcd::PcdReader::new(&options).read(),
        "readers.pts" => crate::pts::PtsReader::new(&options).read(),
        "readers.ptx" => crate::ptx::PtxReader::new(&options).read(),
        "readers.ilvis2" => crate::ilvis2::Ilvis2Reader::new(&options).read(),
        "readers.obj" => crate::obj::ObjReader::new(&options).read(),
        "readers.optech" => crate::optech::OptechReader::new(&options).read(),
        "readers.qfit" => crate::qfit::QfitReader::new(&options).read(),
        "readers.sbet" => crate::sbet::SbetReader::new(&options).read(),
        "readers.smrmsg" => crate::smrmsg::SmrmsgReader::new(&options).read(),
        "readers.terrasolid" => crate::terrasolid::TerrasolidReader::new(&options).read(),
        "readers.copc" | "readers.las" | "readers.laz" => {
            crate::las::LasReader::new(&options).read()
        }
        "readers.ept" => crate::ept::EptReader::new(&options).read(),
        "readers.ply" => crate::ply::PlyReader::new(&options).read(),
        _ => Err(StageError(format!(
            "TindexReader driver '{driver}' is not available in the Rust port."
        ))),
    }
}

pub(crate) fn append_view(
    merged: &mut Option<PointView>,
    view: &PointView,
    path: &Path,
) -> Result<(), StageError> {
    if merged.is_none() {
        *merged = Some(view.make_new());
    }
    let target = merged.as_mut().unwrap();
    if target.layout().dim_count() != view.layout().dim_count()
        || target.layout().point_size() != view.layout().point_size()
    {
        return Err(StageError(format!(
            "'{}' has a layout incompatible with the tile index.",
            path.display()
        )));
    }
    for idx in 0..target.layout().dim_count() {
        if target.layout().dim_at(idx) != view.layout().dim_at(idx) {
            return Err(StageError(format!(
                "'{}' has a layout incompatible with the tile index.",
                path.display()
            )));
        }
    }
    for idx in 0..view.len() {
        target.append_point(view, idx);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::DimId;

    #[test]
    fn reads_geojson_index_and_merges_referenced_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.geojson");
        std::fs::write(
            &index,
            r#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"location":"simple_text.ply"},"geometry":null},
    {"type":"Feature","properties":{"location":"simple_text.ply"},"geometry":null}
  ]
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", index.display());
        let mut reader = TindexReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 6);
        assert_eq!(views[0].get_f64(0, &DimId::X), -1.0);
        assert_eq!(views[0].get_f64(3, &DimId::X), -1.0);
    }

    #[test]
    fn honors_custom_location_field() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.geojson");
        std::fs::write(
            &index,
            r#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"source_file":"simple_text.ply"},"geometry":null}
  ]
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", index.display());
        options.add("tindex_name", "source_file");
        let mut reader = TindexReader::new(&options);

        assert_eq!(reader.read().unwrap()[0].len(), 3);
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = TindexReader::new(&Options::new());
        let err = reader.read().err().expect("missing filename");
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/tindex.json");
        let mut reader = TindexReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_invalid_json() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"{not json").unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = TindexReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_non_feature_collection() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"{\"type\":\"Other\"}").unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = TindexReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        let reader = TindexReader::new(&Options::new());
        assert_eq!(reader.metadata().name(), "readers.tindex");
    }

    #[test]
    fn resolve_location_absolute_path_preserved() {
        let base = Path::new("/base");
        let resolved = resolve_location(base, "/absolute/file.las");
        assert_eq!(resolved, Path::new("/absolute/file.las"));
    }

    #[test]
    fn resolve_location_relative_path_uses_base() {
        let base = Path::new("/base");
        let resolved = resolve_location(base, "child.las");
        assert_eq!(resolved, Path::new("/base/child.las"));
    }
}
