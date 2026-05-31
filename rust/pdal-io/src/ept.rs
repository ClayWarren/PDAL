//! `readers.ept` -- local LASzip EPT full-read slice.
//!
//! This handles local `ept.json` datasets whose `dataType` is `laszip`,
//! `binary`, or `zstandard` by walking JSON hierarchy files and merging local
//! tiles. Bounds queries prune hierarchy nodes before tile reads and are also
//! applied to individual points. Reprojection, polygon/OGR filters, addons,
//! remote access, and streaming are deferred.

use crate::tindex::append_view;
use pdal_core::bounds::{parse_bounds2d, parse_bounds3d, Bounds2D, Bounds3D};
use pdal_core::geometry::Geometry;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::ogr_spec::parse_ogr_spec_json;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointId, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use pdal_native::srs::{user_input_to_wkt, GdalSrsTransform};
use serde_json::Value;
use std::collections::{BTreeSet, VecDeque};
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;

mod support;
use support::*;

pub struct EptReader {
    filename: String,
    bounds: String,
    origin: String,
    resolution: String,
    polygons: Vec<String>,
    polygon_srs: Vec<String>,
    ogr: String,
    source_srs: String,
    ignore_unreadable: bool,
    addons: String,
    metadata: MetadataNode,
}

impl EptReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            bounds: options.get_str("bounds", ""),
            origin: options.get_str("origin", ""),
            resolution: options.get_str("resolution", ""),
            polygons: option_values(options, "polygon"),
            polygon_srs: option_values(options, "polygon_srs"),
            ogr: options.get_str("ogr", ""),
            source_srs: options.get_str("source_srs", ""),
            ignore_unreadable: options.get_bool("ignore_unreadable", false),
            addons: options.get_str("addons", ""),
            metadata: MetadataNode::new("readers.ept"),
        }
    }

    pub fn validate_origin(&self) -> Result<(), StageError> {
        let ept_path = Path::new(&self.filename);
        let root = ept_path.parent().unwrap_or(Path::new(""));
        self.origin_filter(root).map(|_| ())
    }

    pub fn validate_bounds(&self) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "EptReader requires a filename option.".to_string(),
            ));
        }
        let info = read_json_location(&self.filename)?;
        self.bounds_filter(&info).map(|_| ())
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

        let root = location_parent(&self.filename);
        let info = read_json_location(&self.filename)?;
        let data_type = info["dataType"].as_str().ok_or_else(|| {
            StageError(format!("EPT file '{}' is missing dataType.", self.filename))
        })?;
        if data_type != "laszip" && data_type != "binary" && data_type != "zstandard" {
            return Err(StageError(format!(
                "EptReader Rust slice supports only laszip, binary, and zstandard dataType, not '{data_type}'."
            )));
        }

        self.metadata = metadata_from_info(&info);
        let max_depth = self.resolution_filter(&info)?;
        let bounds = self.bounds_filter(&info)?;
        let polygons = self.polygon_filters()?;
        let root_bounds = ept_bounds(&info)?;
        let hierarchy_bounds = bounds
            .as_ref()
            .and_then(|filter| filter.hierarchy_query_bounds());
        let (tiles, hierarchy_step) =
            hierarchy_tiles(&root, max_depth, hierarchy_bounds, root_bounds)?;
        let mut merged: Option<PointView> = None;
        let mut point_count = 0;
        let mut schema = EptSchema::parse(&info)?;
        let addons = EptAddon::parse_specs(&self.addons)?;
        // Register EptNodeId/EptPointId tracking dims. These are set by the C++
        // reader in processPoint() and used by streamTest to sort points by
        // (nodeId, pointId) before comparing dimension values.
        let ept_node_id = DimId::from_name("EptNodeId");
        let ept_point_id = DimId::from_name("EptPointId");
        if let Some(layout) = Rc::get_mut(&mut schema.layout) {
            for addon in &addons {
                layout.register(addon.dim.clone(), addon.ty);
            }
            layout.register(ept_node_id.clone(), DimType::U32);
            layout.register(ept_point_id.clone(), DimType::U32);
        }
        let mut node_id: u32 = 1;
        // (depth, x, y, z, count, node_id) tuples for every tile we
        // materialized. Serialized into metadata so the C++ wrapper can rebuild
        // an `ept::Artifact` for downstream stages like `writers.ept_addon`.
        let mut artifact_tiles: Vec<(u32, u32, u32, u32, u64, u32)> = Vec::new();
        let srs = info["srs"]["wkt"].as_str().unwrap_or("");
        let origin = self.origin_filter(Path::new(&root))?;
        let tile_count = tiles.len();
        for tile in tiles {
            let extension = match data_type {
                "laszip" => "laz",
                "binary" => "bin",
                "zstandard" => "zst",
                _ => unreachable!(),
            };
            let path = join_location(&root, &format!("ept-data/{}.{extension}", tile.key));
            let views = if data_type == "laszip" {
                let mut options = Options::new();
                options.add("filename", path.as_str());
                crate::las::LasReader::new(&options).read()
            } else if data_type == "zstandard" {
                read_zstandard_tile(Path::new(&path), &schema, srs).map(|view| vec![view])
            } else {
                read_binary_tile(Path::new(&path), &schema, srs).map(|view| vec![view])
            };
            let views = match views {
                Ok(views) => views,
                Err(err) if self.ignore_unreadable => {
                    self.metadata.add_value(
                        "warning",
                        MetadataValue::String(format!(
                            "Ignored unreadable EPT tile '{}': {}",
                            path, err.0
                        )),
                    );
                    continue;
                }
                Err(err) => return Err(err),
            };
            validate_tile_count(&path, &views, tile.expected_points)?;
            let views = if data_type == "laszip" {
                views
                    .into_iter()
                    .map(|v| {
                        let dims = addons
                            .iter()
                            .map(|addon| (addon.dim.clone(), addon.ty))
                            .chain([
                                (ept_node_id.clone(), DimType::U32),
                                (ept_point_id.clone(), DimType::U32),
                            ])
                            .collect::<Vec<_>>();
                        v.with_dimensions(&dims)
                    })
                    .collect::<Vec<_>>()
            } else {
                views
            };
            for view in views {
                let mut view = view;
                apply_addons(&mut view, &addons, &tile.key)?;
                // Assign full-tile EptNodeId/EptPointId values BEFORE filtering
                // so surviving points retain their original tile-local indices.
                // The addon writer's buffer is sized by the full tile count and
                // indexed by EptPointId; filtered points' ids still fall inside
                // [0, full_tile_count).
                let full_tile_count: u64 = view.len();
                for idx in 0..full_tile_count {
                    view.set_f64(idx, &ept_node_id, node_id as f64);
                    view.set_f64(idx, &ept_point_id, idx as f64);
                }
                let view = apply_origin(
                    apply_polygons(apply_bounds(view, bounds.as_ref()), &polygons),
                    origin,
                );
                let tile_kept = view.len();
                point_count += tile_kept;
                // Record the FULL tile count (matches the C++ `Overlap.m_count`
                // shape and what the addon writer's buffer needs).
                let parts: Vec<u32> = tile
                    .key
                    .split('-')
                    .filter_map(|p| p.parse::<u32>().ok())
                    .collect();
                if parts.len() == 4 {
                    artifact_tiles.push((
                        parts[0],
                        parts[1],
                        parts[2],
                        parts[3],
                        full_tile_count,
                        node_id,
                    ));
                }
                node_id += 1;
                append_view(&mut merged, &view, Path::new(&path))?;
            }
        }
        self.metadata
            .add_value("count", MetadataValue::U64(point_count));
        self.metadata
            .add_value("tiles", MetadataValue::U64(tile_count as u64));
        // Stash an `ept::Artifact`-equivalent for the C++ wrapper. Marked with
        // a `__` prefix so downstream callers can identify and strip it.
        let tiles_json: Vec<serde_json::Value> = artifact_tiles
            .iter()
            .map(|(d, x, y, z, c, n)| {
                serde_json::json!({
                    "d": d, "x": x, "y": y, "z": z, "count": c, "node_id": n,
                })
            })
            .collect();
        let artifact = serde_json::json!({
            "hierarchy_step": hierarchy_step,
            "root_bounds": {
                "minx": root_bounds.minx,
                "miny": root_bounds.miny,
                "minz": root_bounds.minz,
                "maxx": root_bounds.maxx,
                "maxy": root_bounds.maxy,
                "maxz": root_bounds.maxz,
            },
            "tiles": tiles_json,
        });
        self.metadata.add_value(
            "__ept_artifact",
            MetadataValue::String(artifact.to_string()),
        );

        // Return at least one (possibly empty) view so wrappers don't
        // mis-treat a clean ignore_unreadable run as a reader failure.
        let merged = merged.unwrap_or_else(|| {
            let mut view = PointView::new(Rc::clone(&schema.layout));
            view.set_spatial_reference(SpatialReference::new(srs));
            view
        });
        Ok(vec![merged])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

/// Header-only EPT preview info, mirroring the fields that the C++
/// `EptReader::inspect` path needs to populate a `QuickInfo`.
pub struct EptPreview {
    pub bounds_conforming: Bounds3D,
    pub point_count: u64,
    pub srs_wkt: String,
    pub dim_names: Vec<String>,
}

/// Read EPT preview metadata from the `ept.json` at `filename`. Mirrors the
/// dim-name expansion in `EptInfo::initialize` so laszip and `ClassFlags`
/// datasets include the Withheld/KeyPoint/Synthetic/Overlap flags.
pub fn read_ept_preview(filename: &str) -> Result<EptPreview, StageError> {
    if filename.is_empty() {
        return Err(StageError(
            "EptReader requires a filename option.".to_string(),
        ));
    }
    let path = Path::new(filename);
    let info = read_json(path)?;

    let bounds_conforming = ept_bounds_field(&info, "boundsConforming")?;
    let point_count = info["points"]
        .as_u64()
        .ok_or_else(|| StageError(format!("EPT file '{}' is missing points.", path.display())))?;
    let srs_wkt = info["srs"]["wkt"].as_str().unwrap_or("").to_string();

    let data_type = info["dataType"].as_str().ok_or_else(|| {
        StageError(format!(
            "EPT file '{}' is missing dataType.",
            path.display()
        ))
    })?;

    let schema = info["schema"]
        .as_array()
        .ok_or_else(|| StageError(format!("EPT file '{}' is missing schema.", path.display())))?;

    let mut dim_names: Vec<String> = Vec::new();
    let mut saw_class_flags = false;
    for entry in schema {
        let name = entry["name"].as_str().ok_or_else(|| {
            StageError(format!(
                "EPT file '{}' schema entry missing name.",
                path.display()
            ))
        })?;
        if name.eq_ignore_ascii_case("ClassFlags") {
            saw_class_flags = true;
            continue;
        }
        dim_names.push(name.to_string());
    }
    if data_type == "laszip" || saw_class_flags {
        for flag in ["Withheld", "Overlap", "Synthetic", "KeyPoint"] {
            if !dim_names.iter().any(|n| n == flag) {
                dim_names.push(flag.to_string());
            }
        }
    }

    Ok(EptPreview {
        bounds_conforming,
        point_count,
        srs_wkt,
        dim_names,
    })
}

/// Build the SRS WKT/user-input string from an EPT info `srs` object, matching
/// the C++ `EptInfo::initialize()` rules. Returns `Ok(None)` when no usable
/// `srs` is present (missing, null, or an empty object), otherwise the string
/// that should be handed to `SpatialReference::set`.
pub fn ept_srs_wkt(info: &Value) -> Result<Option<String>, StageError> {
    let srs = match info.get("srs") {
        Some(srs) => srs,
        None => return Ok(None),
    };
    // C++ treats a null or empty srs as "no srs" (`iSrs->size()` is falsy).
    let is_empty = srs.is_null()
        || srs.as_object().map(|o| o.is_empty()).unwrap_or(false)
        || srs.as_array().map(|a| a.is_empty()).unwrap_or(false);
    if is_empty {
        return Ok(None);
    }

    if let Some(wkt) = srs.get("wkt") {
        let wkt = wkt.as_str().ok_or_else(|| {
            StageError(format!(
                "srs.wkt must be specified as a string. Found '{}'.",
                json_dump(wkt)
            ))
        })?;
        return Ok(Some(wkt.to_string()));
    }

    let authority = srs.get("authority");
    let horizontal = srs.get("horizontal");
    if authority.is_none() || horizontal.is_none() {
        return Err(StageError(
            "srs must be defined with at least one of \
             wkt or both authority and horizontal specifications."
                .to_string(),
        ));
    }
    let authority = authority.expect("checked above");
    let horizontal = horizontal.expect("checked above");

    let mut wkt = authority
        .as_str()
        .ok_or_else(|| {
            StageError(format!(
                "srs.authority must be specified as a string.  Found '{}'.",
                json_dump(authority)
            ))
        })?
        .to_string();

    let horiz = json_unsigned_or_string(horizontal).ok_or_else(|| {
        StageError(format!(
            "srs.horizontal must be specified as a non-negative integer or \
             equivalent string. Found '{}'.",
            json_dump(horizontal)
        ))
    })?;
    wkt.push(':');
    wkt.push_str(&horiz);

    if let Some(vertical) = srs.get("vertical") {
        let vert = json_unsigned_or_string(vertical).ok_or_else(|| {
            StageError(format!(
                "srs.vertical must be specified as a non-negative integer or \
                 equivalent string. Found '{}'.",
                json_dump(vertical)
            ))
        })?;
        wkt.push('+');
        wkt.push_str(&vert);
    }

    Ok(Some(wkt))
}

/// Accept a non-negative integer (rendered as its decimal string) or an
/// already-string value, mirroring the C++ `is_number_unsigned()`/`is_string()`
/// branches.
fn json_unsigned_or_string(value: &Value) -> Option<String> {
    if let Some(n) = value.as_u64() {
        Some(n.to_string())
    } else {
        value.as_str().map(|s| s.to_string())
    }
}

/// Compact JSON rendering used in error messages, matching nlohmann `dump()`
/// for the scalar/compound cases EPT srs validation reports.
fn json_dump(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn ept_bounds_field(info: &Value, field: &str) -> Result<Bounds3D, StageError> {
    let bounds = info[field]
        .as_array()
        .ok_or_else(|| StageError(format!("EPT file is missing {field}.")))?;
    if bounds.len() < 6 {
        return Err(StageError(format!(
            "EPT {field} must contain six coordinates."
        )));
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

fn option_values(options: &Options, key: &str) -> Vec<String> {
    options
        .values(key)
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

impl EptReader {
    fn bounds_filter(&self, info: &Value) -> Result<Option<BoundsFilter>, StageError> {
        if self.bounds.is_empty() {
            return Ok(None);
        }
        if let Ok(parsed) = parse_bounds3d(&self.bounds, 0) {
            let target_srs = parsed_bounds_srs(&self.bounds, parsed.pos);
            return BoundsFilter::new(QueryBounds::Three(parsed.bounds), &target_srs, info)
                .map(Some);
        }
        let parsed = parse_bounds2d(&self.bounds, 0)
            .map_err(|err| StageError(format!("Invalid EPT bounds option: {err}")))?;
        let target_srs = parsed_bounds_srs(&self.bounds, parsed.pos);
        if !target_srs.is_empty() {
            return Err(StageError(
                "For lon/lat 'bounds', bounds must be 3D".to_string(),
            ));
        }
        Ok(Some(BoundsFilter {
            query: QueryBounds::Two(parsed.bounds),
            transform: None,
        }))
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

    fn polygon_filters(&self) -> Result<Vec<PolygonFilter>, StageError> {
        let mut filters: Vec<PolygonFilter> = self
            .polygons
            .iter()
            .enumerate()
            .map(|(idx, wkt)| {
                let geometry = Geometry::from_wkt(wkt).map_err(StageError)?;
                let polygon_srs = self.polygon_srs.get(idx).map_or("", String::as_str);
                Ok(PolygonFilter {
                    geometry,
                    transform: polygon_transform(&self.source_srs, polygon_srs)?,
                })
            })
            .collect::<Result<_, _>>()?;
        filters.extend(self.ogr_polygon_filters()?);
        Ok(filters)
    }

    fn ogr_polygon_filters(&self) -> Result<Vec<PolygonFilter>, StageError> {
        if self.ogr.trim().is_empty() {
            return Ok(Vec::new());
        }
        let spec = parse_ogr_spec_json(&self.ogr).map_err(StageError)?;
        let text = std::fs::read_to_string(&spec.datasource).map_err(|err| {
            StageError(format!(
                "Can't open OGR datasource '{}': {err}",
                spec.datasource
            ))
        })?;
        let json: Value = serde_json::from_str(&text).map_err(|err| {
            StageError(format!(
                "OGR datasource '{}' is not valid GeoJSON: {err}",
                spec.datasource
            ))
        })?;
        let features = json["features"].as_array().ok_or_else(|| {
            StageError(format!(
                "OGR datasource '{}' is missing GeoJSON features.",
                spec.datasource
            ))
        })?;
        let mut filters = Vec::new();
        for feature in features {
            if feature["geometry"].is_null() {
                continue;
            }
            let geometry =
                Geometry::from_geojson(&feature["geometry"].to_string()).map_err(StageError)?;
            filters.push(PolygonFilter {
                geometry,
                transform: polygon_transform(&self.source_srs, "EPSG:4326")?,
            });
        }
        Ok(filters)
    }
}

struct EptTile {
    key: String,
    expected_points: u64,
}

struct EptAddon {
    dim: DimId,
    ty: DimType,
    size: usize,
    data_dir: String,
}

impl EptAddon {
    fn parse_specs(spec: &str) -> Result<Vec<Self>, StageError> {
        if spec.trim().is_empty() {
            return Ok(Vec::new());
        }
        let value: Value = serde_json::from_str(spec)
            .map_err(|err| StageError(format!("Unable to parse EPT addon option: {err}")))?;
        let object = value
            .as_object()
            .ok_or_else(|| StageError("EPT addons option must be a JSON object.".to_string()))?;
        object
            .iter()
            .map(|(dim_name, path_value)| {
                let path = path_value.as_str().ok_or_else(|| {
                    StageError(format!(
                        "EPT addon mapping for '{dim_name}' must be a string path."
                    ))
                })?;
                Self::from_metadata(dim_name, path)
            })
            .collect()
    }

    fn from_metadata(dim_name: &str, path: &str) -> Result<Self, StageError> {
        let metadata_path = addon_metadata_path(path);
        let metadata = read_json_location(&metadata_path)?;
        let kind = metadata["type"]
            .as_str()
            .ok_or_else(|| StageError(format!("EPT addon '{}' is missing type.", metadata_path)))?;
        let size = metadata["size"]
            .as_u64()
            .ok_or_else(|| StageError(format!("EPT addon '{}' is missing size.", metadata_path)))?
            as usize;
        Ok(Self {
            dim: DimId::from_name(dim_name),
            ty: dim_type(kind, size)?,
            size,
            data_dir: format!("{}/ept-data", location_parent(&metadata_path)),
        })
    }

    fn data_path(&self, key: &str) -> String {
        join_location(&self.data_dir, &format!("{key}.bin"))
    }
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

/// Walk the EPT hierarchy. Returns the leaf tiles to materialize **and** the
/// detected `hierarchy_step` (depth at which subtree pointers first appear).
/// The C++ `EptReader` exposes the same step value through `ept::Artifact`;
/// the wrapper picks it up from metadata so downstream stages like
/// `writers.ept_addon` can split hierarchy JSON correctly.
fn hierarchy_tiles(
    root: &str,
    max_depth: Option<u64>,
    query: Option<&QueryBounds>,
    root_bounds: Bounds3D,
) -> Result<(Vec<EptTile>, u64), StageError> {
    let mut tiles = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([String::from("0-0-0-0")]);
    let mut hierarchy_step: u64 = 0;
    while let Some(key) = queue.pop_front() {
        if !seen.insert(key.clone()) {
            continue;
        }
        let path = join_location(root, &format!("ept-hierarchy/{key}.json"));
        let hierarchy = read_json_location(&path)?;
        let object = hierarchy
            .as_object()
            .ok_or_else(|| StageError(format!("EPT hierarchy '{path}' must be a JSON object.")))?;
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
                    if hierarchy_step == 0 {
                        hierarchy_step = depth;
                    }
                    queue.push_back(node.clone())
                }
                _ => {}
            }
        }
    }
    Ok((tiles, hierarchy_step))
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
    path: &str,
    views: &[PointView],
    expected_points: u64,
) -> Result<(), StageError> {
    let actual_points = views.iter().map(PointView::len).sum::<u64>();
    if actual_points != expected_points {
        return Err(StageError(format!(
            "EPT tile '{}' has {actual_points} points but hierarchy expected {expected_points}.",
            path
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

struct BoundsFilter {
    query: QueryBounds,
    transform: Option<GdalSrsTransform>,
}

impl BoundsFilter {
    fn new(query: QueryBounds, target_srs: &str, info: &Value) -> Result<Self, StageError> {
        let transform = if target_srs.is_empty() {
            None
        } else {
            let source_srs = info["srs"]["wkt"].as_str().unwrap_or("");
            let source = user_input_to_wkt(source_srs).map_err(StageError)?;
            let target = user_input_to_wkt(target_srs).map_err(StageError)?;
            Some(
                GdalSrsTransform::new(
                    &source.wkt2,
                    source.epoch,
                    &target.wkt2,
                    target.epoch,
                    &[],
                    &[],
                )
                .map_err(StageError)?,
            )
        };
        Ok(Self { query, transform })
    }

    fn contains(&self, view: &PointView, idx: PointId) -> bool {
        let mut x = view.get_f64(idx, &DimId::X);
        let mut y = view.get_f64(idx, &DimId::Y);
        let mut z = view.get_f64(idx, &DimId::Z);
        if let Some(transform) = &self.transform {
            if !transform.transform_xyz(&mut x, &mut y, &mut z) {
                return false;
            }
        }
        self.query.contains_point(x, y, z)
    }

    fn hierarchy_query_bounds(&self) -> Option<&QueryBounds> {
        self.transform.is_none().then_some(&self.query)
    }
}

impl QueryBounds {
    fn contains_point(&self, x: f64, y: f64, z: f64) -> bool {
        match self {
            QueryBounds::Two(bounds) => bounds.contains_point(x, y),
            QueryBounds::Three(bounds) => bounds.contains_point(x, y, z),
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

fn parsed_bounds_srs(input: &str, pos: usize) -> String {
    input
        .get(pos..)
        .unwrap_or("")
        .trim()
        .strip_prefix('/')
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

fn apply_bounds(view: PointView, bounds: Option<&BoundsFilter>) -> PointView {
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

struct PolygonFilter {
    geometry: Geometry,
    transform: Option<GdalSrsTransform>,
}

fn polygon_transform(
    source_srs: &str,
    polygon_srs: &str,
) -> Result<Option<GdalSrsTransform>, StageError> {
    if source_srs.trim().is_empty() || polygon_srs.trim().is_empty() {
        return Ok(None);
    }
    let source = user_input_to_wkt(source_srs).map_err(StageError)?;
    let target = user_input_to_wkt(polygon_srs).map_err(StageError)?;
    if source.wkt == target.wkt {
        return Ok(None);
    }
    Ok(Some(
        GdalSrsTransform::new(
            &source.wkt2,
            source.epoch,
            &target.wkt2,
            target.epoch,
            &[],
            &[],
        )
        .map_err(StageError)?,
    ))
}

fn apply_polygons(view: PointView, polygons: &[PolygonFilter]) -> PointView {
    if polygons.is_empty() {
        return view;
    }
    let mut output = view.make_new();
    for idx in 0..view.len() {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        if polygons
            .iter()
            .any(|polygon| polygon_contains(polygon, x, y))
        {
            output.append_point(&view, idx);
        }
    }
    output
}

fn polygon_contains(polygon: &PolygonFilter, mut x: f64, mut y: f64) -> bool {
    let mut z = 0.0;
    if let Some(transform) = &polygon.transform {
        if !transform.transform_xyz(&mut x, &mut y, &mut z) {
            return false;
        }
    }
    polygon.geometry.contains(x, y)
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

#[cfg(test)]
mod tests;
