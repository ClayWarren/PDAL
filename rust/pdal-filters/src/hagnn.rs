use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct HagNnFilter {
    count: usize,
    max_distance: f64,
    allow_extrapolation: bool,
    class_label: u8,
}

impl HagNnFilter {
    pub fn new(
        count: usize,
        max_distance: f64,
        allow_extrapolation: bool,
        class_label: u8,
    ) -> Self {
        Self {
            count,
            max_distance,
            allow_extrapolation,
            class_label,
        }
    }
}

impl Filter for HagNnFilter {
    fn name(&self) -> &str {
        "filters.hag_nn"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let ground = (0..view.len())
            .filter(|idx| view.get_f64(*idx, &DimId::Classification) as u8 == self.class_label)
            .collect::<Vec<_>>();
        if ground.is_empty() {
            return Ok(vec![output]);
        }

        let bounds = Bounds2d::new(view, &ground);
        for idx in 0..view.len() {
            if view.get_f64(idx, &DimId::Classification) as u8 == self.class_label {
                output.set_f64(idx, &DimId::HeightAboveGround, 0.0);
                continue;
            }

            let x0 = view.get_f64(idx, &DimId::X);
            let y0 = view.get_f64(idx, &DimId::Y);
            let z0 = view.get_f64(idx, &DimId::Z);
            let neighbors = knn_ground_2d(view, &ground, x0, y0, self.count);

            let (nearest, nearest_dist) = neighbors[0];
            let x = view.get_f64(nearest, &DimId::X);
            let y = view.get_f64(nearest, &DimId::Y);
            let z = view.get_f64(nearest, &DimId::Z);
            let ground_z = if (x0 == x && y0 == y) || neighbors.len() == 1 {
                z
            } else if !self.allow_extrapolation && !bounds.contains(x0, y0) {
                z0
            } else {
                interpolate_ground(view, &neighbors, self.max_distance * self.max_distance, z0)
                    .unwrap_or(if nearest_dist == 0.0 { z } else { z0 })
            };
            output.set_f64(idx, &DimId::HeightAboveGround, z0 - ground_z);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for HagNnFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        false
    }
}

struct Bounds2d {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds2d {
    fn new(view: &PointView, ids: &[PointId]) -> Self {
        let mut bounds = Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for id in ids {
            let x = view.get_f64(*id, &DimId::X);
            let y = view.get_f64(*id, &DimId::Y);
            bounds.min_x = bounds.min_x.min(x);
            bounds.min_y = bounds.min_y.min(y);
            bounds.max_x = bounds.max_x.max(x);
            bounds.max_y = bounds.max_y.max(y);
        }
        bounds
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        self.min_x <= x && x <= self.max_x && self.min_y <= y && y <= self.max_y
    }
}

fn knn_ground_2d(
    view: &PointView,
    ground: &[PointId],
    x: f64,
    y: f64,
    count: usize,
) -> Vec<(PointId, f64)> {
    let mut neighbors = ground
        .iter()
        .map(|idx| {
            let dx = view.get_f64(*idx, &DimId::X) - x;
            let dy = view.get_f64(*idx, &DimId::Y) - y;
            (*idx, dx * dx + dy * dy)
        })
        .collect::<Vec<_>>();
    neighbors.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    neighbors.truncate(count.min(neighbors.len()));
    neighbors
}

fn interpolate_ground(
    view: &PointView,
    neighbors: &[(PointId, f64)],
    max_distance_sqr: f64,
    z_default: f64,
) -> Option<f64> {
    let mut weights = 0.0;
    let mut z_accumulator = 0.0;
    for (idx, sqr_dist) in neighbors {
        if max_distance_sqr > 0.0 && *sqr_dist > max_distance_sqr {
            break;
        }
        if *sqr_dist == 0.0 {
            return Some(view.get_f64(*idx, &DimId::Z));
        }
        let weight = 1.0 / sqr_dist;
        weights += weight;
        z_accumulator += weight * view.get_f64(*idx, &DimId::Z);
    }
    if weights > 0.0 {
        Some(z_accumulator / weights)
    } else {
        Some(z_default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::F64);
        layout.register(DimId::HeightAboveGround, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, class) in [
            (0.0, 0.0, 10.0, 2.0),
            (2.0, 0.0, 14.0, 2.0),
            (0.0, 2.0, 16.0, 2.0),
            (1.0, 0.0, 20.0, 1.0),
            (10.0, 10.0, 30.0, 1.0),
        ] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
            view.set_f64(idx, &DimId::Classification, class);
        }
        view
    }

    #[test]
    fn closest_ground_sets_height() {
        let view = view();
        let mut filter = HagNnFilter::new(1, 0.0, true, 2);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::HeightAboveGround), 0.0);
        assert_eq!(out.get_f64(3, &DimId::HeightAboveGround), 10.0);
    }

    #[test]
    fn interpolation_uses_inverse_squared_distance() {
        let view = view();
        let mut filter = HagNnFilter::new(2, 0.0, true, 2);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(3, &DimId::HeightAboveGround), 8.0);
    }

    #[test]
    fn no_extrapolation_returns_zero_height_outside_ground_bounds() {
        let view = view();
        let mut filter = HagNnFilter::new(2, 0.0, false, 2);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(4, &DimId::HeightAboveGround), 0.0);
    }
}
