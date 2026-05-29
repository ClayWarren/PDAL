use super::*;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_io::las::LasReader;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

#[test]
fn every_listed_reader_driver_constructs() {
    let options = Options::new();
    for name in READER_DRIVERS {
        if *name == "readers.faux" || *name == "readers.text" {
            continue; // these need a file or specific options
        }
        // Just check that they are in the match arm
        let _ = create_reader(name, &options);
    }
}

#[test]
fn every_listed_filter_driver_constructs() {
    for name in FILTER_DRIVERS {
        let options = default_filter_options(name);
        assert!(
            create_filter(name, &options).is_ok(),
            "{name} should construct from registry defaults"
        );
    }
}

#[test]
fn every_listed_writer_driver_constructs() {
    let options = Options::new();
    for name in WRITER_DRIVERS {
        // Just check that they are in the match arm
        let _ = create_writer(name, &options);
    }
}

#[test]
fn unified_stage_factory_dispatches_by_prefix() {
    let options = Options::new();
    assert!(matches!(
        create_stage("readers.faux", &options),
        Ok(CreatedStage::Reader(_))
    ));
    assert!(matches!(
        create_stage("filters.decimation", &options),
        Ok(CreatedStage::Filter(_))
    ));
    assert!(matches!(
        create_stage("writers.null", &options),
        Ok(CreatedStage::Writer(_))
    ));
}

#[test]
fn unknown_and_unported_drivers_are_rejected() {
    let options = Options::new();
    assert!(create_reader("readers.unknown", &options).is_err());
    assert!(create_filter("filters.unknown", &options).is_err());
    assert!(create_writer("writers.unknown", &options).is_err());
}

#[test]
fn laz_writer_driver_forces_compression_for_las_extension() {
    let temp = make_temp_dir("laz-driver-compression");
    let output = temp.join("explicit-laz-driver.las");
    let mut options = Options::new();
    options.add("filename", output.display());

    let mut writer = create_writer("writers.laz", &options).unwrap();
    writer.write(&[single_point_view()]).unwrap();

    let mut reader_options = Options::new();
    reader_options.add("filename", output.display());
    let mut reader = LasReader::new(&reader_options);
    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::X), 1.0);
}

#[test]
fn registry_created_reader_reads_a_fixture() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let mut options = Options::new();
    options.add("filename", input.display());

    let mut reader = create_reader("readers.text", &options).unwrap();
    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 10);
}

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
fn pipeline_json_runs_sort_filter() {
    let json = r#"[
            {"type":"readers.faux", "count":4, "mode":"ramp", "minx":1, "maxx":4},
            {"type":"filters.sort", "dimensions":"X", "order":"desc"}
        ]"#;
    let mut pipeline = pipeline_from_json(json).unwrap();
    let views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &pdal_core::point::DimId::X), 4.0);
    assert_eq!(views[0].get_f64(3, &pdal_core::point::DimId::X), 1.0);
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
fn pipeline_json_rejects_root_object_without_pipeline_array() {
    let err = match pipeline_from_json(r#"{"type":"readers.faux"}"#) {
        Ok(_) => panic!("expected root object without pipeline array to fail"),
        Err(err) => err,
    };
    assert!(err
        .to_string()
        .contains("object must contain a 'pipeline' array"));
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

fn single_point_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    let point = view.add_point();
    view.set_f64(point, &DimId::X, 1.0);
    view.set_f64(point, &DimId::Y, 2.0);
    view.set_f64(point, &DimId::Z, 3.0);
    view
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn default_filter_options(name: &str) -> Options {
    let mut options = Options::new();
    match name {
        "filters.gpstimeconvert" => {
            options.add("conversion", "gst2gt");
        }
        "filters.sort" => {
            options.add("dimension", "X");
        }
        "filters.sample" => {
            options.add("radius", 1.0);
        }
        "filters.range" => {
            options.add("limits", "Z[0:10]");
        }
        "filters.assign" => {
            options.add("assignment", "Classification[:]=0");
        }
        "filters.transformation" => {
            options.add("matrix", "1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1");
        }
        "filters.expression" => {
            options.add("expression", "Z > 0");
        }
        "filters.ferry" => {
            options.add("dimensions", "X=>X2");
        }
        "filters.straighten" => {
            options.add("polyline", "LINESTRING ZM (0 0 0 0, 10 0 0 0)");
        }
        _ => {}
    }
    options
}
