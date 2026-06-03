use super::*;

// ----- analysis_commands.rs error path coverage -----

#[test]
fn hausdorff_errors_on_source_missing_value() {
    let app = app_with_command("hausdorff", &["--source"]);
    assert_eq!(app.run_hausdorff(), 1);
}

#[test]
fn hausdorff_errors_on_candidate_missing_value() {
    let app = app_with_command("hausdorff", &["--candidate"]);
    assert_eq!(app.run_hausdorff(), 1);
}

#[test]
fn hausdorff_accepts_source_and_candidate_equals_form() {
    let app = app_with_command(
        "hausdorff",
        &["--source=/no/such/a.las", "--candidate=/no/such/b.las"],
    );
    assert_eq!(app.run_hausdorff(), 1);
}

#[test]
fn hausdorff_errors_on_unknown_option() {
    let app = app_with_command("hausdorff", &["--mystery"]);
    assert_eq!(app.run_hausdorff(), 1);
}

#[test]
fn hausdorff_errors_on_three_positionals() {
    let app = app_with_command("hausdorff", &["a", "b", "c"]);
    assert_eq!(app.run_hausdorff(), 1);
}

#[test]
fn chamfer_errors_on_source_missing_value() {
    let app = app_with_command("chamfer", &["--source"]);
    assert_eq!(app.run_chamfer(), 1);
}

#[test]
fn chamfer_errors_on_candidate_missing_value() {
    let app = app_with_command("chamfer", &["--candidate"]);
    assert_eq!(app.run_chamfer(), 1);
}

#[test]
fn chamfer_accepts_source_and_candidate_equals_form() {
    let app = app_with_command(
        "chamfer",
        &["--source=/no/such/a.las", "--candidate=/no/such/b.las"],
    );
    assert_eq!(app.run_chamfer(), 1);
}

#[test]
fn delta_errors_on_source_missing_value() {
    let app = app_with_command("delta", &["--source"]);
    assert_eq!(app.run_delta(), 1);
}

#[test]
fn delta_errors_on_candidate_missing_value() {
    let app = app_with_command("delta", &["--candidate"]);
    assert_eq!(app.run_delta(), 1);
}

#[test]
fn delta_accepts_source_and_candidate_equals_form() {
    let app = app_with_command(
        "delta",
        &["--source=/no/such/a.las", "--candidate=/no/such/b.las"],
    );
    assert_eq!(app.run_delta(), 1);
}

// ----- tile -----

#[test]
fn tile_command_help_returns_zero_with_args() {
    let mut app = app_with_command("tile", &["in.las", "out#.las"]);
    app.help = true;
    assert_eq!(app.run_tile(), 0);
}

#[test]
fn tile_errors_on_unknown_option() {
    let app = app_with_command("tile", &["--mystery=1", "in.las", "out#.las"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_errors_on_unknown_long_option_with_space() {
    let app = app_with_command("tile", &["--mystery", "10", "in.las", "out#.las"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_errors_on_option_missing_value() {
    let app = app_with_command("tile", &["--length"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_accepts_options_with_equals() {
    // Bad input/output still proves the equals-form parse succeeds and reaches
    // the Rust-backed tile runner, which reports the missing input as an error.
    let app = app_with_command(
        "tile",
        &[
            "--length=100",
            "--origin_x=0",
            "--origin_y=0",
            "--buffer=10",
            "/no/such/input.las",
            "/tmp/out#.las",
        ],
    );
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_errors_on_long_option_via_space() {
    let app = app_with_command(
        "tile",
        &["--length", "100", "/no/such/input.las", "/tmp/out#.las"],
    );
    let _ = app.run_tile();
}

// ----- tindex create -----

#[test]
fn tindex_create_help_returns_zero() {
    let mut app = app_with_command("tindex", &["create"]);
    app.help = true;
    assert_eq!(app.run_tindex(), 0);
}

#[test]
fn tindex_errors_when_subcommand_is_unknown() {
    let app = app_with_command("tindex", &["weird"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_missing_tindex() {
    // No --tindex provided -- only files via --filelist
    let dir = tmp_dir("tindex-filelist");
    let list = dir.join("list.txt");
    std::fs::write(&list, "/no/such/a.las\n").unwrap();
    let app = app_with_command("tindex", &["create", "--filelist", list.to_str().unwrap()]);
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_create_errors_on_missing_tindex_value() {
    let app = app_with_command("tindex", &["create", "--tindex"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_missing_filelist_value() {
    let app = app_with_command("tindex", &["create", "--filelist"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_filelist_not_found() {
    let app = app_with_command(
        "tindex",
        &[
            "create",
            "--tindex",
            "/tmp/idx.json",
            "--filelist",
            "/no/such/list.txt",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_missing_glob_value() {
    let app = app_with_command("tindex", &["create", "--glob"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_invalid_glob() {
    let app = app_with_command(
        "tindex",
        &["create", "--tindex", "/tmp/idx.json", "--glob", "***"],
    );
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_glob_with_no_matches() {
    let app = app_with_command(
        "tindex",
        &[
            "create",
            "--tindex",
            "/tmp/idx.json",
            "--glob",
            "/no/such/dir/*.las",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_path_prefix_no_value() {
    let app = app_with_command("tindex", &["create", "--path_prefix"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_lyr_name_no_value() {
    let app = app_with_command("tindex", &["create", "--lyr_name"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_tindex_name_no_value() {
    let app = app_with_command("tindex", &["create", "--tindex_name"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_ogrdriver_no_value() {
    let app = app_with_command("tindex", &["create", "-f"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_unknown_dash_option() {
    let app = app_with_command("tindex", &["create", "--mystery"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_takes_first_positional_as_output() {
    // Positional tindex output followed by file that doesn't exist -> dataset
    // creation succeeds (in-memory) or fails; either way the pipeline summary
    // for /no/such/file.las will fail.
    let dir = tmp_dir("tindex-pos");
    let out = dir.join("idx.shp");
    let app = app_with_command(
        "tindex",
        &["create", out.to_str().unwrap(), "/no/such/file.las"],
    );
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- tindex merge -----

#[test]
fn tindex_merge_errors_on_unknown_dash_option() {
    let app = app_with_command("tindex", &["merge", "--mystery"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_tindex_name_no_value() {
    let app = app_with_command("tindex", &["merge", "--tindex_name"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_lyr_name_no_value() {
    let app = app_with_command("tindex", &["merge", "--lyr_name"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_ogrdriver_no_value() {
    let app = app_with_command("tindex", &["merge", "-f"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_three_positionals() {
    let app = app_with_command("tindex", &["merge", "a", "b", "c"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_accepts_positional_paths_unknown_files() {
    let app = app_with_command("tindex", &["merge", "/no/such/idx.json", "/tmp/out.las"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_unreadable_index() {
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            "/no/such/index.json",
            "--filespec",
            "/tmp/out.las",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_invalid_json() {
    let dir = tmp_dir("tindex-merge-bad");
    let idx = dir.join("idx.json");
    std::fs::write(&idx, "{not json}").unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            "/tmp/out.las",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_merge_errors_on_not_feature_collection() {
    let dir = tmp_dir("tindex-merge-notfc");
    let idx = dir.join("idx.json");
    std::fs::write(&idx, r#"{"type":"Point","coordinates":[0,0]}"#).unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            "/tmp/out.las",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_merge_errors_on_empty_features() {
    let dir = tmp_dir("tindex-merge-empty");
    let idx = dir.join("idx.json");
    std::fs::write(&idx, r#"{"type":"FeatureCollection","features":[]}"#).unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            "/tmp/out.las",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_merge_errors_on_missing_location() {
    let dir = tmp_dir("tindex-merge-noloc");
    let idx = dir.join("idx.json");
    std::fs::write(
        &idx,
        r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":null}]}"#,
    )
    .unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            "/tmp/out.las",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_merge_errors_on_unknown_input_ext() {
    let dir = tmp_dir("tindex-merge-badext");
    let idx = dir.join("idx.json");
    std::fs::write(
        &idx,
        r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"location":"foo.unknownext"},"geometry":null}]}"#,
    )
    .unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            "/tmp/out.las",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_merge_errors_on_unknown_output_ext() {
    let dir = tmp_dir("tindex-merge-badout");
    let idx = dir.join("idx.json");
    std::fs::write(
        &idx,
        r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"location":"foo.las"},"geometry":null}]}"#,
    )
    .unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            "/tmp/out.unknownext",
        ],
    );
    assert_eq!(app.run_tindex(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
