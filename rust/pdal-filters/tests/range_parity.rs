//! Behavioral parity tests for RangeFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::Filter;
use pdal_filters::range::{RangeFilter, RangeLimit};
use std::rc::Rc;

fn make_test_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    // Create points with coordinates (i, i * 10)
    for i in 1..=10 {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, i as f64);
        view.set_f64(idx, &DimId::Y, i as f64 * 10.0);
    }
    view
}

#[test]
fn test_range_filter_simple_inclusive() {
    let view = make_test_view();
    // Pass points where X is in [3.0, 7.0]
    let limit = RangeLimit {
        dim_name: "X".to_string(),
        lower_bound: 3.0,
        upper_bound: 7.0,
        inclusive_lower: true,
        inclusive_upper: true,
        negate: false,
    };
    let mut filter = RangeFilter::new(vec![limit]);
    let out = &filter.run(std::slice::from_ref(&view)).unwrap()[0];

    assert_eq!(out.len(), 5);
    assert_eq!(out.get_f64(0, &DimId::X), 3.0);
    assert_eq!(out.get_f64(4, &DimId::X), 7.0);
}

#[test]
fn test_range_filter_exclusive() {
    let view = make_test_view();
    // Pass points where X is in (3.0, 7.0)
    let limit = RangeLimit {
        dim_name: "X".to_string(),
        lower_bound: 3.0,
        upper_bound: 7.0,
        inclusive_lower: false,
        inclusive_upper: false,
        negate: false,
    };
    let mut filter = RangeFilter::new(vec![limit]);
    let out = &filter.run(std::slice::from_ref(&view)).unwrap()[0];

    assert_eq!(out.len(), 3);
    assert_eq!(out.get_f64(0, &DimId::X), 4.0);
    assert_eq!(out.get_f64(2, &DimId::X), 6.0);
}

#[test]
fn test_range_filter_negated() {
    let view = make_test_view();
    // Pass points where X is NOT in [3.0, 8.0]
    let limit = RangeLimit {
        dim_name: "X".to_string(),
        lower_bound: 3.0,
        upper_bound: 8.0,
        inclusive_lower: true,
        inclusive_upper: true,
        negate: true,
    };
    let mut filter = RangeFilter::new(vec![limit]);
    let out = &filter.run(std::slice::from_ref(&view)).unwrap()[0];

    assert_eq!(out.len(), 4);
    assert_eq!(out.get_f64(0, &DimId::X), 1.0);
    assert_eq!(out.get_f64(1, &DimId::X), 2.0);
    assert_eq!(out.get_f64(2, &DimId::X), 9.0);
    assert_eq!(out.get_f64(3, &DimId::X), 10.0);
}

#[test]
fn test_range_filter_multiple_dimensions_and() {
    let view = make_test_view();
    // Pass points where X is in [3, 8] AND Y is in [40, 60]
    let limit_x = RangeLimit {
        dim_name: "X".to_string(),
        lower_bound: 3.0,
        upper_bound: 8.0,
        inclusive_lower: true,
        inclusive_upper: true,
        negate: false,
    };
    let limit_y = RangeLimit {
        dim_name: "Y".to_string(),
        lower_bound: 40.0,
        upper_bound: 60.0,
        inclusive_lower: true,
        inclusive_upper: true,
        negate: false,
    };
    let mut filter = RangeFilter::new(vec![limit_x, limit_y]);
    let out = &filter.run(std::slice::from_ref(&view)).unwrap()[0];

    // Intersection is X in [4, 6] (since Y is X * 10)
    assert_eq!(out.len(), 3);
    assert_eq!(out.get_f64(0, &DimId::X), 4.0);
    assert_eq!(out.get_f64(2, &DimId::X), 6.0);
}

#[test]
fn test_range_filter_same_dimension_or() {
    let view = make_test_view();
    // Pass points where X is in [1, 2] OR X is in [9, 10]
    let limit1 = RangeLimit {
        dim_name: "X".to_string(),
        lower_bound: 1.0,
        upper_bound: 2.0,
        inclusive_lower: true,
        inclusive_upper: true,
        negate: false,
    };
    let limit2 = RangeLimit {
        dim_name: "X".to_string(),
        lower_bound: 9.0,
        upper_bound: 10.0,
        inclusive_lower: true,
        inclusive_upper: true,
        negate: false,
    };
    let mut filter = RangeFilter::new(vec![limit1, limit2]);
    let out = &filter.run(std::slice::from_ref(&view)).unwrap()[0];

    assert_eq!(out.len(), 4);
    assert_eq!(out.get_f64(0, &DimId::X), 1.0);
    assert_eq!(out.get_f64(1, &DimId::X), 2.0);
    assert_eq!(out.get_f64(2, &DimId::X), 9.0);
    assert_eq!(out.get_f64(3, &DimId::X), 10.0);
}
