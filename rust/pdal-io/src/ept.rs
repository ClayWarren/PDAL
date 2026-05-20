//! `readers.ept` -- local LASzip EPT full-read slice.
//!
//! This handles local `ept.json` datasets whose `dataType` is `laszip`,
//! `binary`, or `zstandard` by walking JSON hierarchy files and merging local
//! tiles. Bounds queries prune hierarchy nodes before tile reads and are also
//! applied to individual points. Reprojection, polygon/OGR filters, addons,
//! remote access, and streaming are deferred.

use crate::tindex::append_view;
use pdal_core::bounds::{parse_bounds2d, parse_bounds3d, Bounds2D, Bounds3D};
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointId, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use serde_json::Value;
use std::collections::{BTreeSet, VecDeque};
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;

pub struct EptReader {
    filename: String,
    bounds: String,
    origin: String,
    resolution: String,
    ignore_unreadable: bool,
    metadata: MetadataNode,
}

impl EptReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            bounds: options.get_str("bounds", ""),
            origin: options.get_str("origin", ""),
            resolution: options.get_str("resolution", ""),
            ignore_unreadable: options.get_bool("ignore_unreadable", false),
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
        if data_type != "laszip" && data_type != "binary" && data_type != "zstandard" {
            return Err(StageError(format!(
                "EptReader Rust slice supports only laszip, binary, and zstandard dataType, not '{data_type}'."
            )));
        }

        self.metadata = metadata_from_info(&info);
        let max_depth = self.resolution_filter(&info)?;
        let bounds = self.bounds_filter()?;
        let root_bounds = ept_bounds(&info)?;
        let tiles = hierarchy_tiles(root, max_depth, bounds.as_ref(), root_bounds)?;
        let mut merged: Option<PointView> = None;
        let mut point_count = 0;
        let schema = EptSchema::parse(&info)?;
        let srs = info["srs"]["wkt"].as_str().unwrap_or("");
        let origin = self.origin_filter(root)?;
        let tile_count = tiles.len();
        for tile in tiles {
            let extension = match data_type {
                "laszip" => "laz",
                "binary" => "bin",
                "zstandard" => "zst",
                _ => unreachable!(),
            };
            let path = root
                .join("ept-data")
                .join(format!("{}.{extension}", tile.key));
            let views = if data_type == "laszip" {
                let mut options = Options::new();
                options.add("filename", path.display());
                crate::las::LasReader::new(&options).read()
            } else if data_type == "zstandard" {
                read_zstandard_tile(&path, &schema, srs).map(|view| vec![view])
            } else {
                read_binary_tile(&path, &schema, srs).map(|view| vec![view])
            };
            let views = match views {
                Ok(views) => views,
                Err(err) if self.ignore_unreadable => {
                    self.metadata.add_value(
                        "warning",
                        MetadataValue::String(format!(
                            "Ignored unreadable EPT tile '{}': {}",
                            path.display(),
                            err.0
                        )),
                    );
                    continue;
                }
                Err(err) => return Err(err),
            };
            validate_tile_count(&path, &views, tile.expected_points)?;
            for view in views {
                let view = apply_origin(apply_bounds(view, bounds.as_ref()), origin);
                point_count += view.len();
                append_view(&mut merged, &view, &path)?;
            }
        }
        self.metadata
            .add_value("count", MetadataValue::U64(point_count));
        self.metadata
            .add_value("tiles", MetadataValue::U64(tile_count as u64));

        Ok(merged.into_iter().collect())
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

impl EptReader {
    fn bounds_filter(&self) -> Result<Option<QueryBounds>, StageError> {
        if self.bounds.is_empty() {
            return Ok(None);
        }
        if let Ok(parsed) = parse_bounds3d(&self.bounds, 0) {
            return Ok(Some(QueryBounds::Three(parsed.bounds)));
        }
        parse_bounds2d(&self.bounds, 0)
            .map(|parsed| Some(QueryBounds::Two(parsed.bounds)))
            .map_err(|err| StageError(format!("Invalid EPT bounds option: {err}")))
    }

    fn origin_filter(&self, root: &Path) -> Result<Option<u64>, StageError> {
        if self.origin.is_empty() {
            return Ok(None);
        }
        let sources = source_origins(root)?;
        if let Ok(origin) = self.origin.parse::<u64>() {
            if !sources.is_empty() && !sources.iter().any(|source| source.id == origin) {
                return Err(StageError(format!("Invalid EPT origin '{}'.", self.origin)));
            }
            return Ok(Some(origin));
        }
        sources
            .iter()
            .find(|source| source.matches(&self.origin))
            .map(|source| Some(source.id))
            .ok_or_else(|| StageError(format!("Invalid EPT origin '{}'.", self.origin)))
    }

    fn resolution_filter(&self, info: &Value) -> Result<Option<u64>, StageError> {
        if self.resolution.is_empty() {
            return Ok(None);
        }
        let resolution = self
            .resolution
            .parse::<f64>()
            .map_err(|_| StageError("EPT resolution option must be numeric.".to_string()))?;
        if resolution <= 0.0 {
            return Err(StageError(
                "EPT resolution option must be positive.".to_string(),
            ));
        }
        let span = info["span"]
            .as_f64()
            .ok_or_else(|| StageError("EPT file is missing span.".to_string()))?;
        if span <= 0.0 {
            return Err(StageError("EPT span must be positive.".to_string()));
        }
        let Bounds3D {
            minx: min_x,
            miny: min_y,
            minz: min_z,
            maxx: max_x,
            maxy: max_y,
            maxz: max_z,
        } = ept_bounds(info)?;
        let cube_width = (max_x - min_x).max(max_y - min_y).max(max_z - min_z) / span;
        if cube_width <= 0.0 {
            return Err(StageError("EPT bounds cube width is invalid.".to_string()));
        }
        let depth = (cube_width / resolution).log2().ceil().max(0.0) as u64;
        Ok(Some(depth))
    }
}

struct EptTile {
    key: String,
    expected_points: u64,
}

struct SourceOrigin {
    id: u64,
    names: Vec<String>,
}

impl SourceOrigin {
    fn matches(&self, query: &str) -> bool {
        self.names.iter().any(|name| name == query)
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

fn hierarchy_tiles(
    root: &Path,
    max_depth: Option<u64>,
    query: Option<&QueryBounds>,
    root_bounds: Bounds3D,
) -> Result<Vec<EptTile>, StageError> {
    let mut tiles = Vec::new();
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
            let key = EptKey::parse(node, root_bounds)?;
            let depth = key.depth;
            if query.is_some_and(|query| !query.overlaps_box(&key.bounds)) {
                continue;
            }
            match count.as_i64() {
                Some(points) if points > 0 && max_depth.is_none_or(|max| depth <= max) => tiles
                    .push(EptTile {
                        key: node.clone(),
                        expected_points: points as u64,
                    }),
                Some(-1) if max_depth.is_none_or(|max| depth <= max) => {
                    queue.push_back(node.clone())
                }
                _ => {}
            }
        }
    }
    Ok(tiles)
}

struct EptKey {
    depth: u64,
    bounds: Bounds3D,
}

impl EptKey {
    fn parse(key: &str, root_bounds: Bounds3D) -> Result<Self, StageError> {
        let mut parts = key.split('-');
        let depth = parse_key_part(parts.next(), key)?;
        let x = parse_key_part(parts.next(), key)?;
        let y = parse_key_part(parts.next(), key)?;
        let z = parse_key_part(parts.next(), key)?;
        if parts.next().is_some() {
            return Err(StageError(format!("Invalid EPT hierarchy key '{key}'.")));
        }

        let shift = u32::try_from(depth)
            .map_err(|_| StageError(format!("EPT hierarchy key '{key}' depth is too large.")))?;
        let divisions = 1_u64
            .checked_shl(shift)
            .ok_or_else(|| StageError(format!("EPT hierarchy key '{key}' depth is too large.")))?
            as f64;
        let bounds = Bounds3D {
            minx: tile_min(root_bounds.minx, root_bounds.maxx, divisions, x),
            maxx: tile_max(root_bounds.minx, root_bounds.maxx, divisions, x),
            miny: tile_min(root_bounds.miny, root_bounds.maxy, divisions, y),
            maxy: tile_max(root_bounds.miny, root_bounds.maxy, divisions, y),
            minz: tile_min(root_bounds.minz, root_bounds.maxz, divisions, z),
            maxz: tile_max(root_bounds.minz, root_bounds.maxz, divisions, z),
        };
        Ok(Self { depth, bounds })
    }
}

fn parse_key_part(part: Option<&str>, key: &str) -> Result<u64, StageError> {
    part.ok_or_else(|| StageError(format!("Invalid EPT hierarchy key '{key}'.")))?
        .parse()
        .map_err(|_| StageError(format!("Invalid EPT hierarchy key '{key}'.")))
}

fn tile_min(min: f64, max: f64, divisions: f64, index: u64) -> f64 {
    min + ((max - min) / divisions) * index as f64
}

fn tile_max(min: f64, max: f64, divisions: f64, index: u64) -> f64 {
    min + ((max - min) / divisions) * (index + 1) as f64
}

fn ept_bounds(info: &Value) -> Result<Bounds3D, StageError> {
    let bounds = info["bounds"]
        .as_array()
        .ok_or_else(|| StageError("EPT file is missing bounds.".to_string()))?;
    if bounds.len() < 6 {
        return Err(StageError(
            "EPT bounds must contain six coordinates.".to_string(),
        ));
    }
    Ok(Bounds3D {
        minx: ept_bound_value(bounds, 0, "min X")?,
        miny: ept_bound_value(bounds, 1, "min Y")?,
        minz: ept_bound_value(bounds, 2, "min Z")?,
        maxx: ept_bound_value(bounds, 3, "max X")?,
        maxy: ept_bound_value(bounds, 4, "max Y")?,
        maxz: ept_bound_value(bounds, 5, "max Z")?,
    })
}

fn ept_bound_value(bounds: &[Value], index: usize, name: &str) -> Result<f64, StageError> {
    bounds[index]
        .as_f64()
        .ok_or_else(|| StageError(format!("EPT bounds {name} is not numeric.")))
}

fn validate_tile_count(
    path: &Path,
    views: &[PointView],
    expected_points: u64,
) -> Result<(), StageError> {
    let actual_points = views.iter().map(PointView::len).sum::<u64>();
    if actual_points != expected_points {
        return Err(StageError(format!(
            "EPT tile '{}' has {actual_points} points but hierarchy expected {expected_points}.",
            path.display()
        )));
    }
    Ok(())
}

fn source_origins(root: &Path) -> Result<Vec<SourceOrigin>, StageError> {
    let sources = root.join("ept-sources");
    let manifest = sources.join("manifest.json");
    if manifest.exists() {
        return source_origins_from_array(&read_json(&manifest)?);
    }
    let list = sources.join("list.json");
    if list.exists() {
        return source_origins_from_array(&read_json(&list)?);
    }
    Ok(Vec::new())
}

fn source_origins_from_array(value: &Value) -> Result<Vec<SourceOrigin>, StageError> {
    let array = value
        .as_array()
        .ok_or_else(|| StageError("EPT source list must be a JSON array.".to_string()))?;
    Ok(array
        .iter()
        .enumerate()
        .map(|(idx, item)| SourceOrigin {
            id: item["origin"].as_u64().unwrap_or(idx as u64),
            names: source_origin_names(item),
        })
        .collect())
}

fn source_origin_names(item: &Value) -> Vec<String> {
    let mut names = Vec::new();
    for field in ["id", "metadataPath", "path"] {
        if let Some(value) = item[field].as_str() {
            names.push(value.to_string());
            if let Some(stem) = Path::new(value).file_stem().and_then(|stem| stem.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names
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

enum QueryBounds {
    Two(Bounds2D),
    Three(Bounds3D),
}

impl QueryBounds {
    fn contains(&self, view: &PointView, idx: PointId) -> bool {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        match self {
            QueryBounds::Two(bounds) => bounds.contains_point(x, y),
            QueryBounds::Three(bounds) => bounds.contains_point(x, y, view.get_f64(idx, &DimId::Z)),
        }
    }

    fn overlaps_box(&self, bounds: &Bounds3D) -> bool {
        match self {
            QueryBounds::Two(query) => query.overlaps(&Bounds2D {
                minx: bounds.minx,
                maxx: bounds.maxx,
                miny: bounds.miny,
                maxy: bounds.maxy,
            }),
            QueryBounds::Three(query) => query.overlaps(bounds),
        }
    }
}

fn apply_bounds(view: PointView, bounds: Option<&QueryBounds>) -> PointView {
    let Some(bounds) = bounds else {
        return view;
    };
    let mut output = view.make_new();
    for idx in 0..view.len() {
        if bounds.contains(&view, idx) {
            output.append_point(&view, idx);
        }
    }
    output
}

fn apply_origin(view: PointView, origin: Option<u64>) -> PointView {
    let Some(origin) = origin else {
        return view;
    };
    let origin_dim = DimId::from_name("OriginId");
    let mut output = view.make_new();
    for idx in 0..view.len() {
        if view.get_f64(idx, &origin_dim) as u64 == origin {
            output.append_point(&view, idx);
        }
    }
    output
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
    view_from_binary_tile(path, bytes, schema, srs)
}

fn read_zstandard_tile(
    path: &Path,
    schema: &EptSchema,
    srs: &str,
) -> Result<PointView, StageError> {
    let bytes = std::fs::read(path)
        .map_err(|err| StageError(format!("Can't open EPT tile '{}': {err}", path.display())))?;
    let decoded = zstd::stream::decode_all(Cursor::new(bytes)).map_err(|err| {
        StageError(format!(
            "Can't decompress EPT tile '{}': {err}",
            path.display()
        ))
    })?;
    view_from_binary_tile(path, decoded, schema, srs)
}

fn view_from_binary_tile(
    path: &Path,
    bytes: Vec<u8>,
    schema: &EptSchema,
    srs: &str,
) -> Result<PointView, StageError> {
    if schema.point_size == 0 || !bytes.len().is_multiple_of(schema.point_size) {
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
    fn reads_local_zstandard_ept() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/ellipsoid-zstandard/ept.json").display(),
        );
        let mut reader = EptReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 100000);
        assert!((views[0].get_f64(42, &DimId::X) + 8242697.94).abs() < 1e-9);
    }

    #[test]
    fn applies_3d_bounds_filter() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/ellipsoid-binary/ept.json").display(),
        );
        options.add("bounds", "([-8242746,-8242600],[4966506,4966706],[-50,50])");
        let mut reader = EptReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert!(!views[0].is_empty());
        assert!(views[0].len() < 100000);
        for idx in 0..views[0].len() {
            let x = views[0].get_f64(idx, &DimId::X);
            let y = views[0].get_f64(idx, &DimId::Y);
            let z = views[0].get_f64(idx, &DimId::Z);
            assert!((-8242746.0..=-8242600.0).contains(&x));
            assert!((4966506.0..=4966706.0).contains(&y));
            assert!((-50.0..=50.0).contains(&z));
        }
    }

    #[test]
    fn applies_resolution_limit_to_hierarchy_depth() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/lone-star-laszip/ept.json").display(),
        );
        options.add("resolution", "0.1");
        let mut reader = EptReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 479269);
    }

    #[test]
    fn rejects_non_positive_resolution() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/lone-star-laszip/ept.json").display(),
        );
        options.add("resolution", "0");
        let mut reader = EptReader::new(&options);

        let Err(err) = reader.read() else {
            panic!("expected bad EPT resolution to fail");
        };
        assert!(err.0.contains("resolution option must be positive"));
    }

    #[test]
    fn rejects_bad_laszip_point_count() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/bad-pointcount/laszip/ept.json").display(),
        );
        let mut reader = EptReader::new(&options);

        let Err(err) = reader.read() else {
            panic!("expected bad EPT point count to fail");
        };
        assert!(err.0.contains("hierarchy expected 1000"));
    }

    #[test]
    fn rejects_bad_binary_point_count() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/bad-pointcount/binary/ept.json").display(),
        );
        let mut reader = EptReader::new(&options);

        let Err(err) = reader.read() else {
            panic!("expected bad EPT point count to fail");
        };
        assert!(err.0.contains("hierarchy expected 1000004"));
    }

    #[test]
    fn ignores_unreadable_tile_when_requested() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/ellipsoid-nopoints/ept.json").display(),
        );
        options.add("ignore_unreadable", true);
        let mut reader = EptReader::new(&options);

        let views = reader.read().unwrap();
        assert!(views.is_empty());
    }

    #[test]
    fn filters_named_origin_from_manifest() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/ellipsoid-binary/ept.json").display(),
        );
        options.add("origin", "ellipsoid");
        let mut reader = EptReader::new(&options);

        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 100000);
        let origin_id = DimId::from_name("OriginId");
        assert_eq!(views[0].get_f64(0, &origin_id), 0.0);
        assert_eq!(views[0].get_f64(99999, &origin_id), 0.0);
    }

    #[test]
    fn rejects_unknown_numeric_origin() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("ept/lone-star-laszip/ept.json").display(),
        );
        options.add("origin", "4");
        let mut reader = EptReader::new(&options);

        let Err(err) = reader.read() else {
            panic!("expected bad EPT origin to fail");
        };
        assert!(err.0.contains("Invalid EPT origin '4'"));
    }

    #[test]
    fn prunes_non_overlapping_hierarchy_tiles_before_reading_data() {
        let temp = tempfile::tempdir().unwrap();
        let ept = temp.path().join("ept.json");
        let hierarchy_dir = temp.path().join("ept-hierarchy");
        std::fs::create_dir_all(&hierarchy_dir).unwrap();
        std::fs::write(
            &ept,
            r#"{
  "dataType": "binary",
  "hierarchyType": "json",
  "span": 128,
  "bounds": [0, 0, 0, 8, 8, 8],
  "schema": [
    {"name": "X", "type": "float", "size": 8},
    {"name": "Y", "type": "float", "size": 8},
    {"name": "Z", "type": "float", "size": 8}
  ]
}"#,
        )
        .unwrap();
        std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{"1-0-0-0":1}"#).unwrap();
        let mut options = Options::new();
        options.add("filename", ept.display());
        options.add("bounds", "([6,7],[6,7],[6,7])");
        let mut reader = EptReader::new(&options);

        let views = reader.read().unwrap();

        assert!(views.is_empty());
        let metadata = reader.metadata();
        assert_eq!(
            metadata.find_child("tiles").and_then(MetadataNode::value),
            Some(&MetadataValue::U64(0))
        );
    }

    #[test]
    fn rejects_unsupported_ept_data_type() {
        let temp = tempfile::tempdir().unwrap();
        let ept = temp.path().join("ept.json");
        std::fs::write(
            &ept,
            r#"{"dataType":"unsupported","hierarchyType":"json","schema":[]}"#,
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", ept.display());
        let mut reader = EptReader::new(&options);

        assert!(reader.read().is_err());
    }
}
