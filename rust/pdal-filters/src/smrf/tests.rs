use super::*;
use crate::range::parse_range_limit;
use pdal_core::point::{DimType, PointLayout};
use std::rc::Rc;

fn to_limit(spec: &str) -> RangeLimit {
    let parsed = parse_range_limit(spec).unwrap();
    RangeLimit {
        dim_name: parsed.dim_name,
        lower_bound: parsed.lower_bound,
        upper_bound: parsed.upper_bound,
        inclusive_lower: parsed.inclusive_lower,
        inclusive_upper: parsed.inclusive_upper,
        negate: parsed.negate,
    }
}

fn grid_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z) in &[
        (0.5, 0.5, 10.0),
        (0.5, 1.5, 12.0),
        (1.5, 0.5, 8.0),
        (1.5, 1.5, 11.0),
    ] {
        let id = view.add_point();
        view.set_f64(id, &DimId::X, *x);
        view.set_f64(id, &DimId::Y, *y);
        view.set_f64(id, &DimId::Z, *z);
    }
    view
}

fn grid_view_with_returns() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::ReturnNumber, DimType::U8);
    layout.register(DimId::NumberOfReturns, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z, rn, nr) in &[
        (0.5, 0.5, 10.0, 1, 1),
        (0.5, 1.5, 12.0, 1, 2),
        (1.5, 0.5, 8.0, 2, 2),
    ] {
        let id = view.add_point();
        view.set_f64(id, &DimId::X, *x);
        view.set_f64(id, &DimId::Y, *y);
        view.set_f64(id, &DimId::Z, *z);
        view.set_f64(id, &DimId::ReturnNumber, *rn as f64);
        view.set_f64(id, &DimId::NumberOfReturns, *nr as f64);
    }
    view
}

fn flat_3x3_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for cx in 0..3 {
        for cy in 0..3 {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, cx as f64 * 2.0 + 0.5);
            view.set_f64(id, &DimId::Y, cy as f64 * 2.0 + 0.5);
            view.set_f64(id, &DimId::Z, 10.0);
        }
    }
    view
}

#[test]
fn rejects_non_positive_cell_size() {
    let mut filter = SmrfFilter::new(0.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("cell"));
}

#[test]
fn rejects_negative_slope() {
    let mut filter = SmrfFilter::new(1.0, -0.1, None, 1.25, 0.5, 2, 1, true, Vec::new());
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("slope"));
}

#[test]
fn rejects_negative_scalar() {
    let mut filter = SmrfFilter::new(1.0, 0.15, None, -1.0, 0.5, 2, 1, true, Vec::new());
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("scalar"));
}

#[test]
fn rejects_negative_threshold() {
    let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, -0.5, 2, 1, true, Vec::new());
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("threshold"));
}

#[test]
fn rejects_non_positive_window() {
    let mut filter = SmrfFilter::new(1.0, 0.15, Some(-1.0), 1.25, 0.5, 2, 1, true, Vec::new());
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("window"));
}

#[test]
fn rejects_negative_cut() {
    let mut filter = SmrfFilter::with_cut(1.0, 0.15, None, 1.25, 0.5, -1.0, 2, 1, true, Vec::new());
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("cut"));
}

#[test]
fn rejects_equal_classes_when_not_only_ground() {
    let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 2, false, Vec::new());
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("class"));
}

#[test]
fn rejects_unknown_returns_value() {
    let mut filter = SmrfFilter::new(
        1.0,
        0.15,
        None,
        1.25,
        0.5,
        2,
        1,
        true,
        vec!["middle".to_string()],
    );
    let err = filter.run_one(&grid_view()).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("Unrecognized 'returns'"));
}

#[test]
fn rejects_empty_input() {
    let layout = PointLayout::new();
    let empty = PointView::new(Rc::new(layout));
    let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    let err = filter.run_one(&empty).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("No returns"));
}

#[test]
fn rejects_mixed_zero_and_nonzero_returns() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::ReturnNumber, DimType::U8);
    layout.register(DimId::NumberOfReturns, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for (rn, nr) in &[(1u8, 1u8), (0, 0), (1, 2)] {
        let id = view.add_point();
        view.set_f64(id, &DimId::X, 0.5);
        view.set_f64(id, &DimId::Y, 0.5);
        view.set_f64(id, &DimId::Z, 10.0);
        view.set_f64(id, &DimId::ReturnNumber, *rn as f64);
        view.set_f64(id, &DimId::NumberOfReturns, *nr as f64);
    }
    let mut filter = SmrfFilter::new(
        1.0,
        0.15,
        None,
        1.25,
        0.5,
        2,
        1,
        true,
        vec!["last".to_string()],
    );
    let err = filter.run_one(&view).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains("NumberOfReturns or ReturnNumber"));
}

#[test]
fn all_zero_returns_falls_back_to_all_points() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::ReturnNumber, DimType::U8);
    layout.register(DimId::NumberOfReturns, DimType::U8);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for _ in 0..4 {
        let id = view.add_point();
        view.set_f64(id, &DimId::X, 0.5);
        view.set_f64(id, &DimId::Y, 0.5);
        view.set_f64(id, &DimId::Z, 10.0);
    }
    let mut filter = SmrfFilter::new(
        1.0,
        0.15,
        None,
        1.25,
        0.5,
        2,
        1,
        true,
        vec!["last".to_string()],
    );
    let result = filter.run_one(&view).unwrap();
    assert_eq!(result[0].len(), 4);
}

#[test]
fn smrf_names() {
    let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    assert_eq!(filter.name(), "filters.smrf");
}

#[test]
fn smrf_metadata() {
    let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    let m = filter.metadata();
    assert_eq!(m.name(), "filters.smrf");
}

#[test]
fn smrf_output_dimensions() {
    let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    let dims = filter.output_dimensions();
    assert_eq!(dims, vec![(DimId::Classification, DimType::U8)]);
}

#[test]
fn smrf_process_one_passes_through() {
    let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    let mut view = grid_view();
    assert!(filter.process_one(&mut view, 0));
}

#[test]
fn smrf_classifies_flat_ground() {
    let mut filter = SmrfFilter::new(2.0, 0.15, None, 0.5, 0.5, 2, 1, true, Vec::new());
    let result = filter.run_one(&flat_3x3_view()).unwrap();
    assert_eq!(result.len(), 1);
    for i in 0..result[0].len() {
        assert_eq!(result[0].get_f64(i, &DimId::Classification), 2.0);
    }
}

#[test]
fn smrf_returns_filter_first_only() {
    let mut filter = SmrfFilter::new(
        1.0,
        0.15,
        None,
        0.5,
        0.5,
        2,
        1,
        true,
        vec!["first".to_string()],
    );
    let result = filter.run_one(&grid_view_with_returns()).unwrap();
    assert_eq!(result.len(), 1);
    assert!(!result[0].is_empty());
}

#[test]
fn smrf_returns_filter_last_only() {
    let mut filter = SmrfFilter::new(
        1.0,
        0.15,
        None,
        0.5,
        0.5,
        2,
        1,
        true,
        vec!["last".to_string()],
    );
    let result = filter.run_one(&grid_view_with_returns()).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn smrf_returns_filter_only() {
    let mut filter = SmrfFilter::new(
        1.0,
        0.15,
        None,
        0.5,
        0.5,
        2,
        1,
        true,
        vec!["only".to_string()],
    );
    let result = filter.run_one(&grid_view_with_returns()).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn smrf_returns_filter_intermediate() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::ReturnNumber, DimType::U8);
    layout.register(DimId::NumberOfReturns, DimType::U8);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for (rn, nr) in &[(1u8, 3u8), (2, 3), (3, 3)] {
        let id = view.add_point();
        view.set_f64(id, &DimId::X, 0.5);
        view.set_f64(id, &DimId::Y, 0.5);
        view.set_f64(id, &DimId::Z, 10.0);
        view.set_f64(id, &DimId::ReturnNumber, *rn as f64);
        view.set_f64(id, &DimId::NumberOfReturns, *nr as f64);
    }
    let mut filter = SmrfFilter::new(
        1.0,
        0.15,
        None,
        0.5,
        0.5,
        2,
        1,
        true,
        vec!["intermediate".to_string()],
    );
    let result = filter.run_one(&view).unwrap();
    assert_eq!(result[0].len(), 3);
    // The intermediate return (rn=2, nr=3) should be classified ground.
    assert_eq!(result[0].get_f64(1, &DimId::Classification), 2.0);
    // First and last returns were not selected, so they keep their original
    // (zero) Classification.
    assert_eq!(result[0].get_f64(0, &DimId::Classification), 0.0);
    assert_eq!(result[0].get_f64(2, &DimId::Classification), 0.0);
}

#[test]
fn pre_classifies_other_when_not_only_ground() {
    // An obvious object point well above the ground should end up as
    // other_class even though its 1x1 cell may have NaN gradient.
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    // 4x4 flat ground with one high spike.
    for cx in 0..4 {
        for cy in 0..4 {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, cx as f64 + 0.5);
            view.set_f64(id, &DimId::Y, cy as f64 + 0.5);
            view.set_f64(id, &DimId::Z, 10.0);
        }
    }
    let spike = view.add_point();
    view.set_f64(spike, &DimId::X, 1.5);
    view.set_f64(spike, &DimId::Y, 1.5);
    view.set_f64(spike, &DimId::Z, 110.0);

    let mut filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, false, Vec::new());
    let result = filter.run_one(&view).unwrap();
    // The spike must be classified as object (1), not ground.
    let cls = result[0].get_f64(spike, &DimId::Classification);
    assert_eq!(cls, 1.0, "spike point should be classified as other");
}

#[test]
fn net_mask_off_when_cut_zero() {
    let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    let mask = filter.net_mask(5, 5);
    assert!(mask.iter().all(|&v| v == 0));
}

#[test]
fn net_mask_marks_grid_when_cut_positive() {
    let filter = SmrfFilter::with_cut(1.0, 0.15, None, 1.25, 0.5, 3.0, 2, 1, true, Vec::new());
    let mask = filter.net_mask(6, 6);
    // First column (c=0) is fully set; row 0 of every column is set.
    for value in mask.iter().take(6) {
        assert_eq!(*value, 1);
    }
    for c in 0..6 {
        assert_eq!(mask[c * 6], 1);
    }
    // (c=1,r=1) should not be on the net.
    assert_eq!(mask[7], 0);
}

fn flat_with_marked_point(class_value: f64) -> (PointView, PointId) {
    // 4x4 flat ground at z=10 plus one extra point sharing a ground cell
    // but pre-tagged with `class_value` so we can watch whether smrf
    // reclassifies it.
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for cx in 0..4 {
        for cy in 0..4 {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, cx as f64 + 0.5);
            view.set_f64(id, &DimId::Y, cy as f64 + 0.5);
            view.set_f64(id, &DimId::Z, 10.0);
        }
    }
    let marked = view.add_point();
    view.set_f64(marked, &DimId::X, 1.5);
    view.set_f64(marked, &DimId::Y, 1.5);
    view.set_f64(marked, &DimId::Z, 10.0);
    view.set_f64(marked, &DimId::Classification, class_value);
    (view, marked)
}

#[test]
fn ignore_dimrange_leaves_matching_points_untouched() {
    let (view, marked) = flat_with_marked_point(7.0);
    let ignore = vec![to_limit("Classification[7:7]")];
    let mut filter = SmrfFilter::with_segmentation(
        1.0,
        0.15,
        None,
        1.25,
        0.5,
        0.0,
        2,
        1,
        false,
        Vec::new(),
        ignore,
        0,
    );
    let result = filter.run_one(&view).unwrap();
    // The ignored point keeps its original Classification (7), never reset
    // to other_class or reclassified as ground.
    assert_eq!(result[0].get_f64(marked, &DimId::Classification), 7.0);
    // The surrounding flat ground is still segmented.
    assert_eq!(result[0].get_f64(0, &DimId::Classification), 2.0);
}

#[test]
fn classbits_excludes_flagged_points() {
    // Mark the extra point synthetic (bit 32) and ask smrf to ignore it.
    let (view, marked) = flat_with_marked_point(CLASSBIT_SYNTHETIC as f64);
    let mut filter = SmrfFilter::with_segmentation(
        1.0,
        0.15,
        None,
        1.25,
        0.5,
        0.0,
        2,
        1,
        false,
        Vec::new(),
        Vec::new(),
        CLASSBIT_SYNTHETIC,
    );
    let result = filter.run_one(&view).unwrap();
    // Synthetic point untouched; flat ground still classified.
    assert_eq!(
        result[0].get_f64(marked, &DimId::Classification),
        CLASSBIT_SYNTHETIC as f64
    );
    assert_eq!(result[0].get_f64(0, &DimId::Classification), 2.0);
}

#[test]
fn classbits_zero_keeps_every_point_in_segmentation() {
    // With classbits unset, a synthetic-flagged point is still segmented
    // (reset to other_class here since it sits on flat ground that classifies
    // as ground -> 2).
    let (view, marked) = flat_with_marked_point(CLASSBIT_SYNTHETIC as f64);
    let candidates = segmentation_candidates(&view, &[], 0);
    assert!(candidates.contains(&marked));
    let mut filter = SmrfFilter::with_cut(1.0, 0.15, None, 1.25, 0.5, 0.0, 2, 1, false, Vec::new());
    let result = filter.run_one(&view).unwrap();
    assert_eq!(result[0].get_f64(marked, &DimId::Classification), 2.0);
}

#[test]
fn dir_writes_intermediate_rasters() {
    use pdal_core::gdal::Raster;
    let dir = std::env::temp_dir().join(format!("pdal-rs-smrf-dir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut filter = SmrfFilter::new(2.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new())
        .with_dir(Some(dir.to_str().unwrap().to_string()));
    filter.run_one(&flat_3x3_view()).unwrap();

    // The 3x3 flat view (X,Y from 0.5..4.5, cell 2) yields a 3x3 grid.
    for name in [
        "zimin.tif",
        "zimin_fill.tif",
        "zilow.tif",
        "zinet.tif",
        "ziobj.tif",
        "zipro.tif",
        "zipro_fill.tif",
        "gx.tif",
        "gy.tif",
        "gsurfs.tif",
        "gsurfs_fill.tif",
        "thresh.tif",
    ] {
        let path = dir.join(name);
        assert!(path.exists(), "missing debug raster {name}");
        let raster = Raster::open(path.to_str().unwrap()).unwrap();
        assert_eq!(raster.width(), 3, "{name} width");
        assert_eq!(raster.height(), 3, "{name} height");
        let gt = raster.get_geo_transform().unwrap();
        // South-up geotransform [minx, cell, 0, miny, 0, +cell].
        assert_eq!(gt[0], 0.5, "{name} minx");
        assert_eq!(gt[1], 2.0, "{name} x pixel size");
        assert_eq!(gt[3], 0.5, "{name} miny");
        assert_eq!(gt[5], 2.0, "{name} y pixel size");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn smrf_knn_fill_all_nan_stays_nan() {
    let mut data = vec![f64::NAN; 9];
    let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    filter.knn_fill(&mut data, 3, 3, 0.0, 0.0);
    for v in &data {
        assert!(v.is_nan());
    }
}

#[test]
fn smrf_knn_fill_single_nan_uses_neighbors() {
    let mut data = vec![1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0, 7.0, 8.0, 9.0];
    let filter = SmrfFilter::new(1.0, 0.15, None, 1.25, 0.5, 2, 1, true, Vec::new());
    filter.knn_fill(&mut data, 3, 3, 0.0, 0.0);
    // Eight nearest fill values are the eight non-NaN cells; their mean is 5.0.
    assert!((data[4] - 5.0).abs() < 1e-9);
}
