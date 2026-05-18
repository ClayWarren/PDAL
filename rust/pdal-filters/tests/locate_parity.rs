//! Behavioral parity tests for LocateFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::locate::LocateFilter;
use std::rc::Rc;

/// Build a custom view with Z values to locate min/max
fn make_custom_view(values: &[f64]) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for &val in values {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, 0.0);
        view.set_f64(idx, &DimId::Y, 0.0);
        view.set_f64(idx, &DimId::Z, val);
    }
    view
}

#[test]
fn test_locate_max() {
    let view = make_custom_view(&[10.0, 50.0, 20.0, 30.0]);
    let mut filter = LocateFilter::new("Z".to_string(), "max".to_string());
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 1);
    assert_eq!(out.get_f64(0, &DimId::Z), 50.0);
}

#[test]
fn test_locate_min() {
    let view = make_custom_view(&[10.0, 50.0, 5.0, 30.0]);
    let mut filter = LocateFilter::new("Z".to_string(), "min".to_string());
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 1);
    assert_eq!(out.get_f64(0, &DimId::Z), 5.0);
}

#[test]
fn test_locate_empty() {
    let view = make_custom_view(&[]);
    let mut filter = LocateFilter::new("Z".to_string(), "max".to_string());
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].len(), 0);
}

#[test]
fn test_locate_stream() {
    let mut filter = LocateFilter::new("Z".to_string(), "max".to_string());
    filter.reset();
    // Streamable::process_one should always return false for LocateFilter as it does not stream
    let scratch =
        pdal_core::point::PointView::new(std::rc::Rc::new(pdal_core::point::PointLayout::new()));
    assert!(!filter.process_one(&scratch, 0));
}
