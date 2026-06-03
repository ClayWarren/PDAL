use super::*;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_io::las::LasReader;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

type ExpectedDims = Vec<(DimId, DimType)>;
type RegistryCase = (&'static str, Options, ExpectedDims);

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
fn registry_filters_declare_output_dimensions() {
    let mut assign_options = Options::new();
    assign_options.add("value", "NewDim = Z + 1");

    let mut covariance_options = Options::new();
    covariance_options.add("feature_set", "all");

    let mut colorization_options = Options::new();
    colorization_options.add("raster", "dummy.tif");
    colorization_options.add("dimensions", "Red:1:1.0,Green:2:1.0,Blue:3:1.0");

    let mut geom_options = Options::new();
    geom_options.add("geometry", "POINT (0 0)");
    geom_options.add("dimension", "DistanceToOrigin");

    let cases: Vec<RegistryCase> = vec![
        (
            "filters.approximatecoplanar",
            Options::new(),
            vec![(DimId::Coplanar, DimType::F64)],
        ),
        (
            "filters.assign",
            assign_options,
            vec![(DimId::from_name("NewDim"), DimType::F64)],
        ),
        (
            "filters.cluster",
            Options::new(),
            vec![(DimId::ClusterID, DimType::F64)],
        ),
        (
            "filters.colorinterp",
            Options::new(),
            vec![
                (DimId::Red, DimType::U16),
                (DimId::Green, DimType::U16),
                (DimId::Blue, DimType::U16),
            ],
        ),
        (
            "filters.colorization",
            colorization_options,
            vec![
                (DimId::Red, DimType::F64),
                (DimId::Green, DimType::F64),
                (DimId::Blue, DimType::F64),
            ],
        ),
        (
            "filters.covariancefeatures",
            covariance_options,
            vec![
                (DimId::from_name("Linearity"), DimType::F64),
                (DimId::from_name("Density"), DimType::F64),
            ],
        ),
        (
            "filters.csf",
            Options::new(),
            vec![(DimId::Classification, DimType::U8)],
        ),
        (
            "filters.dbscan",
            Options::new(),
            vec![(DimId::ClusterID, DimType::F64)],
        ),
        (
            "filters.eigenvalues",
            Options::new(),
            vec![
                (DimId::Eigenvalue0, DimType::F64),
                (DimId::Eigenvalue1, DimType::F64),
                (DimId::Eigenvalue2, DimType::F64),
            ],
        ),
        (
            "filters.elm",
            Options::new(),
            vec![(DimId::Classification, DimType::F64)],
        ),
        (
            "filters.geomdistance",
            geom_options,
            vec![(DimId::from_name("DistanceToOrigin"), DimType::F64)],
        ),
        (
            "filters.h3",
            default_filter_options("filters.h3"),
            vec![(DimId::H3, DimType::U64)],
        ),
        (
            "filters.hag_delaunay",
            Options::new(),
            vec![(DimId::HeightAboveGround, DimType::F64)],
        ),
        (
            "filters.hag_dem",
            default_filter_options("filters.hag_dem"),
            vec![(DimId::HeightAboveGround, DimType::F64)],
        ),
        (
            "filters.hag_nn",
            Options::new(),
            vec![(DimId::HeightAboveGround, DimType::F64)],
        ),
        (
            "filters.label_duplicates",
            Options::new(),
            vec![(DimId::from_name("Duplicate"), DimType::F64)],
        ),
        (
            "filters.litree",
            Options::new(),
            vec![(DimId::ClusterID, DimType::F64)],
        ),
        (
            "filters.lloydkmeans",
            Options::new(),
            vec![(DimId::ClusterID, DimType::F64)],
        ),
        (
            "filters.lof",
            Options::new(),
            vec![
                (DimId::NNDistance, DimType::F64),
                (DimId::LocalReachabilityDistance, DimType::F64),
                (DimId::LocalOutlierFactor, DimType::F64),
            ],
        ),
        (
            "filters.m3c2",
            Options::new(),
            vec![
                (DimId::from_name("m3c2_distance"), DimType::F64),
                (DimId::from_name("m3c2_significant"), DimType::U8),
            ],
        ),
        (
            "filters.miniball",
            Options::new(),
            vec![(DimId::from_name("Miniball"), DimType::F64)],
        ),
        (
            "filters.nndistance",
            Options::new(),
            vec![(DimId::NNDistance, DimType::F64)],
        ),
        (
            "filters.normal",
            Options::new(),
            vec![
                (DimId::NormalX, DimType::F64),
                (DimId::NormalY, DimType::F64),
                (DimId::NormalZ, DimType::F64),
                (DimId::from_name("Curvature"), DimType::F64),
            ],
        ),
        (
            "filters.optimalneighborhood",
            Options::new(),
            vec![
                (DimId::OptimalKNN, DimType::F64),
                (DimId::OptimalRadius, DimType::F64),
            ],
        ),
        (
            "filters.outlier",
            Options::new(),
            vec![(DimId::Classification, DimType::F64)],
        ),
        (
            "filters.planefit",
            Options::new(),
            vec![(DimId::PlaneFit, DimType::F64)],
        ),
        (
            "filters.pmf",
            Options::new(),
            vec![(DimId::Classification, DimType::U8)],
        ),
        (
            "filters.radialdensity",
            Options::new(),
            vec![(DimId::RadialDensity, DimType::F64)],
        ),
        (
            "filters.reciprocity",
            Options::new(),
            vec![(DimId::Reciprocity, DimType::F64)],
        ),
        (
            "filters.smrf",
            Options::new(),
            vec![(DimId::Classification, DimType::U8)],
        ),
        (
            "filters.supervoxel",
            Options::new(),
            vec![(DimId::ClusterID, DimType::F64)],
        ),
        (
            "filters.zsmooth",
            default_filter_options("filters.zsmooth"),
            vec![(DimId::Z, DimType::F64)],
        ),
    ];

    for (name, options, expected) in cases {
        let filter = create_filter(name, &options).unwrap_or_else(|err| {
            panic!("{name} should construct before checking output dimensions: {err}")
        });
        let declared = filter.output_dimensions();
        for dim in expected {
            assert!(
                declared.contains(&dim),
                "{name} should declare output dimension {dim:?}; declared {declared:?}"
            );
        }
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
fn inferred_but_unported_reader_drivers_fail_cleanly() {
    let options = Options::new();
    for (filename, driver) in [
        ("scene.slpk", "readers.slpk"),
        ("service.i3s", "readers.i3s"),
        ("scan.e57", "readers.e57"),
    ] {
        assert_eq!(infer_reader_driver(filename), Some(driver));
        let err = match create_reader(driver, &options) {
            Ok(_) => panic!("{driver} should not construct yet"),
            Err(err) => err,
        };
        assert!(
            err.to_string()
                .contains("is not available in the Rust port"),
            "{driver} should report a Rust-port availability error, got {err}"
        );
    }
}

#[test]
fn inferred_but_unported_writer_drivers_fail_cleanly() {
    let options = Options::new();
    for (filename, driver) in [
        ("out.e57", "writers.e57"),
        ("out.drc", "writers.draco"),
        ("out.mat", "writers.matlab"),
        ("out.parquet", "writers.arrow"),
    ] {
        assert_eq!(infer_writer_driver(filename), Some(driver));
        let err = match create_writer(driver, &options) {
            Ok(_) => panic!("{driver} should not construct yet"),
            Err(err) => err,
        };
        assert!(
            err.to_string()
                .contains("is not available in the Rust port"),
            "{driver} should report a Rust-port availability error, got {err}"
        );
    }
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
    // Mirrors the C++ DEMFilterTest: keep points whose Z is within
    // [v, v + 100] of the float32.tif raster sample at their X/Y.
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
    // Mirrors the C++ HAGFilterTest.dem: a ground-classified point gets HAG 0,
    // an unclassified point at Z=200 over the float32.tif DEM (value 107) gets
    // HAG 93. Also confirms the output dimension is prepared by the registry.
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

    // The pipeline prepares output dimensions before running a filter; the
    // registry-built wrapper must declare HeightAboveGround for that to work.
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
    // `pdal pipeline` with filters.h3 must construct through the registry,
    // prepare the uint64 H3 dimension, and store the full 64-bit index. A
    // resolution-12 index uses low bits an f64 cannot hold, so this also
    // guards the typed (non-f64) storage path.
    use pdal_core::srs::SpatialReference;

    let mut options = Options::new();
    options.add("resolution", 12u64);
    let mut filter = create_filter("filters.h3", &options).unwrap();

    // The pipeline prepares output dimensions before running a filter; the
    // registry-built wrapper must declare H3 as U64 for set_u64 to land.
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
    // `pdal pipeline` with filters.colorinterp and no min/max must construct
    // through the registry, resolve the default `pestel_shades` named ramp
    // (no file), prepare Red/Green/Blue as uint16, and assign colors.
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
    // Auto bounds span Z [0,9]; the ramp ends differ, so the lowest and
    // highest points get different colors, and every channel stays in 0..=255.
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
fn registry_assign_filter_supports_value_expressions() {
    let mut options = Options::new();
    options.add("value", "Y = X * 2");
    options.add("value", "Classification = Y WHERE X >= 5");
    let mut filter = create_filter("filters.assign", &options).unwrap();

    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for x in [1.0, 5.0, 10.0] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Classification, 1.0);
    }

    let output_dims = filter.output_dimensions();
    assert!(output_dims.contains(&(DimId::Y, DimType::F64)));
    let views = filter.run(&[view.with_dimensions(&output_dims)]).unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Y), 2.0);
    assert_eq!(views[0].get_f64(1, &DimId::Y), 10.0);
    assert_eq!(views[0].get_f64(2, &DimId::Y), 20.0);
    assert_eq!(views[0].get_f64(0, &DimId::Classification), 1.0);
    assert_eq!(views[0].get_f64(1, &DimId::Classification), 10.0);
    assert_eq!(views[0].get_f64(2, &DimId::Classification), 20.0);
}

#[test]
fn registry_radiusassign_filter_supports_value_expressions() {
    let mut options = Options::new();
    options.add("radius", 1.0);
    options.add("is3d", true);
    options.add("reference_domain", "Classification[1:1]");
    options.add("update_expression", "Classification = Z + 3 WHERE X < 1");
    let mut filter = create_filter("filters.radiusassign", &options).unwrap();

    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z, class) in [
        (0.0, 0.0, 0.0, 1.0),
        (0.5, 0.0, 0.0, 0.0),
        (0.0, 0.5, -2.0, 0.0),
        (10.0, 0.0, 0.0, 0.0),
    ] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Y, y);
        view.set_f64(idx, &DimId::Z, z);
        view.set_f64(idx, &DimId::Classification, class);
    }

    let views = filter.run(&[view]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Classification), 3.0);
    assert_eq!(views[0].get_f64(1, &DimId::Classification), 3.0);
    assert_eq!(views[0].get_f64(2, &DimId::Classification), 0.0);
    assert_eq!(views[0].get_f64(3, &DimId::Classification), 0.0);
}

#[test]
fn pipeline_json_runs_colorization_against_raster() {
    // Mirrors the C++ ColorizationFilterTest.test1: colorize autzen points from
    // autzen.jpg with "Red, Green,Blue::255" and check point 0 == 210/205/47175
    // (Blue raw 185 scaled by 255).
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
    // filters.overlay assigns the shapefile 'cls' attribute (values 2/5/6) to
    // Classification for points inside each polygon. Assert the overlay
    // actually reclassifies points by comparing the count of points with a
    // shapefile class before and after the filter.
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let las = repo.join("test/data/autzen/autzen-dd.las");
    let shp = repo.join("test/data/autzen/attributes.shp");

    // Baseline classifications straight from the reader.
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

    // At least one point must be reclassified by a polygon, and every changed
    // value must come from the shapefile 'cls' set {2, 5, 6}.
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
        "filters.expressionstats" => {
            options.add("dimension", "Classification");
            options.add("expressions", "Z > 0");
        }
        "filters.ferry" => {
            options.add("dimensions", "X=>X2");
        }
        "filters.mongo" => {
            options.add("expression", "{\"Z\":{\"$gt\":0}}");
        }
        "filters.neighborclassifier" => {
            options.add("k", 8u64);
        }
        "filters.radiusassign" => {
            options.add("radius", 1.0);
            options.add("update_expression", "Classification = 2");
        }
        "filters.geomdistance" => {
            options.add("geometry", "POLYGON((0 0, 0 1, 1 1, 1 0, 0 0))");
        }
        "filters.dem" => {
            options.add("raster", "dummy.tif");
            options.add("limits", "Z[0:100]");
        }
        "filters.hag_dem" => {
            options.add("raster", "dummy.tif");
        }
        "filters.colorization" => {
            options.add("raster", "dummy.tif");
        }
        "filters.overlay" => {
            options.add("dimension", "Classification");
            options.add("datasource", "dummy.shp");
        }
        "filters.straighten" => {
            options.add("polyline", "LINESTRING ZM (0 0 0 0, 10 0 0 0)");
        }
        "filters.h3" => {
            options.add("resolution", 12u64);
        }
        _ => {}
    }
    options
}
