use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
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

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let local_max = compute_local_max(view);
        let mut ui = (0..view.len()).collect::<Vec<_>>();
        let mut tree_id = 1.0;

        while ui.len() > self.min_size {
            let t0 = locate_highest_point(view, &ui)?;
            if view.get_f64(t0, &DimId::HeightAboveGround) < self.min_hag {
                break;
            }
            segment_tree(
                view,
                &mut output,
                &local_max,
                &mut ui,
                &mut tree_id,
                t0,
                self,
            );
        }

        Ok(vec![output])
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

fn locate_highest_point(view: &PointView, ui: &[PointId]) -> Result<PointId, StageError> {
    if view.is_empty() || ui.is_empty() {
        return Err(StageError("Empty PointView or PointIdList.".to_string()));
    }

    let mut t0 = ui[0];
    let mut vmax = view.get_f64(t0, &DimId::HeightAboveGround);
    for idx in ui {
        let value = view.get_f64(*idx, &DimId::HeightAboveGround);
        if value > vmax {
            vmax = value;
            t0 = *idx;
        }
    }
    Ok(t0)
}

fn locate_dummy_point(view: &PointView, ui: &[PointId], t0: PointId, radius: f64) -> PointId {
    let tx = view.get_f64(t0, &DimId::X);
    let ty = view.get_f64(t0, &DimId::Y);
    let radius_sqr = radius * radius;
    let mut farthest = (t0, 0.0);
    for idx in ui {
        let dx = view.get_f64(*idx, &DimId::X) - tx;
        let dy = view.get_f64(*idx, &DimId::Y) - ty;
        let distance_sqr = dx * dx + dy * dy;
        if distance_sqr <= radius_sqr
            && (distance_sqr > farthest.1 || (distance_sqr == farthest.1 && *idx > farthest.0))
        {
            farthest = (*idx, distance_sqr);
        }
    }
    farthest.0
}

fn compute_local_max(view: &PointView) -> Vec<bool> {
    let index = SpatialIndex3d::new(view);
    let mut local_max = vec![true; view.len() as usize];

    for idx in 0..view.len() {
        let hag = view.get_f64(idx, &DimId::HeightAboveGround);
        for neighbor in index.radius_2d_excluding(idx, 2.0) {
            if view.get_f64(neighbor, &DimId::HeightAboveGround) > hag {
                local_max[idx as usize] = false;
                break;
            }
        }
    }

    local_max
}

fn segment_tree(
    view: &PointView,
    output: &mut PointView,
    local_max: &[bool],
    ui: &mut Vec<PointId>,
    tree_id: &mut f64,
    t0: PointId,
    filter: &LiTreeFilter,
) {
    let mut pi = vec![t0];
    let mut ni = Vec::new();

    let n0 = locate_dummy_point(view, ui, t0, filter.dummy_radius);
    if n0 == t0 {
        remove_point(ui, t0);
        return;
    }
    ni.push(n0);

    let tx = view.get_f64(t0, &DimId::X);
    let ty = view.get_f64(t0, &DimId::Y);
    for point_id in ui.iter().copied() {
        if point_id == t0 || point_id == n0 {
            continue;
        }

        let ux = view.get_f64(point_id, &DimId::X);
        let uy = view.get_f64(point_id, &DimId::Y);
        let distance_sqr = (ux - tx) * (ux - tx) + (uy - ty) * (uy - ty);
        if distance_sqr < 100.0 {
            classify_point(point_id, view, local_max, &mut ni, &mut pi);
        } else {
            ni.push(point_id);
        }
    }

    if pi.len() >= filter.min_size {
        for point_id in pi {
            output.set_f64(point_id, &DimId::ClusterID, *tree_id);
        }
        *tree_id += 1.0;
    }

    *ui = ni;
}

fn classify_point(
    point_id: PointId,
    view: &PointView,
    local_max: &[bool],
    ni: &mut Vec<PointId>,
    pi: &mut Vec<PointId>,
) {
    let dmin1 = min_2d_distance(view, point_id, pi);
    let dmin2 = min_2d_distance(view, point_id, ni);

    if !local_max[point_id as usize] {
        if dmin1 <= dmin2 {
            pi.push(point_id);
        } else {
            ni.push(point_id);
        }
        return;
    }

    let threshold = if view.get_f64(point_id, &DimId::HeightAboveGround) <= 15.0 {
        1.5
    } else {
        2.0
    };
    if dmin1 > threshold {
        ni.push(point_id);
    } else if dmin1 <= dmin2 {
        pi.push(point_id);
    } else {
        ni.push(point_id);
    }
}

fn min_2d_distance(view: &PointView, point_id: PointId, ids: &[PointId]) -> f64 {
    let ux = view.get_f64(point_id, &DimId::X);
    let uy = view.get_f64(point_id, &DimId::Y);
    ids.iter()
        .map(|idx| {
            let dx = view.get_f64(*idx, &DimId::X) - ux;
            let dy = view.get_f64(*idx, &DimId::Y) - uy;
            dx * dx + dy * dy
        })
        .fold(f64::INFINITY, f64::min)
        .sqrt()
}

fn remove_point(ids: &mut Vec<PointId>, point_id: PointId) {
    if let Some(pos) = ids.iter().position(|id| *id == point_id) {
        ids.remove(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::HeightAboveGround, DimType::F64);
        layout.register(DimId::ClusterID, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, hag) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *hag);
            view.set_f64(idx, &DimId::HeightAboveGround, *hag);
        }
        view
    }

    #[test]
    fn labels_cluster_when_min_size_is_met() {
        let view = view(&[
            (0.0, 0.0, 20.0),
            (0.5, 0.0, 12.0),
            (0.0, 0.5, 11.0),
            (50.0, 0.0, 5.0),
        ]);
        let mut filter = LiTreeFilter::new(2, 3.0, 100.0);
        let out = filter.run(&view).unwrap().remove(0);

        assert_eq!(out.get_f64(0, &DimId::ClusterID), 1.0);
        assert_eq!(out.get_f64(1, &DimId::ClusterID), 1.0);
        assert_eq!(out.get_f64(2, &DimId::ClusterID), 1.0);
        assert_eq!(out.get_f64(3, &DimId::ClusterID), 0.0);
    }
}
