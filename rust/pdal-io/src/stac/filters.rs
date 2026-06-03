use super::ReaderArgs;
use crate::source;
use crate::tindex::resolve_location;
use chrono::{DateTime, FixedOffset};
use pdal_core::ogr_spec::parse_ogr_spec_json;
use pdal_core::options::Options;
use pdal_core::stage::StageError;
use pdal_native::geometry::Geometry;
use regex::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) fn validate_stac_object(json: &Value, location: &str) -> Result<(), StageError> {
    let Some(obj) = json.as_object() else {
        return Err(StageError(format!(
            "STAC object '{location}' must be a JSON object."
        )));
    };
    let Some(type_name) = obj.get("type").and_then(Value::as_str) else {
        return Err(StageError(format!(
            "STAC object '{location}' is missing a string type field."
        )));
    };
    match type_name {
        "Feature" => validate_stac_feature(json, location),
        "FeatureCollection" => {
            let features = json["features"].as_array().ok_or_else(|| {
                StageError(format!(
                    "STAC FeatureCollection '{location}' is missing features."
                ))
            })?;
            for feature in features {
                validate_stac_feature(feature, location)?;
            }
            Ok(())
        }
        "Catalog" | "Collection" => {
            require_string(json, "id", location)?;
            require_string(json, "stac_version", location)?;
            json["links"].as_array().ok_or_else(|| {
                StageError(format!("STAC {type_name} '{location}' is missing links."))
            })?;
            Ok(())
        }
        other => Err(StageError(format!(
            "Unsupported STAC object type '{other}' in '{location}'."
        ))),
    }
}

pub(super) fn validate_stac_feature(feature: &Value, location: &str) -> Result<(), StageError> {
    require_string(feature, "id", location)?;
    require_string(feature, "stac_version", location)?;
    if !feature["geometry"].is_object() && !feature["geometry"].is_null() {
        return Err(StageError(format!(
            "STAC Feature in '{location}' has invalid geometry."
        )));
    }
    feature["bbox"]
        .as_array()
        .ok_or_else(|| StageError(format!("STAC Feature in '{location}' is missing bbox.")))?;
    feature["properties"].as_object().ok_or_else(|| {
        StageError(format!(
            "STAC Feature in '{location}' is missing properties."
        ))
    })?;
    let assets = feature["assets"]
        .as_object()
        .ok_or_else(|| StageError(format!("STAC Feature in '{location}' is missing assets.")))?;
    for (name, asset) in assets {
        if asset["href"].as_str().is_none() {
            return Err(StageError(format!(
                "STAC asset '{name}' in '{location}' is missing an href."
            )));
        }
    }
    Ok(())
}

pub(super) fn require_string(json: &Value, field: &str, location: &str) -> Result<(), StageError> {
    if json[field].as_str().is_some() {
        Ok(())
    } else {
        Err(StageError(format!(
            "STAC object '{location}' is missing string field '{field}'."
        )))
    }
}

pub(super) fn item_has_requested_asset(item: &Value, asset_names: &[String]) -> bool {
    let Some(assets) = item["assets"].as_object() else {
        return false;
    };
    asset_names.iter().any(|name| assets.contains_key(name))
}

pub(super) struct PropertyFilter {
    key: String,
    values: Vec<Value>,
}

pub(super) fn parse_property_filters(input: &str) -> Result<Vec<PropertyFilter>, StageError> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    let json: Value = serde_json::from_str(input)
        .map_err(|err| StageError(format!("Properties argument must be valid JSON: {err}")))?;
    let object = json.as_object().ok_or_else(|| {
        StageError("Properties argument must be a valid JSON object.".to_string())
    })?;
    Ok(object
        .iter()
        .map(|(key, value)| {
            let values = value
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![value.clone()]);
            PropertyFilter {
                key: key.clone(),
                values,
            }
        })
        .collect())
}

pub(super) fn item_matches_id_filters(item: &Value, item_filters: &[Regex]) -> bool {
    if item_filters.is_empty() {
        return true;
    }
    let Some(id) = item["id"].as_str() else {
        return false;
    };
    item_filters.iter().any(|regex| regex.is_match(id))
}

pub(super) fn item_matches_property_filters(
    item: &Value,
    filters: &[PropertyFilter],
) -> Result<bool, StageError> {
    if filters.is_empty() {
        return Ok(true);
    }
    let properties = item["properties"]
        .as_object()
        .ok_or_else(|| StageError("STAC Item is missing properties.".to_string()))?;
    for filter in filters {
        let Some(value) = properties.get(&filter.key) else {
            return Ok(false);
        };
        if !filter.values.iter().any(|candidate| candidate == value) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn driver_for_asset(asset: &Value, location: &str) -> Result<String, StageError> {
    if asset["type"]
        .as_str()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("application/vnd.laszip+copc"))
    {
        return Ok("readers.copc".to_string());
    }
    pdal_core::driver::infer_reader_driver(location)
        .map(str::to_string)
        .ok_or_else(|| {
            StageError(format!(
                "StacReader cannot infer a reader for '{location}'."
            ))
        })
}

pub(super) fn parse_reader_args(input: &str) -> Result<Vec<ReaderArgs>, StageError> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    let stripped = pdal_core::pipeline_reader::strip_json_comments(input);
    let json: Value = serde_json::from_str(&stripped)
        .map_err(|err| StageError(format!("reader_args must be valid JSON: {err}")))?;
    let entries = json
        .as_array()
        .ok_or_else(|| StageError("reader_args must be a JSON array.".to_string()))?;
    let mut out = Vec::new();
    for entry in entries {
        let object = entry
            .as_object()
            .ok_or_else(|| StageError("reader_args entries must be JSON objects.".to_string()))?;
        let driver = object
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| StageError("reader_args entry is missing type.".to_string()))?
            .to_string();
        let mut options = Options::new();
        for (key, value) in object {
            if key == "type" {
                continue;
            }
            add_json_option(&mut options, key, value);
        }
        out.push(ReaderArgs { driver, options });
    }
    Ok(out)
}

pub(super) fn add_json_option(options: &mut Options, key: &str, value: &Value) {
    match value {
        Value::String(text) => {
            options.add(key, text.as_str());
        }
        Value::Number(number) => {
            options.add(key, number.to_string());
        }
        Value::Bool(value) => {
            options.add(key, if *value { "true" } else { "false" });
        }
        _ => {
            options.add(key, value.to_string());
        }
    };
}

pub(super) fn collection_matches(item: &Value, collections: &[Regex]) -> bool {
    if collections.is_empty() {
        return true;
    }
    let Some(collection) = item["collection"].as_str() else {
        return false;
    };
    collections.iter().any(|regex| regex.is_match(collection))
}

pub(super) fn catalog_matches(catalog: &Value, catalogs: &[Regex]) -> bool {
    if catalogs.is_empty() {
        return true;
    }
    let Some(id) = catalog["id"].as_str() else {
        return false;
    };
    catalogs.iter().any(|regex| regex.is_match(id))
}

pub(super) fn compile_regexes(values: &[String], label: &str) -> Result<Vec<Regex>, StageError> {
    values
        .iter()
        .map(|value| {
            Regex::new(value).map_err(|err| {
                StageError(format!(
                    "Invalid {label} regular expression '{value}': {err}"
                ))
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(super) struct DateRange {
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
}

#[derive(Clone, Copy)]
pub(super) struct Bounds2D {
    pub(super) minx: f64,
    pub(super) maxx: f64,
    pub(super) miny: f64,
    pub(super) maxy: f64,
}

impl Bounds2D {
    fn point(x: f64, y: f64) -> Self {
        Self {
            minx: x,
            maxx: x,
            miny: y,
            maxy: y,
        }
    }

    fn grow(&mut self, x: f64, y: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
    }

    fn grow_bounds(&mut self, other: &Bounds2D) {
        self.minx = self.minx.min(other.minx);
        self.maxx = self.maxx.max(other.maxx);
        self.miny = self.miny.min(other.miny);
        self.maxy = self.maxy.max(other.maxy);
    }
}

pub(super) fn parse_date_ranges(values: &[String]) -> Result<Vec<DateRange>, StageError> {
    values
        .iter()
        .map(|value| {
            let range: Value = serde_json::from_str(value).map_err(|_| {
                StageError(format!(
                    "User defined dates ({value}) must be a range of [min, max]."
                ))
            })?;
            let Some(items) = range.as_array() else {
                return Err(StageError(format!(
                    "User defined dates ({value}) must be a range of [min, max]."
                )));
            };
            if items.len() != 2 {
                return Err(StageError(format!(
                    "User defined dates ({value}) must be a range of [min, max]."
                )));
            }
            let min = parse_stac_time(items[0].as_str().ok_or_else(|| {
                StageError(format!(
                    "User defined date range ({value}) is invalid. It must be of type string and comply with  RFC 3339."
                ))
            })?)?;
            let max = parse_stac_time(items[1].as_str().ok_or_else(|| {
                StageError(format!(
                    "User defined date range ({value}) is invalid. It must be of type string and comply with  RFC 3339."
                ))
            })?)?;
            Ok(DateRange {
                start: min,
                end: max,
            })
        })
        .collect()
}

pub(super) fn parse_stac_time(value: &str) -> Result<DateTime<FixedOffset>, StageError> {
    DateTime::parse_from_rfc3339(value)
        .or_else(|_| DateTime::parse_from_rfc3339(&normalize_stac_time(value)))
        .map_err(|_| {
            StageError(format!(
                "User defined date range is invalid. It must comply with  RFC 3339: {value}"
            ))
        })
}

pub(super) fn normalize_stac_time(value: &str) -> String {
    let Some((date, time)) = value.split_once('T') else {
        return value.to_string();
    };
    let Some(time) = time.strip_suffix('Z') else {
        return value.to_string();
    };
    let parts: Vec<_> = time.split(':').collect();
    if parts.len() != 3 {
        return value.to_string();
    }
    let hour = format!("{:0>2}", parts[0]);
    let minute = format!("{:0>2}", parts[1]);
    let second = if let Some((whole, frac)) = parts[2].split_once('.') {
        format!("{:0>2}.{frac}", whole)
    } else {
        format!("{:0>2}", parts[2])
    };
    format!("{date}T{hour}:{minute}:{second}Z")
}

pub(super) fn parse_bounds(value: &str) -> Result<Option<Bounds2D>, StageError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let prefix = value.split('/').next().unwrap_or(value);
    let numbers = Regex::new(r"-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?")
        .expect("bounds number regex")
        .find_iter(prefix)
        .map(|matched| matched.as_str().parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StageError("Supplied bounds are not valid.".to_string()))?;
    if numbers.len() < 4 {
        return Err(StageError("Supplied bounds are not valid.".to_string()));
    }
    Ok(Some(Bounds2D {
        minx: numbers[0].min(numbers[1]),
        maxx: numbers[0].max(numbers[1]),
        miny: numbers[2].min(numbers[3]),
        maxy: numbers[2].max(numbers[3]),
    }))
}

pub(super) fn parse_ogr_bounds(value: &str) -> Result<Option<Bounds2D>, StageError> {
    let spec = parse_ogr_spec_json(value).map_err(StageError)?;
    let id_filter = ogr_sql_id_filter(&spec.sql);
    match source::read_to_string(&spec.datasource) {
        Ok(text) => match parse_geojson_ogr_bounds(&spec.datasource, &text, id_filter) {
            Ok(bounds) => Ok(bounds),
            Err(json_err) => parse_native_ogr_bounds(&spec.datasource, id_filter).or(Err(json_err)),
        },
        Err(text_err) => parse_native_ogr_bounds(&spec.datasource, id_filter)
            .map_err(|ogr_err| StageError(format!("{text_err}; {ogr_err}"))),
    }
}

fn parse_geojson_ogr_bounds(
    datasource: &str,
    text: &str,
    id_filter: Option<i64>,
) -> Result<Option<Bounds2D>, StageError> {
    let json: Value = serde_json::from_str(text).map_err(|err| {
        StageError(format!(
            "OGR datasource '{datasource}' is not valid GeoJSON: {err}"
        ))
    })?;
    let features = json["features"].as_array().ok_or_else(|| {
        StageError(format!(
            "OGR datasource '{datasource}' is missing GeoJSON features."
        ))
    })?;
    let mut out: Option<Bounds2D> = None;
    for feature in features {
        if let Some(id) = id_filter {
            if feature["properties"]["id"].as_i64() != Some(id) {
                continue;
            }
        }
        let Some(bounds) = geojson_geometry_bounds(&feature["geometry"])? else {
            continue;
        };
        match &mut out {
            Some(out) => out.grow_bounds(&bounds),
            None => out = Some(bounds),
        }
    }
    Ok(out)
}

fn parse_native_ogr_bounds(
    datasource: &str,
    id_filter: Option<i64>,
) -> Result<Option<Bounds2D>, StageError> {
    let vector = pdal_native::gdal::Vector::open(datasource).map_err(StageError)?;
    let wkts = if let Some(id_filter) = id_filter {
        vector
            .get_features(0, "id")
            .map_err(StageError)?
            .into_iter()
            .filter_map(|(wkt, id)| (id as i64 == id_filter).then_some(wkt))
            .collect()
    } else {
        vector.get_feature_wkts(0).map_err(StageError)?
    };
    let mut out: Option<Bounds2D> = None;
    for wkt in wkts {
        let geometry = Geometry::from_wkt(&wkt).map_err(StageError)?;
        let (minx, maxx, miny, maxy, _, _) = geometry.bounds().map_err(StageError)?;
        let bounds = Bounds2D {
            minx,
            maxx,
            miny,
            maxy,
        };
        match &mut out {
            Some(out) => out.grow_bounds(&bounds),
            None => out = Some(bounds),
        }
    }
    Ok(out)
}

pub(super) fn ogr_sql_id_filter(sql: &str) -> Option<i64> {
    let regex = Regex::new(r"(?i)\bid\s*=\s*(-?\d+)").expect("OGR id SQL regex");
    regex
        .captures(sql)
        .and_then(|captures| captures.get(1))
        .and_then(|matched| matched.as_str().parse::<i64>().ok())
}

pub(super) fn geojson_geometry_bounds(geometry: &Value) -> Result<Option<Bounds2D>, StageError> {
    if geometry.is_null() {
        return Ok(None);
    }
    match geometry["type"].as_str().unwrap_or("") {
        "Polygon" => polygon_bounds(geometry),
        "MultiPolygon" => multipolygon_bounds(geometry),
        geom_type => Err(StageError(format!(
            "Unsupported OGR geometry type '{geom_type}'."
        ))),
    }
}

fn multipolygon_bounds(geometry: &Value) -> Result<Option<Bounds2D>, StageError> {
    let Some(polygons) = geometry["coordinates"].as_array() else {
        return Err(StageError("Invalid OGR multipolygon geometry.".to_string()));
    };
    let mut bounds: Option<Bounds2D> = None;
    for polygon in polygons {
        let Some(rings) = polygon.as_array() else {
            return Err(StageError("Invalid OGR multipolygon geometry.".to_string()));
        };
        let Some(polygon_bounds) = rings_bounds(rings)? else {
            continue;
        };
        match &mut bounds {
            Some(bounds) => bounds.grow_bounds(&polygon_bounds),
            None => bounds = Some(polygon_bounds),
        }
    }
    Ok(bounds)
}

fn polygon_bounds(geometry: &Value) -> Result<Option<Bounds2D>, StageError> {
    let Some(rings) = geometry["coordinates"].as_array() else {
        return Err(StageError("Invalid OGR polygon geometry.".to_string()));
    };
    rings_bounds(rings)
}

fn rings_bounds(rings: &[Value]) -> Result<Option<Bounds2D>, StageError> {
    let Some(outer) = rings.first().and_then(Value::as_array) else {
        return Err(StageError("Invalid OGR polygon geometry.".to_string()));
    };
    if outer.len() < 4 {
        return Err(StageError("Invalid OGR polygon geometry.".to_string()));
    }
    let mut bounds: Option<Bounds2D> = None;
    for coord in outer {
        let values = coord
            .as_array()
            .ok_or_else(|| StageError("Invalid OGR polygon coordinate.".to_string()))?;
        let x = values
            .first()
            .and_then(Value::as_f64)
            .ok_or_else(|| StageError("Invalid OGR polygon coordinate.".to_string()))?;
        let y = values
            .get(1)
            .and_then(Value::as_f64)
            .ok_or_else(|| StageError("Invalid OGR polygon coordinate.".to_string()))?;
        match &mut bounds {
            Some(bounds) => bounds.grow(x, y),
            None => bounds = Some(Bounds2D::point(x, y)),
        }
    }
    Ok(bounds)
}

pub(super) fn item_matches_dates(item: &Value, ranges: &[DateRange]) -> bool {
    let properties = &item["properties"];
    let item_range = if let Some(datetime) = properties["datetime"].as_str() {
        parse_stac_time(datetime).ok().map(|time| DateRange {
            start: time,
            end: time,
        })
    } else {
        let start = properties["start_datetime"]
            .as_str()
            .and_then(|value| parse_stac_time(value).ok());
        let end = properties["end_datetime"]
            .as_str()
            .and_then(|value| parse_stac_time(value).ok());
        start.zip(end).map(|(start, end)| DateRange { start, end })
    };
    let Some(item_range) = item_range else {
        return false;
    };
    ranges.iter().any(|range| {
        range.start <= range.end && item_range.start <= range.end && item_range.end >= range.start
    })
}

pub(super) fn item_matches_bounds(item: &Value, bounds: &Bounds2D) -> bool {
    let Some(bbox) = item["bbox"].as_array() else {
        return false;
    };
    let Some((minx_idx, miny_idx, maxx_idx, maxy_idx)) = (match bbox.len() {
        4 => Some((0, 1, 2, 3)),
        6 => Some((0, 1, 3, 4)),
        _ => None,
    }) else {
        return false;
    };
    let Some(item_minx) = bbox[minx_idx].as_f64() else {
        return false;
    };
    let Some(item_miny) = bbox[miny_idx].as_f64() else {
        return false;
    };
    let Some(item_maxx) = bbox[maxx_idx].as_f64() else {
        return false;
    };
    let Some(item_maxy) = bbox[maxy_idx].as_f64() else {
        return false;
    };
    item_minx <= bounds.maxx
        && item_maxx >= bounds.minx
        && item_miny <= bounds.maxy
        && item_maxy >= bounds.miny
}

pub(super) fn read_stac_text(location: &str) -> Result<(String, String), StageError> {
    if is_remote(location) || location.starts_with("/vsi") {
        let text = source::read_to_string(location)
            .map_err(|err| StageError(format!("Can't open STAC file '{location}': {err}")))?;
        Ok((text, remote_base(location)))
    } else {
        let path = canonical_or_original(Path::new(location));
        let path_text = path.to_string_lossy();
        let text = source::read_to_string(&path_text).map_err(|err| {
            StageError(format!("Can't open STAC file '{}': {err}", path.display()))
        })?;
        let base = path.parent().unwrap_or(Path::new("")).display().to_string();
        Ok((text, base))
    }
}

pub(super) fn remote_base(location: &str) -> String {
    location
        .rsplit_once('/')
        .map(|(base, _)| base.to_string())
        .unwrap_or_default()
}

pub(super) fn resolve_stac_link(base: &str, href: &str) -> String {
    if is_remote(href) || Path::new(href).is_absolute() {
        href.to_string()
    } else if is_remote(base) {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            href.trim_start_matches("./")
        )
    } else {
        resolve_location(Path::new(base), href)
            .display()
            .to_string()
    }
}

pub(super) fn normalize_local_location(location: &str) -> String {
    if is_remote(location) || location.starts_with("/vsi") {
        location.to_string()
    } else {
        canonical_or_original(Path::new(location))
            .display()
            .to_string()
    }
}

pub(super) fn canonical_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(super) fn is_remote(value: &str) -> bool {
    value.contains("://")
}
