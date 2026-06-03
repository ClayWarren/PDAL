use super::*;
use pdal_core::options::Options;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use std::path::Path;
use std::rc::Rc;

#[test]
fn pipeline_json_runs_reader_filter_writer_with_inferred_drivers() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let output =
        std::env::temp_dir().join(format!("pdal-rust-registry-{}.pcd", std::process::id()));
    let _ = std::fs::remove_file(&output);

    let json = format!(
        r#"[
                {{"filename":"{}"}},
                {{"type":"filters.decimation", "step":2}},
                {{"filename":"{}"}}
            ]"#,
        escape_json_path(&input),
        escape_json_path(&output)
    );

    let mut pipeline = pipeline_from_json(&json).unwrap();
    let result = pipeline.execute_with_result(Vec::new()).unwrap();
    assert_eq!(result.point_count, 5);
    assert_eq!(result.view_count, 1);

    assert!(output.exists());
    let written = std::fs::read_to_string(&output).unwrap();
    assert!(written.contains("POINTS 5"));
    let _ = std::fs::remove_file(&output);
}

#[test]
fn pipeline_json_accepts_root_pipeline_object() {
    let json = r#"{
            "pipeline": [
                {"type":"readers.faux", "count":4},
                {"type":"filters.decimation", "step":2}
            ]
        }"#;
    let mut pipeline = pipeline_from_json(json).unwrap();
    let result = pipeline.execute_with_result(Vec::new()).unwrap();
    assert_eq!(result.point_count, 2);
    assert_eq!(result.view_count, 1);
}

#[test]
fn pipeline_json_accepts_comments_in_runnable_pipeline() {
    let json = r#"{
            // accepted by C++ PipelineReaderJSON
            "pipeline": [
                {"type":"readers.faux", "count":6, "mode":"ramp"},
                {"type":"filters.decimation", "step":2},
                {"type":"filters.assign", "value":"Z = 42"}
            ]
        }"#;

    let mut pipeline = pipeline_from_json(json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 3);
    assert_eq!(views[0].get_f64(0, &DimId::Z), 42.0);
}

#[test]
fn pipeline_json_accepts_null_stage_options_as_empty_strings() {
    let object = serde_json::json!({
        "type": "filters.head",
        "tag": "head",
        "where": null
    });
    let options = options_from_object(object.as_object().unwrap()).unwrap();

    assert_eq!(options.value("where"), Some(""));
}

#[test]
fn pipeline_json_accepts_filename_string_stages() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let output =
        std::env::temp_dir().join(format!("pdal-rust-string-stage-{}.pcd", std::process::id()));
    let _ = std::fs::remove_file(&output);

    let json = format!(
        r#"[
                "{}",
                {{"type":"filters.decimation", "step":2}},
                "{}"
            ]"#,
        escape_json_path(&input),
        escape_json_path(&output)
    );

    let mut pipeline = pipeline_from_json(&json).unwrap();
    let result = pipeline.execute_with_result(Vec::new()).unwrap();
    assert_eq!(result.point_count, 5);
    assert_eq!(result.view_count, 1);
    assert!(output.exists());
    let _ = std::fs::remove_file(&output);
}

#[test]
fn pipeline_json_accepts_multiple_filename_string_readers() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");

    let json = format!(
        r#"[
                "{}",
                "{}",
                {{"type":"writers.null"}}
            ]"#,
        escape_json_path(&input),
        escape_json_path(&input)
    );

    let mut pipeline = pipeline_from_json(&json).unwrap();
    let result = pipeline.execute_with_result(Vec::new()).unwrap();
    assert_eq!(result.point_count, 20);
    assert_eq!(result.view_count, 2);
}

#[test]
fn pipeline_json_executes_filespec_string_filename() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/las/epsg_4326.las");
    let filespec = serde_json::json!({
        "path": input,
        "headers": {"header_key": "header_value"}
    })
    .to_string();

    let json = format!(
        r#"[
                {{"type":"readers.las", "filename":{}}},
                {{"type":"writers.null"}}
            ]"#,
        serde_json::to_string(&filespec).unwrap()
    );

    let mut pipeline = pipeline_from_json(&json).unwrap();
    let result = pipeline.execute_with_result(Vec::new()).unwrap();
    assert_eq!(result.point_count, 5380);
}

#[test]
fn pipeline_json_rejects_las_writer_with_mixed_input_srs() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let simple = repo.join("test/data/las/simple.las");
    let autzen = repo.join("test/data/las/autzen_trim.las");
    let output =
        std::env::temp_dir().join(format!("pdal-rust-mixed-srs-{}.las", std::process::id()));
    let _ = std::fs::remove_file(&output);

    let json = format!(
        r#"[
                "{}",
                "{}",
                "{}"
            ]"#,
        escape_json_path(&simple),
        escape_json_path(&autzen),
        escape_json_path(&output)
    );

    let mut pipeline = pipeline_from_json(&json).unwrap();
    let err = match pipeline.execute_with_result(Vec::new()) {
        Ok(_) => panic!("mixed-SRS LAS pipeline unexpectedly succeeded"),
        Err(err) => err,
    };

    assert!(err
        .to_string()
        .contains("multiple point spatial references"));
    let _ = std::fs::remove_file(&output);
}

#[test]
fn later_reader_stage_does_not_implicitly_depend_on_previous_stage() {
    let pipeline = pipeline_from_json(
        r#"[
            {"type":"writers.null", "tag":"W"},
            {"type":"readers.faux", "tag":"R", "count":1}
        ]"#,
    )
    .unwrap();

    let writer = pipeline.find_by_tag("W").unwrap();
    let reader = pipeline.find_by_tag("R").unwrap();
    assert_eq!(pipeline.input_count(writer).unwrap(), 0);
    assert_eq!(pipeline.input_count(reader).unwrap(), 0);
    assert!(!pipeline.roots_are_readers());
}

#[test]
fn pipeline_json_runs_sort_filter() {
    let json = r#"[
            {"type":"readers.faux", "count":4, "mode":"ramp", "minx":1, "maxx":4},
            {"type":"filters.sort", "dimensions":"X", "order":"desc"}
        ]"#;
    let mut pipeline = pipeline_from_json(json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::X), 4.0);
    assert_eq!(views[0].get_f64(3, &DimId::X), 1.0);
}

#[test]
fn pipeline_json_runs_groupby_filter() {
    let json = r#"[
            {"type":"readers.faux", "count":2, "mode":"ramp", "minx":1, "maxx":2},
            {"type":"filters.groupby", "dimension":"X"}
        ]"#;
    let mut pipeline = pipeline_from_json(json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 2);
    assert_eq!(views[0].len(), 1);
    assert_eq!(views[1].len(), 1);
}

#[test]
fn pipeline_json_preserves_repeated_expression_options() {
    let json = r#"[
            {"type":"readers.faux", "count":5, "mode":"ramp", "minx":0, "maxx":4},
            {"type":"filters.expression", "expression":["X < 2", "X > 3"]}
        ]"#;
    let mut pipeline = pipeline_from_json(json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 2);
    assert_eq!(views[0].len(), 2);
    assert_eq!(views[1].len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::X), 0.0);
    assert_eq!(views[1].get_f64(0, &DimId::X), 4.0);
}

#[test]
fn pipeline_json_runs_newly_registry_visible_filter_families() {
    let cases = [
        (
            "filters.nndistance",
            r#"{"type":"filters.nndistance", "knn":1, "mode":"kth"}"#,
            DimId::NNDistance,
        ),
        (
            "filters.radialdensity",
            r#"{"type":"filters.radialdensity", "radius":2.0}"#,
            DimId::RadialDensity,
        ),
        (
            "filters.eigenvalues",
            r#"{"type":"filters.eigenvalues", "knn":4}"#,
            DimId::Eigenvalue0,
        ),
        (
            "filters.cluster",
            r#"{"type":"filters.cluster", "tolerance":10.0, "min_points":1}"#,
            DimId::ClusterID,
        ),
        (
            "filters.zsmooth",
            r#"{"type":"filters.zsmooth", "radius":10.0, "dimension":"Zsmoothed"}"#,
            DimId::from_name("Zsmoothed"),
        ),
    ];

    for (name, filter_json, dim) in cases {
        let json = format!(
            r#"[
                    {{"type":"readers.faux", "count":5, "mode":"ramp", "minx":0, "maxx":4, "miny":0, "maxy":4, "minz":0, "maxz":4}},
                    {filter_json}
                ]"#
        );
        let mut pipeline = pipeline_from_json(&json).unwrap();
        let views = pipeline.execute(Vec::new()).unwrap();
        assert_eq!(views.len(), 1, "{name} should produce one view");
        assert_eq!(views[0].len(), 5, "{name} should preserve point count");
        assert!(
            views[0].layout().dim(&dim).is_some(),
            "{name} should prepare its output dimension"
        );
    }
}

#[test]
fn registry_divider_expression_mode_splits_on_condition() {
    let json = r#"[
        {"type":"readers.faux", "count":5, "mode":"ramp", "minx":0, "maxx":4, "miny":0, "maxy":0, "minz":0, "maxz":0},
        {"type":"filters.divider", "mode":"expression", "expression":"X >= 2 && X < 4"}
    ]"#;
    let mut pipeline = pipeline_from_json(json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();

    assert_eq!(views.len(), 3);
    assert_eq!(views[0].len(), 2);
    assert_eq!(views[0].get_f64(0, &DimId::X), 0.0);
    assert_eq!(views[0].get_f64(1, &DimId::X), 1.0);
    assert_eq!(views[1].len(), 1);
    assert_eq!(views[1].get_f64(0, &DimId::X), 2.0);
    assert_eq!(views[2].len(), 2);
    assert_eq!(views[2].get_f64(0, &DimId::X), 3.0);
    assert_eq!(views[2].get_f64(1, &DimId::X), 4.0);
}

#[test]
fn registry_dem_filter_keeps_points_within_raster_limits() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raster = repo.join("test/data/gdal/float32.tif");

    let mut options = Options::new();
    options.add("raster", raster.display());
    options.add("limits", "Z[0:100]");
    let mut filter = create_filter("filters.dem", &options).unwrap();

    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    for z in [200.0, 208.0] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, 440750.0);
        view.set_f64(idx, &DimId::Y, 3751290.0);
        view.set_f64(idx, &DimId::Z, z);
    }

    let views = filter.run(&[view]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 1, "only the in-range point should remain");
    assert_eq!(views[0].get_f64(0, &DimId::Z), 200.0);
}

#[test]
fn registry_hag_dem_filter_computes_height_above_raster() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raster = repo.join("test/data/gdal/float32.tif");

    let mut options = Options::new();
    options.add("raster", raster.display());
    let mut filter = create_filter("filters.hag_dem", &options).unwrap();

    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for class in [2.0, 1.0] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, 440750.0);
        view.set_f64(idx, &DimId::Y, 3751290.0);
        view.set_f64(idx, &DimId::Z, 200.0);
        view.set_f64(idx, &DimId::Classification, class);
    }

    let output_dims = filter.output_dimensions();
    assert!(
        output_dims.contains(&(DimId::HeightAboveGround, DimType::F64)),
        "registry wrapper should declare the HeightAboveGround output dimension"
    );
    let view = view.with_dimensions(&output_dims);

    let views = filter.run(&[view]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 2);
    assert_eq!(views[0].get_f64(0, &DimId::HeightAboveGround), 0.0);
    assert_eq!(views[0].get_f64(1, &DimId::HeightAboveGround), 93.0);
}

#[test]
fn registry_h3_filter_assigns_lossless_index() {
    use pdal_core::srs::SpatialReference;

    let mut options = Options::new();
    options.add("resolution", 12u64);
    let mut filter = create_filter("filters.h3", &options).unwrap();

    let output_dims = filter.output_dimensions();
    assert!(
        output_dims.contains(&(DimId::H3, DimType::U64)),
        "registry wrapper should declare the uint64 H3 output dimension"
    );

    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    view.set_spatial_reference(SpatialReference::new("EPSG:4326"));
    let idx = view.add_point();
    view.set_f64(idx, &DimId::X, -122.0);
    view.set_f64(idx, &DimId::Y, 47.0);
    view.set_f64(idx, &DimId::Z, 0.0);
    let view = view.with_dimensions(&output_dims);

    let views = filter.run(&[view]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 1);
    let h3 = views[0].get_u64(0, &DimId::H3);
    assert!(h3 > (1u64 << 53), "H3 index should be a full 64-bit value");
    assert_ne!(
        h3,
        (h3 as f64) as u64,
        "the typed path must preserve low bits an f64 would drop"
    );
}

#[test]
fn registry_h3_filter_requires_resolution() {
    let options = Options::new();
    match create_filter("filters.h3", &options) {
        Err(err) => assert!(err.0.contains("resolution"), "got: {}", err.0),
        Ok(_) => panic!("filters.h3 should require a resolution option"),
    }
}

#[test]
fn registry_colorinterp_filter_uses_named_ramp_and_auto_bounds() {
    let mut options = Options::new();
    options.add("ramp", "pestel_shades");
    let mut filter = create_filter("filters.colorinterp", &options).unwrap();

    let output_dims = filter.output_dimensions();
    for dim in [DimId::Red, DimId::Green, DimId::Blue] {
        assert!(
            output_dims.contains(&(dim.clone(), DimType::U16)),
            "colorinterp should declare {dim:?} as uint16"
        );
    }

    let mut layout = PointLayout::new();
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    for z in 0..10 {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::Z, z as f64);
    }
    let view = view.with_dimensions(&output_dims);

    let views = filter.run(&[view]).unwrap();
    assert_eq!(views.len(), 1);
    let out = &views[0];
    assert_eq!(out.len(), 10);
    for i in 0..out.len() {
        for dim in [DimId::Red, DimId::Green, DimId::Blue] {
            let c = out.get_u64(i, &dim);
            assert!(c <= 255, "channel {dim:?} out of byte range: {c}");
        }
    }
    let first = out.get_u64(0, &DimId::Red);
    let last = out.get_u64(9, &DimId::Red);
    assert_ne!(
        first, last,
        "ramp ends should differ across the value range"
    );
}

#[test]
fn pipeline_json_runs_colorization_against_raster() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let las = repo.join("test/data/autzen/autzen-point-format-3.las");
    let raster = repo.join("test/data/autzen/autzen.jpg");

    let json = format!(
        r#"[
                {{"filename":"{}"}},
                {{"type":"filters.colorization", "raster":"{}", "dimensions":"Red, Green,Blue::255  "}}
            ]"#,
        escape_json_path(&las),
        escape_json_path(&raster)
    );

    let mut pipeline = pipeline_from_json(&json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Red), 210.0);
    assert_eq!(views[0].get_f64(0, &DimId::Green), 205.0);
    assert_eq!(views[0].get_f64(0, &DimId::Blue), 47175.0);
}

#[test]
fn pipeline_json_runs_overlay_from_shapefile() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let las = repo.join("test/data/autzen/autzen-dd.las");
    let shp = repo.join("test/data/autzen/attributes.shp");

    let mut ro = Options::new();
    ro.add("filename", las.display());
    let mut reader = create_reader("readers.las", &ro).unwrap();
    let base = reader.read().unwrap();
    let total = base[0].len();
    assert!(total > 0);
    let base_cls: Vec<f64> = (0..total)
        .map(|idx| base[0].get_f64(idx, &DimId::Classification))
        .collect();

    let json = format!(
        r#"[
                {{"filename":"{}"}},
                {{"type":"filters.overlay", "dimension":"Classification", "datasource":"{}", "column":"cls"}}
            ]"#,
        escape_json_path(&las),
        escape_json_path(&shp)
    );

    let mut pipeline = pipeline_from_json(&json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), total, "overlay should preserve point count");

    let mut changed = 0u64;
    for idx in 0..views[0].len() {
        let cls = views[0].get_f64(idx, &DimId::Classification);
        if cls != base_cls[idx as usize] {
            changed += 1;
            assert!(
                matches!(cls as u32, 2 | 5 | 6),
                "reassigned class {cls} not in shapefile cls set"
            );
        }
    }
    assert!(changed > 0, "overlay should reclassify some points");
}

#[test]
fn sort_rejects_unknown_order() {
    let mut options = Options::new();
    options.add("dimensions", "X").add("order", "sideways");
    let err = match create_filter("filters.sort", &options) {
        Ok(_) => panic!("expected invalid sort order to fail"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("order must be 'asc' or 'desc'"));
}

#[test]
fn nndistance_rejects_unknown_mode() {
    let mut options = Options::new();
    options.add("mode", "median");
    let err = match create_filter("filters.nndistance", &options) {
        Ok(_) => panic!("expected invalid nndistance mode to fail"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("mode must be 'kth' or 'avg'"));
}

#[test]
fn pipeline_json_rejects_invalid_typed_options() {
    let err = match pipeline_from_json(
        r#"[
                {"type":"readers.faux", "count":null},
                {"type":"writers.null"}
            ]"#,
    ) {
        Ok(_) => panic!("expected null count option to fail"),
        Err(err) => err,
    };
    assert!(err
        .to_string()
        .contains("Option 'count' must be an unsigned integer"));

    let err = match pipeline_from_json(
        r#"[
                {"type":"readers.faux", "count":4},
                {"type":"filters.head", "count":"many"}
            ]"#,
    ) {
        Ok(_) => panic!("expected invalid count option to fail"),
        Err(err) => err,
    };
    assert!(err
        .to_string()
        .contains("Option 'count' must be an unsigned integer"));

    let err = match pipeline_from_json(
        r#"[
                {"type":"readers.faux", "count":4},
                {"type":"filters.tail", "invert":"sometimes"}
            ]"#,
    ) {
        Ok(_) => panic!("expected invalid bool option to fail"),
        Err(err) => err,
    };
    assert!(err
        .to_string()
        .contains("Option 'invert' must be a boolean value"));

    let err = match pipeline_from_json(
        r#"[
                {"type":"readers.faux", "count":4},
                {"type":"filters.radialdensity", "radius":"wide"}
            ]"#,
    ) {
        Ok(_) => panic!("expected invalid radius option to fail"),
        Err(err) => err,
    };
    assert!(err
        .to_string()
        .contains("Option 'radius' must be a floating-point value"));
}

#[test]
fn pipeline_json_rejects_unknown_assign_option() {
    let err = match pipeline_from_json(
        r#"[
                {"type":"readers.faux", "count":4},
                {"type":"filters.assign", "assignment":"Classification[:]=2", "ignore":true},
                {"type":"writers.null"}
            ]"#,
    ) {
        Ok(_) => panic!("expected unknown assign option to fail"),
        Err(err) => err,
    };

    assert!(err
        .to_string()
        .contains("filters.assign: Unexpected argument 'ignore'"));
}

#[test]
fn pipeline_execution_rejects_assignment_to_missing_dimension() {
    let mut pipeline = pipeline_from_json(
        r#"[
                {"type":"readers.faux", "count":4},
                {"type":"filters.assign", "assignment":"Classification[:]=2"},
                {"type":"writers.null"}
            ]"#,
    )
    .unwrap();

    let err = match pipeline.execute_with_result(Vec::new()) {
        Ok(_) => panic!("expected missing assignment dimension to fail"),
        Err(err) => err,
    };

    assert!(err
        .to_string()
        .contains("Invalid dimension name in 'assignment' option: 'Classification'"));
}

#[test]
fn pipeline_json_rejects_root_object_without_pipeline_array() {
    let err = match pipeline_from_json(r#"{"type":"readers.faux"}"#) {
        Ok(_) => panic!("expected root object without pipeline array to fail"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("root element is not a pipeline"));
}

#[test]
fn pipeline_json_rejects_invalid_stage_metadata() {
    let cases = [
        (
            r#"[{"type":7,"filename":"in.las"}]"#,
            "'type' must be specified as a string",
        ),
        (
            r#"[{"type":"readers.faux","tag":7}]"#,
            "tag must be specified as a string",
        ),
        (
            r#"[{"type":"readers.faux","tag":"1bad"}]"#,
            "Invalid tag name '1bad'",
        ),
        (
            r#"[{"type":"readers.faux","tag":"A"},{"type":"readers.faux","tag":"A"}]"#,
            "duplicate tag 'A'",
        ),
        (
            r#"[{"type":"readers.faux","tag":"A"},{"type":"readers.faux","inputs":["A"]}]"#,
            "Inputs not permitted for reader",
        ),
    ];

    for (json, message) in cases {
        let err = match pipeline_from_json(json) {
            Ok(_) => panic!("{json} should fail"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains(message),
            "{message:?} not found in {err}"
        );
    }
}

#[test]
fn pipeline_json_uses_tagged_inputs() {
    let json = r#"[
            {"type":"readers.faux", "count":10, "tag":"A"},
            {"type":"readers.faux", "count":5, "tag":"B"},
            {"type":"filters.merge", "inputs":["A", "B"]}
        ]"#;
    let mut pipeline = pipeline_from_json(json).unwrap();
    let result = pipeline.execute_with_result(Vec::new()).unwrap();
    assert_eq!(result.point_count, 15);
}

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
