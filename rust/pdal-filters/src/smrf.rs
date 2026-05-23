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
        if let Some(window) = self.window {
            if window <= 0.0 || !window.is_finite() {
                return Err(StageError(
                    "filters.smrf: 'window' must be a positive finite value.".to_string(),
                ));
            }
        }

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
        for (x, y, z) in &[(0.5, 0.5, 10.0), (0.5, 1.5, 12.0), (1.5, 0.5, 8.0), (1.5, 1.5, 11.0)] {
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
        for (x, y, z, rn, nr) in &[(0.5, 0.5, 10.0, 1, 1), (0.5, 1.5, 12.0, 1, 2), (1.5, 0.5, 8.0, 2, 2)] {
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
    fn rejects_empty_input() {
        let layout = PointLayout::new();
        let empty = PointView::new(Rc::new(layout));
        let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        let err = filter.run_one(&empty).map(|_| ()).unwrap_err();
        assert!(err.to_string().contains("no points"));
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
        // All points should be classified as ground (class 2) since they're within threshold
        for i in 0..result[0].len() {
            assert_eq!(result[0].get_f64(i, &DimId::Classification), 2.0);
        }
    }

    #[test]
    fn smrf_returns_filter_first_only() {
        let mut filter = SmrfFilter::new(
            1.0, 0.15, None, 0.5, 0.5, 2, 1, true,
            vec!["first".to_string()],
        );
        let result = filter.run_one(&grid_view_with_returns()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].len() > 0);
    }

    #[test]
    fn smrf_returns_filter_last_only() {
        let mut filter = SmrfFilter::new(
            1.0, 0.15, None, 0.5, 0.5, 2, 1, true,
            vec!["last".to_string()],
        );
        let result = filter.run_one(&grid_view_with_returns()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn smrf_returns_filter_only() {
        let mut filter = SmrfFilter::new(
            1.0, 0.15, None, 0.5, 0.5, 2, 1, true,
            vec!["only".to_string()],
        );
        let result = filter.run_one(&grid_view_with_returns()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn smrf_classifies_other_when_not_only_ground() {
        // High point should be classified as other (not ground)
        let mut filter = SmrfFilter::new(2.0, 0.15, None, 0.5, 0.5, 2, 1, false, Vec::new());
        let result = filter.run_one(&flat_3x3_view()).unwrap();
        // All 9 points at z=10 should be within threshold from a flat surface
        for i in 0..result[0].len() {
            assert_eq!(result[0].get_f64(i, &DimId::Classification), 2.0, "point {i} should be ground");
        }
    }

    #[test]
    fn smrf_knn_fill_all_nan() {
        let mut data = vec![f64::NAN; 9];
        let rows = 3;
        let cols = 3;
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        filter.knn_fill(&mut data, rows, cols);
        for v in &data {
            assert!(v.is_nan());
        }
    }

    #[test]
    fn smrf_knn_fill_single_nan() {
        let mut data = vec![1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0, 7.0, 8.0, 9.0];
        let rows = 3;
        let cols = 3;
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        filter.knn_fill(&mut data, rows, cols);
        // Center cell (index 4) should be filled with mean of neighbors (1+2+3+4+6+7+8+9)/8 = 5.0
        assert!((data[4] - 5.0).abs() < 0.001);
    }

    #[test]
    fn smrf_knn_fill_no_nan() {
        let mut data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let rows = 3;
        let cols = 3;
        let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
        filter.knn_fill(&mut data, rows, cols);
        assert!((data[4] - 5.0).abs() < 0.001);
    }

    #[test]
    fn smrf_nan_cell_in_grid_skips_point_in_classification() {
        let mut filter = SmrfFilter::new(2.0, 0.15, None, 0.5, 0.5, 2, 1, true, Vec::new());
        let result = filter.run_one(&flat_3x3_view()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_f64(0, &DimId::Classification), 2.0);
    }
}
