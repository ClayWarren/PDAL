use crate::range::RangeLimit;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

pub struct NeighborClassifierFilter {
    k: usize,
    dim_name: String,
    domain: Vec<RangeLimit>,
}

impl NeighborClassifierFilter {
    pub fn new(k: usize, dim_name: String, domain: Vec<RangeLimit>) -> Self {
        Self {
            k,
            dim_name,
            domain,
        }
    }
}

impl Filter for NeighborClassifierFilter {
    fn name(&self) -> &str {
        "filters.neighborclassifier"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        let dim = DimId::from_name(&self.dim_name);
        let mut updates = Vec::new();
        for idx in 0..view.len() {
            if !domain_passes(&self.domain, view, idx) {
                continue;
            }

            if let Some(new_class) = vote(view, &index, idx, self.k, &dim) {
                let old_class = view.get_f64(idx, &dim) as i64;
                if old_class != new_class {
                    updates.push((idx, new_class));
                }
            }
        }

        for (idx, new_class) in updates {
            output.set_f64(idx, &dim, new_class as f64);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for NeighborClassifierFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

fn vote(
    view: &PointView,
    index: &SpatialIndex3d,
    idx: PointId,
    k: usize,
    dim: &DimId,
) -> Option<i64> {
    let neighbors = index.knn(idx, k);
    if neighbors.is_empty() {
        return None;
    }

    let threshold = neighbors.len() as f64 / 2.0;
    let mut counts = BTreeMap::<i64, u64>::new();
    for (neighbor, _) in neighbors {
        *counts
            .entry(view.get_f64(neighbor, dim) as i64)
            .or_insert(0) += 1;
    }

    let mut winner = None;
    let mut winning_count = 0;
    for (class, count) in counts {
        if count > winning_count {
            winner = Some(class);
            winning_count = count;
        }
    }

    winner.filter(|_| winning_count as f64 > threshold)
}

fn domain_passes(domain: &[RangeLimit], view: &PointView, idx: PointId) -> bool {
    if domain.is_empty() {
        return true;
    }

    let mut sorted = domain.to_vec();
    sorted.sort_by(|a, b| a.dim_name.cmp(&b.dim_name));

    let mut last_dim = &sorted[0].dim_name;
    let mut passes = false;
    for range in &sorted {
        if &range.dim_name != last_dim {
            if !passes {
                return false;
            }
            last_dim = &range.dim_name;
        } else if passes {
            continue;
        }

        let value = view.get_f64(idx, &DimId::from_name(&range.dim_name));
        passes = range.value_passes(value);
    }

    passes
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view(classes: &[f64]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (i, class) in classes.iter().enumerate() {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, i as f64);
            view.set_f64(idx, &DimId::Y, 0.0);
            view.set_f64(idx, &DimId::Z, 0.0);
            view.set_f64(idx, &DimId::Classification, *class);
        }
        view
    }

    #[test]
    fn reclassifies_domain_points_by_majority_vote() {
        let view = view(&[14.0, 2.0, 2.0]);
        let domain = vec![RangeLimit {
            dim_name: "Classification".to_string(),
            lower_bound: 14.0,
            upper_bound: 14.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
        let mut filter = NeighborClassifierFilter::new(3, "Classification".to_string(), domain);
        let out = filter.run(&view).unwrap().remove(0);

        assert_eq!(out.get_f64(0, &DimId::Classification), 2.0);
        assert_eq!(out.get_f64(1, &DimId::Classification), 2.0);
        assert_eq!(out.get_f64(2, &DimId::Classification), 2.0);
    }
}
