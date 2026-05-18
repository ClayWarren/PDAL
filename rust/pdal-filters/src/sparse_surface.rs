use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct SparseSurfaceFilter {
    radius: f64,
    ground_class: u8,
    low_point_class: u8,
}

impl SparseSurfaceFilter {
    pub fn new(radius: f64, ground_class: u8, low_point_class: u8) -> Self {
        Self {
            radius,
            ground_class,
            low_point_class,
        }
    }
}

impl Filter for SparseSurfaceFilter {
    fn name(&self) -> &str {
        "filters.sparsesurface"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let lower = self.ground_class.min(self.low_point_class);
        let higher = self.ground_class.max(self.low_point_class);
        let unclassified = if higher - lower > 1 {
            lower + 1
        } else if lower > 0 {
            lower - 1
        } else {
            higher + 1
        };

        for idx in 0..output.len() {
            output.set_f64(idx, &DimId::Classification, unclassified as f64);
        }

        let mut z_index = (0..view.len()).collect::<Vec<_>>();
        z_index.sort_by(|a, b| {
            view.get_f64(*a, &DimId::Z)
                .total_cmp(&view.get_f64(*b, &DimId::Z))
        });

        let index = SpatialIndex3d::new(view);
        for idx in z_index {
            let classification = output.get_f64(idx, &DimId::Classification) as u8;
            if classification != unclassified {
                continue;
            }

            output.set_f64(idx, &DimId::Classification, self.ground_class as f64);
            for neighbor in index.radius_dims(idx, self.radius, &[DimId::X, DimId::Y]) {
                let neighbor_class = output.get_f64(neighbor, &DimId::Classification) as u8;
                if neighbor_class == unclassified {
                    output.set_f64(
                        neighbor,
                        &DimId::Classification,
                        self.low_point_class as f64,
                    );
                }
            }
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for SparseSurfaceFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
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
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn labels_lowest_in_each_radius_as_ground() {
        let view = view(&[
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 2.0),
            (10.0, 0.0, 0.5),
            (10.0, 0.0, 1.5),
            (10.0, 0.0, 2.5),
        ]);
        let mut filter = SparseSurfaceFilter::new(1.0, 2, 7);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);
        let classes = (0..out.len())
            .map(|idx| out.get_f64(idx, &DimId::Classification) as u8)
            .collect::<Vec<_>>();
        assert_eq!(classes, vec![2, 7, 7, 2, 7, 7]);
    }

    #[test]
    fn chooses_available_unclassified_label_between_classes() {
        let view = view(&[(0.0, 0.0, 0.0), (0.0, 0.0, 1.0), (10.0, 0.0, 2.0)]);
        let mut filter = SparseSurfaceFilter::new(1.0, 1, 3);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Classification) as u8, 1);
        assert_eq!(out.get_f64(1, &DimId::Classification) as u8, 3);
        assert_eq!(out.get_f64(2, &DimId::Classification) as u8, 1);
    }
}
