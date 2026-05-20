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
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use serde_json::Value;
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;
use std::rc::Rc;

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
        if data_type != "laszip" && data_type != "binary" {
            return Err(StageError(format!(
                "EptReader Rust slice supports only laszip and binary dataType, not '{data_type}'."
            )));
        }

        self.metadata = metadata_from_info(&info);
        let keys = hierarchy_keys(root)?;
        let mut merged: Option<PointView> = None;
        let mut point_count = 0;
        let schema = EptSchema::parse(&info)?;
        let srs = info["srs"]["wkt"].as_str().unwrap_or("");
        for key in keys {
            let path = root.join("ept-data").join(format!(
                "{key}.{}",
                if data_type == "laszip" { "laz" } else { "bin" }
            ));
            let views = if data_type == "laszip" {
                let mut options = Options::new();
                options.add("filename", path.display());
                crate::las::LasReader::new(&options).read()?
            } else {
                vec![read_binary_tile(&path, &schema, srs)?]
            };
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

#[derive(Clone)]
struct EptSchema {
    entries: Vec<SchemaEntry>,
    point_size: usize,
    layout: Rc<PointLayout>,
}

#[derive(Clone)]
struct SchemaEntry {
    dim: DimId,
    ty: DimType,
    size: usize,
    scale: f64,
    offset: f64,
}

impl EptSchema {
    fn parse(info: &Value) -> Result<Self, StageError> {
        let schema = info["schema"]
            .as_array()
            .ok_or_else(|| StageError("EPT file is missing schema.".to_string()))?;
        let mut entries = Vec::with_capacity(schema.len());
        let mut layout = PointLayout::new();
        let mut point_size = 0;
        for item in schema {
            let name = item["name"]
                .as_str()
                .ok_or_else(|| StageError("EPT schema entry is missing name.".to_string()))?;
            let kind = item["type"]
                .as_str()
                .ok_or_else(|| StageError(format!("EPT schema '{name}' is missing type.")))?;
            let size = item["size"]
                .as_u64()
                .ok_or_else(|| StageError(format!("EPT schema '{name}' is missing size.")))?
                as usize;
            let storage_ty = dim_type(kind, size)?;
            let scale = item["scale"].as_f64().unwrap_or(1.0);
            let offset = item["offset"].as_f64().unwrap_or(0.0);
            let ty = if scale != 1.0 || offset != 0.0 {
                DimType::F64
            } else {
                storage_ty
            };
            let dim = DimId::from_name(name);
            layout.register(dim.clone(), ty);
            entries.push(SchemaEntry {
                dim,
                ty: storage_ty,
                size,
                scale,
                offset,
            });
            point_size += size;
        }
        Ok(Self {
            entries,
            point_size,
            layout: Rc::new(layout),
        })
    }
}

fn dim_type(kind: &str, size: usize) -> Result<DimType, StageError> {
    match (kind, size) {
        ("unsigned", 1) => Ok(DimType::U8),
        ("unsigned", 2) => Ok(DimType::U16),
        ("unsigned", 4) => Ok(DimType::U32),
        ("unsigned", 8) => Ok(DimType::U64),
        ("signed", 1) => Ok(DimType::I8),
        ("signed", 2) => Ok(DimType::I16),
        ("signed", 4) => Ok(DimType::I32),
        ("signed", 8) => Ok(DimType::I64),
        ("float", 4) => Ok(DimType::F32),
        ("float", 8) => Ok(DimType::F64),
        _ => Err(StageError(format!(
            "Unsupported EPT schema type '{kind}' with size {size}."
        ))),
    }
}

fn read_binary_tile(path: &Path, schema: &EptSchema, srs: &str) -> Result<PointView, StageError> {
    let bytes = std::fs::read(path)
        .map_err(|err| StageError(format!("Can't open EPT tile '{}': {err}", path.display())))?;
    if schema.point_size == 0 || bytes.len() % schema.point_size != 0 {
        return Err(StageError(format!(
            "EPT tile '{}' size does not match schema.",
            path.display()
        )));
    }

    let mut view = PointView::new(Rc::clone(&schema.layout));
    if !srs.is_empty() {
        view.set_spatial_reference(SpatialReference::new(srs));
    }
    for record in bytes.chunks_exact(schema.point_size) {
        let point = view.add_point();
        let mut offset = 0;
        for entry in &schema.entries {
            let raw = read_binary_value(&record[offset..offset + entry.size], entry.ty);
            view.set_f64(point, &entry.dim, raw * entry.scale + entry.offset);
            offset += entry.size;
        }
    }
    Ok(view)
}

fn read_binary_value(bytes: &[u8], ty: DimType) -> f64 {
    match ty {
        DimType::U8 => f64::from(bytes[0]),
        DimType::I8 => f64::from(bytes[0] as i8),
        DimType::U16 => f64::from(u16::from_le_bytes(bytes.try_into().unwrap())),
        DimType::I16 => f64::from(i16::from_le_bytes(bytes.try_into().unwrap())),
        DimType::U32 => u32::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::I32 => i32::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::U64 => u64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::I64 => i64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::F32 => f64::from(f32::from_le_bytes(bytes.try_into().unwrap())),
        DimType::F64 => f64::from_le_bytes(bytes.try_into().unwrap()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn reads_local_binary_ept() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/ellipsoid-binary/ept.json").display(),
        );
        let mut reader = EptReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 100000);
        let origin_id = DimId::from_name("OriginId");
        assert!(views[0].layout().dim(&origin_id).is_some());
        for idx in [0, 42, 99999] {
            let x = views[0].get_f64(idx, &DimId::X);
            let y = views[0].get_f64(idx, &DimId::Y);
            let z = views[0].get_f64(idx, &DimId::Z);
            assert!((-8242746.0..=-8242446.0).contains(&x));
            assert!((4966506.0..=4966706.0).contains(&y));
            assert!((-50.0..=50.0).contains(&z));
            assert_eq!(views[0].get_f64(idx, &origin_id), 0.0);
        }
    }

    #[test]
    fn rejects_unsupported_ept_data_type() {
        let temp = tempfile::tempdir().unwrap();
        let ept = temp.path().join("ept.json");
        std::fs::write(
            &ept,
            r#"{"dataType":"zstandard","hierarchyType":"json","schema":[]}"#,
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", ept.display());
        let mut reader = EptReader::new(&options);

        assert!(reader.read().is_err());
    }
}
