//! `readers.copc` -- local + remote COPC reader.
//!
//! `read()` still reuses the LAS/LAZ reader (with VSI byte-range support for
//! http(s)/`/vsi*` paths) for full-file materialization. `preview()` walks the
//! COPC hierarchy directly, applying 2D/3D bounds and resolution pruning to
//! match the C++ `CopcReader::inspect()` semantics for the remote/sample-mode
//! use case.

use std::fs::File;
use std::io::{BufReader, Cursor};

use crate::copc_hierarchy::{
    self, CopcInfo, CopcPreview, LasBounds, QueryBounds as HierarchyBounds,
};
use crate::las::LasReader;
use pdal_core::bounds::{parse_bounds2d, parse_bounds3d, Bounds2D, Bounds3D};
use pdal_core::geometry::Geometry;
use pdal_core::metadata::MetadataNode;
use pdal_core::ogr_spec::parse_ogr_spec_json;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::StageError;
use pdal_native::srs::{user_input_to_wkt, GdalSrsTransform};
use serde_json::Value;

pub struct CopcReader {
    inner: LasReader,
    filename: String,
    bounds: String,
    polygons: Vec<String>,
    polygon_srs: Vec<String>,
    ogr: String,
    source_srs: String,
    resolution: f64,
    metadata: MetadataNode,
}

impl CopcReader {
    pub fn new(options: &Options) -> Self {
        Self {
            inner: LasReader::new(options),
            filename: options.get_str("filename", ""),
            bounds: options.get_str("bounds", ""),
            polygons: option_values(options, "polygon"),
            polygon_srs: option_values(options, "polygon_srs"),
            ogr: options.get_str("ogr", ""),
            source_srs: options.get_str("source_srs", ""),
            resolution: options.get_f64("resolution", 0.0),
            metadata: MetadataNode::new("readers.copc"),
        }
    }

    /// Hierarchy-driven preview: returns (point_count, bounds[6]) after
    /// applying the bounds and resolution options. Mirrors the C++
    /// `CopcReader::inspect()` count/bbox behavior for sample-mode queries.
    pub fn preview(&self) -> Result<CopcPreview, StageError> {
        let query = self
            .bounds_filter()
            .map_err(|e| StageError(e.0))?
            .map(|b| match b {
                QueryBounds::Two(b) => HierarchyBounds::Two(b),
                QueryBounds::Three(b) => HierarchyBounds::Three(b),
            });
        let resolution = self.resolution;
        with_byte_source(&self.filename, |reader| {
            let (info, full_bounds) = copc_hierarchy::read_copc_info(reader)?;
            copc_hierarchy::walk_preview(reader, &info, full_bounds, query.as_ref(), resolution)
        })
        .map_err(StageError)
    }

    pub fn copc_info(&self) -> Result<(CopcInfo, LasBounds), StageError> {
        with_byte_source(&self.filename, |reader| {
            copc_hierarchy::read_copc_info(reader)
        })
        .map_err(StageError)
    }
}

impl Reader for CopcReader {
    fn name(&self) -> &str {
        "readers.copc"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        let bounds = self.bounds_filter()?;
        let polygons = self.polygon_filters()?;
        if self.resolution > 0.0 {
            return self.read_resolution_limited(bounds, polygons);
        }
        let views = self
            .inner
            .read()?
            .into_iter()
            .map(|view| apply_bounds(view, bounds.as_ref()))
            .map(|view| apply_polygons(view, &polygons))
            .filter(|view| !view.is_empty())
            .collect();
        self.metadata = self.inner.metadata();
        Ok(views)
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

impl CopcReader {
    fn read_resolution_limited(
        &mut self,
        bounds: Option<QueryBounds>,
        polygons: Vec<PolygonFilter>,
    ) -> Result<Vec<PointView>, StageError> {
        // Walk hierarchy collecting leaf entries that pass the resolution
        // and (optional) bounds filter.
        let hierarchy_bounds = bounds.as_ref().map(|b| match b {
            QueryBounds::Two(b) => HierarchyBounds::Two(*b),
            QueryBounds::Three(b) => HierarchyBounds::Three(*b),
        });
        let entries = {
            let mut source = open_byte_source(&self.filename).map_err(StageError)?;
            let (info, _) = copc_hierarchy::read_copc_info(&mut *source).map_err(StageError)?;
            copc_hierarchy::walk_entries(
                &mut *source,
                &info,
                hierarchy_bounds.as_ref(),
                self.resolution,
            )
            .map_err(StageError)?
        };
        if entries.is_empty() {
            self.metadata = self.inner.metadata();
            return Ok(Vec::new());
        }

        // Read the LAZ chunk table to map each entry's file offset to a
        // (start_point_idx, count) range we can hand to `LasReader::read_ranges`.
        let locators = read_chunk_locators_for_filename(&self.filename).map_err(StageError)?;
        let by_offset: std::collections::HashMap<u64, copc_hierarchy::ChunkLocator> =
            locators.iter().map(|loc| (loc.file_offset, *loc)).collect();
        let mut ranges: Vec<(u64, u64)> = Vec::with_capacity(entries.len());
        for entry in &entries {
            let Some(loc) = by_offset.get(&entry.offset) else {
                return Err(StageError(format!(
                    "COPC entry at offset {} not found in chunk table",
                    entry.offset
                )));
            };
            ranges.push((loc.point_start, loc.point_count));
        }
        ranges.sort_by_key(|(s, _)| *s);

        let view = self.inner.read_ranges(&ranges)?;
        self.metadata = self.inner.metadata();

        // Post-filter by polygons (bounds were already applied via hierarchy
        // pruning, but the COPC voxel can extend slightly past the query box;
        // re-apply for exact dataset-coordinate parity).
        let view = apply_bounds(view, bounds.as_ref());
        let view = apply_polygons(view, &polygons);
        if view.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![view])
        }
    }

    fn bounds_filter(&self) -> Result<Option<QueryBounds>, StageError> {
        if self.bounds.is_empty() {
            return Ok(None);
        }
        if let Ok(parsed) = parse_bounds3d(&self.bounds, 0) {
            return Ok(Some(QueryBounds::Three(parsed.bounds)));
        }
        parse_bounds2d(&self.bounds, 0)
            .map(|parsed| Some(QueryBounds::Two(parsed.bounds)))
            .map_err(|err| StageError(format!("Invalid COPC bounds option: {err}")))
    }

    fn polygon_filters(&self) -> Result<Vec<PolygonFilter>, StageError> {
        let mut filters: Vec<PolygonFilter> = self
            .polygons
            .iter()
            .enumerate()
            .map(|(idx, wkt)| {
                let geometry = Geometry::from_wkt(wkt).map_err(StageError)?;
                let polygon_srs = self.polygon_srs.get(idx).map_or("", String::as_str);
                let transform = polygon_transform(&self.source_srs, polygon_srs)?;
                Ok(PolygonFilter {
                    geometry,
                    transform,
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

struct PolygonFilter {
    geometry: Geometry,
    transform: Option<GdalSrsTransform>,
}

impl PolygonFilter {
    fn contains(&self, mut x: f64, mut y: f64, mut z: f64) -> bool {
        if let Some(transform) = &self.transform {
            if !transform.transform_xyz(&mut x, &mut y, &mut z) {
                return false;
            }
        }
        self.geometry.contains(x, y)
    }
}

fn polygon_transform(
    source_srs: &str,
    polygon_srs: &str,
) -> Result<Option<GdalSrsTransform>, StageError> {
    if source_srs.trim().is_empty() || polygon_srs.trim().is_empty() {
        return Ok(None);
    }
    if source_srs == polygon_srs {
        return Ok(None);
    }
    let source_wkt = user_input_to_wkt(source_srs).map_err(StageError)?.wkt;
    let polygon_wkt = user_input_to_wkt(polygon_srs).map_err(StageError)?.wkt;
    if source_wkt == polygon_wkt {
        return Ok(None);
    }
    GdalSrsTransform::new(&source_wkt, 0.0, &polygon_wkt, 0.0, &[], &[])
        .map(Some)
        .map_err(StageError)
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
}

fn is_vsi_path(filename: &str) -> bool {
    filename.starts_with("/vsi")
        || filename.starts_with("http://")
        || filename.starts_with("https://")
}

fn vsi_path(filename: &str) -> String {
    if filename.starts_with("http://") || filename.starts_with("https://") {
        format!("/vsicurl/{filename}")
    } else {
        filename.to_string()
    }
}

fn open_byte_source(filename: &str) -> Result<Box<dyn ReadSeek>, String> {
    if filename.is_empty() {
        return Err("COPC: missing filename option".to_string());
    }
    if is_vsi_path(filename) {
        let vsi_path = vsi_path(filename);
        let vsi = pdal_native::vsi::VsiFile::open(&vsi_path)
            .map_err(|e| format!("COPC: failed to open VSI path {vsi_path}: {e}"))?;
        Ok(Box::new(vsi))
    } else {
        let file =
            File::open(filename).map_err(|e| format!("COPC: failed to open {filename}: {e}"))?;
        Ok(Box::new(BufReader::new(file)))
    }
}

fn read_chunk_locators_for_filename(
    filename: &str,
) -> Result<Vec<copc_hierarchy::ChunkLocator>, String> {
    if is_vsi_path(filename) {
        let path = vsi_path(filename);
        let mut first = pdal_native::vsi::VsiFile::open(&path)
            .map_err(|e| format!("COPC: failed to open VSI path {path}: {e}"))?;
        let mut second = pdal_native::vsi::VsiFile::open(&path)
            .map_err(|e| format!("COPC: failed to open VSI path {path}: {e}"))?;
        let len = second
            .len()
            .map_err(|e| format!("COPC: failed to size VSI path {path}: {e}"))?;
        let data = second
            .read_exact_at(0, len as usize)
            .map_err(|e| format!("COPC: failed to read VSI path {path}: {e}"))?;
        copc_hierarchy::read_chunk_locators_from(&mut first, Cursor::new(data))
    } else {
        copc_hierarchy::read_chunk_locators(std::path::Path::new(filename))
    }
}

fn with_byte_source<T>(
    filename: &str,
    mut f: impl FnMut(&mut dyn ReadSeek) -> Result<T, String>,
) -> Result<T, String> {
    let mut source = open_byte_source(filename)?;
    f(&mut *source)
}

/// Combined Read+Seek so we can pass either `BufReader<File>` or `VsiFile`
/// through a single trait object.
pub trait ReadSeek: std::io::Read + std::io::Seek {}
impl<T: std::io::Read + std::io::Seek + ?Sized> ReadSeek for T {}

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

fn apply_polygons(view: PointView, polygons: &[PolygonFilter]) -> PointView {
    if polygons.is_empty() {
        return view;
    }
    let mut output = view.make_new();
    for idx in 0..view.len() {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);
        if polygons.iter().any(|polygon| polygon.contains(x, y, z)) {
            output.append_point(&view, idx);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn data_path(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data")
            .join(path)
    }

    /// Network smoke test mirroring the C++ `pdal_io_copc_remote_reader_test`:
    /// hits the canonical autzen-classified COPC over https and `/vsicurl/`
    /// and expects the 61201/2D-bounds preview after bounds + resolution
    /// pruning. Ignored by default to keep CI hermetic.
    #[test]
    #[ignore = "network smoke for remote COPC preview"]
    fn preview_remote_autzen_matches_cpp_expectation() {
        for filename in [
            "https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz",
            "/vsicurl/https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz",
        ] {
            let mut options = Options::new();
            options.add("filename", filename.to_string());
            options.add("bounds", "([635700,637000],[848900,853300])");
            options.add("resolution", 1000.0_f64);
            let reader = CopcReader::new(&options);
            let preview = reader.preview().unwrap();
            assert_eq!(preview.point_count, 61_201, "url={filename}");
            assert!(preview.bounds.min_x >= 635_700.0);
            assert!(preview.bounds.max_x <= 637_000.0);
            assert!(preview.bounds.min_y >= 848_900.0);
            assert!(preview.bounds.max_y <= 853_300.0);
        }
    }

    #[test]
    fn preview_unfiltered_local_matches_full_count() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        let reader = CopcReader::new(&options);
        let preview = reader.preview().unwrap();
        assert_eq!(preview.point_count, 518_862);
    }

    #[test]
    fn preview_with_2d_bounds_prunes_count_below_full() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        options.add("bounds", "([515380,515400],[4918350,4918370])");
        let reader = CopcReader::new(&options);
        let preview = reader.preview().unwrap();
        assert!(preview.point_count > 0);
        assert!(preview.point_count < 518_862);
        assert!(preview.bounds.min_x >= 515380.0);
        assert!(preview.bounds.max_x <= 515400.0);
    }

    #[test]
    fn reads_local_copc_with_las_path() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        let mut reader = CopcReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 518862);
    }

    #[test]
    fn applies_2d_bounds_filter() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        options.add("bounds", "([515380,515400],[4918350,4918370])");
        let mut reader = CopcReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 354211);
        for idx in 0..views[0].len() {
            let x = views[0].get_f64(idx, &DimId::X);
            let y = views[0].get_f64(idx, &DimId::Y);
            assert!((515380.0..=515400.0).contains(&x));
            assert!((4918350.0..=4918370.0).contains(&y));
        }
    }

    #[test]
    fn applies_3d_bounds_filter() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        options.add("bounds", "([515380,515400],[4918350,4918370],[2320,2325])");
        let mut reader = CopcReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 45930);
        for idx in 0..views[0].len() {
            let x = views[0].get_f64(idx, &DimId::X);
            let y = views[0].get_f64(idx, &DimId::Y);
            let z = views[0].get_f64(idx, &DimId::Z);
            assert!((515380.0..=515400.0).contains(&x));
            assert!((4918350.0..=4918370.0).contains(&y));
            assert!((2320.0..=2325.0).contains(&z));
        }
    }

    #[test]
    fn applies_reprojected_polygon_filter() {
        let source_srs = std::fs::read_to_string(data_path("autzen/autzen-srs.wkt")).unwrap();
        let polygon = std::fs::read_to_string(data_path("autzen/autzen-selection-dd.wkt")).unwrap();
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("copc/1.2-with-color.copc.laz").display(),
        );
        options.add("source_srs", source_srs);
        options.add("polygon", polygon);
        options.add("polygon_srs", "EPSG:4326");
        let mut reader = CopcReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert!((40..=50).contains(&views[0].len()));
    }
}
