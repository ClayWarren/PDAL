use std::collections::BTreeMap;

use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

use crate::range::RangeLimit;

pub struct NeighborClassifierFilter {
    domain: Vec<RangeLimit>,
    k: usize,
    dim_name: String,
}

impl NeighborClassifierFilter {
    pub fn new(domain: Vec<RangeLimit>, k: usize, dim_name: String) -> Self {
        Self {
            domain,
            k,
            dim_name,
        }
    }

    fn point_passes_domain(&self, view: &PointView, idx: u64) -> bool {
        self.domain.is_empty()
            || self.domain.iter().any(|range| {
                let dim = DimId::from_name(&range.dim_name);
                range.value_passes(view.get_f64(idx, &dim))
            })
    }

    fn nearest_neighbors(view: &PointView, x: f64, y: f64, z: f64, k: usize) -> Vec<u64> {
        let mut neighbors = (0..view.len())
            .map(|idx| {
                let dx = view.get_f64(idx, &DimId::X) - x;
                let dy = view.get_f64(idx, &DimId::Y) - y;
                let dz = view.get_f64(idx, &DimId::Z) - z;
                (idx, dx * dx + dy * dy + dz * dz)
            })
            .collect::<Vec<_>>();
        neighbors.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        neighbors.truncate(k.min(neighbors.len()));
        neighbors.into_iter().map(|(idx, _)| idx).collect()
    }
}

impl Filter for NeighborClassifierFilter {
    fn name(&self) -> &str {
        "filters.neighborclassifier"
    }

    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        if self.k == 0 {
            return Err(StageError(
                "filters.neighborclassifier: k must be greater than zero.".to_string(),
            ));
        }

        let input = &inputs[0];
        let reference = inputs.get(1).unwrap_or(input);
        let dim = DimId::from_name(&self.dim_name);
        let mut output = input.make_new();
        for idx in 0..input.len() {
            output.append_point(input, idx);
        }

        for idx in 0..input.len() {
            if !self.point_passes_domain(input, idx) {
                continue;
            }

            let x = input.get_f64(idx, &DimId::X);
            let y = input.get_f64(idx, &DimId::Y);
            let z = input.get_f64(idx, &DimId::Z);
            let neighbors = Self::nearest_neighbors(reference, x, y, z, self.k);
            if neighbors.is_empty() {
                continue;
            }

            let mut counts = BTreeMap::<i64, usize>::new();
            for neighbor in neighbors {
                let value = reference.get_f64(neighbor, &dim) as i64;
                *counts.entry(value).or_default() += 1;
            }

            let threshold = counts.values().sum::<usize>() as f64 / 2.0;
            if let Some((new_value, count)) = counts
                .iter()
                .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
            {
                let old_value = input.get_f64(idx, &dim) as i64;
                if *count as f64 > threshold && old_value != *new_value {
                    output.set_f64(idx, &dim, *new_value as f64);
                }
            }
        }

        Ok(vec![output])
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.run(std::slice::from_ref(input))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for NeighborClassifierFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, class) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
            view.set_f64(idx, &DimId::Classification, *class);
        }
        view
    }

    #[test]
    fn majority_vote_updates_requested_domain() {
        let input = view(&[
            (0.0, 0.0, 0.0, 14.0),
            (0.1, 0.0, 0.0, 2.0),
            (0.0, 0.1, 0.0, 2.0),
            (10.0, 0.0, 0.0, 14.0),
        ]);
        let domain = vec![RangeLimit {
            dim_name: "Classification".to_string(),
            lower_bound: 14.0,
            upper_bound: 14.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
        let mut filter = NeighborClassifierFilter::new(domain, 3, "Classification".to_string());
        let out = filter.run_one(&input).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Classification), 2.0);
        assert_eq!(out.get_f64(3, &DimId::Classification), 14.0);
    }

    #[test]
    fn candidate_view_supplies_votes() {
        let input = view(&[(0.0, 0.0, 0.0, 1.0)]);
        let candidate = view(&[(0.0, 0.0, 0.0, 7.0)]);
        let mut filter = NeighborClassifierFilter::new(Vec::new(), 1, "Classification".to_string());
        let out = filter.run(&[input, candidate]).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Classification), 7.0);
    }
}
