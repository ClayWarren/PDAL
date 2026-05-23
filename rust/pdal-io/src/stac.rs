//! `readers.stac` -- local STAC asset reader.
//!
//! This is a narrow local-file slice: STAC Item assets and local
//! Catalog/Collection/FeatureCollection traversal. Remote assets, schema
//! validation, EPT/COPC-specific behavior, and STAC filtering stay with the
//! later vendor/remote I/O milestone.

use crate::tindex::{append_view, read_point_file, resolve_location};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::PointView;
use pdal_core::stage::StageError;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub struct StacReader {
    filename: String,
    asset_names: Vec<String>,
}

impl StacReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            asset_names: asset_names(options),
        }
    }
}

impl Reader for StacReader {
    fn name(&self) -> &str {
        "readers.stac"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "StacReader requires a filename option.".to_string(),
            ));
        }

        let mut visited = BTreeSet::new();
        let mut assets = Vec::new();
        collect_assets(
            Path::new(&self.filename),
            &self.asset_names,
            &mut visited,
            &mut assets,
        )?;

        let mut merged: Option<PointView> = None;
        for asset in assets {
            let views = read_point_file(&asset)?;
            for view in views {
                append_view(&mut merged, &view, &asset)?;
            }
        }

        Ok(merged.into_iter().collect())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.stac")
    }
}

fn asset_names(options: &Options) -> Vec<String> {
    let values = options.values("asset_names");
    if values.is_empty() {
        return vec!["data".to_string()];
    }
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn collect_assets(
    path: &Path,
    asset_names: &[String],
    visited: &mut BTreeSet<PathBuf>,
    assets: &mut Vec<PathBuf>,
) -> Result<(), StageError> {
    let path = canonical_or_original(path);
    if !visited.insert(path.clone()) {
        return Ok(());
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|err| StageError(format!("Can't open STAC file '{}': {err}", path.display())))?;
    let json: Value = serde_json::from_str(&text).map_err(|err| {
        StageError(format!(
            "StacReader expected a STAC JSON object in '{}': {err}",
            path.display()
        ))
    })?;
    let base = path.parent().unwrap_or(Path::new(""));
    match json["type"].as_str() {
        Some("Feature") => collect_item_assets(&json, base, asset_names, assets),
        Some("Catalog") | Some("Collection") => {
            collect_linked_items(&json, base, asset_names, visited, assets)
        }
        Some("FeatureCollection") => {
            let features = json["features"].as_array().ok_or_else(|| {
                StageError(format!(
                    "STAC FeatureCollection '{}' is missing features.",
                    path.display()
                ))
            })?;
            for feature in features {
                collect_item_assets(feature, base, asset_names, assets)?;
            }
            collect_linked_items(&json, base, asset_names, visited, assets)
        }
        Some(other) => Err(StageError(format!(
            "Unsupported STAC object type '{other}' in '{}'.",
            path.display()
        ))),
        None => Err(StageError(format!(
            "STAC file '{}' is missing a type field.",
            path.display()
        ))),
    }
}

fn collect_item_assets(
    item: &Value,
    base: &Path,
    asset_names: &[String],
    assets: &mut Vec<PathBuf>,
) -> Result<(), StageError> {
    let map = item["assets"]
        .as_object()
        .ok_or_else(|| StageError("STAC Item is missing assets.".to_string()))?;
    for name in asset_names {
        let Some(asset) = map.get(name) else {
            continue;
        };
        let href = asset["href"]
            .as_str()
            .ok_or_else(|| StageError(format!("STAC asset '{name}' is missing an href.")))?;
        if is_remote(href) {
            return Err(StageError(format!(
                "STAC asset '{name}' is remote; remote STAC I/O is not in this Rust slice."
            )));
        }
        assets.push(resolve_location(base, href));
    }
    Ok(())
}

fn collect_linked_items(
    json: &Value,
    base: &Path,
    asset_names: &[String],
    visited: &mut BTreeSet<PathBuf>,
    assets: &mut Vec<PathBuf>,
) -> Result<(), StageError> {
    let Some(links) = json["links"].as_array() else {
        return Ok(());
    };
    for link in links {
        let rel = link["rel"].as_str().unwrap_or("");
        if !matches!(rel, "item" | "child" | "next") {
            continue;
        }
        let Some(href) = link["href"].as_str() else {
            continue;
        };
        if is_remote(href) {
            continue;
        }
        collect_assets(&resolve_location(base, href), asset_names, visited, assets)?;
    }
    Ok(())
}

fn canonical_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn is_remote(value: &str) -> bool {
    value.contains("://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::DimId;

    #[test]
    fn reads_local_item_asset() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
        let item = temp.path().join("item.json");
        std::fs::write(
            &item,
            r#"{
  "type": "Feature",
  "assets": {
    "data": {"href": "simple_text.ply", "type": "application/octet-stream"}
  }
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", item.display());
        let mut reader = StacReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 3);
        assert_eq!(views[0].get_f64(0, &DimId::X), -1.0);
    }

    #[test]
    fn follows_local_collection_item_links() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
        std::fs::write(
            temp.path().join("item.json"),
            r#"{
  "type": "Feature",
  "assets": {"data": {"href": "simple_text.ply"}}
}"#,
        )
        .unwrap();
        let collection = temp.path().join("collection.json");
        std::fs::write(
            &collection,
            r#"{
  "type": "Collection",
  "links": [{"rel": "item", "href": "item.json"}]
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", collection.display());
        let mut reader = StacReader::new(&options);

        assert_eq!(reader.read().unwrap()[0].len(), 3);
    }

    #[test]
    fn honors_custom_asset_names() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
        let item = temp.path().join("item.json");
        std::fs::write(
            &item,
            r#"{
  "type": "Feature",
  "assets": {"pointcloud": {"href": "simple_text.ply"}}
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", item.display());
        options.add("asset_names", "pointcloud");
        let mut reader = StacReader::new(&options);

        assert_eq!(reader.read().unwrap()[0].len(), 3);
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = StacReader::new(&Options::new());
        let err = reader.read().err().expect("missing filename");
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/stac.json");
        let mut reader = StacReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_invalid_json() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"{not-json").unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = StacReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        let reader = StacReader::new(&Options::new());
        assert_eq!(reader.metadata().name(), "readers.stac");
    }

    #[test]
    fn asset_names_defaults_to_data() {
        let names = asset_names(&Options::new());
        assert_eq!(names, vec!["data".to_string()]);
    }

    #[test]
    fn asset_names_splits_comma_separated_and_trims() {
        let mut options = Options::new();
        options.add("asset_names", "foo, bar,baz,");
        let names = asset_names(&options);
        assert_eq!(names, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn reader_errors_on_unknown_type() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), br#"{"type":"Weird"}"#).unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = StacReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("Unsupported"));
    }

    #[test]
    fn reader_errors_on_missing_type() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), br#"{}"#).unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = StacReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("type"));
    }

    #[test]
    fn reader_errors_on_remote_asset() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            br#"{"type":"Feature","assets":{"data":{"href":"http://example.com/x.las"}}}"#,
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = StacReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("remote"));
    }

    #[test]
    fn reader_errors_on_missing_assets() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), br#"{"type":"Feature"}"#).unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = StacReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("assets"));
    }

    #[test]
    fn reader_errors_on_asset_missing_href() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), br#"{"type":"Feature","assets":{"data":{}}}"#).unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = StacReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("href"));
    }

    #[test]
    fn reader_errors_on_feature_collection_missing_features() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), br#"{"type":"FeatureCollection"}"#).unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = StacReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("features"));
    }

    #[test]
    fn reader_handles_empty_feature_collection_with_links() {
        let temp = tempfile::tempdir().unwrap();
        let coll = temp.path().join("fc.json");
        std::fs::write(&coll, br#"{"type":"FeatureCollection","features":[]}"#).unwrap();
        let mut options = Options::new();
        options.add("filename", coll.display());
        let mut reader = StacReader::new(&options);
        // FeatureCollection with empty features -> Ok with no views
        let views = reader.read().unwrap();
        assert!(views.is_empty());
    }

    #[test]
    fn is_remote_detects_url_schemes() {
        assert!(is_remote("http://example.com/x"));
        assert!(is_remote("https://example.com/x"));
        assert!(!is_remote("/local/path.las"));
    }
}
