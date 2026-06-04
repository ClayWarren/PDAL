use super::*;

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-kernels-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn create_accepts_positionals_and_defaults() {
    let parsed = parse_tindex_create_args(&strings(&["out.geojson", "in.las"])).unwrap();
    assert_eq!(parsed.tindex_file, "out.geojson");
    assert_eq!(parsed.files, vec!["in.las"]);
    assert_eq!(parsed.driver_name, "ESRI Shapefile");
    assert_eq!(parsed.target_srs, "EPSG:4326");
    assert_eq!(parsed.layer_name, "pdal");
    assert_eq!(parsed.location_field, "location");
}

#[test]
fn create_tracks_boundary_and_srs_options() {
    let parsed = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filespec=in.las",
        "--threshold=20",
        "--resolution=5.5",
        "--sample_size=100",
        "--simplify=false",
        "--fast_boundary=true",
        "--where=Classification == 2",
        "--a_srs=EPSG:3857",
        "--skip_different_srs=yes",
    ]))
    .unwrap();
    assert_eq!(parsed.files, vec!["in.las"]);
    assert!(parsed.rich_boundary_options);
    assert_eq!(parsed.boundary.density, 20);
    assert_eq!(parsed.boundary.edge_length, 5.5);
    assert_eq!(parsed.boundary.sample_size, 100);
    assert!(!parsed.boundary.smooth);
    assert!(!parsed.boundary.exact());
    assert_eq!(
        parsed.boundary.where_expr.as_deref(),
        Some("Classification == 2")
    );
    assert!(parsed.override_source_srs);
    assert_eq!(parsed.assign_srs, "EPSG:3857");
    assert!(parsed.skip_different_srs);
}

#[test]
fn create_accepts_cpp_synonym_switch_forms() {
    let dir = scratch_dir("tindex-synonyms");
    let input = dir.join("in.las");
    std::fs::write(&input, b"placeholder").unwrap();
    let pattern = dir.join("*.las").to_string_lossy().into_owned();

    let parsed = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filespec",
        &pattern,
        "--smooth",
        "false",
        "--skip_different_srs",
    ]))
    .unwrap();

    assert_eq!(parsed.files, vec![input.to_string_lossy().into_owned()]);
    assert!(parsed.rich_boundary_options);
    assert!(!parsed.boundary.smooth);
    assert!(parsed.skip_different_srs);
}

#[test]
fn create_rejects_multiple_input_methods() {
    let Err(err) = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filespec=a.las",
        "--filelist=list.txt",
    ])) else {
        panic!("expected multiple input methods to fail");
    };
    assert_eq!(
        err,
        TindexParseResult::Error(
            "Can't specify more than one source of tindex input files.".to_string()
        )
    );
}

#[test]
fn create_accepts_tindex_equals_form() {
    // `--tindex=PATH` must be equivalent to `--tindex PATH` (TIndexTest test4/7/8).
    let parsed =
        parse_tindex_create_args(&strings(&["--tindex=/vsistdout/", "--filespec=a.las"])).unwrap();
    assert_eq!(parsed.tindex_file, "/vsistdout/");
    assert_eq!(parsed.files, vec!["a.las".to_string()]);
}

#[test]
fn create_rejects_path_prefix_with_write_absolute_path() {
    // C++ TIndexKernel::validateSwitches rejects this combination (TIndexTest test2).
    let Err(err) = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--path_prefix=a",
        "--write_absolute_path=true",
        "--filespec=a.las",
    ])) else {
        panic!("expected path_prefix + write_absolute_path conflict to fail");
    };
    assert_eq!(
        err,
        TindexParseResult::Error(
            "Can't specify both --write_absolute_path and --path_prefix options.".to_string()
        )
    );
}

#[test]
fn create_accepts_filelist_and_named_options() {
    let temp = scratch_dir("tindex-filelist");
    let filelist = temp.join("files.txt");
    std::fs::write(&filelist, " a.las \n\nb.laz\n").unwrap();
    let parsed = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.gpkg",
        "--filelist",
        filelist.to_str().unwrap(),
        "--path_prefix",
        "/data",
        "--lyr_name",
        "tiles",
        "--tindex_name",
        "path",
        "-f",
        "GPKG",
        "--t_srs",
        "EPSG:3857",
        "--lco",
        "DESCRIPTION=sample index",
        "--threads",
        "4",
        "--requests",
        "2",
        "--log",
        "debug",
    ]))
    .unwrap();

    assert_eq!(parsed.files, vec!["a.las", "b.laz"]);
    assert_eq!(parsed.path_prefix.as_deref(), Some("/data"));
    assert_eq!(parsed.layer_name, "tiles");
    assert_eq!(parsed.location_field, "path");
    assert_eq!(parsed.driver_name, "GPKG");
    assert_eq!(parsed.target_srs, "EPSG:3857");
    assert_eq!(parsed.lco_options, vec!["DESCRIPTION=sample index"]);
    assert_eq!(parsed.lco_description.as_deref(), Some("sample index"));
}

#[test]
fn create_accepts_equals_options_and_glob_inputs() {
    let temp = scratch_dir("tindex-glob");
    let a = temp.join("a.las");
    let b = temp.join("b.las");
    std::fs::write(&a, "").unwrap();
    std::fs::write(&b, "").unwrap();
    let glob = format!("{}/*.las", temp.display());
    let parsed = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        &format!("--glob={glob}"),
        "--write_absolute_path=false",
        "--path_prefix=/prefix",
        "--a_srs=EPSG:26915",
        "--skip_different_srs=off",
        "--lco=DESCRIPTION=glob index",
        "--threshold",
        "7",
        "--edge_length",
        "2.5",
        "--sample_size",
        "42",
        "--simplify",
        "on",
        "--fast_boundary",
        "--where",
        "Classification == 2",
    ]))
    .unwrap();

    assert_eq!(parsed.files.len(), 2);
    assert!(!parsed.write_absolute_path);
    assert_eq!(parsed.path_prefix.as_deref(), Some("/prefix"));
    assert_eq!(parsed.assign_srs, "EPSG:26915");
    assert!(parsed.override_source_srs);
    assert!(!parsed.skip_different_srs);
    assert_eq!(parsed.lco_options, vec!["DESCRIPTION=glob index"]);
    assert_eq!(parsed.lco_description.as_deref(), Some("glob index"));
    assert_eq!(parsed.boundary.density, 7);
    assert_eq!(parsed.boundary.edge_length, 2.5);
    assert_eq!(parsed.boundary.sample_size, 42);
    assert!(parsed.boundary.smooth);
    assert!(parsed.boundary.fast_boundary);
    assert_eq!(
        parsed.boundary.where_expr.as_deref(),
        Some("Classification == 2")
    );
}

#[test]
fn create_reports_filelist_and_glob_errors() {
    let temp = scratch_dir("tindex-empty-filelist");
    let filelist = temp.join("files.txt");
    std::fs::write(&filelist, "\n\n").unwrap();
    let Err(empty_filelist) = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filelist",
        filelist.to_str().unwrap(),
    ])) else {
        panic!("expected empty filelist error");
    };
    assert!(
        matches!(empty_filelist, TindexParseResult::Error(message) if message.contains("contained no tindex input files"))
    );

    let Err(missing_glob) = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--glob",
        "/definitely/no/tindex/files/*.las",
    ])) else {
        panic!("expected missing glob error");
    };
    assert!(
        matches!(missing_glob, TindexParseResult::Error(message) if message.contains("did not match"))
    );
}

#[test]
fn create_rejects_invalid_option_values() {
    for (arg, value, expected) in [
        ("--threshold", "not-int", "integer"),
        ("--resolution", "not-float", "numeric"),
        ("--sample_size", "-1", "non-negative"),
        ("--simplify", "maybe", "boolean"),
        ("--skip_different_srs=sometimes", "", "boolean"),
    ] {
        let mut args = vec!["--tindex", "out.geojson", "--filespec=in.las", arg];
        if !value.is_empty() {
            args.push(value);
        }
        let Err(err) = parse_tindex_create_args(&strings(&args)) else {
            panic!("expected {arg} to fail");
        };
        assert!(
            matches!(err, TindexParseResult::Error(message) if message.contains(expected)),
            "{arg}"
        );
    }
}

#[test]
fn create_accepts_layer_creation_options_and_rejects_bad_lco_shape() {
    let parsed = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filespec=in.las",
        "--lco",
        "ENCODING=UTF-8",
    ]))
    .unwrap();
    assert_eq!(parsed.lco_options, vec!["ENCODING=UTF-8"]);
    assert_eq!(parsed.lco_description, None);

    let Err(bad_lco) = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filespec=in.las",
        "--lco",
        "ENCODING",
    ])) else {
        panic!("expected malformed lco");
    };
    assert!(matches!(bad_lco, TindexParseResult::Error(message) if message.contains("NAME=VALUE")));
}

#[test]
fn create_rejects_filter_options() {
    let Err(filter) = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filespec=in.las",
        "--filters.hexbin.smooth=true",
    ])) else {
        panic!("expected filter option rejection");
    };
    assert_eq!(
        filter,
        TindexParseResult::Error(INVALID_TINDEX_FILTER_STAGE_MESSAGE.to_string())
    );
}

#[test]
fn create_rejects_missing_required_values_and_unknown_options() {
    let Err(missing_value) = parse_tindex_create_args(&strings(&["--tindex"])) else {
        panic!("expected missing value");
    };
    assert!(
        matches!(missing_value, TindexParseResult::Error(message) if message.contains("requires a value"))
    );

    let Err(unknown) = parse_tindex_create_args(&strings(&[
        "--tindex",
        "out.geojson",
        "--filespec=in.las",
        "--bogus",
    ])) else {
        panic!("expected unknown option");
    };
    assert!(
        matches!(unknown, TindexParseResult::Error(message) if message.contains("unknown tindex option"))
    );
}

#[test]
fn merge_accepts_positionals_and_options() {
    let parsed = parse_tindex_merge_args(&strings(&[
        "--tindex",
        "idx.geojson",
        "--filespec",
        "out.las",
        "--tindex_name",
        "path",
        "--t_srs=EPSG:3857",
        "--bounds=([0,1],[2,3])",
    ]))
    .unwrap();
    assert_eq!(parsed.tindex_file, "idx.geojson");
    assert_eq!(parsed.output_file, "out.las");
    assert_eq!(parsed.location_field, "path");
    assert_eq!(parsed.target_srs, "EPSG:3857");
    match parsed.clip.unwrap() {
        TindexMergeClip::Bounds { bounds, value } => {
            assert_eq!(value, "([0,1],[2,3])");
            assert_eq!(bounds.minx, 0.0);
            assert_eq!(bounds.maxx, 1.0);
            assert_eq!(bounds.miny, 2.0);
            assert_eq!(bounds.maxy, 3.0);
        }
        TindexMergeClip::Polygon { .. } => panic!("expected bounds clip"),
    }
}

#[test]
fn merge_tracks_polygon_without_native_geometry() {
    let parsed = parse_tindex_merge_args(&strings(&[
        "idx.geojson",
        "out.las",
        "--polygon=POLYGON ((0 0, 1 0, 1 1, 0 0))",
    ]))
    .unwrap();
    match parsed.clip.unwrap() {
        TindexMergeClip::Polygon { value } => {
            assert!(value.starts_with("POLYGON"));
        }
        TindexMergeClip::Bounds { .. } => panic!("expected polygon clip"),
    }
}

#[test]
fn merge_accepts_separate_clip_and_ignored_driver_options() {
    let parsed = parse_tindex_merge_args(&strings(&[
        "--tindex",
        "idx.geojson",
        "--filespec",
        "out.laz",
        "--bounds",
        "([10,20],[30,40])",
        "--log",
        "debug",
        "--lyr_name",
        "tiles",
        "--ogrdriver",
        "GeoJSON",
        "-f",
        "GeoJSON",
    ]))
    .unwrap();

    assert_eq!(parsed.output_file, "out.laz");
    assert_eq!(parsed.layer_name, "tiles");
    match parsed.clip.unwrap() {
        TindexMergeClip::Bounds { bounds, value } => {
            assert_eq!(value, "([10,20],[30,40])");
            assert_eq!(bounds.minx, 10.0);
            assert_eq!(bounds.maxx, 20.0);
            assert_eq!(bounds.miny, 30.0);
            assert_eq!(bounds.maxy, 40.0);
        }
        TindexMergeClip::Polygon { .. } => panic!("expected bounds clip"),
    }
}

#[test]
fn merge_accepts_equals_layer_option() {
    let parsed =
        parse_tindex_merge_args(&strings(&["idx.gpkg", "out.las", "--lyr_name=tiles"])).unwrap();

    assert_eq!(parsed.layer_name, "tiles");
}

#[test]
fn merge_rejects_bad_arguments() {
    let cases = [
        (vec!["--tindex", "idx.geojson"], "merge requires --filespec"),
        (vec!["--filespec", "out.las"], "merge requires --tindex"),
        (
            vec!["idx.geojson", "out.las", "extra.las"],
            "too many merge arguments",
        ),
        (
            vec!["idx.geojson", "out.las", "--bounds=bad"],
            "Invalid bounds",
        ),
    ];
    for (args, expected) in cases {
        let Err(err) = parse_tindex_merge_args(&strings(&args)) else {
            panic!("expected merge parse failure for {args:?}");
        };
        assert!(
            matches!(err, TindexParseResult::Error(message) if message.contains(expected)),
            "{args:?}"
        );
    }

    let Err(unknown) =
        parse_tindex_merge_args(&strings(&["idx.geojson", "out.las", "--unsupported"]))
    else {
        panic!("expected unknown merge option");
    };
    assert!(
        matches!(unknown, TindexParseResult::Error(message) if message.contains("unknown tindex merge option"))
    );
}

#[test]
fn merge_plan_builds_reader_merge_writer_graph() {
    let parsed = parse_tindex_merge_args(&strings(&[
        "--tindex",
        "idx.geojson",
        "--filespec",
        "out.las",
    ]))
    .unwrap();
    let index = serde_json::json!({
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": { "location": "a.las", "srs": "EPSG:4326" },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]
                }
            },
            {
                "type": "Feature",
                "properties": { "location": "b.las", "srs": "EPSG:3857" },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[2.0, 2.0], [3.0, 2.0], [3.0, 3.0], [2.0, 2.0]]]
                }
            }
        ]
    });
    let plan = build_tindex_merge_plan(&parsed, &index.to_string(), None).unwrap();
    assert_eq!(plan.file_count, 2);
    let stages = plan.pipeline_json.as_array().unwrap();
    assert_eq!(stages[0]["type"], "readers.las");
    assert_eq!(stages[1]["type"], "readers.las");
    assert_eq!(stages[2]["type"], "filters.reprojection");
    assert_eq!(stages[3]["type"], "filters.merge");
    assert_eq!(stages[4]["type"], "writers.las");
}

#[test]
fn merge_plan_handles_single_reader_without_merge_stage() {
    let parsed = parse_tindex_merge_args(&strings(&["idx.geojson", "out.copc.laz"])).unwrap();
    let index = serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": { "location": "only.copc.laz" },
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]
            }
        }]
    });

    let plan = build_tindex_merge_plan(&parsed, &index.to_string(), None).unwrap();
    let stages = plan.pipeline_json.as_array().unwrap();
    assert_eq!(plan.file_count, 1);
    assert_eq!(stages.len(), 2);
    assert_eq!(stages[0]["type"], "readers.copc");
    assert_eq!(stages[1]["type"], "writers.copc");
}

#[test]
fn merge_plan_applies_clip_bounds_and_crop_stage() {
    let parsed = parse_tindex_merge_args(&strings(&["idx.geojson", "out.las"])).unwrap();
    let index = serde_json::json!({
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": { "location": "keep.las", "srs": "" },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0, 0.0], [5.0, 0.0], [5.0, 5.0], [0.0, 0.0]]]
                }
            },
            {
                "type": "Feature",
                "properties": { "location": "skip.las", "srs": "" },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[10.0, 10.0], [11.0, 10.0], [11.0, 11.0], [10.0, 10.0]]]
                }
            }
        ]
    });
    let clip = TindexResolvedClip {
        bounds: Bounds2D {
            minx: 1.0,
            maxx: 2.0,
            miny: 1.0,
            maxy: 2.0,
        },
        stage_key: "bounds",
        stage_value: "([1,2],[1,2])".to_string(),
    };
    let plan = build_tindex_merge_plan(&parsed, &index.to_string(), Some(clip)).unwrap();
    assert_eq!(plan.file_count, 1);
    let stages = plan.pipeline_json.as_array().unwrap();
    assert_eq!(stages.len(), 4);
    assert_eq!(stages[0]["filename"], "keep.las");
    assert_eq!(stages[1]["type"], "filters.crop");
    assert_eq!(stages[1]["bounds"], "([1,2],[1,2])");
    assert_eq!(stages[2]["type"], "filters.merge");
    assert_eq!(stages[3]["type"], "writers.las");
}

#[test]
fn merge_plan_uses_multipolygon_bounds_for_clip_matching() {
    let parsed = parse_tindex_merge_args(&strings(&["idx.geojson", "out.las"])).unwrap();
    let index = serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": { "location": "multi.las" },
            "geometry": {
                "type": "MultiPolygon",
                "coordinates": [
                    [[[10.0, 10.0], [11.0, 10.0], [11.0, 11.0], [10.0, 10.0]]],
                    [[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 0.0]]]
                ]
            }
        }]
    });
    let clip = TindexResolvedClip {
        bounds: Bounds2D {
            minx: 1.0,
            maxx: 1.5,
            miny: 1.0,
            maxy: 1.5,
        },
        stage_key: "polygon",
        stage_value: "POLYGON ((1 1, 1.5 1, 1.5 1.5, 1 1))".to_string(),
    };

    let plan = build_tindex_merge_plan(&parsed, &index.to_string(), Some(clip)).unwrap();
    let stages = plan.pipeline_json.as_array().unwrap();
    assert_eq!(plan.file_count, 1);
    assert_eq!(stages[1]["type"], "filters.crop");
    assert!(stages[1]["polygon"]
        .as_str()
        .unwrap()
        .starts_with("POLYGON"));
}

#[test]
fn merge_plan_reports_invalid_index_inputs() {
    let parsed = parse_tindex_merge_args(&strings(&["idx.geojson", "out.las"])).unwrap();
    for (index_json, expected) in [
        ("not json", "Unable to parse GeoJSON"),
        (
            r#"{"type":"FeatureCollection"}"#,
            "GeoJSON FeatureCollection",
        ),
        (
            r#"{"type":"FeatureCollection","features":[]}"#,
            "contains no features",
        ),
        (
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"location":"a.las"},"geometry":{"type":"Point","coordinates":[0,0]}}]}"#,
            "Feature has invalid geometry",
        ),
        (
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[0,0]]]}}]}"#,
            "Feature is missing 'location'",
        ),
        (
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"location":"a.unknown"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[0,0]]]}}]}"#,
            "unable to infer reader driver",
        ),
    ] {
        let clip = if expected == "Feature has invalid geometry" {
            Some(TindexResolvedClip {
                bounds: Bounds2D {
                    minx: 0.0,
                    maxx: 1.0,
                    miny: 0.0,
                    maxy: 1.0,
                },
                stage_key: "bounds",
                stage_value: "([0,1],[0,1])".to_string(),
            })
        } else {
            None
        };
        let Err(err) = build_tindex_merge_plan(&parsed, index_json, clip) else {
            panic!("expected merge plan error for {expected}");
        };
        assert!(
            matches!(err, TindexParseResult::Error(message) if message.contains(expected)),
            "{expected}"
        );
    }
}

#[test]
fn merge_plan_reports_no_clip_matches_and_unknown_writer() {
    let parsed = parse_tindex_merge_args(&strings(&["idx.geojson", "out.las"])).unwrap();
    let index = serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": { "location": "a.las" },
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[10.0, 10.0], [11.0, 10.0], [11.0, 11.0], [10.0, 10.0]]]
            }
        }]
    });
    let clip = TindexResolvedClip {
        bounds: Bounds2D {
            minx: 0.0,
            maxx: 1.0,
            miny: 0.0,
            maxy: 1.0,
        },
        stage_key: "bounds",
        stage_value: "([0,1],[0,1])".to_string(),
    };
    let Err(no_match) = build_tindex_merge_plan(&parsed, &index.to_string(), Some(clip)) else {
        panic!("expected no matching indexed files");
    };
    assert!(
        matches!(no_match, TindexParseResult::Error(message) if message.contains("No indexed files matched"))
    );

    let bad_writer = parse_tindex_merge_args(&strings(&["idx.geojson", "out.unknown"])).unwrap();
    let Err(writer) = build_tindex_merge_plan(&bad_writer, &index.to_string(), None) else {
        panic!("expected writer inference error");
    };
    assert!(
        matches!(writer, TindexParseResult::Error(message) if message.contains("Unable to infer writer driver"))
    );
}
