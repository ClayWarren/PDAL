//! `filters.smrf` -- Simple Morphological Filter (Pingel et al., 2013).
//!
//! Port of `filters/SMRFilter.cpp`. The Rust path covers the simple-case the
//! C++ wrapper now delegates: no debug `dir`, no `ignored` DimRanges, no
//! synthetic/keypoint/withheld `classbits` filter. It performs the full
//! algorithm: minimum-Z grid, low-outlier mask, net cutting, progressive
//! morphological opening, KD-tree inpainting, gradient-scaled threshold, and
//! per-point ground/object classification.

use crate::math;
use pdal_core::metadata::MetadataNode;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use rstar::primitives::GeomWithData;
use rstar::RTree;
use std::collections::HashSet;

const VALID_RETURNS: &[&str] = &["first", "intermediate", "last", "only"];

pub struct SmrfFilter {
    cell: f64,
    slope: f64,
    window: Option<f64>,
    scalar: f64,
    threshold: f64,
    cut: f64,
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
    returns: HashSet<String>,
}

impl SmrfFilter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cell: f64,
        slope: f64,
        window: Option<f64>,
        scalar: f64,
        threshold: f64,
        ground_class: u8,
        other_class: u8,
        only_ground: bool,
        returns: Vec<String>,
    ) -> Self {
        Self::with_cut(
            cell,
            slope,
            window,
            scalar,
            threshold,
            0.0,
            ground_class,
            other_class,
            only_ground,
            returns,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_cut(
        cell: f64,
        slope: f64,
        window: Option<f64>,
        scalar: f64,
        threshold: f64,
        cut: f64,
        ground_class: u8,
        other_class: u8,
        only_ground: bool,
        returns: Vec<String>,
    ) -> Self {
        Self {
            cell,
            slope,
            window,
            scalar,
            threshold,
            cut,
            ground_class,
            other_class,
            only_ground,
            returns: returns.into_iter().map(|r| r.trim().to_string()).collect(),
        }
    }

    fn validate(&self) -> Result<(), StageError> {
        if self.cell <= 0.0 || !self.cell.is_finite() {
            return Err(StageError(
                "filters.smrf: 'cell' must be a positive finite value.".to_string(),
            ));
        }
        if self.slope < 0.0 || !self.slope.is_finite() {
            return Err(StageError(
                "filters.smrf: 'slope' must be a non-negative finite value.".to_string(),
            ));
        }
        if self.scalar < 0.0 || !self.scalar.is_finite() {
            return Err(StageError(
                "filters.smrf: 'scalar' must be a non-negative finite value.".to_string(),
            ));
        }
        if self.threshold < 0.0 || !self.threshold.is_finite() {
            return Err(StageError(
                "filters.smrf: 'threshold' must be a non-negative finite value.".to_string(),
            ));
        }
        if self.cut < 0.0 || !self.cut.is_finite() {
            return Err(StageError(
                "filters.smrf: 'cut' must be a non-negative finite value.".to_string(),
            ));
        }
        if let Some(window) = self.window {
            if window <= 0.0 || !window.is_finite() {
                return Err(StageError(
                    "filters.smrf: 'window' must be a positive finite value.".to_string(),
                ));
            }
        }
        if !self.only_ground && self.ground_class == self.other_class {
            return Err(StageError(
                "filters.smrf: Ground and non-ground class cannot be equal when \
                 only_ground is false."
                    .to_string(),
            ));
        }
        for r in &self.returns {
            if !VALID_RETURNS.contains(&r.as_str()) {
                return Err(StageError(format!(
                    "filters.smrf: Unrecognized 'returns' value: '{r}'."
                )));
            }
        }
        Ok(())
    }

    /// Fill NaN cells with the running mean of their K=8 nearest filled
    /// neighbors by 2D cell-center distance, matching C++ `knnfill`.
    fn knn_fill(&self, data: &mut [f64], rows: usize, cols: usize, minx: f64, miny: f64) {
        // Build an R-tree over filled cell centers.
        let mut entries: Vec<GeomWithData<[f64; 2], usize>> = Vec::new();
        for c in 0..cols {
            for r in 0..rows {
                let idx = c * rows + r;
                if data[idx].is_nan() {
                    continue;
                }
                let x = minx + (c as f64 + 0.5) * self.cell;
                let y = miny + (r as f64 + 0.5) * self.cell;
                entries.push(GeomWithData::new([x, y], idx));
            }
        }
        if entries.is_empty() {
            return;
        }
        let tree = RTree::bulk_load(entries);

        let mut updates: Vec<(usize, f64)> = Vec::new();
        for c in 0..cols {
            for r in 0..rows {
                let idx = c * rows + r;
                if !data[idx].is_nan() {
                    continue;
                }
                let x = minx + (c as f64 + 0.5) * self.cell;
                let y = miny + (r as f64 + 0.5) * self.cell;
                let mut m1 = 0.0;
                let mut j = 0u64;
                for nn in tree.nearest_neighbor_iter(&[x, y]).take(8) {
                    j += 1;
                    let delta = data[nn.data] - m1;
                    m1 += delta / j as f64;
                }
                if j > 0 {
                    updates.push((idx, m1));
                }
            }
        }
        for (idx, val) in updates {
            data[idx] = val;
        }
    }

    /// Progressive morphological opening; marks cells whose surface drops by
    /// more than the slope-scaled threshold as non-ground (object) cells.
    fn progressive_filter(
        &self,
        zimin: &[f64],
        rows: usize,
        cols: usize,
        slope: f64,
        max_window: f64,
    ) -> Vec<u8> {
        let max_radius = (max_window / self.cell).ceil() as usize;
        let mut prev_surface = zimin.to_vec();
        let mut erosion = zimin.to_vec();
        let mut obj = vec![0u8; rows * cols];

        for radius in 1..=max_radius {
            math::erode_diamond(&mut erosion, rows, cols, 1);
            let mut cur_opening = erosion.clone();
            math::dilate_diamond(&mut cur_opening, rows, cols, radius);

            let threshold = slope * self.cell * radius as f64;
            for i in 0..obj.len() {
                if (prev_surface[i] - cur_opening[i]).abs() > threshold {
                    obj[i] = 1;
                }
            }
            prev_surface = cur_opening;
        }
        obj
    }

    /// Net mask: when `cut > 0`, every v-th row/column (v = ceil(cut/cell)) is
    /// flagged so its ZImin value is replaced by a wide morphological opening.
    fn net_mask(&self, rows: usize, cols: usize) -> Vec<u8> {
        let mut mask = vec![0u8; rows * cols];
        if self.cut <= 0.0 {
            return mask;
        }
        let v = (self.cut / self.cell).ceil() as usize;
        if v == 0 {
            return mask;
        }
        let mut c = 0;
        while c < cols {
            for r in 0..rows {
                mask[c * rows + r] = 1;
            }
            c += v;
        }
        for c in 0..cols {
            let mut r = 0;
            while r < rows {
                mask[c * rows + r] = 1;
                r += v;
            }
        }
        mask
    }

    fn apply_net(&self, zimin: &[f64], rows: usize, cols: usize, is_net: &[u8]) -> Vec<f64> {
        let mut zinet = zimin.to_vec();
        if self.cut <= 0.0 {
            return zinet;
        }
        let v = (self.cut / self.cell).ceil() as usize;
        if v == 0 {
            return zinet;
        }
        let mut dilated = zimin.to_vec();
        math::erode_diamond(&mut dilated, rows, cols, 2 * v);
        math::dilate_diamond(&mut dilated, rows, cols, 2 * v);
        for i in 0..zinet.len() {
            if is_net[i] == 1 {
                zinet[i] = dilated[i];
            }
        }
        zinet
    }

    /// Low-outlier mask: invert Z and run a one-cell-window progressive filter
    /// with slope 5.0; cells flagged here are also dropped from ZIpro.
    fn low_mask(&self, zimin: &[f64], rows: usize, cols: usize) -> Vec<u8> {
        let neg: Vec<f64> = zimin
            .iter()
            .map(|v| if v.is_nan() { f64::NAN } else { -v })
            .collect();
        self.progressive_filter(&neg, rows, cols, 5.0, self.cell)
    }
}

/// Choose inlier point ids based on optional return-number filtering.
///
/// Returns the inlier id list, or an error if NumberOfReturns/ReturnNumber
/// values mix zero and non-zero entries. When all values are zero, return
/// filtering is silently dropped and every point becomes an inlier.
fn select_inliers(view: &PointView, returns: &HashSet<String>) -> Result<Vec<PointId>, StageError> {
    let has_returns = view.layout().dim(&DimId::ReturnNumber).is_some()
        && view.layout().dim(&DimId::NumberOfReturns).is_some();
    if !has_returns || returns.is_empty() {
        return Ok((0..view.len()).collect());
    }

    let mut nr_has_zero = false;
    let mut rn_has_zero = false;
    let mut nr_all_zero = true;
    let mut rn_all_zero = true;
    for i in 0..view.len() {
        let nr = view.get_f64(i, &DimId::NumberOfReturns) as i32;
        let rn = view.get_f64(i, &DimId::ReturnNumber) as i32;
        if nr == 0 {
            nr_has_zero = true;
        } else {
            nr_all_zero = false;
        }
        if rn == 0 {
            rn_has_zero = true;
        } else {
            rn_all_zero = false;
        }
    }
    if (nr_has_zero || rn_has_zero) && !(nr_all_zero && rn_all_zero) {
        return Err(StageError(
            "filters.smrf: Some NumberOfReturns or ReturnNumber values were 0, but \
             not all. Check that all values in the input file are >= 1."
                .to_string(),
        ));
    }
    if nr_all_zero && rn_all_zero {
        return Ok((0..view.len()).collect());
    }

    let mut inliers = Vec::new();
    for i in 0..view.len() {
        let rn = view.get_f64(i, &DimId::ReturnNumber) as i32;
        let nr = view.get_f64(i, &DimId::NumberOfReturns) as i32;
        let mut keep = false;
        if returns.contains("last") && rn == nr && nr > 0 {
            keep = true;
        }
        if returns.contains("first") && rn == 1 {
            keep = true;
        }
        if returns.contains("only") && rn == 1 && nr == 1 {
            keep = true;
        }
        if returns.contains("intermediate") && rn > 1 && rn < nr {
            keep = true;
        }
        if keep {
            inliers.push(i);
        }
    }
    Ok(inliers)
}

impl Filter for SmrfFilter {
    fn name(&self) -> &str {
        "filters.smrf"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.validate()?;

        let inlier_ids = select_inliers(input, &self.returns)?;
        if inlier_ids.is_empty() {
            return Err(StageError(
                "filters.smrf: No returns to process.".to_string(),
            ));
        }

        // Compute bounds from the inlier subset, matching the C++ wrapper.
        let mut minx = f64::INFINITY;
        let mut miny = f64::INFINITY;
        let mut maxx = f64::NEG_INFINITY;
        let mut maxy = f64::NEG_INFINITY;
        for &id in &inlier_ids {
            let x = input.get_f64(id, &DimId::X);
            let y = input.get_f64(id, &DimId::Y);
            if x < minx {
                minx = x;
            }
            if x > maxx {
                maxx = x;
            }
            if y < miny {
                miny = y;
            }
            if y > maxy {
                maxy = y;
            }
        }
        if !minx.is_finite() || !miny.is_finite() {
            return Err(StageError(
                "filters.smrf: input has no points.".to_string(),
            ));
        }
        let cols = ((maxx - minx) / self.cell) as usize + 1;
        let rows = ((maxy - miny) / self.cell) as usize + 1;
        let window = self.window.unwrap_or(18.0 * self.cell);

        let grid_cell = |x: f64, y: f64| -> usize {
            let c = (((x - minx) / self.cell).floor() as usize).min(cols - 1);
            let r = (((y - miny) / self.cell).floor() as usize).min(rows - 1);
            c * rows + r
        };

        // Pre-classify inliers as other_class when !only_ground, so cells that
        // end up NaN still get a deterministic classification.
        let mut out = input.clone();
        if !self.only_ground {
            for &id in &inlier_ids {
                out.set_f64(id, &DimId::Classification, self.other_class as f64);
            }
        }

        // Step 1: minimum-Z grid (ZImin), filled with KD-tree neighbors.
        let mut zimin = vec![f64::NAN; rows * cols];
        for &id in &inlier_ids {
            let x = input.get_f64(id, &DimId::X);
            let y = input.get_f64(id, &DimId::Y);
            let z = input.get_f64(id, &DimId::Z);
            let idx = grid_cell(x, y);
            if zimin[idx].is_nan() || z < zimin[idx] {
                zimin[idx] = z;
            }
        }
        self.knn_fill(&mut zimin, rows, cols, minx, miny);

        // Step 2: low-outlier mask via inverted-Z progressive filter.
        let low = self.low_mask(&zimin, rows, cols);

        // Step 3: net cutting (no-op when cut == 0).
        let is_net = self.net_mask(rows, cols);
        let zinet = self.apply_net(&zimin, rows, cols, &is_net);

        // Step 4: object mask via main progressive filter on ZInet.
        let obj = self.progressive_filter(&zinet, rows, cols, self.slope, window);

        // Step 5: provisional DEM — strip object/low/net cells from ZImin and
        // inpaint the resulting voids.
        let mut zipro = zimin.clone();
        for i in 0..zipro.len() {
            if obj[i] == 1 || low[i] == 1 || is_net[i] == 1 {
                zipro[i] = f64::NAN;
            }
        }
        self.knn_fill(&mut zipro, rows, cols, minx, miny);

        // Step 6: gradient magnitude of ZIpro/cell, inpainted to fill edges.
        let scaled_zipro: Vec<f64> = zipro.iter().map(|z| z / self.cell).collect();
        let gx = math::grad_x(&scaled_zipro, rows, cols);
        let gy = math::grad_y(&scaled_zipro, rows, cols);
        let mut gsurfs: Vec<f64> = gx
            .iter()
            .zip(gy.iter())
            .map(|(x, y)| (x * x + y * y).sqrt())
            .collect();
        self.knn_fill(&mut gsurfs, rows, cols, minx, miny);

        // Step 7: classify each inlier point against the provisional DEM.
        for &id in &inlier_ids {
            let x = input.get_f64(id, &DimId::X);
            let y = input.get_f64(id, &DimId::Y);
            let z = input.get_f64(id, &DimId::Z);
            let cell = grid_cell(x, y);
            if zipro[cell].is_nan() || gsurfs[cell].is_nan() {
                continue;
            }
            let thresh = self.threshold + self.scalar * gsurfs[cell];
            if (zipro[cell] - z).abs() > thresh {
                if !self.only_ground {
                    out.set_f64(id, &DimId::Classification, self.other_class as f64);
                }
            } else {
                out.set_f64(id, &DimId::Classification, self.ground_class as f64);
            }
        }
        Ok(vec![out])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::Classification, DimType::U8)]
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("filters.smrf")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for SmrfFilter {
    /// SMRF needs the whole view to build its grid; it has no streaming mode,
    /// so a streamed point is passed through unchanged.
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn grid_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in &[
            (0.5, 0.5, 10.0),
            (0.5, 1.5, 12.0),
            (1.5, 0.5, 8.0),
            (1.5, 1.5, 11.0),
        ] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, *x);
            view.set_f64(id, &DimId::Y, *y);
            view.set_f64(id, &DimId::Z, *z);
        }
        view
    }

    fn grid_view_with_returns() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, rn, nr) in &[
            (0.5, 0.5, 10.0, 1, 1),
            (0.5, 1.5, 12.0, 1, 2),
            (1.5, 0.5, 8.0, 2, 2),
        ] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, *x);
            view.set_f64(id, &DimId::Y, *y);
            view.set_f64(id, &DimId::Z, *z);
            view.set_f64(id, &DimId::ReturnNumber, *rn as f64);
            view.set_f64(id, &DimId::NumberOfReturns, *nr as f64);
        }
        view
    }

    fn flat_3x3_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for cx in 0..3 {
            for cy in 0..3 {
                let id = view.add_point();
                view.set_f64(id, &DimId::X, cx as f64 * 2.0 + 0.5);
                view.set_f64(id, &DimId::Y, cy as f64 * 2.0 + 0.5);
                view.set_f64(id, &DimId::Z, 10.0);
            }
        }
        view
    }

    #[test]
    fn rejects_non_positive_cell_size() {
        let mut filter = SmrfFilter::new(0.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("cell"));
    }

    #[test]
    fn rejects_negative_slope() {
        let mut filter = SmrfFilter::new(1.0, -0.1, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("slope"));
    }

    #[test]
    fn rejects_negative_scalar() {
        let mut filter = SmrfFilter::new(1.0, 0.15, None, -1.0, 0.5, 2, 1, true, Vec::new());
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("scalar"));
    }

    #[test]
    fn rejects_negative_threshold() {
        let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, -0.5, 2, 1, true, Vec::new());
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("threshold"));
    }

    #[test]
    fn rejects_non_positive_window() {
        let mut filter = SmrfFilter::new(1.0, 0.15, Some(-1.0), 1.25, 0.5, 2, 1, true, Vec::new());
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("window"));
    }

    #[test]
    fn rejects_negative_cut() {
        let mut filter = SmrfFilter::with_cut(
            1.0,
            0.15,
            None,
            1.25,
            0.5,
            -1.0,
            2,
            1,
            true,
            Vec::new(),
        );
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("cut"));
    }

    #[test]
    fn rejects_equal_classes_when_not_only_ground() {
        let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 2, false, Vec::new());
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("class"));
    }

    #[test]
    fn rejects_unknown_returns_value() {
        let mut filter = SmrfFilter::new(
            1.0,
            0.15,
            None,
            1.25,
            0.5,
            2,
            1,
            true,
            vec!["middle".to_string()],
        );
        let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("Unrecognized 'returns'"));
    }

    #[test]
    fn rejects_empty_input() {
        let layout = PointLayout::new();
        let empty = PointView::new(Rc::new(layout));
        let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let err = filter.run_one(&empty).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("No returns"));
    }

    #[test]
    fn rejects_mixed_zero_and_nonzero_returns() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (rn, nr) in &[(1u8, 1u8), (0, 0), (1, 2)] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, 0.5);
            view.set_f64(id, &DimId::Y, 0.5);
            view.set_f64(id, &DimId::Z, 10.0);
            view.set_f64(id, &DimId::ReturnNumber, *rn as f64);
            view.set_f64(id, &DimId::NumberOfReturns, *nr as f64);
        }
        let mut filter = SmrfFilter::new(
            1.0,
            0.15,
            None,
            1.25,
            0.5,
            2,
            1,
            true,
            vec!["last".to_string()],
        );
        let err = filter.run_one(&view).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("NumberOfReturns or ReturnNumber"));
    }

    #[test]
    fn all_zero_returns_falls_back_to_all_points() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for _ in 0..4 {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, 0.5);
            view.set_f64(id, &DimId::Y, 0.5);
            view.set_f64(id, &DimId::Z, 10.0);
        }
        let mut filter = SmrfFilter::new(
            1.0,
            0.15,
            None,
            1.25,
            0.5,
            2,
            1,
            true,
            vec!["last".to_string()],
        );
        let result = filter.run_one(&view).unwrap();
        assert_eq!(result[0].len(), 4);
    }

    #[test]
    fn smrf_names() {
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        assert_eq!(filter.name(), "filters.smrf");
    }

    #[test]
    fn smrf_metadata() {
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let m = filter.metadata();
        assert_eq!(m.name(), "filters.smrf");
    }

    #[test]
    fn smrf_output_dimensions() {
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let dims = filter.output_dimensions();
        assert_eq!(dims, vec![(DimId::Classification, DimType::U8)]);
    }

    #[test]
    fn smrf_process_one_passes_through() {
        let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let mut view = grid_view();
        assert!(filter.process_one(&mut view, 0));
    }

    #[test]
    fn smrf_classifies_flat_ground() {
        let mut filter = SmrfFilter::new(2.0, 0.15, None, 0.5, 0.5, 2, 1, true, Vec::new());
        let result = filter.run_one(&flat_3x3_view()).unwrap();
        assert_eq!(result.len(), 1);
        for i in 0..result[0].len() {
            assert_eq!(result[0].get_f64(i, &DimId::Classification), 2.0);
        }
    }

    #[test]
    fn smrf_returns_filter_first_only() {
        let mut filter = SmrfFilter::new(
            1.0,
            0.15,
            None,
            0.5,
            0.5,
            2,
            1,
            true,
            vec!["first".to_string()],
        );
        let result = filter.run_one(&grid_view_with_returns()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].len() > 0);
    }

    #[test]
    fn smrf_returns_filter_last_only() {
        let mut filter = SmrfFilter::new(
            1.0,
            0.15,
            None,
            0.5,
            0.5,
            2,
            1,
            true,
            vec!["last".to_string()],
        );
        let result = filter.run_one(&grid_view_with_returns()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn smrf_returns_filter_only() {
        let mut filter = SmrfFilter::new(
            1.0,
            0.15,
            None,
            0.5,
            0.5,
            2,
            1,
            true,
            vec!["only".to_string()],
        );
        let result = filter.run_one(&grid_view_with_returns()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn smrf_returns_filter_intermediate() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (rn, nr) in &[(1u8, 3u8), (2, 3), (3, 3)] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, 0.5);
            view.set_f64(id, &DimId::Y, 0.5);
            view.set_f64(id, &DimId::Z, 10.0);
            view.set_f64(id, &DimId::ReturnNumber, *rn as f64);
            view.set_f64(id, &DimId::NumberOfReturns, *nr as f64);
        }
        let mut filter = SmrfFilter::new(
            1.0,
            0.15,
            None,
            0.5,
            0.5,
            2,
            1,
            true,
            vec!["intermediate".to_string()],
        );
        let result = filter.run_one(&view).unwrap();
        assert_eq!(result[0].len(), 3);
        // The intermediate return (rn=2, nr=3) should be classified ground.
        assert_eq!(result[0].get_f64(1, &DimId::Classification), 2.0);
        // First and last returns were not selected, so they keep their original
        // (zero) Classification.
        assert_eq!(result[0].get_f64(0, &DimId::Classification), 0.0);
        assert_eq!(result[0].get_f64(2, &DimId::Classification), 0.0);
    }

    #[test]
    fn pre_classifies_other_when_not_only_ground() {
        // An obvious object point well above the ground should end up as
        // other_class even though its 1x1 cell may have NaN gradient.
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        // 4x4 flat ground with one high spike.
        for cx in 0..4 {
            for cy in 0..4 {
                let id = view.add_point();
                view.set_f64(id, &DimId::X, cx as f64 + 0.5);
                view.set_f64(id, &DimId::Y, cy as f64 + 0.5);
                view.set_f64(id, &DimId::Z, 10.0);
            }
        }
        let spike = view.add_point();
        view.set_f64(spike, &DimId::X, 1.5);
        view.set_f64(spike, &DimId::Y, 1.5);
        view.set_f64(spike, &DimId::Z, 110.0);

        let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, false, Vec::new());
        let result = filter.run_one(&view).unwrap();
        // The spike must be classified as object (1), not ground.
        let cls = result[0].get_f64(spike, &DimId::Classification);
        assert_eq!(cls, 1.0, "spike point should be classified as other");
    }

    #[test]
    fn net_mask_off_when_cut_zero() {
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let mask = filter.net_mask(5, 5);
        assert!(mask.iter().all(|&v| v == 0));
    }

    #[test]
    fn net_mask_marks_grid_when_cut_positive() {
        let filter = SmrfFilter::with_cut(1.0, 0.15, None, 1.25, 0.5, 3.0, 2, 1, true, Vec::new());
        let mask = filter.net_mask(6, 6);
        // First column (c=0) is fully set; row 0 of every column is set.
        for r in 0..6 {
            assert_eq!(mask[r], 1);
        }
        for c in 0..6 {
            assert_eq!(mask[c * 6], 1);
        }
        // (c=1,r=1) should not be on the net.
        assert_eq!(mask[1 * 6 + 1], 0);
    }

    #[test]
    fn smrf_knn_fill_all_nan_stays_nan() {
        let mut data = vec![f64::NAN; 9];
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        filter.knn_fill(&mut data, 3, 3, 0.0, 0.0);
        for v in &data {
            assert!(v.is_nan());
        }
    }

    #[test]
    fn smrf_knn_fill_single_nan_uses_neighbors() {
        let mut data = vec![1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0, 7.0, 8.0, 9.0];
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        filter.knn_fill(&mut data, 3, 3, 0.0, 0.0);
        // Eight nearest fill values are the eight non-NaN cells; their mean is 5.0.
        assert!((data[4] - 5.0).abs() < 1e-9);
    }
}
