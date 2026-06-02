//! `readers.stac` -- fixture-scoped STAC asset reader.
//!
//! This handles local and covered remote STAC Item/Catalog/Collection/
//! FeatureCollection traversal, then dispatches assets through already-ported
//! readers. Date, item/catalog/collection, property, bbox, and GeoJSON
//! OGR-boundary filters are intentionally narrow and fixture-backed. Full JSON
//! schema resolution, broad remote traversal, and threaded catalog crawling
//! remain outside this module's current contract.

use crate::tindex::{append_view, read_point_location};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::PointView;
use pdal_core::stage::StageError;
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub struct StacReader {
    filename: String,
    asset_names: Vec<String>,
    items: Vec<String>,
    catalogs: Vec<String>,
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
            catalogs: comma_values(options, "catalogs"),
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
        let catalog_filters = compile_regexes(&self.catalogs, "catalogs")?;
        let collection_filters = compile_regexes(&self.collections, "collections")?;
        let property_filters = parse_property_filters(&self.properties)?;
        let date_ranges = parse_date_ranges(&self.date_ranges)?;
        let bounds = if self.ogr.trim().is_empty() {
            parse_bounds(&self.bounds)?
        } else {
            parse_ogr_bounds(&self.ogr)?
        };
        let mut context = StacPreviewContext {
            asset_names: &self.asset_names,
            item_filters: &item_filters,
            catalog_filters: &catalog_filters,
            collection_filters: &collection_filters,
            property_filters: &property_filters,
            date_ranges: &date_ranges,
            bounds: bounds.as_ref(),
            validate_schema: self.validate_schema,
            visited: &mut visited,
            preview: &mut preview,
            root: true,
        };
        collect_preview(&self.filename, &mut context)?;
        if preview.item_ids.is_empty()
            && (!item_filters.is_empty()
                || !catalog_filters.is_empty()
                || !collection_filters.is_empty()
                || !property_filters.is_empty()
                || !date_ranges.is_empty()
                || bounds.is_some())
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
        let catalogs = compile_regexes(&self.catalogs, "catalogs")?;
        let item_filters = compile_regexes(&self.items, "items")?;
        let property_filters = parse_property_filters(&self.properties)?;
        let reader_args = parse_reader_args(&self.reader_args)?;
        let date_ranges = parse_date_ranges(&self.date_ranges)?;
        let bounds = if self.ogr.trim().is_empty() {
            parse_bounds(&self.bounds)?
        } else {
            parse_ogr_bounds(&self.ogr)?
        };
        let mut context = StacAssetContext {
            asset_names: &self.asset_names,
            item_filters: &item_filters,
            date_ranges: &date_ranges,
            bounds: bounds.as_ref(),
            catalogs: &catalogs,
            collections: &collections,
            property_filters: &property_filters,
            validate_schema: self.validate_schema,
            visited: &mut visited,
            assets: &mut assets,
            root: true,
        };
        collect_assets(&self.filename, &mut context)?;

        if assets.is_empty()
            && (!self.catalogs.is_empty()
                || !self.collections.is_empty()
                || !item_filters.is_empty()
                || !property_filters.is_empty()
                || !date_ranges.is_empty()
                || bounds.is_some())
        {
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
        Some("Catalog") => {
            if !context.root && !catalog_matches(&json, context.catalogs) {
                return Ok(());
            }
            collect_linked_items(&json, &base, context)
        }
        Some("Collection") => collect_linked_items(&json, &base, context),
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
    date_ranges: &'a [DateRange],
    bounds: Option<&'a Bounds2D>,
    catalogs: &'a [Regex],
    collections: &'a [Regex],
    property_filters: &'a [PropertyFilter],
    validate_schema: bool,
    visited: &'a mut BTreeSet<String>,
    assets: &'a mut Vec<StacAsset>,
    root: bool,
}

fn collect_item_assets(
    item: &Value,
    base: &str,
    context: &mut StacAssetContext<'_>,
) -> Result<(), StageError> {
    if !item_matches_id_filters(item, context.item_filters) {
        return Ok(());
    }
    if !context.date_ranges.is_empty() && !item_matches_dates(item, context.date_ranges) {
        return Ok(());
    }
    if let Some(bounds) = context.bounds {
        if !item_matches_bounds(item, bounds) {
            return Ok(());
        }
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
        if !matches!(rel, "item" | "child" | "catalog" | "collection" | "next") {
            continue;
        }
        let Some(href) = link["href"].as_str() else {
            continue;
        };
        let root = context.root;
        context.root = false;
        collect_assets(&resolve_stac_link(base, href), context)?;
        context.root = root;
    }
    Ok(())
}

struct StacPreviewContext<'a> {
    asset_names: &'a [String],
    item_filters: &'a [Regex],
    catalog_filters: &'a [Regex],
    collection_filters: &'a [Regex],
    property_filters: &'a [PropertyFilter],
    date_ranges: &'a [DateRange],
    bounds: Option<&'a Bounds2D>,
    validate_schema: bool,
    visited: &'a mut BTreeSet<String>,
    preview: &'a mut StacPreview,
    root: bool,
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
            collect_item_preview(&json, context)?;
            Ok(())
        }
        Some("Catalog") | Some("Collection") | Some("FeatureCollection") => {
            if json["type"].as_str() == Some("Catalog")
                && !context.root
                && !catalog_matches(&json, context.catalog_filters)
            {
                return Ok(());
            }
            if let Some(id) = json["id"].as_str() {
                match json["type"].as_str() {
                    Some("Catalog") => context.preview.catalog_ids.push(id.to_string()),
                    Some("Collection") => context.preview.collection_ids.push(id.to_string()),
                    _ => {}
                }
            }
            if let Some(features) = json["features"].as_array() {
                for feature in features {
                    collect_item_preview(feature, context)?;
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
                    let root = context.root;
                    context.root = false;
                    if is_remote(href) {
                        collect_preview(href, context)?;
                        context.root = root;
                        continue;
                    }
                    collect_preview(&resolve_stac_link(&base, href), context)?;
                    context.root = root;
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
    context: &mut StacPreviewContext<'_>,
) -> Result<(), StageError> {
    if !item_has_requested_asset(item, context.asset_names) {
        return Ok(());
    }
    let Some(id) = item["id"].as_str() else {
        return Ok(());
    };
    if !context.item_filters.is_empty()
        && !context.item_filters.iter().any(|regex| regex.is_match(id))
    {
        return Ok(());
    }
    if !context.date_ranges.is_empty() && !item_matches_dates(item, context.date_ranges) {
        return Ok(());
    }
    if let Some(bounds) = context.bounds {
        if !item_matches_bounds(item, bounds) {
            return Ok(());
        }
    }
    if !collection_matches(item, context.collection_filters) {
        return Ok(());
    }
    if !item_matches_property_filters(item, context.property_filters)? {
        return Ok(());
    }
    context.preview.item_ids.push(id.to_string());
    if let Some(count) = item["properties"]["pc:count"].as_u64() {
        context.preview.point_count += count;
    }
    Ok(())
}

mod filters;
#[cfg(test)]
mod tests;

use filters::*;
