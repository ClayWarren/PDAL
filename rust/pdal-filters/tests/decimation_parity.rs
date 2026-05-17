//! Behavioral parity tests for DecimationFilter.
//!
//! These replicate the assertions from the C++ `DecimationFilterTest` suite,
//! validating that the Rust implementation produces identical outputs for
//! identical inputs.

use pdal_core::options::Options;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::decimation::DecimationFilter;
use std::rc::Rc;

/// Build a PointView with `count` points where OffsetTime = 0, 1, 2, …
fn make_sequential_view(count: u64) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::OffsetTime, DimType::U64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for i in 0..count {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::OffsetTime, i as f64);
    }
    view
}

/// Build a ramp view where X=i, Y=i for i in 0..count (matches FauxReader
/// "ramp" mode with bounds (0,0,0)-(count-1, count-1, count-1)).
#[allow(dead_code)]
fn make_ramp_view(count: u64) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::OffsetTime, DimType::U64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for i in 0..count {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, i as f64);
        view.set_f64(idx, &DimId::Y, i as f64);
        view.set_f64(idx, &DimId::Z, i as f64);
        view.set_f64(idx, &DimId::OffsetTime, i as f64);
    }
    view
}

/// C++ test: DecimationFilterTest.test1
/// 30 points, step=10 → 3 output points at OffsetTime 0, 10, 20
#[test]
fn test1_step10_count30() {
    let view = make_sequential_view(30);

    let mut opts = Options::new();
    opts.add("step", 10);

    let mut filter = DecimationFilter::new(&opts);
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 3);

    assert_eq!(out.get_f64(0, &DimId::OffsetTime) as u64, 0);
    assert_eq!(out.get_f64(1, &DimId::OffsetTime) as u64, 10);
    assert_eq!(out.get_f64(2, &DimId::OffsetTime) as u64, 20);
}

/// C++ test: DecimationFilterTest.fpstep
/// 30 points, step=4.2 → 7 output points at OffsetTime 0, 4, 8, 13, 17, 21, 25
#[test]
fn fpstep_step4_2_count30() {
    let view = make_sequential_view(30);

    let mut opts = Options::new();
    opts.add("step", 4.2);

    let mut filter = DecimationFilter::new(&opts);
    let outputs = filter.run(&view).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 7);

    let expected: Vec<u64> = vec![0, 4, 8, 13, 17, 21, 25];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(
            out.get_f64(i as u64, &DimId::OffsetTime) as u64,
            exp,
            "mismatch at output index {i}"
        );
    }
}

/// C++ test: DecimationFilterTest.stream
/// 100 ramp points, step=10, offset=10, limit=90
/// Streaming should keep 8 points at indices 10, 20, 30, ..., 80
#[test]
fn stream_step10_offset10_limit90() {
    let mut opts = Options::new();
    opts.add("step", 10);
    opts.add("offset", 10u64);
    opts.add("limit", 90u64);

    let mut filter = DecimationFilter::new(&opts);
    filter.reset();

    let mut view = make_sequential_view(100);
    let mut kept_count = 0u32;
    let mut kept_indices: Vec<u64> = Vec::new();
    for i in 0u64..100 {
        if filter.process_one(&mut view, i) {
            kept_count += 1;
            kept_indices.push(i);
        }
    }

    assert_eq!(kept_count, 8);
    let expected: Vec<u64> = vec![10, 20, 30, 40, 50, 60, 70, 80];
    assert_eq!(kept_indices, expected);
}

/// C++ test: DecimationFilterTest.stream_fpstep
/// 100 ramp points, step=2.6, offset=10, limit=90
/// Streaming should keep exactly the 31 indices from the C++ test.
#[test]
fn stream_fpstep_step2_6_offset10_limit90() {
    let mut opts = Options::new();
    opts.add("step", 2.6);
    opts.add("offset", 10u64);
    opts.add("limit", 90u64);

    let mut filter = DecimationFilter::new(&opts);
    filter.reset();

    let mut view = make_sequential_view(100);
    let mut kept: Vec<u64> = Vec::new();
    for i in 0u64..100 {
        if filter.process_one(&mut view, i) {
            kept.push(i);
        }
    }

    let expected: Vec<u64> = vec![
        10, 13, 15, 18, 20, 23, 26, 28, 31, 33, 36, 39, 41, 44, 46, 49, 52, 54, 57, 59, 62, 65, 67,
        70, 72, 75, 78, 80, 83, 85, 88,
    ];

    assert_eq!(kept.len(), 31);
    assert_eq!(kept, expected);
}
