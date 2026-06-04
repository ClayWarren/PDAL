//! `filters.hexbin` -- hexagonal tessellation, density, and boundary.
//!
//! Port of `filters/HexBinFilter.cpp` and the `hexer` `HexGrid`. The Rust
//! filter computes the hexagonal binning over the input's X/Y domain and
//! exposes:
//!
//! - `threshold`, `sample_size`, `edge_length`, `estimated_edge`, `hex_offsets`
//!   metadata (pass-through values + computed grid geometry);
//! - the raw `hex_boundary` MULTIPOLYGON WKT emitted by hexer (when
//!   `output_tesselation`);
//! - the unsmoothed boundary as `hex_boundary_raw` so the C++ wrapper can
//!   apply `pdal::Polygon::simplify` to produce the user-facing `boundary`
//!   metadata using the existing GEOS path;
//! - a GeoJSON density tessellation when `density` output is requested.

use std::fmt::Write as _;
use std::fs;

use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use pdal_native::gdal::{VectorFieldType, VectorFieldValue, VectorPointWriter};

use std::f64::consts::PI;

use crate::hexer::{h3_resolution_from_height, H3Grid, HexGrid, HexId, SQRT_3};
use h3o::{LatLng, Resolution};

pub struct HexBinFilter {
    edge_length: Option<f64>,
    threshold: u32,
    sample_size: usize,
    density_output: Option<String>,
    boundary_output: Option<String>,
    driver_name: String,
    layer_name: String,
    output_tesselation: bool,
    /// H3 mode. `None` = standard hexagonal grid. `Some(None)` = H3 with the
    /// resolution auto-estimated from a sample. `Some(Some(res))` = H3 at a
    /// fixed resolution. Mirrors HexBinFilter's `h3_grid`/`h3_resolution`.
    h3: Option<Option<u8>>,
    fixed_origin: Option<(f64, f64)>,
    /// Built grid + outputs. Populated by `run_one`; consumed by
    /// `metadata()`.
    state: Option<HexBinState>,
}

struct HexBinState {
    height: f64,
    width: f64,
    offsets: [(f64, f64); 6],
    hex_boundary_wkt: Option<String>,
    point_count: u64,
    dense_hex_count: u64,
    dense_point_count: u64,
    /// `sample_size` to report in metadata. When the edge length is estimated
    /// and the input has fewer points than the requested sample size, hexer
    /// runs out of sample points and reports the actual count instead, matching
    /// `HexBinFilter`/`BaseGrid::computeHexSize`.
    effective_sample_size: u64,
    /// Present only for H3 grids: the resolution used. When set, `metadata()`
    /// emits `h3_resolution` instead of the standard-grid `estimated_edge`/
    /// `hex_offsets`/`hex_width`.
    h3_resolution: Option<u8>,
}

impl HexBinFilter {
    pub fn new(
        edge_length: Option<f64>,
        threshold: u32,
        sample_size: usize,
        density_output: Option<String>,
    ) -> Self {
        Self::with_options(
            edge_length,
            threshold,
            sample_size,
            density_output,
            None,
            false,
        )
    }

    pub fn with_options(
        edge_length: Option<f64>,
        threshold: u32,
        sample_size: usize,
        density_output: Option<String>,
        boundary_output: Option<String>,
        output_tesselation: bool,
    ) -> Self {
        Self {
            edge_length,
            threshold,
            sample_size,
            density_output,
            boundary_output,
            driver_name: "GeoJSON".to_string(),
            layer_name: "hexbins".to_string(),
            output_tesselation,
            h3: None,
            fixed_origin: None,
            state: None,
        }
    }

    pub fn set_output_driver(&mut self, driver_name: impl Into<String>) {
        self.driver_name = driver_name.into();
    }

    pub fn set_layer_name(&mut self, layer_name: impl Into<String>) {
        let layer_name = layer_name.into();
        self.layer_name = if layer_name.is_empty() {
            "hexbins".to_string()
        } else {
            layer_name
        };
    }

    /// Enable H3 grid mode. `resolution` is `None` for auto-estimation or
    /// `Some(res)` for a fixed H3 resolution.
    pub fn set_h3(&mut self, resolution: Option<u8>) {
        self.h3 = Some(resolution);
    }

    pub fn set_origin(&mut self, x: f64, y: f64) {
        self.fixed_origin = Some((x, y));
    }

    /// `sample_size` to report: clamped to the point count when the size was
    /// estimated from a sample smaller than the requested sample size.
    fn effective_sample_size(&self, point_count: usize, estimated: bool) -> u64 {
        if estimated && point_count < self.sample_size {
            point_count as u64
        } else {
            self.sample_size as u64
        }
    }

    /// Standard hexagonal grid path (`hexer::HexGrid`).
    fn run_standard(&self, xy: &[(f64, f64)]) -> Result<HexBinState, StageError> {
        let estimated = self.edge_length.is_none();
        let height = match self.edge_length {
            Some(edge) => edge * SQRT_3,
            None => compute_hex_size(xy, self.sample_size, self.threshold)?,
        };
        if height <= 0.0 || !height.is_finite() {
            return Err(StageError(
                "filters.hexbin: unable to determine a hex size; set 'edge_length'.".to_string(),
            ));
        }

        let mut grid = HexGrid::with_height(height, self.threshold as i32);
        if let Some((x, y)) = self.fixed_origin {
            grid.set_origin(x, y);
        }
        for &(x, y) in xy {
            grid.add_xy(x, y);
        }

        if let Some(path) = &self.density_output {
            self.write_density_standard(path, &grid)?;
        }

        // Trace the boundary. If there's no dense region, hexer returns an
        // error and we leave hex_boundary unset so the C++ wrapper can emit
        // `MULTIPOLYGON EMPTY` (matching the failure path).
        // Fixed precision-8 matches C++ HexGrid::toWKT (OStringStreamClassicLocale
        // + std::fixed). HexBinFilter re-parses hex_boundary_raw and smooths it
        // through GEOS, so byte-identical raw WKT yields a byte-identical boundary.
        let hex_boundary_wkt = if grid.find_shapes().is_ok() {
            grid.find_parent_paths();
            if let Some(path) = &self.boundary_output {
                self.write_boundary(path, &grid.boundary_geojson(), &grid.to_wkt_fixed(8))?;
            }
            Some(grid.to_wkt_fixed(8))
        } else {
            None
        };
        let (dense_hex_count, dense_point_count) = dense_stats(grid.counts(), self.threshold);

        let o = grid.offsets();
        let offsets = [
            (o[0].x, o[0].y),
            (o[1].x, o[1].y),
            (o[2].x, o[2].y),
            (o[3].x, o[3].y),
            (o[4].x, o[4].y),
            (o[5].x, o[5].y),
        ];

        Ok(HexBinState {
            height: grid.height(),
            width: grid.width(),
            offsets,
            hex_boundary_wkt,
            point_count: xy.len() as u64,
            dense_hex_count,
            dense_point_count,
            effective_sample_size: self.effective_sample_size(xy.len(), estimated),
            h3_resolution: None,
        })
    }

    /// H3 grid path (`hexer::H3Grid`). `xy` is `(lng, lat)` in degrees.
    fn run_h3(&self, xy: &[(f64, f64)], resolution: Option<u8>) -> Result<HexBinState, StageError> {
        let estimated = resolution.is_none();
        let res = match resolution {
            Some(r) => r,
            None => {
                // Mirror H3Grid: the sample distance is computed on radian
                // coordinates (addXY stores degsToRads(x/y) before
                // computeHexSize), then mapped to a resolution.
                let rad: Vec<(f64, f64)> = xy
                    .iter()
                    .map(|&(x, y)| (x * PI / 180.0, y * PI / 180.0))
                    .collect();
                let height_rad = compute_hex_size(&rad, self.sample_size, self.threshold)?;
                h3_resolution_from_height(height_rad).map_err(StageError)?
            }
        };
        let res_enum = Resolution::try_from(res)
            .map_err(|err| StageError(format!("filters.hexbin: {err}")))?;

        // The origin cell (from the first point) fixes the local IJ frame.
        let (lng0, lat0) = xy[0];
        let origin = LatLng::new(lat0, lng0)
            .map_err(|err| StageError(format!("filters.hexbin: invalid origin lat/lng: {err}")))?
            .to_cell(res_enum);
        let mut grid = H3Grid::new(res, self.threshold as i32, origin).map_err(StageError)?;
        for &(lng, lat) in xy {
            grid.add_lat_lng(lat, lng).map_err(StageError)?;
        }

        if let Some(path) = &self.density_output {
            self.write_density_h3(path, &grid)?;
        }

        // H3 grids are not smoothed (HexBinFilter forbids smoothing options for
        // H3), so the boundary precision only feeds GEOS WKT reformatting.
        let hex_boundary_wkt = if grid.find_shapes().is_ok() {
            grid.find_parent_paths();
            if let Some(path) = &self.boundary_output {
                self.write_boundary(path, &grid.boundary_geojson(), &grid.to_wkt(8))?;
            }
            Some(grid.to_wkt(8))
        } else {
            None
        };
        let (dense_hex_count, dense_point_count) = dense_stats(grid.counts(), self.threshold);

        Ok(HexBinState {
            height: 0.0,
            width: 0.0,
            offsets: [(0.0, 0.0); 6],
            hex_boundary_wkt,
            point_count: xy.len() as u64,
            dense_hex_count,
            dense_point_count,
            effective_sample_size: self.effective_sample_size(xy.len(), estimated),
            h3_resolution: Some(res),
        })
    }

    fn writes_geojson(&self) -> bool {
        self.driver_name.eq_ignore_ascii_case("GeoJSON")
    }

    fn create_polygon_writer(&self, path: &str) -> Result<VectorPointWriter, StageError> {
        let writer =
            VectorPointWriter::create_polygon(path, &self.driver_name, "", &self.layer_name)
                .map_err(|err| {
                    StageError(format!(
                        "filters.hexbin: unable to create OGR output '{path}': {err}"
                    ))
                })?;
        writer
            .create_field("ID", VectorFieldType::Integer64)
            .map_err(|err| {
                StageError(format!("filters.hexbin: unable to create ID field: {err}"))
            })?;
        writer
            .create_field("COUNT", VectorFieldType::Integer)
            .map_err(|err| {
                StageError(format!(
                    "filters.hexbin: unable to create COUNT field: {err}"
                ))
            })?;
        Ok(writer)
    }

    fn write_density_standard(&self, path: &str, grid: &HexGrid) -> Result<(), StageError> {
        if self.writes_geojson() {
            fs::write(path, density_geojson(grid, self.threshold)).map_err(|err| {
                StageError(format!(
                    "filters.hexbin: unable to write density output '{path}': {err}"
                ))
            })?;
            return Ok(());
        }

        let writer = self.create_polygon_writer(path)?;
        for (hex, count) in dense_hexes(grid.counts(), self.threshold) {
            let ring: Vec<_> = grid.hex_vertices(*hex).iter().map(|p| (p.x, p.y)).collect();
            writer
                .write_polygon(
                    &ring,
                    &[
                        VectorFieldValue::Integer64(HexGrid::hex_id_u64(*hex) as i64),
                        VectorFieldValue::Integer(*count),
                    ],
                )
                .map_err(|err| {
                    StageError(format!(
                        "filters.hexbin: unable to write density feature: {err}"
                    ))
                })?;
        }
        Ok(())
    }

    fn write_density_h3(&self, path: &str, grid: &H3Grid) -> Result<(), StageError> {
        if self.writes_geojson() {
            fs::write(path, density_geojson_h3(grid, self.threshold)?).map_err(|err| {
                StageError(format!(
                    "filters.hexbin: unable to write density output '{path}': {err}"
                ))
            })?;
            return Ok(());
        }

        let writer = self.create_polygon_writer(path)?;
        for (hex, count) in dense_hexes(grid.counts(), self.threshold) {
            let ring: Vec<_> = grid
                .hex_vertices(*hex)
                .map_err(|err| StageError(format!("filters.hexbin: {err}")))?
                .iter()
                .map(|p| (p.x, p.y))
                .collect();
            writer
                .write_polygon(
                    &ring,
                    &[
                        VectorFieldValue::Integer64(HexGrid::hex_id_u64(*hex) as i64),
                        VectorFieldValue::Integer(*count),
                    ],
                )
                .map_err(|err| {
                    StageError(format!(
                        "filters.hexbin: unable to write density feature: {err}"
                    ))
                })?;
        }
        Ok(())
    }

    fn write_boundary(
        &self,
        path: &str,
        geojson: &str,
        boundary_wkt: &str,
    ) -> Result<(), StageError> {
        if self.writes_geojson() {
            fs::write(path, geojson).map_err(|err| {
                StageError(format!(
                    "filters.hexbin: unable to write boundary output '{path}': {err}"
                ))
            })?;
            return Ok(());
        }

        let writer = self.create_polygon_writer(path)?;
        writer
            .write_geometry_wkt(
                boundary_wkt,
                &[VectorFieldValue::Integer64(0), VectorFieldValue::Integer(0)],
            )
            .map_err(|err| {
                StageError(format!(
                    "filters.hexbin: unable to write boundary feature: {err}"
                ))
            })
    }
}

/// Estimate a hexagon height from the average spacing of the leading sample,
/// mirroring `hexer::BaseGrid::computeHexSize`.
fn compute_hex_size(
    xy: &[(f64, f64)],
    sample_size: usize,
    dense_limit: u32,
) -> Result<f64, StageError> {
    let count = xy.len().min(sample_size.max(1));
    if count < 2 {
        return Err(StageError(
            "filters.hexbin: not enough points to estimate 'edge_length'; set it explicitly."
                .to_string(),
        ));
    }
    let mut dist = 0.0;
    for window in xy[..count].windows(2) {
        let dx = window[0].0 - window[1].0;
        let dy = window[0].1 - window[1].1;
        dist += (dx * dx + dy * dy).sqrt();
    }
    Ok(dense_limit as f64 * dist / count as f64)
}

/// Serialize the dense hexagons as a GeoJSON `FeatureCollection`, matching
/// the C++ OGR density writer's per-cell `ID`/`COUNT` schema.
fn density_geojson(grid: &HexGrid, dense_limit: u32) -> String {
    let mut out = String::from("{\n  \"type\": \"FeatureCollection\",\n  \"features\": [");
    for (idx, (hex, count)) in dense_hexes(grid.counts(), dense_limit).iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str("\n    {\"type\": \"Feature\", \"properties\": {\"ID\": ");
        let _ = write!(out, "{}", HexGrid::hex_id_u64(**hex));
        let _ = write!(out, ", \"COUNT\": {count}}}, \"geometry\": ");
        out.push_str("{\"type\": \"Polygon\", \"coordinates\": [[");
        let verts = grid.hex_vertices(**hex);
        for (vi, v) in verts.iter().enumerate() {
            if vi > 0 {
                out.push_str(", ");
            }
            let _ = write!(out, "[{}, {}]", v.x, v.y);
        }
        out.push_str("]]}}");
    }
    out.push_str("\n  ]\n}\n");
    out
}

fn density_geojson_h3(grid: &H3Grid, dense_limit: u32) -> Result<String, StageError> {
    let mut out = String::from("{\n  \"type\": \"FeatureCollection\",\n  \"features\": [");
    for (idx, (hex, count)) in dense_hexes(grid.counts(), dense_limit).iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str("\n    {\"type\": \"Feature\", \"properties\": {\"ID\": ");
        let _ = write!(out, "{}", HexGrid::hex_id_u64(**hex));
        let _ = write!(out, ", \"COUNT\": {count}}}, \"geometry\": ");
        out.push_str("{\"type\": \"Polygon\", \"coordinates\": [[");
        let verts = grid
            .hex_vertices(**hex)
            .map_err(|err| StageError(format!("filters.hexbin: {err}")))?;
        for (vi, v) in verts.iter().enumerate() {
            if vi > 0 {
                out.push_str(", ");
            }
            let _ = write!(out, "[{}, {}]", v.x, v.y);
        }
        out.push_str("]]}}");
    }
    out.push_str("\n  ]\n}\n");
    Ok(out)
}

fn dense_hexes(
    counts: &std::collections::HashMap<HexId, i32>,
    dense_limit: u32,
) -> Vec<(&HexId, &i32)> {
    let mut dense: Vec<_> = counts
        .iter()
        .filter(|(_, &count)| count as u32 >= dense_limit)
        .collect();
    dense.sort_by_key(|(hex, _)| **hex);
    dense
}

fn format_hex_offsets(offsets: &[(f64, f64); 6]) -> String {
    let mut out = String::from("MULTIPOINT (");
    for (i, (x, y)) in offsets.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{x} {y}");
    }
    out.push(')');
    out
}

impl Filter for HexBinFilter {
    fn name(&self) -> &str {
        "filters.hexbin"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let xy: Vec<(f64, f64)> = (0..input.len())
            .map(|i| (input.get_f64(i, &DimId::X), input.get_f64(i, &DimId::Y)))
            .collect();
        if xy.is_empty() {
            return Err(StageError(
                "filters.hexbin: input has no points.".to_string(),
            ));
        }

        self.state = Some(match self.h3 {
            Some(resolution) => self.run_h3(&xy, resolution)?,
            None => self.run_standard(&xy)?,
        });

        // hexbin is a pass-through filter: it writes side files and computes
        // metadata but leaves the point stream untouched.
        Ok(vec![input.clone()])
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("filters.hexbin");
        node.add_value("threshold", MetadataValue::U64(self.threshold as u64));
        let sample_size = self
            .state
            .as_ref()
            .map(|s| s.effective_sample_size)
            .unwrap_or(self.sample_size as u64);
        node.add_value("sample_size", MetadataValue::U64(sample_size));
        node.add_value(
            "edge_length",
            MetadataValue::F64(self.edge_length.unwrap_or(0.0)),
        );
        if let Some(state) = &self.state {
            if let Some(res) = state.h3_resolution {
                // H3 grids report resolution instead of the standard-grid
                // estimated_edge / hex_offsets / hex_width.
                node.add_value("h3_resolution", MetadataValue::I64(res as i64));
            } else {
                node.add_value("estimated_edge", MetadataValue::F64(state.height));
                node.add_value(
                    "hex_offsets",
                    MetadataValue::String(format_hex_offsets(&state.offsets)),
                );
                // Convenience for the C++ wrapper: hex width helps compute area
                // bookkeeping without re-deriving from height.
                node.add_value("hex_width", MetadataValue::F64(state.width));
            }
            node.add_value("point_count", MetadataValue::U64(state.point_count));
            // hex_boundary_raw is the unsmoothed MULTIPOLYGON; the C++ side
            // applies Polygon::simplify to produce the user `boundary`. We
            // also emit the same WKT as `hex_boundary` when
            // `output_tesselation` is requested, matching the C++ filter.
            if let Some(wkt) = &state.hex_boundary_wkt {
                node.add_value("hex_boundary_raw", MetadataValue::String(wkt.clone()));
                if self.output_tesselation {
                    node.add_value("hex_boundary", MetadataValue::String(wkt.clone()));
                }
                node.add_value("dense_hex_count", MetadataValue::U64(state.dense_hex_count));
                node.add_value(
                    "dense_point_count",
                    MetadataValue::U64(state.dense_point_count),
                );
            } else {
                node.add_value(
                    "hex_boundary_raw",
                    MetadataValue::String("MULTIPOLYGON EMPTY".to_string()),
                );
            }
        }
        node
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn dense_stats(counts: &std::collections::HashMap<HexId, i32>, threshold: u32) -> (u64, u64) {
    counts
        .values()
        .filter(|&&count| count as u32 >= threshold)
        .fold((0, 0), |(hexes, points), &count| {
            (hexes + 1, points + count as u64)
        })
}

impl Streamable for HexBinFilter {
    /// hexbin needs the whole view to build its grid; it has no streaming
    /// mode, so a streamed point is passed through unchanged.
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use pdal_native::gdal::Vector;
    use std::rc::Rc;

    fn flat_view(n: usize) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for i in 0..n {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, i as f64 * 0.5);
            view.set_f64(id, &DimId::Y, i as f64 * 0.5);
            view.set_f64(id, &DimId::Z, 0.0);
        }
        view
    }

    /// A tightly-clustered geographic point cloud (lng = X, lat = Y, degrees)
    /// suitable for the H3 path.
    fn geo_view(n: usize) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for i in 0..n {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, -74.044 + i as f64 * 1e-5);
            view.set_f64(id, &DimId::Y, 40.689 + i as f64 * 1e-5);
            view.set_f64(id, &DimId::Z, 0.0);
        }
        view
    }

    #[test]
    fn name_and_metadata_root() {
        let f = HexBinFilter::new(Some(1.0), 1, 10, None);
        assert_eq!(f.name(), "filters.hexbin");
        let m = f.metadata();
        assert_eq!(m.name(), "filters.hexbin");
    }

    #[test]
    fn h3_fixed_resolution_metadata() {
        let view = geo_view(30);
        let mut f = HexBinFilter::new(None, 1, 5000, None);
        f.set_h3(Some(10));
        f.run_one(&view).unwrap();
        let m = f.metadata();
        let res = m
            .find_child("h3_resolution")
            .and_then(|c| c.value().map(|v| v.as_i64()))
            .expect("h3_resolution");
        assert_eq!(res, 10);
        // Standard-grid metadata must not appear for an H3 grid.
        assert!(m.find_child("estimated_edge").is_none());
        assert!(m.find_child("hex_offsets").is_none());
    }

    #[test]
    fn h3_density_output_writes_geojson_features() {
        let view = geo_view(30);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h3-density.geojson");
        let mut f = HexBinFilter::new(None, 1, 5000, Some(path.display().to_string()));
        f.set_h3(Some(10));

        f.run_one(&view).unwrap();

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(json["type"], "FeatureCollection");
        let features = json["features"].as_array().unwrap();
        assert!(!features.is_empty());
        assert!(features[0]["properties"]["COUNT"].as_i64().unwrap() > 0);
        assert_eq!(features[0]["geometry"]["type"], "Polygon");
    }

    #[test]
    fn h3_auto_resolution_clamps_sample_size() {
        let view = geo_view(20);
        let mut f = HexBinFilter::new(None, 1, 5000, None);
        f.set_h3(None);
        f.run_one(&view).unwrap();
        let m = f.metadata();
        // Auto resolution must resolve to some H3 level.
        assert!(m.find_child("h3_resolution").is_some());
        // Fewer points than the sample size -> reported sample size is the count.
        let sample = m
            .find_child("sample_size")
            .and_then(|c| c.value().map(|v| v.as_u64()))
            .expect("sample_size");
        assert_eq!(sample, 20);
    }

    #[test]
    fn empty_input_errors() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let view = PointView::new(Rc::new(layout));
        let mut f = HexBinFilter::new(Some(1.0), 1, 10, None);
        let err = match f.run_one(&view) {
            Ok(_) => panic!("expected empty input to fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("no points"));
    }

    #[test]
    fn run_populates_grid_state_and_boundary_metadata() {
        let view = flat_view(50);
        let mut f = HexBinFilter::new(Some(1.0), 1, 10, None);
        f.run_one(&view).unwrap();
        let m = f.metadata();
        // The boundary should be non-empty for a non-trivial input.
        let raw = m.find_child("hex_boundary_raw").expect("hex_boundary_raw");
        assert!(raw
            .value()
            .map(|v| v.as_string().starts_with("MULTIPOLYGON ("))
            .unwrap_or(false));
        assert!(m.find_child("estimated_edge").is_some());
        assert!(m.find_child("hex_offsets").is_some());
    }

    #[test]
    fn no_dense_region_emits_empty_multipolygon() {
        // A 1-point input with threshold=2 -> no dense hex -> no boundary
        let view = flat_view(1);
        let mut f = HexBinFilter::new(Some(1.0), 2, 10, None);
        f.run_one(&view).unwrap();
        let m = f.metadata();
        let raw = m.find_child("hex_boundary_raw").unwrap();
        assert_eq!(
            raw.value().map(|v| v.as_string()).unwrap_or_default(),
            "MULTIPOLYGON EMPTY"
        );
    }

    #[test]
    fn output_tesselation_emits_hex_boundary() {
        let view = flat_view(50);
        let mut f = HexBinFilter::with_options(Some(1.0), 1, 10, None, None, true);
        f.run_one(&view).unwrap();
        let m = f.metadata();
        assert!(m.find_child("hex_boundary").is_some());
    }

    #[test]
    fn boundary_output_writes_geojson_multipolygon() {
        let view = flat_view(50);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boundary.geojson");
        let mut f = HexBinFilter::with_options(
            Some(1.0),
            1,
            10,
            None,
            Some(path.display().to_string()),
            false,
        );

        f.run_one(&view).unwrap();

        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(json["type"], "FeatureCollection");
        let features = json["features"].as_array().unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0]["properties"]["ID"], 0);
        assert_eq!(features[0]["geometry"]["type"], "MultiPolygon");
    }

    #[test]
    fn non_geojson_density_and_boundary_use_native_polygon_writer() {
        let view = flat_view(50);
        let dir = tempfile::tempdir().unwrap();
        let density = dir.path().join("density.gpkg");
        let boundary = dir.path().join("boundary.gpkg");

        let mut density_filter = HexBinFilter::with_options(
            Some(1.0),
            1,
            10,
            Some(density.display().to_string()),
            None,
            false,
        );
        density_filter.set_output_driver("GPKG");
        density_filter.run_one(&view).unwrap();

        let density_vector = Vector::open(density.to_str().unwrap()).unwrap();
        let density_features = density_vector.get_features(0, "COUNT").unwrap();
        assert!(!density_features.is_empty());
        assert!(density_features
            .iter()
            .all(|(wkt, count)| wkt.contains("POLYGON") && *count >= 1));
        drop(density_vector);

        let mut boundary_filter = HexBinFilter::with_options(
            Some(1.0),
            1,
            10,
            None,
            Some(boundary.display().to_string()),
            false,
        );
        boundary_filter.set_output_driver("GPKG");
        boundary_filter.set_layer_name("custom_hexbins");
        boundary_filter.run_one(&view).unwrap();

        let boundary_vector = Vector::open(boundary.to_str().unwrap()).unwrap();
        let boundary_features = boundary_vector.get_features(0, "ID").unwrap();
        assert_eq!(boundary_features.len(), 1);
        assert!(boundary_features[0].0.contains("MULTIPOLYGON"));
        assert_eq!(boundary_features[0].1, 0);
    }
}
