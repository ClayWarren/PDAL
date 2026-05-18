//! Behavioral parity tests for RandomizeFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::Filter;
use pdal_filters::randomize::RandomizeFilter;
use std::rc::Rc;

fn make_test_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for i in 1..=10 {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::Z, i as f64);
    }
    view
}

#[test]
fn test_randomize_shuffled_elements() {
    let view = make_test_view();
    let mut filter = RandomizeFilter::new(Some(12345));
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 1);

    let out = &outputs[0];
    assert_eq!(out.len(), 10);

    // Verify all original elements are present in the output
    let mut vals: Vec<f64> = (0..10).map(|i| out.get_f64(i, &DimId::Z)).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        vals,
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    );

    // Verify ordering is shuffled (not identical sequential 1..10)
    let ordered: Vec<f64> = (0..10).map(|i| out.get_f64(i, &DimId::Z)).collect();
    assert_ne!(
        ordered,
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    );
}

#[test]
fn test_randomize_seeded_reproducibility() {
    let view = make_test_view();

    let mut filter1 = RandomizeFilter::new(Some(999));
    let out1 = &filter1.run(std::slice::from_ref(&view)).unwrap()[0];

    let mut filter2 = RandomizeFilter::new(Some(999));
    let out2 = &filter2.run(std::slice::from_ref(&view)).unwrap()[0];

    let vals1: Vec<f64> = (0..10).map(|i| out1.get_f64(i, &DimId::Z)).collect();
    let vals2: Vec<f64> = (0..10).map(|i| out2.get_f64(i, &DimId::Z)).collect();

    // Verify seeded shuffle is perfectly deterministic
    assert_eq!(vals1, vals2);
}
