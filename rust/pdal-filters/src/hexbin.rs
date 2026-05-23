//! `filters.hexbin` -- hexagonal tessellation and point density.
//!
//! Port of `filters/HexBinFilter.cpp` and the `hexer` `HexGrid`. This is a
//! simplified port: it builds the hexagonal grid over the point's X/Y domain
//! and, when a `density` output path is given, writes the dense-cell
//! tessellation as GeoJSON polygons (one `Feature` per dense hexagon, carrying
//! `ID` and `COUNT` properties, mirroring PDAL's OGR density writer). The
//! boundary tessellation and the H3 grid are not modeled.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;

use pdal_core::metadata::MetadataNode;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

const SQRT_3: f64 = 1.732_050_808;

pub struct HexBinFilter {
    edge_length: Option<f64>,
    threshold: u32,
    sample_size: usize,
    density_output: Option<String>,
}

impl HexBinFilter {
    pub fn new(
        edge_length: Option<f64>,
        threshold: u32,
        sample_size: usize,
        density_output: Option<String>,
    ) -> Self {
        Self {
            edge_length,
            threshold,
            sample_size,
            density_output,
        }
    }
}

/// A tessellation of regular hexagons (one side parallel to the X axis),
/// accumulating a point count per hexagon.
struct HexGrid {
    /// Hexagon height (twice the apothem).
    height: f64,
    /// Hexagon column width.
    width: f64,
    /// Vertex offsets, anti-clockwise from the upper-left.
    offsets: [(f64, f64); 6],
    /// Grid origin -- the first point added defines hexagon (0, 0).
    origin: (f64, f64),
    has_origin: bool,
    /// Point count per `(i, j)` hexagon coordinate.
    counts: HashMap<(i32, i32), u32>,
}

impl HexGrid {
    fn new(height: f64) -> Self {
        let width = (3.0 / (2.0 * SQRT_3)) * height;
        let offsets = [
            (0.0, 0.0),
            (-width / 3.0, height / 2.0),
            (0.0, height),
            (2.0 * width / 3.0, height),
            (width, height / 2.0),
            (2.0 * width / 3.0, 0.0),
        ];
        Self {
            height,
            width,
            offsets,
            origin: (0.0, 0.0),
            has_origin: false,
            counts: HashMap::new(),
        }
    }

    fn add_point(&mut self, x: f64, y: f64) {
        let hex = self.find_hexagon(x, y);
        *self.counts.entry(hex).or_insert(0) += 1;
    }

    /// Map a point to the hexagon that contains it. Ported verbatim from
    /// `hexer::HexGrid::findHexagon`.
    fn find_hexagon(&mut self, x: f64, y: f64) -> (i32, i32) {
        if !self.has_origin {
            self.origin = (x, y);
            self.has_origin = true;
            return (0, 0);
        }

        let px = x - self.origin.0;
        let py = y - self.origin.1;
        let col = px / self.width;

        // Treat the columns as offset rectangles first; correct for the
        // overlapping "mini-column" strip below.
        let mut hx = col.floor() as i32;
        let mut hy = if hx % 2 == 0 {
            (py / self.height).floor() as i32
        } else {
            ((py - self.height / 2.0) / self.height).floor() as i32
        };

        let mut xcol_offset = col - col.floor();
        if xcol_offset > 2.0 / 3.0 {
            // Re-scale the offset to the width of the 1/3-wide mini-column.
            xcol_offset -= 2.0 / 3.0;
            xcol_offset *= 3.0;

            let halfrow = py / (self.height / 2.0);
            let halfy = halfrow as i32;
            let yrow_offset = halfrow - halfrow.floor();

            if (halfy % 2 == 0 && hx % 2 == 0) || (hx % 2 != 0 && halfy % 2 != 0) {
                // Negative slope case.
                if xcol_offset > yrow_offset {
                    if hx % 2 == 0 {
                        hy -= 1;
                    }
                    hx += 1;
                }
            } else {
                // Positive slope case.
                if yrow_offset > xcol_offset {
                    if hx % 2 != 0 {
                        hy += 1;
                    }
                    hx += 1;
                }
            }
        }

        (hx, hy)
    }

    /// Point coordinates of one vertex of a hexagon, identified by an edge
    /// index. Ported from `hexer::HexGrid::findPoint`.
    fn find_point(&self, hex: (i32, i32), edge: i32) -> (f64, f64) {
        let side = if edge - 1 < 0 { 5 } else { (edge - 1) as usize };
        let mut pos_y = hex.1 as f64 * self.height;
        if hex.0 % 2 != 0 {
            pos_y += self.height / 2.0;
        }
        let pos_x = hex.0 as f64 * self.width;
        let (ox, oy) = self.offsets[side];
        (pos_x + ox + self.origin.0, pos_y + oy + self.origin.1)
    }

    /// Stable 64-bit identifier for a hexagon, matching `hexer`'s `getID`.
    fn id(hex: (i32, i32)) -> u64 {
        (((hex.0 as i64) as u64) << 32) | ((hex.1 as u32) as u64)
    }

    /// Serialize the dense hexagons as a GeoJSON `FeatureCollection`.
    fn density_geojson(&self, dense_limit: u32) -> String {
        let mut dense: Vec<(&(i32, i32), &u32)> = self
            .counts
            .iter()
            .filter(|(_, &count)| count >= dense_limit)
            .collect();
        dense.sort_by_key(|(hex, _)| **hex);

        let mut out = String::from("{\n  \"type\": \"FeatureCollection\",\n  \"features\": [");
        for (idx, (hex, count)) in dense.iter().enumerate() {
            if idx > 0 {
                out.push(',');
            }
            out.push_str("\n    {\"type\": \"Feature\", \"properties\": {\"ID\": ");
            let _ = write!(out, "{}", Self::id(**hex));
            let _ = write!(out, ", \"COUNT\": {count}}}, \"geometry\": ");
            out.push_str("{\"type\": \"Polygon\", \"coordinates\": [[");
            // Edges 0..=5, then back to 0 to close the ring.
            for edge in 0..=6 {
                let (vx, vy) = self.find_point(**hex, edge % 6);
                if edge > 0 {
                    out.push_str(", ");
                }
                let _ = write!(out, "[{vx}, {vy}]");
            }
            out.push_str("]]}}");
        }
        out.push_str("\n  ]\n}\n");
        out
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

        let height = match self.edge_length {
            Some(edge) => edge * SQRT_3,
            None => compute_hex_size(&xy, self.sample_size, self.threshold)?,
        };
        if height <= 0.0 || !height.is_finite() {
            return Err(StageError(
                "filters.hexbin: unable to determine a hex size; set 'edge_length'.".to_string(),
            ));
        }

        let mut grid = HexGrid::new(height);
        for &(x, y) in &xy {
            grid.add_point(x, y);
        }

        if let Some(path) = &self.density_output {
            let geojson = grid.density_geojson(self.threshold);
            fs::write(path, geojson).map_err(|err| {
                StageError(format!(
                    "filters.hexbin: unable to write density output '{path}': {err}"
                ))
            })?;
        }

        // hexbin is a pass-through filter: it writes side files but leaves the
        // point stream untouched.
        Ok(vec![input.clone()])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("filters.hexbin")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
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

    #[test]
    fn first_point_defines_the_origin_hexagon() {
        let mut grid = HexGrid::new(3.0_f64.sqrt());
        assert_eq!(grid.find_hexagon(100.0, 200.0), (0, 0));
        assert_eq!(grid.origin, (100.0, 200.0));
    }

    #[test]
    fn dense_cells_are_emitted_as_geojson_features() {
        let mut grid = HexGrid::new(10.0);
        // Pile every point into the origin hexagon.
        for _ in 0..20 {
            grid.add_point(0.0, 0.0);
        }
        let geojson = grid.density_geojson(15);
        let parsed: serde_json::Value = serde_json::from_str(&geojson).unwrap();
        assert_eq!(parsed["type"], "FeatureCollection");
        let features = parsed["features"].as_array().unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0]["properties"]["COUNT"], 20);
        let ring = features[0]["geometry"]["coordinates"][0]
            .as_array()
            .unwrap();
        assert_eq!(ring.len(), 7);
    }

    #[test]
    fn sparse_cells_are_dropped() {
        let mut grid = HexGrid::new(10.0);
        grid.add_point(0.0, 0.0);
        let parsed: serde_json::Value = serde_json::from_str(&grid.density_geojson(15)).unwrap();
        assert!(parsed["features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn compute_hex_size_with_sufficient_points() {
        let xy = vec![(0.0, 0.0), (3.0, 4.0)];
        let size = compute_hex_size(&xy, 10, 5).unwrap();
        assert!(size > 0.0);
    }

    #[test]
    fn compute_hex_size_not_enough_points_is_error() {
        let xy = vec![(1.0, 2.0)];
        assert!(compute_hex_size(&xy, 10, 5).is_err());
    }

    #[test]
    fn compute_hex_size_empty_input_is_error() {
        let xy: Vec<(f64, f64)> = vec![];
        assert!(compute_hex_size(&xy, 10, 5).is_err());
    }

    #[test]
    fn hex_grid_new_sets_defaults() {
        let grid = HexGrid::new(10.0);
        assert!(!grid.has_origin);
        assert_eq!(grid.origin, (0.0, 0.0));
        assert!(grid.counts.is_empty());
        assert!((grid.height - 10.0).abs() < 0.001);
    }

    #[test]
    fn hex_grid_id_combines_coordinates() {
        assert_eq!(HexGrid::id((0, 0)), 0);
        let id = HexGrid::id((1, 2));
        assert_eq!(id >> 32, 1);
        assert_eq!(id as u32, 2);
    }

    #[test]
    fn hex_grid_find_point_returns_coordinates() {
        let grid = HexGrid::new(SQRT_3);
        let pt = grid.find_point((0, 0), 0);
        assert!((pt.0 - 1.0).abs() < 0.001);
        assert!((pt.1 - 0.0).abs() < 0.001);
    }

    #[test]
    fn multiple_points_map_to_different_hexagons() {
        let mut grid = HexGrid::new(10.0);
        grid.add_point(0.0, 0.0);
        grid.add_point(0.0, 0.0);
        assert_eq!(*grid.counts.get(&(0, 0)).unwrap(), 2);

        grid.add_point(100.0, 50.0);
        assert_eq!(grid.counts.len(), 2);
    }

    #[test]
    fn hexbin_filter_name() {
        let filter = HexBinFilter::new(Some(1.0), 5, 10, None);
        assert_eq!(filter.name(), "filters.hexbin");
    }

    #[test]
    fn hexbin_filter_metadata() {
        let filter = HexBinFilter::new(Some(1.0), 5, 10, None);
        let meta = filter.metadata();
        assert_eq!(meta.name(), "filters.hexbin");
    }

    #[test]
    fn hexbin_streamable_process_one_passes_through() {
        let mut filter = HexBinFilter::new(Some(1.0), 5, 10, None);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(std::rc::Rc::new(layout));
        let pt = view.add_point();
        assert!(filter.process_one(&mut view, pt));
    }

    #[test]
    fn hexbin_empty_input_is_error() {
        let mut filter = HexBinFilter::new(Some(1.0), 5, 10, None);
        let layout = PointLayout::new();
        let view = PointView::new(std::rc::Rc::new(layout));
        assert!(filter.run_one(&view).is_err());
    }

    #[test]
    fn hexbin_zero_height_is_error() {
        let mut filter = HexBinFilter::new(Some(0.0), 5, 10, None);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        let mut view = PointView::new(std::rc::Rc::new(layout));
        view.add_point();
        view.set_f64(0, &DimId::X, 1.0);
        view.set_f64(0, &DimId::Y, 2.0);
        assert!(filter.run_one(&view).is_err());
    }

    #[test]
    fn hexbin_default_height_with_fewer_points_than_sample() {
        let mut filter = HexBinFilter::new(None, 5, 100, None);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        let mut view = PointView::new(std::rc::Rc::new(layout));
        for (x, y) in &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)] {
            let pt = view.add_point();
            view.set_f64(pt, &DimId::X, *x);
            view.set_f64(pt, &DimId::Y, *y);
        }
        let result = filter.run_one(&view).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 3);
    }

    #[test]
    fn hexbin_passes_through_points() {
        let mut filter = HexBinFilter::new(Some(10.0), 5, 10, None);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(std::rc::Rc::new(layout));
        for (x, y, z) in &[(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)] {
            let pt = view.add_point();
            view.set_f64(pt, &DimId::X, *x);
            view.set_f64(pt, &DimId::Y, *y);
            view.set_f64(pt, &DimId::Z, *z);
        }
        let result = filter.run_one(&view).unwrap();
        assert_eq!(result.len(), 1);
        let output = &result[0];
        assert_eq!(output.len(), 2);
        assert_eq!(output.get_f64(0, &DimId::Z), 3.0);
        assert_eq!(output.get_f64(1, &DimId::X), 4.0);
    }
}
