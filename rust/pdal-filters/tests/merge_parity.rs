//! Behavioral parity tests for MergeFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::merge::MergeFilter;
use std::rc::Rc;

fn make_view(values: &[f64]) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for &v in values {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::Z, v);
    }
    view
}

#[test]
fn test_merge_two_views() {
    let view1 = make_view(&[1.0, 2.0]);
    let view2 = make_view(&[3.0, 4.0]);
    let mut filter = MergeFilter::new();
    let outputs = filter.run(&[view1, view2]).unwrap();
    assert_eq!(outputs.len(), 1);
    let out = &outputs[0];
    assert_eq!(out.len(), 4);
    let vals: Vec<f64> = (0..4).map(|i| out.get_f64(i, &DimId::Z)).collect();
    assert_eq!(vals, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_merge_empty_inputs() {
    let mut filter = MergeFilter::new();
    let outputs = filter.run(&[]).unwrap();
    assert!(outputs.is_empty());
}

#[test]
fn test_merge_single_view() {
    let view = make_view(&[1.0, 2.0]);
    let mut filter = MergeFilter::new();
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].len(), 2);
}

#[test]
fn test_merge_reset_clears_accumulated() {
    let view1 = make_view(&[1.0, 2.0]);
    let view2 = make_view(&[3.0, 4.0]);
    let mut filter = MergeFilter::new();

    let outputs = filter.run(&[view1]).unwrap();
    assert_eq!(outputs[0].len(), 2);

    filter.reset();

    let outputs = filter.run(&[view2]).unwrap();
    assert_eq!(outputs[0].len(), 2);
}

#[test]
fn test_merge_run_one_delegates_to_run() {
    let view = make_view(&[1.0, 2.0, 3.0]);
    let mut filter = MergeFilter::new();
    let outputs = filter.run_one(&view).unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].len(), 3);
}

#[test]
fn test_merge_process_one_returns_true() {
    let mut filter = MergeFilter::new();
    let mut scratch = PointView::new(Rc::new(PointLayout::new()));
    assert!(filter.process_one(&mut scratch, 0));
}

#[test]
fn test_merge_default_constructs() {
    let mut filter: MergeFilter = Default::default();
    let view = make_view(&[1.0]);
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs[0].len(), 1);
}

#[test]
fn test_merge_names() {
    let filter = MergeFilter::new();
    assert_eq!(filter.name(), "filters.merge");
    assert!(filter.as_any().downcast_ref::<MergeFilter>().is_some());
}
