//! Behavioral parity tests for HeadFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::head::HeadFilter;
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
fn test_head_no_invert() {
    let view = make_ramp_view(10);
    let mut filter = HeadFilter::new(4, false);
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 4);

    for i in 0..4 {
        assert_eq!(out.get_f64(i, &DimId::Z), (i + 1) as f64);
    }
}

#[test]
fn test_head_invert() {
    let view = make_ramp_view(10);
    let mut filter = HeadFilter::new(4, true);
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 6);

    for i in 0..6 {
        assert_eq!(out.get_f64(i, &DimId::Z), (i + 5) as f64);
    }
}

#[test]
fn test_head_stream_no_invert() {
    let mut filter = HeadFilter::new(4, false);
    filter.reset();

    let mut kept = Vec::new();
    for i in 1..=10 {
        if filter.process_one() {
            kept.push(i);
        }
    }
    assert_eq!(kept, vec![1, 2, 3, 4]);
}

#[test]
fn test_head_stream_invert() {
    let mut filter = HeadFilter::new(4, true);
    filter.reset();

    let mut kept = Vec::new();
    for i in 1..=10 {
        if filter.process_one() {
            kept.push(i);
        }
    }
    assert_eq!(kept, vec![5, 6, 7, 8, 9, 10]);
}
