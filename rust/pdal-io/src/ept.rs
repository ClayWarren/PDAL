//! `readers.ept` -- local LASzip EPT full-read slice.
//!
//! This handles local `ept.json` datasets whose `dataType` is `laszip` by
//! walking JSON hierarchy files and merging local LAZ tiles. Binary/zstd EPT,
//! spatial filtering, resolution limits, addons, remote access, and streaming
//! are deferred.

use crate::tindex::append_view;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::PointView;
use pdal_core::stage::StageError;
use serde_json::Value;
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;

pub struct EptReader {
    filename: String,
    metadata: MetadataNode,
}

impl EptReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            metadata: MetadataNode::new("readers.ept"),
        }
    }
}

impl Reader for EptReader {
    fn name(&self) -> &str {
        "readers.ept"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "EptReader requires a filename option.".to_string(),
            ));
        }

        let ept_path = Path::new(&self.filename);
        let root = ept_path.parent().unwrap_or(Path::new(""));
        let info = read_json(ept_path)?;
        let data_type = info["dataType"].as_str().ok_or_else(|| {
            StageError(format!(
                "EPT file '{}' is missing dataType.",
                ept_path.display()
            ))
        })?;
        if data_type != "laszip" {
            return Err(StageError(format!(
                "EptReader Rust slice supports only laszip dataType, not '{data_type}'."
            )));
        }

        self.metadata = metadata_from_info(&info);
        let keys = hierarchy_keys(root)?;
        let mut merged: Option<PointView> = None;
        let mut point_count = 0;
        for key in keys {
            let path = root.join("ept-data").join(format!("{key}.laz"));
            let mut options = Options::new();
            options.add("filename", path.display());
            let mut reader = crate::las::LasReader::new(&options);
            let views = reader.read()?;
            for view in views {
                point_count += view.len();
                append_view(&mut merged, &view, &path)?;
            }
        }
        self.metadata
            .add_value("count", MetadataValue::U64(point_count));

        Ok(merged.into_iter().collect())
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

fn read_json(path: &Path) -> Result<Value, StageError> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| StageError(format!("Can't open EPT file '{}': {err}", path.display())))?;
    serde_json::from_str(&text).map_err(|err| {
        StageError(format!(
            "EPT file '{}' is not valid JSON: {err}",
            path.display()
        ))
    })
}

fn hierarchy_keys(root: &Path) -> Result<Vec<String>, StageError> {
    let mut keys = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([String::from("0-0-0-0")]);
    while let Some(key) = queue.pop_front() {
        if !seen.insert(key.clone()) {
            continue;
        }
        let path = root.join("ept-hierarchy").join(format!("{key}.json"));
        let hierarchy = read_json(&path)?;
        let object = hierarchy.as_object().ok_or_else(|| {
            StageError(format!(
                "EPT hierarchy '{}' must be a JSON object.",
                path.display()
            ))
        })?;
        for (node, count) in object {
            match count.as_i64() {
                Some(points) if points > 0 => keys.push(node.clone()),
                Some(-1) => queue.push_back(node.clone()),
                _ => {}
            }
        }
    }
    Ok(keys)
}

fn metadata_from_info(info: &Value) -> MetadataNode {
    let mut node = MetadataNode::new("readers.ept");
    if let Some(data_type) = info["dataType"].as_str() {
        node.add_value("dataType", MetadataValue::String(data_type.to_string()));
    }
    if let Some(hierarchy_type) = info["hierarchyType"].as_str() {
        node.add_value(
            "hierarchyType",
            MetadataValue::String(hierarchy_type.to_string()),
        );
    }
    if let Some(span) = info["span"].as_u64() {
        node.add_value("span", MetadataValue::U64(span));
    }
    if let Some(wkt) = info["srs"]["wkt"].as_str() {
        node.add_value("srs", MetadataValue::String(wkt.to_string()));
    }
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::DimId;
    use std::path::PathBuf;

    fn data_path(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data")
            .join(path)
    }

    #[test]
    fn reads_local_laszip_ept() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/1.2-with-color/ept.json").display(),
        );
        let mut reader = EptReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 1065);
        assert!((views[0].get_f64(0, &DimId::X) - 638806.73).abs() < 1e-9);
        assert!(views[0].layout().dim(&DimId::Red).is_some());
    }

    #[test]
    fn rejects_non_laszip_ept() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/ellipsoid-binary/ept.json").display(),
        );
        let mut reader = EptReader::new(&options);

        assert!(reader.read().is_err());
    }
}
