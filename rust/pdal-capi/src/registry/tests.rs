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
