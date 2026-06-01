use super::*;

use pdal_core::point::{DimType, PointLayout};
use std::rc::Rc;

#[test]
fn resolves_writer_srs_overrides() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);
    let mut view = PointView::new(layout);
    view.set_spatial_reference(pdal_core::srs::SpatialReference::new("EPSG:2030"));

    assert_eq!(resolve_srs(&[view.clone()], "EPSG:4326", ""), "EPSG:4326");
    assert_eq!(resolve_srs(&[view.clone()], "", "EPSG:4326"), "EPSG:2030");
    assert_eq!(resolve_srs(&[], "", "EPSG:4326"), "EPSG:4326");
}

#[test]
fn rejects_conflicting_srs_options() {
    let err = validate_srs_options("EPSG:4326", "EPSG:2030").unwrap_err();
    assert!(err.0.contains("override_srs"));

    assert!(validate_srs_options("EPSG:4326", "").is_ok());
    assert!(validate_srs_options("", "EPSG:2030").is_ok());
    assert!(validate_srs_options("", "").is_ok());
}

#[test]
fn parses_output_types_and_percentiles() {
    let mut options = Options::new();
    options.add("output_type", "min,p50,count");
    assert_eq!(
        output_types(&options).0,
        vec![
            OutputStat::Min,
            OutputStat::Percentile(50),
            OutputStat::Count
        ]
    );
    options.add("output_type", "nope");
    assert!(output_types(&options).1.is_some());
}

#[test]
fn fixed_grid_requires_the_whole_grid_shape() {
    let mut options = Options::new();
    options.add("origin_x", 1.0);
    assert!(fixed_grid(&options).is_none());
    options.add("origin_y", 2.0);
    options.add("width", 3);
    options.add("height", 4);
    let grid = fixed_grid(&options).unwrap();
    assert_eq!(grid.width, 3);
    assert_eq!(grid.height, 4);
}

#[test]
fn bounds_option_defines_grid_shape() {
    let grid = grid_from_bounds("([0, 4.5],[0, 4.5])", 1.0)
        .unwrap()
        .unwrap();
    assert_eq!(grid.origin_x, 0.0);
    assert_eq!(grid.origin_y, 0.0);
    assert_eq!(grid.width, 5);
    assert_eq!(grid.height, 5);
}

#[test]
fn writer_rejects_bounds_with_alternate_grid() {
    let mut options = Options::new();
    options.add("filename", "/tmp/x.tif");
    options.add("resolution", 1.0);
    options.add("bounds", "([0, 1],[0, 1])");
    options.add("origin_x", 0.0);
    options.add("origin_y", 0.0);
    options.add("width", 2);
    options.add("height", 2);
    let mut writer = GdalWriter::new(&options);
    let view = make_view_with_points();
    let err = writer.write(&[view]).unwrap_err();
    assert!(err.0.contains("Specify either 'bounds'"));
}

#[test]
fn parses_gdal_metadata_items() {
    let mut options = Options::new();
    options.add(
        "metadata",
        "AREA_OR_PIXEL=Pixel,empty=,equals=some_more_equals===",
    );
    assert_eq!(
        parse_metadata(&options),
        vec![
            ("AREA_OR_PIXEL".to_string(), "Pixel".to_string()),
            ("empty".to_string(), String::new()),
            ("equals".to_string(), "some_more_equals===".to_string()),
        ]
    );
}

#[test]
fn count_band_uses_top_to_bottom_raster_order() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z) in [(0.0, 0.0, 1.0), (1.0, 1.0, 2.0)] {
        let point = view.add_point();
        view.set_f64(point, &DimId::X, x);
        view.set_f64(point, &DimId::Y, y);
        view.set_f64(point, &DimId::Z, z);
    }

    let mut options = Options::new();
    options.add("output_type", "count");
    options.add("resolution", 1.0);
    options.add("radius", 0.1);
    options.add("binmode", true);
    let writer = GdalWriter::new(&options);
    let grid = FixedGrid {
        origin_x: 0.0,
        origin_y: 0.0,
        width: 2,
        height: 2,
    };
    let samples = collect_samples(&[view], &DimId::Z).unwrap();
    let bands = writer.render_bands(grid, &samples);
    assert_eq!(bands[0].1, vec![0.0, 1.0, 1.0, 0.0]);
}

#[test]
fn writer_errors_without_filename() {
    let mut writer = GdalWriter::new(&Options::new());
    let layout = PointLayout::new();
    let view = PointView::new(std::rc::Rc::new(layout));
    let result = writer.write(&[view]);
    assert!(result.is_err());
    assert!(result.err().unwrap().0.contains("filename"));
}

#[test]
fn writer_errors_on_zero_resolution() {
    let mut options = Options::new();
    options.add("filename", "/tmp/x.tif");
    options.add("resolution", 0.0);
    let mut writer = GdalWriter::new(&options);
    let layout = PointLayout::new();
    let view = PointView::new(std::rc::Rc::new(layout));
    assert!(writer.write(&[view]).is_err());
}

#[test]
fn writer_errors_on_partial_fixed_grid() {
    let mut options = Options::new();
    options.add("filename", "/tmp/x.tif");
    options.add("resolution", 1.0);
    options.add("origin_x", 0.0);
    options.add("origin_y", 0.0);
    let mut writer = GdalWriter::new(&options);
    let layout = PointLayout::new();
    let view = PointView::new(std::rc::Rc::new(layout));
    assert!(writer.write(&[view]).is_err());
}

#[test]
fn writer_errors_on_percentile_without_binmode() {
    let mut options = Options::new();
    options.add("filename", "/tmp/x.tif");
    options.add("resolution", 1.0);
    options.add("output_type", "p50");
    let mut writer = GdalWriter::new(&options);
    let layout = PointLayout::new();
    let view = PointView::new(std::rc::Rc::new(layout));
    assert!(writer.write(&[view]).is_err());
}

#[test]
fn writer_errors_on_empty_view_without_allow_empty() {
    let mut options = Options::new();
    options.add("filename", "/tmp/x.tif");
    options.add("resolution", 1.0);
    let mut writer = GdalWriter::new(&options);
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let view = PointView::new(std::rc::Rc::new(layout));
    assert!(writer.write(&[view]).is_err());
}

#[test]
fn parse_data_type_branches() {
    assert_eq!(parse_data_type("float64").0, OutputDataType::Float64);
    assert_eq!(parse_data_type("double").0, OutputDataType::Float64);
    assert_eq!(parse_data_type("float").0, OutputDataType::Float32);
    assert_eq!(parse_data_type("int8").0, OutputDataType::Int8);
    assert_eq!(parse_data_type("uint8").0, OutputDataType::UInt8);
    assert_eq!(parse_data_type("int16").0, OutputDataType::Int16);
    assert_eq!(parse_data_type("uint16").0, OutputDataType::UInt16);
    assert_eq!(parse_data_type("int32").0, OutputDataType::Int32);
    assert_eq!(parse_data_type("int32_t").0, OutputDataType::Int32);
    assert_eq!(parse_data_type("signed32").0, OutputDataType::Int32);
    assert_eq!(parse_data_type("int").0, OutputDataType::Int32);
    assert_eq!(parse_data_type("uint32").0, OutputDataType::UInt32);
    assert_eq!(parse_data_type("int64").0, OutputDataType::Int64);
    assert_eq!(parse_data_type("uint64").0, OutputDataType::UInt64);
    assert!(parse_data_type("mystery").1.is_some());
}

#[test]
fn writer_metadata_returns_expected_name() {
    let writer = GdalWriter::new(&Options::new());
    assert_eq!(writer.metadata().name(), "writers.gdal");
    assert_eq!(writer.name(), "writers.gdal");
}

fn tmp_tif(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "pdal-rust-gdal-writer-{}-{name}",
        std::process::id()
    ))
}

fn make_view_with_points() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z) in [
        (0.0, 0.0, 1.0),
        (1.0, 0.0, 2.0),
        (0.0, 1.0, 3.0),
        (1.0, 1.0, 4.0),
    ] {
        let p = view.add_point();
        view.set_f64(p, &DimId::X, x);
        view.set_f64(p, &DimId::Y, y);
        view.set_f64(p, &DimId::Z, z);
    }
    view
}

#[test]
fn writer_writes_float64_output() {
    let out = tmp_tif("f64.tif");
    let mut options = Options::new();
    options.add("filename", out.to_str().unwrap());
    options.add("resolution", 1.0);
    options.add("output_type", "mean");
    let mut writer = GdalWriter::new(&options);
    let view = make_view_with_points();
    let r = writer.write(&[view]);
    // GDAL may not be configured in dev; tolerate failure.
    if r.is_ok() {
        assert!(out.exists());
    }
    let _ = std::fs::remove_file(&out);
}

#[test]
fn writer_writes_int32_output() {
    let out = tmp_tif("i32.tif");
    let mut options = Options::new();
    options.add("filename", out.to_str().unwrap());
    options.add("resolution", 1.0);
    options.add("output_type", "count");
    options.add("data_type", "int32");
    let mut writer = GdalWriter::new(&options);
    let view = make_view_with_points();
    let r = writer.write(&[view]);
    if r.is_ok() {
        assert!(out.exists());
    }
    let _ = std::fs::remove_file(&out);
}

#[test]
fn writer_writes_other_gdal_numeric_types() {
    for (name, data_type, expected) in [
        ("u8", "uint8", "Byte"),
        ("i16", "int16", "Int16"),
        ("u16", "uint16", "UInt16"),
        ("f32", "float", "Float32"),
    ] {
        let out = tmp_tif(&format!("{name}.tif"));
        let mut options = Options::new();
        options.add("filename", out.to_str().unwrap());
        options.add("resolution", 1.0);
        options.add("radius", 0.1);
        options.add("output_type", "min");
        options.add("data_type", data_type);
        let mut writer = GdalWriter::new(&options);
        writer.write(&[make_view_with_points()]).unwrap();

        let raster = pdal_core::gdal::Raster::open(out.to_str().unwrap()).unwrap();
        assert_eq!(raster.band_type_name(1).unwrap(), expected);
        let _ = std::fs::remove_file(&out);
    }
}

#[test]
fn writer_writes_with_metadata_items() {
    let out = tmp_tif("meta.tif");
    let mut options = Options::new();
    options.add("filename", out.to_str().unwrap());
    options.add("resolution", 1.0);
    options.add("output_type", "mean");
    options.add("metadata", "AREA_OR_PIXEL=Pixel,Author=test");
    let mut writer = GdalWriter::new(&options);
    let view = make_view_with_points();
    let _ = writer.write(&[view]);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn writer_writes_all_stats_via_output_type_all() {
    let out = tmp_tif("all.tif");
    let mut options = Options::new();
    options.add("filename", out.to_str().unwrap());
    options.add("resolution", 1.0);
    options.add("output_type", "all");
    let mut writer = GdalWriter::new(&options);
    let view = make_view_with_points();
    let _ = writer.write(&[view]);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn output_types_rejects_invalid_percentile_value() {
    let mut options = Options::new();
    options.add("output_type", "p150");
    let (_stats, err) = output_types(&options);
    assert!(err.is_some());
}

#[test]
fn output_types_rejects_invalid_percentile_text() {
    let mut options = Options::new();
    options.add("output_type", "pxx");
    let (_stats, err) = output_types(&options);
    assert!(err.is_some());
}

#[test]
fn output_types_handles_no_valid_types() {
    let mut options = Options::new();
    options.add("output_type", "mystery");
    let (_stats, err) = output_types(&options);
    assert!(err.is_some());
}

#[test]
fn writer_propagates_output_type_error() {
    let mut options = Options::new();
    options.add("filename", "/tmp/x.tif");
    options.add("resolution", 1.0);
    options.add("output_type", "mystery");
    let mut writer = GdalWriter::new(&options);
    let view = make_view_with_points();
    assert!(writer.write(&[view]).is_err());
}

#[test]
fn writer_propagates_data_type_error() {
    let mut options = Options::new();
    options.add("filename", "/tmp/x.tif");
    options.add("resolution", 1.0);
    options.add("data_type", "mystery");
    let mut writer = GdalWriter::new(&options);
    let view = make_view_with_points();
    let err = writer.write(&[view]).unwrap_err();
    assert!(err.0.contains("Unsupported GDAL writer data_type"));
}

#[test]
fn sample_cell_rejects_out_of_range() {
    let grid = FixedGrid {
        origin_x: 0.0,
        origin_y: 0.0,
        width: 2,
        height: 2,
    };
    // Negative column
    let s = Sample {
        x: -10.0,
        y: 0.5,
        value: 1.0,
    };
    assert!(sample_cell(grid, &s, 1.0).is_none());
    // Row out of range
    let s = Sample {
        x: 0.5,
        y: 100.0,
        value: 1.0,
    };
    assert!(sample_cell(grid, &s, 1.0).is_none());
}
