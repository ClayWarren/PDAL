use super::*;

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
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
