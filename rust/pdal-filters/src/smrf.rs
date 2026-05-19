//! `filters.smrf` -- Simple Morphological Filter (Pingel et al., 2013).
//!
//! Port of `filters/SMRFilter.cpp`. This is a simplified port: it builds the
//! minimum-elevation grid, runs the progressive morphological opening, and
//! classifies points by their residual against the provisional surface. The
//! synthetic/ignored-point segmentation and the spike-net (`cut`) refinement
//! of the C++ filter are not yet modeled.

use crate::math;
use pdal_core::metadata::MetadataNode;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::HashSet;

pub struct SmrfFilter {
    cell: f64,
    slope: f64,
    window: Option<f64>,
    scalar: f64,
    threshold: f64,
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
        Self {
            cell,
            slope,
            window,
            scalar,
            threshold,
            ground_class,
            other_class,
            only_ground,
            returns: returns.into_iter().collect(),
        }
    }

    /// Fill grid NaNs with a single-pass 3x3 mean of their valid neighbors.
    fn knn_fill(&self, data: &mut [f64], rows: usize, cols: usize) {
        let mut out = data.to_vec();
        for c in 0..cols {
            for r in 0..rows {
                let idx = c * rows + r;
                if !data[idx].is_nan() {
                    continue;
                }
                let mut sum = 0.0;
                let mut count = 0;
                for dc in -1..=1 {
                    for dr in -1..=1 {
                        let nc = c as isize + dc;
                        let nr = r as isize + dr;
                        if nc >= 0 && nc < cols as isize && nr >= 0 && nr < rows as isize {
                            let nidx = (nc as usize) * rows + (nr as usize);
                            if !data[nidx].is_nan() {
                                sum += data[nidx];
                                count += 1;
                            }
                        }
                    }
                }
                if count > 0 {
                    out[idx] = sum / count as f64;
                }
            }
        }
        data.copy_from_slice(&out);
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
}

impl Filter for SmrfFilter {
    fn name(&self) -> &str {
        "filters.smrf"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        // Honor the 'returns' selection when return dimensions are present.
        let has_returns = input.layout().dim(&DimId::ReturnNumber).is_some()
            && input.layout().dim(&DimId::NumberOfReturns).is_some();
        let mut inlier_ids = Vec::new();
        for i in 0..input.len() {
            if has_returns && !self.returns.is_empty() {
                let rn = input.get_f64(i, &DimId::ReturnNumber) as u8;
                let nr = input.get_f64(i, &DimId::NumberOfReturns) as u8;
                let mut keep = false;
                if self.returns.contains("last") && rn == nr && nr > 0 {
                    keep = true;
                }
                if self.returns.contains("first") && rn == 1 {
                    keep = true;
                }
                if self.returns.contains("only") && rn == 1 && nr == 1 {
                    keep = true;
                }
                if !keep {
                    continue;
                }
            }
            inlier_ids.push(i);
        }
        // If the return filter excluded every point (e.g. the file carries no
        // return information), fall back to processing all points.
        if inlier_ids.is_empty() {
            inlier_ids = (0..input.len()).collect();
        }
        if inlier_ids.is_empty() {
            return Err(StageError("filters.smrf: input has no points.".to_string()));
        }

        let bounds = input
            .calculate_bounds_2d()
            .ok_or_else(|| StageError("filters.smrf: input has no points.".to_string()))?;
        let cols = ((bounds.maxx - bounds.minx) / self.cell) as usize + 1;
        let rows = ((bounds.maxy - bounds.miny) / self.cell) as usize + 1;
        let window = self.window.unwrap_or(18.0 * self.cell);

        let grid_cell = |x: f64, y: f64| -> usize {
            let c = (((x - bounds.minx) / self.cell).floor() as usize).min(cols - 1);
            let r = (((y - bounds.miny) / self.cell).floor() as usize).min(rows - 1);
            c * rows + r
        };

        // Minimum-elevation grid (ZImin).
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
        self.knn_fill(&mut zimin, rows, cols);

        // Provisional ground surface (ZIpro): drop object cells, then refill.
        let obj = self.progressive_filter(&zimin, rows, cols, self.slope, window);
        let mut zipro = zimin.clone();
        for (i, &is_obj) in obj.iter().enumerate() {
            if is_obj == 1 {
                zipro[i] = f64::NAN;
            }
        }
        self.knn_fill(&mut zipro, rows, cols);

        // Surface-gradient magnitude, used to scale the residual threshold.
        let scaled_zipro: Vec<f64> = zipro.iter().map(|z| z / self.cell).collect();
        let gx = math::grad_x(&scaled_zipro, rows, cols);
        let gy = math::grad_y(&scaled_zipro, rows, cols);
        let mut gsurfs: Vec<f64> = gx
            .iter()
            .zip(gy.iter())
            .map(|(x, y)| (x * x + y * y).sqrt())
            .collect();
        self.knn_fill(&mut gsurfs, rows, cols);

        // Classify each point by its residual against the provisional surface.
        let mut out = input.clone();
        for &id in &inlier_ids {
            let x = input.get_f64(id, &DimId::X);
            let y = input.get_f64(id, &DimId::Y);
            let z = input.get_f64(id, &DimId::Z);
            let cell = grid_cell(x, y);
            if zipro[cell].is_nan() || gsurfs[cell].is_nan() {
                continue;
            }
            let threshold = self.threshold + self.scalar * gsurfs[cell];
            if (zipro[cell] - z).abs() > threshold {
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
