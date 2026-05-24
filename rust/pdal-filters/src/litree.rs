use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct LiTreeFilter {
    min_size: usize,
    min_hag: f64,
    dummy_radius: f64,
}

impl LiTreeFilter {
    pub fn new(min_size: usize, min_hag: f64, dummy_radius: f64) -> Self {
        Self {
            min_size,
            min_hag,
            dummy_radius,
        }
    }
}

impl Filter for LiTreeFilter {
    fn name(&self) -> &str {
        "filters.litree"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        if input.layout().dim(&DimId::HeightAboveGround).is_none() {
            return Err(StageError(
                "Missing HeightAboveGround dimension in input PointView.".to_string(),
            ));
        }

        let mut out = input.clone();
        let local_max = compute_local_max(&out);
        let mut remaining = (0..out.len()).collect::<Vec<_>>();
        let mut tree_id = 1.0;

        while remaining.len() > self.min_size {
            let top = locate_highest_point(&out, &remaining)?;
            if out.get_f64(top, &DimId::HeightAboveGround) < self.min_hag {
                break;
            }
            if segment_tree(&mut out, &mut remaining, &local_max, tree_id, self)? {
                tree_id += 1.0;
            }
        }

        Ok(vec![out])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::ClusterID, DimType::F64)]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for LiTreeFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

fn locate_highest_point(view: &PointView, ids: &[PointId]) -> Result<PointId, StageError> {
    ids.iter()
        .copied()
        .max_by(|&a, &b| {
            view.get_f64(a, &DimId::HeightAboveGround)
                .total_cmp(&view.get_f64(b, &DimId::HeightAboveGround))
        })
        .ok_or_else(|| StageError("Empty PointView or PointIdList.".to_string()))
}

fn locate_dummy_point(view: &PointView, ids: &[PointId], top: PointId, radius: f64) -> PointId {
    let tx = view.get_f64(top, &DimId::X);
    let ty = view.get_f64(top, &DimId::Y);
    let radius2 = radius * radius;
    ids.iter()
        .copied()
        .filter(|&id| distance2(view, id, tx, ty) <= radius2)
        .max_by(|&a, &b| distance2(view, a, tx, ty).total_cmp(&distance2(view, b, tx, ty)))
        .unwrap_or(top)
}

fn compute_local_max(view: &PointView) -> Vec<bool> {
    (0..view.len())
        .map(|id| {
            let x = view.get_f64(id, &DimId::X);
            let y = view.get_f64(id, &DimId::Y);
            let hag = view.get_f64(id, &DimId::HeightAboveGround);
            !(0..view.len()).any(|other| {
                distance2(view, other, x, y) <= 4.0
                    && view.get_f64(other, &DimId::HeightAboveGround) > hag
            })
        })
        .collect()
}

fn segment_tree(
    view: &mut PointView,
    remaining: &mut Vec<PointId>,
    local_max: &[bool],
    tree_id: f64,
    filter: &LiTreeFilter,
) -> Result<bool, StageError> {
    let top = locate_highest_point(view, remaining)?;
    let dummy = locate_dummy_point(view, remaining, top, filter.dummy_radius);
    if dummy == top {
        remaining.retain(|&id| id != top);
        return Ok(false);
    }

    let mut tree = vec![top];
    let mut not_tree = vec![dummy];
    let tx = view.get_f64(top, &DimId::X);
    let ty = view.get_f64(top, &DimId::Y);

    for point in remaining.iter().copied() {
        if point == top || point == dummy {
            continue;
        }
        if distance2(view, point, tx, ty) < 100.0 {
            classify_point(point, view, local_max, &mut not_tree, &mut tree);
        } else {
            not_tree.push(point);
        }
    }

    if tree.len() >= filter.min_size {
        for point in tree {
            view.set_f64(point, &DimId::ClusterID, tree_id);
        }
        *remaining = not_tree;
        return Ok(true);
    }
    *remaining = not_tree;
    Ok(false)
}

fn classify_point(
    point: PointId,
    view: &PointView,
    local_max: &[bool],
    not_tree: &mut Vec<PointId>,
    tree: &mut Vec<PointId>,
) {
    let dmin_tree = min_distance(point, view, tree);
    let dmin_not_tree = min_distance(point, view, not_tree);

    if !local_max[point as usize] {
        if dmin_tree <= dmin_not_tree {
            tree.push(point);
        } else {
            not_tree.push(point);
        }
        return;
    }

    let threshold = if view.get_f64(point, &DimId::HeightAboveGround) <= 15.0 {
        1.5
    } else {
        2.0
    };
    if dmin_tree > threshold {
        not_tree.push(point);
    } else if dmin_tree <= dmin_not_tree {
        tree.push(point);
    } else {
        not_tree.push(point);
    }
}

fn min_distance(point: PointId, view: &PointView, ids: &[PointId]) -> f64 {
    let x = view.get_f64(point, &DimId::X);
    let y = view.get_f64(point, &DimId::Y);
    ids.iter()
        .map(|&id| distance2(view, id, x, y))
        .fold(f64::MAX, f64::min)
        .sqrt()
}

fn distance2(view: &PointView, point: PointId, x: f64, y: f64) -> f64 {
    let dx = view.get_f64(point, &DimId::X) - x;
    let dy = view.get_f64(point, &DimId::Y) - y;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn segments_synthetic_tree_cluster() {
        let mut layout = PointLayout::new();
        for dim in [
            DimId::X,
            DimId::Y,
            DimId::Z,
            DimId::HeightAboveGround,
            DimId::ClusterID,
        ] {
            layout.register(dim, DimType::F64);
        }
        let mut view = PointView::new(Rc::new(layout));
        let mut id = 0;
        for i in 0..6 {
            for j in 0..6 {
                let point = view.add_point();
                let hag = 1.0 + (id as f64 / 35.0) * 9.0;
                view.set_f64(point, &DimId::X, i as f64 * 0.2);
                view.set_f64(point, &DimId::Y, j as f64 * 0.2);
                view.set_f64(point, &DimId::Z, hag);
                view.set_f64(point, &DimId::HeightAboveGround, hag);
                id += 1;
            }
        }

        let mut filter = LiTreeFilter::new(10, 3.0, 100.0);
        let out = filter.run_one(&view).unwrap().pop().unwrap();
        let in_tree = (0..out.len())
            .filter(|&idx| out.get_f64(idx, &DimId::ClusterID) == 1.0)
            .count();
        assert!(in_tree >= 10);
    }
}
