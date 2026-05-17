//! Behavioral parity tests for TailFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::tail::TailFilter;
use std::rc::Rc;

/// Build a ramp view matching FauxReader "ramp" mode with bounds Z=1..10
fn make_ramp_view(count: u64) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for i in 1..=count {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, 0.0);
        view.set_f64(idx, &DimId::Y, 0.0);
        view.set_f64(idx, &DimId::Z, i as f64);
    }
    view
}

#[test]
fn test_tail_no_invert() {
    let view = make_ramp_view(10);
    let mut filter = TailFilter::new(4, false);
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 4);

    for i in 0..4 {
        assert_eq!(out.get_f64(i, &DimId::Z), (i + 7) as f64);
    }
}

#[test]
fn test_tail_invert() {
    let view = make_ramp_view(10);
    let mut filter = TailFilter::new(4, true);
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 6);

    for i in 0..6 {
        assert_eq!(out.get_f64(i, &DimId::Z), (i + 1) as f64);
    }
}

#[test]
fn test_tail_stream() {
    let mut filter = TailFilter::new(4, false);
    filter.reset();
    // Streamable::process_one should always return false for TailFilter as it does not stream
    assert!(!filter.process_one());
}
