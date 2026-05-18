//! Behavioral parity tests for FerryFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::Filter;
use pdal_filters::ferry::FerryFilter;
use std::rc::Rc;

fn make_test_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Intensity, DimType::F64);
    layout.register(DimId::Other("TargetDim".to_string()), DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for i in 1..=5 {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, i as f64 * 10.0);
        view.set_f64(idx, &DimId::Y, i as f64 * 20.0);
        view.set_f64(idx, &DimId::Z, 0.0);
        view.set_f64(idx, &DimId::Intensity, 0.0);
        view.set_f64(idx, &DimId::Other("TargetDim".to_string()), 0.0);
    }
    view
}

#[test]
fn test_ferry_copy_single() {
    let view = make_test_view();
    let mut filter = FerryFilter::new(vec![("X".to_string(), "Z".to_string())]);
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 5);

    for i in 0..5 {
        let x_val = out.get_f64(i, &DimId::X);
        let z_val = out.get_f64(i, &DimId::Z);
        assert_eq!(z_val, x_val);
        assert_eq!(z_val, (i as f64 + 1.0) * 10.0);
    }
}

#[test]
fn test_ferry_copy_multiple() {
    let view = make_test_view();
    let mut filter = FerryFilter::new(vec![
        ("X".to_string(), "Z".to_string()),
        ("Y".to_string(), "TargetDim".to_string()),
    ]);
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 5);

    for i in 0..5 {
        let x_val = out.get_f64(i, &DimId::X);
        let y_val = out.get_f64(i, &DimId::Y);
        let z_val = out.get_f64(i, &DimId::Z);
        let target_val = out.get_f64(i, &DimId::Other("TargetDim".to_string()));

        assert_eq!(z_val, x_val);
        assert_eq!(target_val, y_val);
    }
}
