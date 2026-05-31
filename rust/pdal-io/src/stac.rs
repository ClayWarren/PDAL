//! `readers.stac` -- local STAC asset reader.
//!
//! This is a narrow local-file slice: STAC Item assets and local
//! Catalog/Collection/FeatureCollection traversal. Remote assets, schema
//! validation, EPT/COPC-specific behavior, and STAC filtering stay with the
//! later vendor/remote I/O milestone.

use crate::tindex::{append_view, read_point_location, resolve_location};
use chrono::{DateTime, FixedOffset};
use pdal_core::metadata::MetadataNode;
use pdal_core::ogr_spec::parse_ogr_spec_json;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::PointView;
use pdal_core::stage::StageError;
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct StacReader {
    filename: String,
    asset_names: Vec<String>,
    items: Vec<String>,
    date_ranges: Vec<String>,
    bounds: String,
    ogr: String,
    collections: Vec<String>,
    validate_schema: bool,
    properties: String,
    reader_args: String,
}

pub struct StacPreview {
    pub catalog_ids: Vec<String>,
    pub collection_ids: Vec<String>,
    pub item_ids: Vec<String>,
    pub point_count: u64,
}

impl StacReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            asset_names: asset_names(options),
            items: option_values(options, "items"),
            date_ranges: option_values(options, "date_ranges"),
            bounds: options.get_str("bounds", ""),
            ogr: options.get_str("ogr", ""),
            collections: comma_values(options, "collections"),
            validate_schema: options.get_bool("validate_schema", false),
            properties: options.get_str("properties", ""),
            reader_args: options.get_str("reader_args", ""),
        }
    }

    pub fn preview(&self) -> Result<StacPreview, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "StacReader requires a filename option.".to_string(),
            ));
        }
        let mut visited = BTreeSet::new();
        let mut preview = StacPreview {
            catalog_ids: Vec::new(),
            collection_ids: Vec::new(),
            item_ids: Vec::new(),
            point_count: 0,
        };
        let item_filters = compile_regexes(&self.items, "items")?;
        let date_ranges = parse_date_ranges(&self.date_ranges)?;
        let bounds = if self.ogr.trim().is_empty() {
            parse_bounds(&self.bounds)?
        } else {
            parse_ogr_bounds(&self.ogr)?
        };
        let mut context = StacPreviewContext {
            asset_names: &self.asset_names,
            item_filters: &item_filters,
            date_ranges: &date_ranges,
            bounds: bounds.as_ref(),
            validate_schema: self.validate_schema,
            visited: &mut visited,
            preview: &mut preview,
        };
        collect_preview(&self.filename, &mut context)?;
        if preview.item_ids.is_empty()
            && (!item_filters.is_empty() || !date_ranges.is_empty() || bounds.is_some())
        {
            return Err(StageError(
                "Reader list is empty after filtering.".to_string(),
            ));
        }
        Ok(preview)
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
        let collections = compile_regexes(&self.collections, "collections")?;
        let item_filters = compile_regexes(&self.items, "items")?;
        let property_filters = parse_property_filters(&self.properties)?;
        let reader_args = parse_reader_args(&self.reader_args)?;
        let mut context = StacAssetContext {
            asset_names: &self.asset_names,
            item_filters: &item_filters,
            collections: &collections,
            property_filters: &property_filters,
            validate_schema: self.validate_schema,
            visited: &mut visited,
            assets: &mut assets,
        };
        collect_assets(&self.filename, &mut context)?;

        if assets.is_empty() && !self.collections.is_empty() {
            return Err(StageError(
                "Reader list is empty after filtering.".to_string(),
            ));
        }

        let mut merged: Option<PointView> = None;
        for asset in assets {
            let options = reader_args
                .iter()
                .find(|args| args.driver == asset.driver)
                .map(|args| &args.options)
                .unwrap_or_else(|| empty_options());
            let views = read_point_location(&asset.location, Some(&asset.driver), options)?;
            for view in views {
                append_view(&mut merged, &view, Path::new(&asset.location))?;
            }
        }

        Ok(merged.into_iter().collect())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.stac")
    }
}

fn asset_names(options: &Options) -> Vec<String> {
    let names = comma_values(options, "asset_names");
    if names.is_empty() {
        return vec!["data".to_string()];
    }
    names
}

fn comma_values(options: &Options, key: &str) -> Vec<String> {
    options
        .values(key)
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
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

#[derive(Clone)]
struct StacAsset {
    location: String,
    driver: String,
}

struct ReaderArgs {
    driver: String,
    options: Options,
}

fn empty_options() -> &'static Options {
    static EMPTY: std::sync::OnceLock<Options> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Options::new)
}

fn collect_assets(location: &str, context: &mut StacAssetContext<'_>) -> Result<(), StageError> {
    let location = normalize_local_location(location);
    if !context.visited.insert(location.clone()) {
        return Ok(());
    }

    let (text, base) = read_stac_text(&location)?;
    let json: Value = serde_json::from_str(&text).map_err(|err| {
        StageError(format!(
            "StacReader expected a STAC JSON object in '{location}': {err}"
        ))
    })?;
    if context.validate_schema {
        validate_stac_object(&json, &location)?;
    }
    match json["type"].as_str() {
        Some("Feature") => collect_item_assets(&json, &base, context),
        Some("Catalog") | Some("Collection") => collect_linked_items(&json, &base, context),
        Some("FeatureCollection") => {
            let features = json["features"].as_array().ok_or_else(|| {
                StageError(format!(
                    "STAC FeatureCollection '{location}' is missing features.",
                ))
            })?;
            for feature in features {
                collect_item_assets(feature, &base, context)?;
            }
            collect_linked_items(&json, &base, context)
        }
        Some(other) => Err(StageError(format!(
            "Unsupported STAC object type '{other}' in '{location}'."
        ))),
        None => Err(StageError(format!(
            "STAC file '{location}' is missing a type field."
        ))),
    }
}

struct StacAssetContext<'a> {
    asset_names: &'a [String],
    item_filters: &'a [Regex],
    collections: &'a [Regex],
    property_filters: &'a [PropertyFilter],
    validate_schema: bool,
    visited: &'a mut BTreeSet<String>,
    assets: &'a mut Vec<StacAsset>,
}

fn collect_item_assets(
    item: &Value,
    base: &str,
    context: &mut StacAssetContext<'_>,
) -> Result<(), StageError> {
    if !item_matches_id_filters(item, context.item_filters) {
        return Ok(());
    }
    if !collection_matches(item, context.collections) {
        return Ok(());
    }
    if !item_matches_property_filters(item, context.property_filters)? {
        return Ok(());
    }

    let map = item["assets"]
        .as_object()
        .ok_or_else(|| StageError("STAC Item is missing assets.".to_string()))?;
    for name in context.asset_names {
        let Some(asset) = map.get(name) else {
            continue;
        };
        let href = asset["href"]
            .as_str()
            .ok_or_else(|| StageError(format!("STAC asset '{name}' is missing an href.")))?;
        let location = resolve_stac_link(base, href);
        let driver = driver_for_asset(asset, &location)?;
        context.assets.push(StacAsset { location, driver });
        return Ok(());
    }
    Ok(())
}

fn collect_linked_items(
    json: &Value,
    base: &str,
    context: &mut StacAssetContext<'_>,
) -> Result<(), StageError> {
    let Some(links) = json["links"].as_array() else {
        return Ok(());
    };
    for link in links {
        let rel = link["rel"].as_str().unwrap_or("");
        if !matches!(rel, "item" | "child" | "collection" | "next") {
            continue;
        }
        let Some(href) = link["href"].as_str() else {
            continue;
        };
        collect_assets(&resolve_stac_link(base, href), context)?;
    }
    Ok(())
}

struct StacPreviewContext<'a> {
    asset_names: &'a [String],
    item_filters: &'a [Regex],
    date_ranges: &'a [DateRange],
    bounds: Option<&'a Bounds2D>,
    validate_schema: bool,
    visited: &'a mut BTreeSet<String>,
    preview: &'a mut StacPreview,
}

fn collect_preview(location: &str, context: &mut StacPreviewContext<'_>) -> Result<(), StageError> {
    if !context.visited.insert(location.to_string()) {
        return Ok(());
    }

    let (text, base) = read_stac_text(location)?;
    let json: Value = serde_json::from_str(&text).map_err(|err| {
        StageError(format!(
            "StacReader expected a STAC JSON object in '{location}': {err}"
        ))
    })?;
    if context.validate_schema {
        validate_stac_object(&json, location)?;
    }
    match json["type"].as_str() {
        Some("Feature") => {
            collect_item_preview(
                &json,
                context.asset_names,
                context.item_filters,
                context.date_ranges,
                context.bounds,
                context.preview,
            );
            Ok(())
        }
        Some("Catalog") | Some("Collection") | Some("FeatureCollection") => {
            if let Some(id) = json["id"].as_str() {
                match json["type"].as_str() {
                    Some("Catalog") => context.preview.catalog_ids.push(id.to_string()),
                    Some("Collection") => context.preview.collection_ids.push(id.to_string()),
                    _ => {}
                }
            }
            if let Some(features) = json["features"].as_array() {
                for feature in features {
                    collect_item_preview(
                        feature,
                        context.asset_names,
                        context.item_filters,
                        context.date_ranges,
                        context.bounds,
                        context.preview,
                    );
                }
            }
            if let Some(links) = json["links"].as_array() {
                for link in links {
                    let rel = link["rel"].as_str().unwrap_or("");
                    if !matches!(rel, "item" | "child" | "catalog" | "collection" | "next") {
                        continue;
                    }
                    let Some(href) = link["href"].as_str() else {
                        continue;
                    };
                    if is_remote(href) {
                        collect_preview(href, context)?;
                        continue;
                    }
                    collect_preview(&resolve_stac_link(&base, href), context)?;
                }
            }
            Ok(())
        }
        Some(other) => Err(StageError(format!(
            "Unsupported STAC object type '{other}' in '{location}'."
        ))),
        None => Err(StageError(format!(
            "STAC file '{location}' is missing a type field."
        ))),
    }
}

fn collect_item_preview(
    item: &Value,
    asset_names: &[String],
    item_filters: &[Regex],
    date_ranges: &[DateRange],
    bounds: Option<&Bounds2D>,
    preview: &mut StacPreview,
) {
    if !item_has_requested_asset(item, asset_names) {
        return;
    }
    let Some(id) = item["id"].as_str() else {
        return;
    };
    if !item_filters.is_empty() && !item_filters.iter().any(|regex| regex.is_match(id)) {
        return;
    }
    if !date_ranges.is_empty() && !item_matches_dates(item, date_ranges) {
        return;
    }
    if let Some(bounds) = bounds {
        if !item_matches_bounds(item, bounds) {
            return;
        }
    }
    preview.item_ids.push(id.to_string());
    if let Some(count) = item["properties"]["pc:count"].as_u64() {
        preview.point_count += count;
    }
}

fn validate_stac_object(json: &Value, location: &str) -> Result<(), StageError> {
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

fn validate_stac_feature(feature: &Value, location: &str) -> Result<(), StageError> {
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

fn require_string(json: &Value, field: &str, location: &str) -> Result<(), StageError> {
    if json[field].as_str().is_some() {
        Ok(())
    } else {
        Err(StageError(format!(
            "STAC object '{location}' is missing string field '{field}'."
        )))
    }
}

fn item_has_requested_asset(item: &Value, asset_names: &[String]) -> bool {
    let Some(assets) = item["assets"].as_object() else {
        return false;
    };
    asset_names.iter().any(|name| assets.contains_key(name))
}

struct PropertyFilter {
    key: String,
    values: Vec<Value>,
}

fn parse_property_filters(input: &str) -> Result<Vec<PropertyFilter>, StageError> {
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

fn item_matches_id_filters(item: &Value, item_filters: &[Regex]) -> bool {
    if item_filters.is_empty() {
        return true;
    }
    let Some(id) = item["id"].as_str() else {
        return false;
    };
    item_filters.iter().any(|regex| regex.is_match(id))
}

fn item_matches_property_filters(
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

fn driver_for_asset(asset: &Value, location: &str) -> Result<String, StageError> {
    if asset["type"].as_str() == Some("application/vnd.laszip+copc") {
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

fn parse_reader_args(input: &str) -> Result<Vec<ReaderArgs>, StageError> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    let json: Value = serde_json::from_str(input)
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

fn add_json_option(options: &mut Options, key: &str, value: &Value) {
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

fn collection_matches(item: &Value, collections: &[Regex]) -> bool {
    if collections.is_empty() {
        return true;
    }
    let Some(collection) = item["collection"].as_str() else {
        return false;
    };
    collections.iter().any(|regex| regex.is_match(collection))
}

fn compile_regexes(values: &[String], label: &str) -> Result<Vec<Regex>, StageError> {
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
struct DateRange {
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
}

#[derive(Clone, Copy)]
struct Bounds2D {
    minx: f64,
    maxx: f64,
    miny: f64,
    maxy: f64,
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
}

fn parse_date_ranges(values: &[String]) -> Result<Vec<DateRange>, StageError> {
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

fn parse_stac_time(value: &str) -> Result<DateTime<FixedOffset>, StageError> {
    DateTime::parse_from_rfc3339(value)
        .or_else(|_| DateTime::parse_from_rfc3339(&normalize_stac_time(value)))
        .map_err(|_| {
            StageError(format!(
                "User defined date range is invalid. It must comply with  RFC 3339: {value}"
            ))
        })
}

fn normalize_stac_time(value: &str) -> String {
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

fn parse_bounds(value: &str) -> Result<Option<Bounds2D>, StageError> {
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

fn parse_ogr_bounds(value: &str) -> Result<Option<Bounds2D>, StageError> {
    let spec = parse_ogr_spec_json(value).map_err(StageError)?;
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
    let id_filter = ogr_sql_id_filter(&spec.sql);
    let features = json["features"].as_array().ok_or_else(|| {
        StageError(format!(
            "OGR datasource '{}' is missing GeoJSON features.",
            spec.datasource
        ))
    })?;
    for feature in features {
        if let Some(id) = id_filter {
            if feature["properties"]["id"].as_i64() != Some(id) {
                continue;
            }
        }
        let Some(bounds) = geojson_geometry_bounds(&feature["geometry"])? else {
            continue;
        };
        return Ok(Some(bounds));
    }
    Ok(None)
}

fn ogr_sql_id_filter(sql: &str) -> Option<i64> {
    let regex = Regex::new(r"(?i)\bid\s*=\s*(-?\d+)").expect("OGR id SQL regex");
    regex
        .captures(sql)
        .and_then(|captures| captures.get(1))
        .and_then(|matched| matched.as_str().parse::<i64>().ok())
}

fn geojson_geometry_bounds(geometry: &Value) -> Result<Option<Bounds2D>, StageError> {
    if geometry.is_null() {
        return Ok(None);
    }
    let geom_type = geometry["type"].as_str().unwrap_or("");
    if geom_type != "Polygon" {
        return Err(StageError(format!(
            "Unsupported OGR geometry type '{geom_type}'."
        )));
    }
    let Some(rings) = geometry["coordinates"].as_array() else {
        return Err(StageError("Invalid OGR polygon geometry.".to_string()));
    };
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

fn item_matches_dates(item: &Value, ranges: &[DateRange]) -> bool {
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

fn item_matches_bounds(item: &Value, bounds: &Bounds2D) -> bool {
    let Some(bbox) = item["bbox"].as_array() else {
        return false;
    };
    if bbox.len() < 4 {
        return false;
    }
    let Some(item_minx) = bbox[0].as_f64() else {
        return false;
    };
    let Some(item_miny) = bbox[1].as_f64() else {
        return false;
    };
    let Some(item_maxx) = bbox[2].as_f64() else {
        return false;
    };
    let Some(item_maxy) = bbox[3].as_f64() else {
        return false;
    };
    item_minx <= bounds.maxx
        && item_maxx >= bounds.minx
        && item_miny <= bounds.maxy
        && item_maxy >= bounds.miny
}

fn read_stac_text(location: &str) -> Result<(String, String), StageError> {
    if is_remote(location) || location.starts_with("/vsi") {
        let vsi_path = if location.starts_with("http://") || location.starts_with("https://") {
            format!("/vsicurl/{location}")
        } else {
            location.to_string()
        };
        let mut file = pdal_native::vsi::VsiFile::open(&vsi_path)
            .map_err(|err| StageError(format!("Can't open STAC file '{location}': {err}")))?;
        let mut text = String::new();
        file.read_to_string(&mut text)
            .map_err(|err| StageError(format!("Can't read STAC file '{location}': {err}")))?;
        Ok((text, remote_base(location)))
    } else {
        let path = canonical_or_original(Path::new(location));
        let text = std::fs::read_to_string(&path).map_err(|err| {
            StageError(format!("Can't open STAC file '{}': {err}", path.display()))
        })?;
        let base = path.parent().unwrap_or(Path::new("")).display().to_string();
        Ok((text, base))
    }
}

fn remote_base(location: &str) -> String {
    location
        .rsplit_once('/')
        .map(|(base, _)| base.to_string())
        .unwrap_or_default()
}

fn resolve_stac_link(base: &str, href: &str) -> String {
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

fn normalize_local_location(location: &str) -> String {
    if is_remote(location) || location.starts_with("/vsi") {
        location.to_string()
    } else {
        canonical_or_original(Path::new(location))
            .display()
            .to_string()
    }
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
    fn collection_filter_accepts_matching_item() {
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
  "collection": "usgs-test",
  "assets": {"data": {"href": "simple_text.ply"}}
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", item.display());
        options.add("collections", "usgs-.*");
        let mut reader = StacReader::new(&options);

        assert_eq!(reader.read().unwrap()[0].len(), 3);
    }

    #[test]
    fn collection_filter_rejects_nonmatching_item() {
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
  "collection": "usgs-test",
  "assets": {"data": {"href": "simple_text.ply"}}
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", item.display());
        options.add("collections", "no-match");
        let mut reader = StacReader::new(&options);

        assert!(reader.read().is_err());
    }

    #[test]
    fn collection_filter_rejects_invalid_regex() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            br#"{"type":"Feature","collection":"x","assets":{"data":{"href":"x.las"}}}"#,
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        options.add("collections", "[");
        let mut reader = StacReader::new(&options);

        let err = reader.read().err().unwrap();
        assert!(err.0.contains("Invalid collections regular expression"));
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
    fn collects_remote_asset_locations() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            br#"{"type":"Feature","assets":{"data":{"href":"http://example.com/x.copc.laz","type":"application/vnd.laszip+copc"}}}"#,
        )
        .unwrap();
        let mut visited = BTreeSet::new();
        let mut assets = Vec::new();
        let asset_names = [String::from("data")];
        let mut context = StacAssetContext {
            asset_names: &asset_names,
            item_filters: &[],
            collections: &[],
            property_filters: &[],
            validate_schema: false,
            visited: &mut visited,
            assets: &mut assets,
        };
        collect_assets(&temp.path().to_string_lossy(), &mut context).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].driver, "readers.copc");
        assert_eq!(assets[0].location, "http://example.com/x.copc.laz");
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
    fn ogr_bounds_filter_reads_geojson_feature_by_sql_id() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            br#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"id":1},"geometry":{"type":"Polygon","coordinates":[[[0,0],[0,1],[1,1],[1,0],[0,0]]]}},
    {"type":"Feature","properties":{"id":2},"geometry":{"type":"Polygon","coordinates":[[[50,-10],[50,0],[51,0],[51,-10],[50,-10]]]}}
  ]
}"#,
        )
        .unwrap();
        let ogr = format!(
            r#"{{"type":"ogr","datasource":"{}","sql":"select * from ogr_boundary WHERE id = 2"}}"#,
            temp.path().display()
        );
        let bounds = parse_ogr_bounds(&ogr).unwrap().unwrap();

        assert_eq!(bounds.minx, 50.0);
        assert_eq!(bounds.maxx, 51.0);
        assert_eq!(bounds.miny, -10.0);
        assert_eq!(bounds.maxy, 0.0);
    }

    #[test]
    fn ogr_bounds_filter_rejects_invalid_polygon() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            br#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"id":3},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,1]]]}}]}"#,
        )
        .unwrap();
        let ogr = format!(
            r#"{{"type":"ogr","datasource":"{}","sql":"select * from ogr_boundary WHERE id = 3"}}"#,
            temp.path().display()
        );

        assert!(parse_ogr_bounds(&ogr).is_err());
    }

    #[test]
    fn is_remote_detects_url_schemes() {
        assert!(is_remote("http://example.com/x"));
        assert!(is_remote("https://example.com/x"));
        assert!(!is_remote("/local/path.las"));
    }
}
