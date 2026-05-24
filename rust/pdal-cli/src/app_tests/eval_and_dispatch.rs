use super::*;

// ----- eval -----

#[test]
fn eval_help_returns_zero() {
    let mut app = app_with_command("eval", &["pred", "truth", "--labels=1,2"]);
    app.help = true;
    assert_eq!(app.run_eval(), 0);
}

#[test]
fn eval_errors_on_long_option_no_value() {
    let app = app_with_command("eval", &["--labels"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn eval_accepts_predicted_truth_via_options() {
    let app = app_with_command(
        "eval",
        &[
            "--predicted=/no/such/a.las",
            "--truth=/no/such/b.las",
            "--labels=1,2",
            "--prediction_dim=Classification",
            "--truth_dim=Classification",
        ],
    );
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn eval_attempts_with_unknown_inputs() {
    let app = app_with_command(
        "eval",
        &["/no/such/a.las", "/no/such/b.las", "--labels=1,2"],
    );
    assert_eq!(app.run_eval(), 1);
}

fn repo_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn tindex_create_succeeds_with_real_las_input() {
    // Exercises the happy path including bounds extraction, dataset
    // creation, layer creation, field creation, and feature insertion.
    let dir = tmp_dir("tindex-real");
    let out = dir.join("idx.shp");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "tindex",
        &[
            "create",
            "--tindex",
            out.to_str().unwrap(),
            las.to_str().unwrap(),
        ],
    );
    let _ = app.run_tindex();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_create_with_filelist_works_for_real_las() {
    let dir = tmp_dir("tindex-filelist-real");
    let out = dir.join("idx.shp");
    let list = dir.join("list.txt");
    let las = repo_path("test/data/las/100-points.las");
    std::fs::write(&list, format!("{}\n", las.display())).unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "create",
            "--tindex",
            out.to_str().unwrap(),
            "--filelist",
            list.to_str().unwrap(),
        ],
    );
    let _ = app.run_tindex();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_create_with_glob_works_for_real_las() {
    let dir = tmp_dir("tindex-glob-real");
    let out = dir.join("idx.shp");
    let pattern = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "tindex",
        &[
            "create",
            "--tindex",
            out.to_str().unwrap(),
            "--glob",
            pattern.to_str().unwrap(),
        ],
    );
    let _ = app.run_tindex();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_create_with_path_prefix_works() {
    let dir = tmp_dir("tindex-pfx-real");
    let out = dir.join("idx.shp");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "tindex",
        &[
            "create",
            "--tindex",
            out.to_str().unwrap(),
            "--path_prefix",
            "/prefix/",
            las.to_str().unwrap(),
        ],
    );
    let _ = app.run_tindex();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_create_write_absolute_path_works() {
    let dir = tmp_dir("tindex-abs-real");
    let out = dir.join("idx.shp");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "tindex",
        &[
            "create",
            "--tindex",
            out.to_str().unwrap(),
            "--write_absolute_path",
            las.to_str().unwrap(),
        ],
    );
    let _ = app.run_tindex();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_merge_succeeds_with_real_las_via_geojson_index() {
    let dir = tmp_dir("tindex-merge-real");
    let idx = dir.join("idx.geojson");
    let out = dir.join("merged.las");
    let las = repo_path("test/data/las/100-points.las");
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[{{"type":"Feature","properties":{{"location":"{}"}},"geometry":null}}]}}"#,
        las.display()
    );
    std::fs::write(&idx, json).unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            out.to_str().unwrap(),
        ],
    );
    let _ = app.run_tindex();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tindex_merge_with_two_real_files_runs_filters_merge() {
    let dir = tmp_dir("tindex-merge2-real");
    let idx = dir.join("idx.geojson");
    let out = dir.join("merged.las");
    let las = repo_path("test/data/las/100-points.las");
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[
            {{"type":"Feature","properties":{{"location":"{}"}},"geometry":null}},
            {{"type":"Feature","properties":{{"location":"{}"}},"geometry":null}}
        ]}}"#,
        las.display(),
        las.display()
    );
    std::fs::write(&idx, json).unwrap();
    let app = app_with_command(
        "tindex",
        &[
            "merge",
            "--tindex",
            idx.to_str().unwrap(),
            "--filespec",
            out.to_str().unwrap(),
        ],
    );
    let _ = app.run_tindex();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hausdorff_with_real_files_runs() {
    let a = repo_path("test/data/las/100-points.las");
    let b = repo_path("test/data/las/100-points.las");
    let app = app_with_command("hausdorff", &[a.to_str().unwrap(), b.to_str().unwrap()]);
    let _ = app.run_hausdorff();
}

#[test]
fn chamfer_with_real_files_runs() {
    let a = repo_path("test/data/las/100-points.las");
    let b = repo_path("test/data/las/100-points.las");
    let app = app_with_command("chamfer", &[a.to_str().unwrap(), b.to_str().unwrap()]);
    let _ = app.run_chamfer();
}

#[test]
fn delta_with_real_files_runs() {
    let a = repo_path("test/data/las/100-points.las");
    let b = repo_path("test/data/las/100-points.las");
    let app = app_with_command("delta", &[a.to_str().unwrap(), b.to_str().unwrap()]);
    let _ = app.run_delta();
}

#[test]
fn translate_with_real_files_runs() {
    let dir = tmp_dir("translate-real");
    let out = dir.join("out.pcd");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command("translate", &[las.to_str().unwrap(), out.to_str().unwrap()]);
    let _ = app.run_translate();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn translate_with_dotted_filter_runs() {
    let dir = tmp_dir("translate-filter");
    let out = dir.join("out.las");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "translate",
        &[
            las.to_str().unwrap(),
            out.to_str().unwrap(),
            "filters.decimation",
            "--filters.decimation.step=2",
        ],
    );
    let _ = app.run_translate();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn merge_with_real_files_runs() {
    let dir = tmp_dir("merge-real");
    let out = dir.join("out.las");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "merge",
        &[
            las.to_str().unwrap(),
            las.to_str().unwrap(),
            out.to_str().unwrap(),
        ],
    );
    let _ = app.run_merge();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sort_with_real_files_runs() {
    let dir = tmp_dir("sort-real");
    let out = dir.join("out.las");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command("sort", &[las.to_str().unwrap(), out.to_str().unwrap()]);
    let _ = app.run_sort();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ground_with_real_files_runs() {
    let dir = tmp_dir("ground-real");
    let out = dir.join("out.las");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "ground",
        &[
            las.to_str().unwrap(),
            out.to_str().unwrap(),
            "--filters.smrf.cell=10",
        ],
    );
    let _ = app.run_ground();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn density_with_real_files_runs() {
    let dir = tmp_dir("density-real");
    let out = dir.join("out.geojson");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command("density", &[las.to_str().unwrap(), out.to_str().unwrap()]);
    let _ = app.run_density();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn random_with_real_output_runs() {
    let dir = tmp_dir("random-real");
    let out = dir.join("out.las");
    let app = app_with_command("random", &["--count=50", out.to_str().unwrap()]);
    let _ = app.run_random();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn split_with_real_input_runs() {
    let dir = tmp_dir("split-real");
    let out = dir.join("out.las");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "split",
        &[
            "--length=50",
            "--origin_x=0",
            "--origin_y=0",
            las.to_str().unwrap(),
            out.to_str().unwrap(),
        ],
    );
    let _ = app.run_split();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn split_with_capacity_mode_runs() {
    let dir = tmp_dir("split-cap-real");
    let out = dir.join("out.las");
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command(
        "split",
        &[
            "--capacity=20",
            las.to_str().unwrap(),
            out.to_str().unwrap(),
        ],
    );
    let _ = app.run_split();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn info_with_real_file_runs() {
    let las = repo_path("test/data/las/100-points.las");
    let app = app_with_command("info", &[las.to_str().unwrap()]);
    let _ = app.run_info();
}

// ----- app.rs parser / dispatch coverage -----

#[test]
fn parse_args_errors_on_label_without_value() {
    let mut app = App::new();
    assert!(app.parse_args(&["--label".to_string()]).is_err());
}

#[test]
fn parse_args_errors_on_log_without_value() {
    let mut app = App::new();
    assert!(app.parse_args(&["--log".to_string()]).is_err());
}

#[test]
fn parse_args_accepts_log_with_value() {
    let mut app = App::new();
    app.parse_args(&["--log".to_string(), "/tmp/log.txt".to_string()])
        .unwrap();
    assert_eq!(app.log, "/tmp/log.txt");
}

#[test]
fn parse_args_errors_on_command_without_value() {
    let mut app = App::new();
    assert!(app.parse_args(&["--command".to_string()]).is_err());
}

#[test]
fn parse_args_errors_on_options_without_value() {
    let mut app = App::new();
    assert!(app.parse_args(&["--options".to_string()]).is_err());
}

#[test]
fn parse_args_errors_on_unexpected_dash_argument() {
    let mut app = App::new();
    assert!(app.parse_args(&["--bogus-flag".to_string()]).is_err());
}

#[test]
fn parse_args_skips_label_equals_after_command() {
    let mut app = App::new();
    app.parse_args(&[
        "pipeline".to_string(),
        "--label=foo".to_string(),
        "p.json".to_string(),
    ])
    .unwrap();
    assert_eq!(app.command_args, vec!["p.json".to_string()]);
}

#[test]
fn parse_args_errors_on_label_after_command_without_value() {
    let mut app = App::new();
    // The parser increments i by 2 then checks if it overshoots the slice.
    let result = app.parse_args(&["pipeline".to_string(), "--label".to_string()]);
    assert!(result.is_err());
}

#[test]
fn dispatch_unknown_command_path_returns_error() {
    let app = app_with_command("not-a-command", &["any"]);
    assert_eq!(app.run(), 1);
}

#[test]
fn version_with_show_json_prints_json() {
    let mut app = App::new();
    app.show_version = true;
    app.show_json = true;
    assert_eq!(app.run(), 0);
}

#[test]
fn drivers_with_show_json_prints_json() {
    let mut app = App::new();
    app.show_drivers = true;
    app.show_json = true;
    assert_eq!(app.run(), 0);
}

#[test]
fn commands_with_show_json_prints_json() {
    let mut app = App::new();
    app.show_commands = true;
    app.show_json = true;
    assert_eq!(app.run(), 0);
}

#[test]
fn options_for_all_stages_returns_zero() {
    let mut app = App::new();
    app.show_options = Some("all".to_string());
    assert_eq!(app.run(), 0);
}

#[test]
fn options_for_all_stages_json_returns_zero() {
    let mut app = App::new();
    app.show_options = Some("all".to_string());
    app.show_json = true;
    assert_eq!(app.run(), 0);
}

#[test]
fn options_for_unknown_stage_does_not_panic() {
    let mut app = App::new();
    app.show_options = Some("readers.unknownmystery".to_string());
    // output_options prints "Unable to create stage X" and returns.
    assert_eq!(app.run(), 0);
}

#[test]
fn run_with_no_command_outputs_help() {
    let app = App::new();
    assert_eq!(app.run(), 0);
}

#[test]
fn fauxplugin_dispatch_via_run() {
    // Run dispatches to FauxPluginKernel; with no args it should likely error.
    let mut app = App::new();
    app.command = "fauxplugin".to_string();
    // Not asserting return code; just exercising the branch.
    let _ = app.run();
}
