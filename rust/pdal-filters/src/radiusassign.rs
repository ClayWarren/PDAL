use crate::range::RangeLimit;
use pdal_core::point::{DimId, PointId, PointView};

pub fn update_indices(
    view: &PointView,
    src_domain: &[RangeLimit],
    reference_domain: &[RangeLimit],
    radius: f64,
    search_3d: bool,
    max_2d_above: f64,
    max_2d_below: f64,
) -> Vec<PointId> {
    let reference = (0..view.len())
        .filter(|idx| domain_passes(reference_domain, view, *idx))
        .collect::<Vec<_>>();
    let mut updates = Vec::new();

    for idx in 0..view.len() {
        if !domain_passes(src_domain, view, idx) {
            continue;
        }
        if has_neighbor(
            view,
            idx,
            &reference,
            radius,
            search_3d,
            max_2d_above,
            max_2d_below,
        ) {
            updates.push(idx);
        }
    }

    updates
}

fn has_neighbor(
    view: &PointView,
    src: PointId,
    reference: &[PointId],
    radius: f64,
    search_3d: bool,
    max_2d_above: f64,
    max_2d_below: f64,
) -> bool {
    let radius_sqr = radius * radius;
    let x = view.get_f64(src, &DimId::X);
    let y = view.get_f64(src, &DimId::Y);
    let z = view.get_f64(src, &DimId::Z);

    for candidate in reference {
        let dx = view.get_f64(*candidate, &DimId::X) - x;
        let dy = view.get_f64(*candidate, &DimId::Y) - y;
        let mut distance = dx * dx + dy * dy;
        if search_3d {
            let dz = view.get_f64(*candidate, &DimId::Z) - z;
            distance += dz * dz;
        }
        if distance >= radius_sqr {
            continue;
        }

        if !search_3d && (max_2d_below >= 0.0 || max_2d_above >= 0.0) {
            let z_ref = view.get_f64(*candidate, &DimId::Z);
            if max_2d_above >= 0.0 && z_ref > z && z_ref - z > max_2d_above {
                continue;
            }
            if max_2d_below >= 0.0 && z > z_ref && z - z_ref > max_2d_below {
                continue;
            }
        }

        return true;
    }

    false
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

        passes = range.value_passes(view.get_f64(idx, &DimId::from_name(&range.dim_name)));
    }

    passes
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
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, class) in [
            (0.0, 0.0, 0.0, 1.0),
            (1.0, 0.0, 0.0, 0.0),
            (5.0, 0.0, 0.0, 0.0),
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
    fn selects_points_near_reference_domain() {
        let reference = vec![RangeLimit {
            dim_name: "Classification".to_string(),
            lower_bound: 1.0,
            upper_bound: 1.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
        assert_eq!(
            update_indices(&view(), &[], &reference, 1.0, false, -1.0, -1.0),
            vec![0]
        );
    }
}
