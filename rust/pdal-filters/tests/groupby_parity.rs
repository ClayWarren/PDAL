//! Behavioral parity tests for GroupByFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::groupby::GroupByFilter;
use std::rc::Rc;

fn make_classification_view(values: &[i64]) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::Classification, DimType::F64);
    layout.register(DimId::X, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for &val in values {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::Classification, val as f64);
        view.set_f64(idx, &DimId::X, 1.0);
    }
    view
}

#[test]
fn test_groupby_splits_by_classification() {
    let view = make_classification_view(&[1, 2, 1, 3, 2, 1]);
    let mut filter = GroupByFilter::new("Classification".to_string());
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();

    // 3 groups: 1, 2, 3
    assert_eq!(outputs.len(), 3);

    let group_sizes: Vec<usize> = outputs.iter().map(|v| v.len() as usize).collect();
    assert!(group_sizes.contains(&3)); // three points with Classification=1
    assert!(group_sizes.contains(&2)); // two points with Classification=2
    assert!(group_sizes.contains(&1)); // one point with Classification=3
}

#[test]
fn test_groupby_single_value() {
    let view = make_classification_view(&[5, 5, 5]);
    let mut filter = GroupByFilter::new("Classification".to_string());
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].len(), 3);
}

#[test]
fn test_groupby_empty_input() {
    let layout = Rc::new(PointLayout::new());
    let view = PointView::new(layout);
    let mut filter = GroupByFilter::new("Classification".to_string());
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 0);
}

#[test]
fn test_groupby_names() {
    let filter = GroupByFilter::new("X".to_string());
    assert_eq!(filter.name(), "filters.groupby");
    assert!(filter.as_any().downcast_ref::<GroupByFilter>().is_some());
}

#[test]
fn test_groupby_process_one_returns_false() {
    let mut filter = GroupByFilter::new("X".to_string());
    let mut scratch = PointView::new(Rc::new(PointLayout::new()));
    assert!(!filter.process_one(&mut scratch, 0));
}

#[test]
fn test_groupby_reset() {
    let view = make_classification_view(&[1, 2, 3]);
    let mut filter = GroupByFilter::new("Classification".to_string());
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 3);

    filter.reset();

    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 3);
}
