//! `filters.pmf` -- Progressive Morphological Filter.

use crate::math;
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::segmentation::segment_returns;
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::{HashSet, VecDeque};

pub struct PmfFilter {
    cell_size: f64,
    exponential: bool,
    initial_distance: f64,
    returns: HashSet<String>,
    max_distance: f64,
    max_window_size: f64,
    slope: f64,
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
}

impl PmfFilter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cell_size: f64,
        exponential: bool,
        initial_distance: f64,
        returns: Vec<String>,
        max_distance: f64,
        max_window_size: f64,
        slope: f64,
        ground_class: u8,
        other_class: u8,
        only_ground: bool,
    ) -> Result<Self, StageError> {
        let returns = returns
            .into_iter()
            .map(|r| r.trim().to_string())
            .collect::<HashSet<_>>();
        validate_returns(&returns)?;
        if !only_ground && ground_class == other_class {
            return Err(StageError(
                "Ground and non-ground class cannot beequal when only_ground is false.".to_string(),
            ));
        }
        Ok(Self {
            cell_size,
            exponential,
            initial_distance,
            returns,
            max_distance,
            max_window_size,
            slope,
            ground_class,
            other_class,
            only_ground,
        })
    }
}

impl Filter for PmfFilter {
    fn name(&self) -> &str {
        "filters.pmf"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        validate_positive("cell_size", self.cell_size)?;
        validate_positive("max_window_size", self.max_window_size)?;
        validate_non_negative("initial_distance", self.initial_distance)?;
        validate_non_negative("max_distance", self.max_distance)?;
        validate_non_negative("slope", self.slope)?;

        let mut out = input.clone();
        let inlier_ids = self.inlier_ids(input)?;
        if inlier_ids.is_empty() {
            return Err(StageError("No returns to process.".to_string()));
        }

        if !self.only_ground {
            for &id in &inlier_ids {
                out.set_f64(id, &DimId::Classification, self.other_class as f64);
            }
        }

        let ground_ids = self.ground_ids(input, &inlier_ids)?;
        for id in ground_ids {
            out.set_f64(id, &DimId::Classification, self.ground_class as f64);
        }
        Ok(vec![out])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::Classification, DimType::U8)]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for PmfFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

impl PmfFilter {
    fn inlier_ids(&self, input: &PointView) -> Result<Vec<u64>, StageError> {
        let has_returns = input.layout().dim(&DimId::ReturnNumber).is_some()
            && input.layout().dim(&DimId::NumberOfReturns).is_some();
        if !has_returns || self.returns.is_empty() {
            return Ok((0..input.len()).collect());
        }

        let mut rn = Vec::with_capacity(input.len() as usize);
        let mut nr = Vec::with_capacity(input.len() as usize);
        let mut rn_one_zero = false;
        let mut nr_one_zero = false;
        let mut rn_all_zero = true;
        let mut nr_all_zero = true;

        for id in 0..input.len() {
            let rn_value = input.get_f64(id, &DimId::ReturnNumber) as u8;
            let nr_value = input.get_f64(id, &DimId::NumberOfReturns) as u8;
            rn_one_zero |= rn_value == 0;
            nr_one_zero |= nr_value == 0;
            rn_all_zero &= rn_value == 0;
            nr_all_zero &= nr_value == 0;
            rn.push(rn_value);
            nr.push(nr_value);
        }

        if (rn_one_zero || nr_one_zero) && !(rn_all_zero && nr_all_zero) {
            return Err(StageError(
                "Some NumberOfReturns or ReturnNumber values were 0, but not all. Check that all values in the input file are >= 1.".to_string(),
            ));
        }
        if rn_all_zero && nr_all_zero {
            return Ok((0..input.len()).collect());
        }

        let keep = segment_returns(
            &rn,
            &nr,
            self.returns.contains("first"),
            self.returns.contains("intermediate"),
            self.returns.contains("last"),
            self.returns.contains("only"),
        );
        Ok(keep
            .into_iter()
            .enumerate()
            .filter_map(|(idx, keep)| keep.then_some(idx as u64))
            .collect())
    }

    fn ground_ids(&self, input: &PointView, inlier_ids: &[u64]) -> Result<Vec<u64>, StageError> {
        let bounds = input
            .calculate_bounds_2d()
            .ok_or_else(|| StageError("filters.pmf: input has no points.".to_string()))?;
        let cols = ((bounds.maxx - bounds.minx) / self.cell_size) as usize + 1;
        let rows = ((bounds.maxy - bounds.miny) / self.cell_size) as usize + 1;
        let grid_cell = |x: f64, y: f64| -> usize {
            let c = (((x - bounds.minx) / self.cell_size).floor() as usize).min(cols - 1);
            let r = (((y - bounds.miny) / self.cell_size).floor() as usize).min(rows - 1);
            c * rows + r
        };

        let mut zimin = vec![f64::NAN; rows * cols];
        for &id in inlier_ids {
            let x = input.get_f64(id, &DimId::X);
            let y = input.get_f64(id, &DimId::Y);
            let z = input.get_f64(id, &DimId::Z);
            let cell = grid_cell(x, y);
            if zimin[cell].is_nan() || z < zimin[cell] {
                zimin[cell] = z;
            }
        }
        fill_nearest(&mut zimin, rows, cols);

        let mut ground_ids = inlier_ids.to_vec();
        for (window_size, height_threshold) in self.window_thresholds() {
            let iterations = (0.5 * (window_size - 1.0)) as usize;
            math::erode_diamond(&mut zimin, rows, cols, iterations);
            math::dilate_diamond(&mut zimin, rows, cols, iterations);

            ground_ids.retain(|&id| {
                let x = input.get_f64(id, &DimId::X);
                let y = input.get_f64(id, &DimId::Y);
                let z = input.get_f64(id, &DimId::Z);
                z - zimin[grid_cell(x, y)] < height_threshold
            });
        }
        Ok(ground_ids)
    }

    fn window_thresholds(&self) -> Vec<(f64, f64)> {
        let mut values: Vec<(f64, f64)> = Vec::new();
        let mut iter = 0_i32;
        let mut window_size = 0.0;
        while window_size < self.max_window_size {
            window_size = if self.exponential {
                self.cell_size * (2.0 * 2.0_f64.powi(iter) + 1.0)
            } else {
                self.cell_size * (2.0 * (iter + 1) as f64 * 2.0 + 1.0)
            };
            let mut height_threshold = if iter == 0 {
                self.initial_distance
            } else {
                self.slope * (window_size - values[iter as usize - 1].0) * self.cell_size
                    + self.initial_distance
            };
            height_threshold = height_threshold.min(self.max_distance);
            values.push((window_size, height_threshold));
            iter += 1;
        }
        values
    }
}

fn fill_nearest(data: &mut [f64], rows: usize, cols: usize) {
    let mut queue = VecDeque::new();
    let mut filled = vec![false; data.len()];
    for c in 0..cols {
        for r in 0..rows {
            let idx = c * rows + r;
            if !data[idx].is_nan() {
                filled[idx] = true;
                queue.push_back((c, r));
            }
        }
    }

    while let Some((c, r)) = queue.pop_front() {
        let idx = c * rows + r;
        let value = data[idx];
        for (nc, nr) in [
            (c.checked_sub(1), Some(r)),
            ((c + 1 < cols).then_some(c + 1), Some(r)),
            (Some(c), r.checked_sub(1)),
            (Some(c), (r + 1 < rows).then_some(r + 1)),
        ] {
            if let (Some(nc), Some(nr)) = (nc, nr) {
                let nidx = nc * rows + nr;
                if !filled[nidx] {
                    data[nidx] = value;
                    filled[nidx] = true;
                    queue.push_back((nc, nr));
                }
            }
        }
    }
}

fn validate_returns(returns: &HashSet<String>) -> Result<(), StageError> {
    for value in returns {
        if !matches!(value.as_str(), "first" | "intermediate" | "last" | "only") {
            return Err(StageError(format!(
                "Unrecognized 'returns' value: '{value}'."
            )));
        }
    }
    Ok(())
}

fn validate_positive(name: &str, value: f64) -> Result<(), StageError> {
    if value > 0.0 && value.is_finite() {
        Ok(())
    } else {
        Err(StageError(format!(
            "filters.pmf: '{name}' must be a positive finite value."
        )))
    }
}

fn validate_non_negative(name: &str, value: f64) -> Result<(), StageError> {
    if value >= 0.0 && value.is_finite() {
        Ok(())
    } else {
        Err(StageError(format!(
            "filters.pmf: '{name}' must be a non-negative finite value."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn rejects_unknown_return_name() {
        assert!(PmfFilter::new(
            1.0,
            true,
            0.15,
            vec!["foo".to_string()],
            2.5,
            33.0,
            1.0,
            2,
            1,
            false
        )
        .is_err());
    }

    #[test]
    fn labels_simple_ground_points() {
        let mut layout = PointLayout::new();
        for dim in [DimId::X, DimId::Y, DimId::Z, DimId::Classification] {
            layout.register(dim, DimType::F64);
        }
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in [(0.0, 0.0, 0.0), (1.0, 0.0, 0.1), (0.0, 1.0, 5.0)] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, z);
        }

        let mut filter =
            PmfFilter::new(1.0, true, 0.15, Vec::new(), 2.5, 3.0, 1.0, 2, 1, false).unwrap();
        let out = filter.run_one(&view).unwrap().pop().unwrap();

        assert!(out.get_f64(0, &DimId::Classification) > 0.0);
        assert!(out.get_f64(1, &DimId::Classification) > 0.0);
        assert!(out.get_f64(2, &DimId::Classification) > 0.0);
    }
}
