//! Behavioral parity tests for SortFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::sort::{SortAlgorithm, SortFilter, SortOrder};
use std::rc::Rc;

fn make_xyz_view(points: &[(f64, f64, f64)]) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for &(x, y, z) in points {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Y, y);
        view.set_f64(idx, &DimId::Z, z);
    }
    view
}

fn extract_xs(view: &PointView) -> Vec<f64> {
    (0..view.len())
        .map(|i| view.get_f64(i, &DimId::X))
        .collect()
}

#[test]
fn test_sort_ascending() {
    let view = make_xyz_view(&[(3.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0)]);
    let mut filter = SortFilter::new(vec!["X".to_string()], SortOrder::Asc, SortAlgorithm::Normal);
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(extract_xs(&outputs[0]), vec![1.0, 2.0, 3.0]);
}

#[test]
fn run_owned_matches_run_output() {
    // The in-place owned path (used by the pipeline executor) must produce the
    // same points, order, and source indices as the allocating borrow path.
    let points = [
        (3.0, 9.0, 0.0),
        (1.0, 7.0, 0.0),
        (2.0, 8.0, 0.0),
        (2.0, 6.0, 0.0),
        (0.0, 5.0, 0.0),
    ];
    for order in [SortOrder::Asc, SortOrder::Desc] {
        let view = make_xyz_view(&points);

        let mut borrow_filter =
            SortFilter::new(vec!["X".to_string()], order, SortAlgorithm::Stable);
        let borrowed = borrow_filter.run(std::slice::from_ref(&view)).unwrap();

        let mut owned_filter = SortFilter::new(vec!["X".to_string()], order, SortAlgorithm::Stable);
        let owned = owned_filter.run_owned(vec![view.clone()]).unwrap();

        assert_eq!(extract_xs(&owned[0]), extract_xs(&borrowed[0]));
        assert_eq!(owned[0].len(), borrowed[0].len());
        for i in 0..owned[0].len() {
            assert_eq!(
                owned[0].source_index(i),
                borrowed[0].source_index(i),
                "source index mismatch at {i} for {order:?}"
            );
            assert_eq!(
                owned[0].get_f64(i, &DimId::Y),
                borrowed[0].get_f64(i, &DimId::Y)
            );
        }
    }
}

#[test]
fn test_sort_descending() {
    let view = make_xyz_view(&[(3.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0)]);
    let mut filter = SortFilter::new(
        vec!["X".to_string()],
        SortOrder::Desc,
        SortAlgorithm::Normal,
    );
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(extract_xs(&outputs[0]), vec![3.0, 2.0, 1.0]);
}

#[test]
fn test_sort_stable_preserves_order_for_equal_values() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Classification, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for &(x, cls) in &[(1.0, 2.0), (1.0, 1.0), (2.0, 3.0)] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Classification, cls);
    }

    let mut filter = SortFilter::new(vec!["X".to_string()], SortOrder::Asc, SortAlgorithm::Stable);
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    let out = &outputs[0];
    assert_eq!(out.get_f64(0, &DimId::Classification), 2.0);
    assert_eq!(out.get_f64(1, &DimId::Classification), 1.0);
    assert_eq!(out.get_f64(2, &DimId::Classification), 3.0);
}

#[test]
fn test_sort_multi_dimension() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for &(x, y) in &[(1.0, 3.0), (1.0, 1.0), (2.0, 2.0)] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Y, y);
    }

    let mut filter = SortFilter::new(
        vec!["X".to_string(), "Y".to_string()],
        SortOrder::Asc,
        SortAlgorithm::Normal,
    );
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    let out = &outputs[0];
    // Multi-dim: each pass re-sorts by that dimension;
    // last pass (Y) is the primary sort order
    assert_eq!(out.get_f64(0, &DimId::Y), 1.0);
    assert_eq!(out.get_f64(1, &DimId::Y), 2.0);
    assert_eq!(out.get_f64(2, &DimId::X), 1.0); // point with Y=3 has X=1
}

#[test]
fn test_sort_empty_input() {
    let layout = Rc::new(PointLayout::new());
    let view = PointView::new(layout);
    let mut filter = SortFilter::new(vec!["X".to_string()], SortOrder::Asc, SortAlgorithm::Normal);
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs[0].len(), 0);
}

#[test]
fn test_sort_process_one_returns_false() {
    let mut filter = SortFilter::new(vec!["X".to_string()], SortOrder::Asc, SortAlgorithm::Normal);
    let mut scratch = PointView::new(Rc::new(PointLayout::new()));
    assert!(!filter.process_one(&mut scratch, 0));
}

#[test]
fn test_sort_reset() {
    let view = make_xyz_view(&[(3.0, 0.0, 0.0), (1.0, 0.0, 0.0)]);
    let mut filter = SortFilter::new(vec!["X".to_string()], SortOrder::Asc, SortAlgorithm::Normal);
    filter.reset();
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(extract_xs(&outputs[0]), vec![1.0, 3.0]);
}
