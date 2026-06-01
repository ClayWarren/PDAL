//! `filters.smrf` -- Simple Morphological Filter (Pingel et al., 2013).
//!
//! Port of `filters/SMRFilter.cpp`. The Rust path covers the simple-case the
//! C++ wrapper now delegates: no debug `dir`, no `ignored` DimRanges, no
//! synthetic/keypoint/withheld `classbits` filter. It performs the full
//! algorithm: minimum-Z grid, low-outlier mask, net cutting, progressive
//! morphological opening, KD-tree inpainting, gradient-scaled threshold, and
//! per-point ground/object classification.

use crate::math;
use crate::range::{ranges_point_passes, RangeLimit};
use pdal_core::metadata::MetadataNode;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use rstar::primitives::GeomWithData;
use rstar::RTree;
use std::collections::HashSet;

const VALID_RETURNS: &[&str] = &["first", "intermediate", "last", "only"];

/// Classification-flag bits the `classbits` option can mask out (LAS-style
/// flags packed into the Classification byte), matching the C++
/// `Segmentation::PointClasses` constants.
pub const CLASSBIT_SYNTHETIC: u8 = 32;
pub const CLASSBIT_KEYPOINT: u8 = 64;
pub const CLASSBIT_WITHHELD: u8 = 128;

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
    /// DimRanges whose matching points are excluded from segmentation (the
    /// `ignore` option); ignored points keep their original classification.
    ignore: Vec<RangeLimit>,
    /// Mask of Classification-flag bits whose set points are excluded from
    /// segmentation (the `classbits` option).
    classbits: u8,
    /// Optional debug-output directory (the `dir` option). When set, the
    /// intermediate grids are written as GeoTIFFs, matching the C++ wrapper.
    dir: Option<String>,
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
        Self::with_segmentation(
            cell,
            slope,
            window,
            scalar,
            threshold,
            cut,
            ground_class,
            other_class,
            only_ground,
            returns,
            Vec::new(),
            0,
        )
    }

    /// Full constructor including the `ignore` DimRanges and `classbits` mask
    /// that exclude points from segmentation. Matches the C++ SMRFilter's
    /// `ignoreDimRanges`/`ignoreClassBits` pre-segmentation step.
    #[allow(clippy::too_many_arguments)]
    pub fn with_segmentation(
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
        ignore: Vec<RangeLimit>,
        classbits: u8,
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
            ignore,
            classbits,
            dir: None,
        }
    }

    /// Set the debug-output directory (the `dir` option). When `Some`, the
    /// intermediate grids are written as GeoTIFFs during `run_one`.
    pub fn with_dir(mut self, dir: Option<String>) -> Self {
        self.dir = dir.filter(|d| !d.is_empty());
        self
    }

    /// Write one grid to `<dir>/<name>` as a single-band Float64 GeoTIFF, using
    /// the same south-up geotransform `[minx, cell, 0, miny, 0, cell]`, nodata
    /// (-9999), and row-major layout as the C++ `math::writeMatrix`. The Rust
    /// grid is column-major (`c*rows + r`), matching the C++ Eigen
    /// `Map(data, rows, cols)`, so pixel (row i, col j) = `grid[j*rows + i]`.
    #[allow(clippy::too_many_arguments)]
    fn write_grid(
        &self,
        dir: &str,
        name: &str,
        grid: &[f64],
        rows: usize,
        cols: usize,
        minx: f64,
        miny: f64,
        srs_wkt: &str,
    ) -> Result<(), StageError> {
        use pdal_core::gdal::Raster;
        let path = format!("{dir}/{name}");
        let gt = [minx, self.cell, 0.0, miny, 0.0, self.cell];
        // Prefer a georeferenced raster, but a debug dump must not fail the whole
        // SMRF run when the view's SRS can't be set (e.g. the LAS 32767
        // "no CRS" sentinel) — fall back to writing without a projection.
        let mut raster = match Raster::create_float64(
            &path,
            "GTiff",
            cols as i32,
            rows as i32,
            1,
            gt,
            srs_wkt,
        ) {
            Ok(raster) => raster,
            Err(_) => Raster::create_float64(&path, "GTiff", cols as i32, rows as i32, 1, gt, "")
                .map_err(StageError)?,
        };
        let mut buf = vec![0.0f64; rows * cols];
        for i in 0..rows {
            for j in 0..cols {
                buf[i * cols + j] = grid[j * rows + i];
            }
        }
        raster
            .write_band_f64(1, cols, rows, &buf, -9999.0, name)
            .map_err(StageError)?;
        Ok(())
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

/// Choose inlier point ids among `candidates` based on optional return-number
/// filtering.
///
/// `candidates` are the points that survived the `ignore`/`classbits`
/// pre-segmentation step (the C++ `realView`); the zero-return checks and the
/// all-zero fallback operate over that subset only, matching the C++ wrapper.
///
/// Returns the inlier id list, or an error if NumberOfReturns/ReturnNumber
/// values mix zero and non-zero entries. When all values are zero, return
/// filtering is silently dropped and every candidate becomes an inlier.
fn select_inliers(
    view: &PointView,
    returns: &HashSet<String>,
    candidates: &[PointId],
) -> Result<Vec<PointId>, StageError> {
    let has_returns = view.layout().dim(&DimId::ReturnNumber).is_some()
        && view.layout().dim(&DimId::NumberOfReturns).is_some();
    if !has_returns || returns.is_empty() {
        return Ok(candidates.to_vec());
    }

    let mut nr_has_zero = false;
    let mut rn_has_zero = false;
    let mut nr_all_zero = true;
    let mut rn_all_zero = true;
    for &i in candidates {
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
        return Ok(candidates.to_vec());
    }

    let mut inliers = Vec::new();
    for &i in candidates {
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

/// Points that survive the `ignore` DimRanges and `classbits` mask — the C++
/// `keptView` after `ignoreDimRanges` then `ignoreClassBits`. A point is
/// excluded if it matches the ignore ranges, or if any masked Classification
/// flag bit is set on it.
fn segmentation_candidates(view: &PointView, ignore: &[RangeLimit], classbits: u8) -> Vec<PointId> {
    let mut candidates = Vec::new();
    for i in 0..view.len() {
        if !ignore.is_empty() && ranges_point_passes(ignore, view, i) {
            continue;
        }
        if classbits != 0 {
            let c = view.get_f64(i, &DimId::Classification) as u8;
            if classbits & c != 0 {
                continue;
            }
        }
        candidates.push(i);
    }
    candidates
}

impl Filter for SmrfFilter {
    fn name(&self) -> &str {
        "filters.smrf"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.validate()?;

        let candidates = segmentation_candidates(input, &self.ignore, self.classbits);
        let inlier_ids = select_inliers(input, &self.returns, &candidates)?;
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
            return Err(StageError("filters.smrf: input has no points.".to_string()));
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
        let zimin_pre = self.dir.is_some().then(|| zimin.clone());
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
        let zipro_pre = self.dir.is_some().then(|| zipro.clone());
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
        let gsurfs_pre = self.dir.is_some().then(|| gsurfs.clone());
        self.knn_fill(&mut gsurfs, rows, cols, minx, miny);

        // Optional debug output: write the intermediate grids as GeoTIFFs,
        // matching the C++ wrapper's `dir` behavior (12 rasters).
        if let Some(dir) = &self.dir {
            // Ensure the GDAL/GTiff driver is available (idempotent); the C++
            // app registers drivers at startup, but the Rust pipeline path may
            // reach here without that.
            pdal_core::gdal::register_drivers();
            let srs_wkt = input.spatial_reference().wkt().to_string();
            let as_f64 = |m: &[u8]| -> Vec<f64> { m.iter().map(|&v| v as f64).collect() };
            let thresh: Vec<f64> = gsurfs
                .iter()
                .map(|g| self.threshold + self.scalar * g)
                .collect();
            let w = |name: &str, grid: &[f64]| {
                self.write_grid(dir, name, grid, rows, cols, minx, miny, &srs_wkt)
            };
            w("zimin.tif", zimin_pre.as_ref().unwrap())?;
            w("zimin_fill.tif", &zimin)?;
            w("zilow.tif", &as_f64(&low))?;
            w("zinet.tif", &zinet)?;
            w("ziobj.tif", &as_f64(&obj))?;
            w("zipro.tif", zipro_pre.as_ref().unwrap())?;
            w("zipro_fill.tif", &zipro)?;
            w("gx.tif", &gx)?;
            w("gy.tif", &gy)?;
            w("gsurfs.tif", gsurfs_pre.as_ref().unwrap())?;
            w("gsurfs_fill.tif", &gsurfs)?;
            w("thresh.tif", &thresh)?;
        }

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
mod tests;
