use super::*;

// ----- Extended coverage tests for pipeline_commands.rs error paths -----

#[test]
fn pipeline_command_writes_serialization_to_bad_path() {
    let dir = tmp_dir("ser-bad");
    let pipeline_path = dir.join("p.json");
    std::fs::write(&pipeline_path, "[]").unwrap();
    let bad = "/no/such/directory/serialized.json";
    let app = app_with_command(
        "pipeline",
        &[
            pipeline_path.to_str().unwrap(),
            "--pipeline-serialization",
            bad,
        ],
    );
    assert_eq!(app.run_pipeline(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pipeline_command_metadata_to_bad_path_errors() {
    let dir = tmp_dir("meta-bad");
    let pipeline_path = dir.join("p.json");
    std::fs::write(&pipeline_path, "[]").unwrap();
    let bad = "/no/such/directory/metadata.json";
    let app = app_with_command(
        "pipeline",
        &[pipeline_path.to_str().unwrap(), "--metadata", bad],
    );
    // Pipeline create may fail or metadata write fails; both go to error code 1.
    assert_eq!(app.run_pipeline(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pipeline_command_executes_validate_only() {
    let dir = tmp_dir("validate");
    let pipeline_path = dir.join("p.json");
    std::fs::write(
        &pipeline_path,
        r#"[{"type":"readers.faux","count":10,"mode":"constant"}]"#,
    )
    .unwrap();
    let app = app_with_command("pipeline", &["--validate", pipeline_path.to_str().unwrap()]);
    assert_eq!(app.run_pipeline(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pipeline_command_with_show_json_summary_runs() {
    let dir = tmp_dir("showjson");
    let pipeline_path = dir.join("p.json");
    std::fs::write(
        &pipeline_path,
        r#"[{"type":"readers.faux","count":4,"mode":"constant"}]"#,
    )
    .unwrap();
    let mut app = app_with_command("pipeline", &[pipeline_path.to_str().unwrap()]);
    app.show_json = true;
    assert_eq!(app.run_pipeline(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pipeline_command_metadata_writes_to_path() {
    let dir = tmp_dir("meta-ok");
    let pipeline_path = dir.join("p.json");
    std::fs::write(
        &pipeline_path,
        r#"[{"type":"readers.faux","count":4,"mode":"constant"}]"#,
    )
    .unwrap();
    let metadata = dir.join("meta.json");
    let app = app_with_command(
        "pipeline",
        &[
            pipeline_path.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
        ],
    );
    assert_eq!(app.run_pipeline(), 0);
    assert!(metadata.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pipeline_command_executes_default_path() {
    let dir = tmp_dir("exec");
    let pipeline_path = dir.join("p.json");
    std::fs::write(
        &pipeline_path,
        r#"[{"type":"readers.faux","count":4,"mode":"constant"}]"#,
    )
    .unwrap();
    let app = app_with_command("pipeline", &[pipeline_path.to_str().unwrap()]);
    assert_eq!(app.run_pipeline(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pipeline_command_execute_fails_on_bad_reader_path() {
    let dir = tmp_dir("badreader");
    let pipeline_path = dir.join("p.json");
    std::fs::write(
        &pipeline_path,
        r#"[{"type":"readers.las","filename":"/no/such/file.las"}]"#,
    )
    .unwrap();
    let app = app_with_command("pipeline", &[pipeline_path.to_str().unwrap()]);
    assert_eq!(app.run_pipeline(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn info_command_help_returns_zero() {
    let mut app = app_with_command("info", &["any.las"]);
    app.help = true;
    assert_eq!(app.run_info(), 0);
}

#[test]
fn info_command_errors_when_short_driver_missing_value() {
    let app = app_with_command("info", &["--driver"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_on_short_input_missing_value() {
    let app = app_with_command("info", &["-i"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_on_two_filenames_via_short_input() {
    let app = app_with_command("info", &["-i", "a.las", "b.las"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_on_unknown_extension() {
    let app = app_with_command("info", &["/no/such/file.weirdext"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_on_missing_file() {
    let app = app_with_command("info", &["/no/such/file.las"]);
    assert_eq!(app.run_info(), 1);
}

// ----- translate -----
#[test]
fn translate_command_help_returns_zero() {
    let mut app = app_with_command("translate", &["a.las", "b.las"]);
    app.help = true;
    assert_eq!(app.run_translate(), 0);
}

#[test]
fn translate_errors_on_input_short_no_value() {
    let app = app_with_command("translate", &["-i"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_output_short_no_value() {
    let app = app_with_command("translate", &["-o"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_reader_short_no_value() {
    let app = app_with_command("translate", &["-r"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_writer_short_no_value() {
    let app = app_with_command("translate", &["-w"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_filter_short_no_value() {
    let app = app_with_command("translate", &["-f"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_unknown_writer_extension() {
    let app = app_with_command("translate", &["in.las", "out.unknownext"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_unknown_reader_extension() {
    let app = app_with_command("translate", &["in.unknownext", "out.las"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_bad_stage_option() {
    let app = app_with_command("translate", &["in.las", "out.las", "--no-equals-or-dot"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_errors_on_apply_stage_options_failure() {
    // option references a stage not in the pipeline
    let app = app_with_command(
        "translate",
        &["in.las", "out.las", "--filters.nope.foo=bar"],
    );
    assert_eq!(app.run_translate(), 1);
}

// ----- merge -----
#[test]
fn merge_command_help_returns_zero() {
    let mut app = app_with_command("merge", &["a.las", "b.las", "out.las"]);
    app.help = true;
    assert_eq!(app.run_merge(), 0);
}

#[test]
fn merge_errors_on_driver_no_value() {
    let app = app_with_command("merge", &["--driver"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn merge_errors_on_files_no_value() {
    let app = app_with_command("merge", &["--files"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn merge_errors_on_files_short_no_value() {
    let app = app_with_command("merge", &["-f"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn merge_errors_on_bad_stage_option() {
    let app = app_with_command("merge", &["a.las", "b.las", "out.las", "--bad-opt"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn merge_errors_on_unknown_input_extension() {
    let app = app_with_command("merge", &["in.unknownext", "out.las"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn merge_errors_on_unknown_writer_extension() {
    let app = app_with_command("merge", &["in.las", "in2.las", "out.unknownext"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn merge_errors_on_apply_stage_options_failure() {
    let app = app_with_command(
        "merge",
        &["a.las", "b.las", "out.las", "--filters.nope.foo=bar"],
    );
    assert_eq!(app.run_merge(), 1);
}

// ----- sort -----
#[test]
fn sort_command_help_returns_zero() {
    let mut app = app_with_command("sort", &["a.las", "b.las"]);
    app.help = true;
    assert_eq!(app.run_sort(), 0);
}

#[test]
fn sort_errors_on_input_short_no_value() {
    let app = app_with_command("sort", &["-i"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_output_short_no_value() {
    let app = app_with_command("sort", &["-o"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_driver_no_value() {
    let app = app_with_command("sort", &["--driver"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_bad_stage_option_format() {
    let app = app_with_command("sort", &["a.las", "b.las", "--bad-opt"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_extra_positional() {
    let app = app_with_command("sort", &["a.las", "b.las", "extra.las"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_missing_output() {
    let app = app_with_command("sort", &["--input", "a.las"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_unknown_input_extension() {
    let app = app_with_command("sort", &["in.unknownext", "out.las"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_unknown_output_extension() {
    let app = app_with_command("sort", &["in.las", "out.unknownext"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_errors_on_apply_stage_options_failure() {
    let app = app_with_command("sort", &["in.las", "out.las", "--filters.nope.foo=bar"]);
    assert_eq!(app.run_sort(), 1);
}

// ----- ground -----
#[test]
fn ground_command_help_returns_zero() {
    let mut app = app_with_command("ground", &["a.las", "b.las"]);
    app.help = true;
    assert_eq!(app.run_ground(), 0);
}

#[test]
fn ground_errors_on_input_short_no_value() {
    let app = app_with_command("ground", &["-i"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_output_short_no_value() {
    let app = app_with_command("ground", &["-o"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_driver_no_value() {
    let app = app_with_command("ground", &["--driver"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_bad_stage_option_format() {
    let app = app_with_command("ground", &["a.las", "b.las", "--bad-opt"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_extra_positional() {
    let app = app_with_command("ground", &["a.las", "b.las", "extra.las"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_missing_output_path() {
    let app = app_with_command("ground", &["--input", "a.las"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_unknown_input_extension() {
    let app = app_with_command("ground", &["in.unknownext", "out.las"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_unknown_output_extension() {
    let app = app_with_command("ground", &["in.las", "out.unknownext"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_errors_on_apply_stage_options_failure() {
    let app = app_with_command("ground", &["in.las", "out.las", "--filters.nope.foo=bar"]);
    assert_eq!(app.run_ground(), 1);
}

// ----- density -----
#[test]
fn density_command_help_returns_zero() {
    let mut app = app_with_command("density", &["a.las", "out.geojson"]);
    app.help = true;
    assert_eq!(app.run_density(), 0);
}

#[test]
fn density_errors_on_input_short_no_value() {
    let app = app_with_command("density", &["-i"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_output_short_no_value() {
    let app = app_with_command("density", &["-o"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_driver_no_value() {
    let app = app_with_command("density", &["--driver"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_ogrdriver_no_value() {
    let app = app_with_command("density", &["--ogrdriver"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_ogrdriver_short_no_value() {
    let app = app_with_command("density", &["-f"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_edge_length_no_value() {
    let app = app_with_command("density", &["--edge_length"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_threshold_no_value() {
    let app = app_with_command("density", &["--threshold"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_accepts_edge_length_equals() {
    let app = app_with_command(
        "density",
        &["--edge_length=10.0", "in.unknownext", "out.geojson"],
    );
    // Reader-driver inference fails on unknown ext.
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_accepts_threshold_equals() {
    let app = app_with_command(
        "density",
        &["--threshold=5", "in.unknownext", "out.geojson"],
    );
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_bad_stage_option_format() {
    let app = app_with_command("density", &["a.las", "out.geojson", "--bad-opt"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_extra_positional() {
    let app = app_with_command("density", &["a.las", "out.geojson", "extra.las"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_missing_output_path() {
    let app = app_with_command("density", &["--input", "a.las"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_errors_on_unknown_input_extension() {
    let app = app_with_command("density", &["in.unknownext", "out.geojson"]);
    assert_eq!(app.run_density(), 1);
}

// ----- random -----
#[test]
fn random_command_help_returns_zero() {
    let mut app = app_with_command("random", &["out.las"]);
    app.help = true;
    assert_eq!(app.run_random(), 0);
}

#[test]
fn random_count_equals_bad_int() {
    let app = app_with_command("random", &["--count=notanumber", "out.las"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_count_no_value() {
    let app = app_with_command("random", &["--count"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_count_bad_int() {
    let app = app_with_command("random", &["--count", "notanumber", "out.las"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_output_no_value() {
    let app = app_with_command("random", &["--output"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_short_output_no_value() {
    let app = app_with_command("random", &["-o"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_errors_on_duplicate_output_via_short() {
    let app = app_with_command("random", &["-o", "a.las", "-o", "b.las"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_errors_on_extra_positional() {
    let app = app_with_command("random", &["out.las", "extra.las"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_errors_on_unknown_writer_extension() {
    let app = app_with_command("random", &["out.unknownext"]);
    assert_eq!(app.run_random(), 1);
}

// ----- split -----
#[test]
fn split_command_help_returns_zero() {
    let mut app = app_with_command("split", &["in.las", "out.las"]);
    app.help = true;
    assert_eq!(app.run_split(), 0);
}

#[test]
fn split_errors_on_unknown_input_extension() {
    let app = app_with_command("split", &["in.unknownext", "out.las"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_unknown_output_extension() {
    let app = app_with_command("split", &["in.las", "out.unknownext"]);
    assert_eq!(app.run_split(), 1);
}
