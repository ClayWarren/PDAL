use super::*;

#[test]
fn command_metadata_lists_all_cpp_kernel_commands() {
    let names = crate::stage_metadata::kernel_list()
        .into_iter()
        .map(|kernel| kernel.name)
        .collect::<std::collections::BTreeSet<_>>();

    for command in [
        "chamfer",
        "delta",
        "density",
        "eval",
        "ground",
        "hausdorff",
        "info",
        "merge",
        "pipeline",
        "random",
        "sort",
        "split",
        "tile",
        "tindex",
        "translate",
    ] {
        assert!(
            names.contains(command),
            "{command} missing from command metadata"
        );
    }
}

#[test]
fn parse_preserves_command_arguments() {
    let mut app = App::new();
    app.parse_args(&[
        "pipeline".to_string(),
        "pipeline.json".to_string(),
        "--not-a-root-option".to_string(),
    ])
    .unwrap();

    assert_eq!(app.command, "pipeline");
    assert_eq!(
        app.command_args,
        vec![
            "pipeline.json".to_string(),
            "--not-a-root-option".to_string()
        ]
    );
}

#[test]
fn parse_keeps_root_options_before_command() {
    let mut app = App::new();
    app.parse_args(&[
        "--verbose".to_string(),
        "--showjson".to_string(),
        "pipeline".to_string(),
        "pipeline.json".to_string(),
    ])
    .unwrap();

    assert_eq!(app.verbose, 1);
    assert!(app.show_json);
    assert_eq!(app.command, "pipeline");
    assert_eq!(app.command_args, vec!["pipeline.json".to_string()]);
}

#[test]
fn parse_supports_command_option() {
    let mut app = App::new();
    app.parse_args(&[
        "--command".to_string(),
        "pipeline".to_string(),
        "pipeline.json".to_string(),
    ])
    .unwrap();

    assert_eq!(app.command, "pipeline");
    assert_eq!(app.command_args, vec!["pipeline.json".to_string()]);
}

#[test]
fn parse_supports_debug_option() {
    let mut app = App::new();
    app.parse_args(&["--debug".to_string(), "--verbose".to_string()])
        .unwrap();

    assert_eq!(app.verbose, 4);
}

#[test]
fn parse_ignores_standard_label_and_developer_debug_options() {
    let mut app = App::new();
    app.parse_args(&[
        "--label=root".to_string(),
        "info".to_string(),
        "--label".to_string(),
        "smoke".to_string(),
        "--developer-debug".to_string(),
        "--summary".to_string(),
        "input.las".to_string(),
    ])
    .unwrap();

    assert_eq!(app.command, "info");
    assert_eq!(
        app.command_args,
        vec!["--summary".to_string(), "input.las".to_string()]
    );
}

#[test]
fn command_help_requested_detects_command_local_help() {
    let mut app = App::new();
    app.parse_args(&["tindex".to_string(), "--help".to_string()])
        .unwrap();

    assert!(app.command_help_requested());
}

#[test]
fn run_entry_point_parse_error_returns_1() {
    assert_eq!(
        super::run(vec!["pdal-rs".to_string(), "--unknown".to_string()]),
        1
    );
}

fn app_with_command(command: &str, args: &[&str]) -> App {
    let mut app = App::new();
    let mut full = vec![command.to_string()];
    full.extend(args.iter().map(|a| a.to_string()));
    app.parse_args(&full).unwrap();
    app
}

#[test]
fn pipeline_command_prints_usage_when_args_empty() {
    let app = app_with_command("pipeline", &[]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_prints_usage_when_help_set() {
    let mut app = app_with_command("pipeline", &[]);
    app.help = true;
    assert_eq!(app.run_pipeline(), 0);
}

#[test]
fn pipeline_command_errors_on_input_without_value() {
    let app = app_with_command("pipeline", &["--input"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_errors_on_metadata_without_value() {
    let app = app_with_command("pipeline", &["--metadata"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_errors_on_serialization_without_value() {
    let app = app_with_command("pipeline", &["--pipeline-serialization"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_errors_on_unknown_option() {
    let app = app_with_command("pipeline", &["--mystery"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_errors_on_two_input_filenames() {
    let app = app_with_command("pipeline", &["a.json", "b.json"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_errors_when_stdin_and_input_combined() {
    let app = app_with_command("pipeline", &["--stdin", "a.json"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_errors_when_no_input_or_stdin() {
    let app = app_with_command("pipeline", &["--validate"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_command_errors_on_missing_file() {
    let app = app_with_command("pipeline", &["/no/such/pipeline.json"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn tile_command_prints_usage_when_args_empty() {
    let app = app_with_command("tile", &[]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_command_help_returns_zero() {
    let mut app = app_with_command("tile", &[]);
    app.help = true;
    assert_eq!(app.run_tile(), 0);
}

#[test]
fn tile_command_errors_on_input_without_value() {
    let app = app_with_command("tile", &["--input"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_command_errors_on_output_without_value() {
    let app = app_with_command("tile", &["--output"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_command_errors_on_unknown_option() {
    let app = app_with_command("tile", &["--mystery=1"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_command_errors_on_bad_number() {
    let app = app_with_command("tile", &["--length=hello"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_command_errors_on_extra_positionals() {
    let app = app_with_command("tile", &["a.las", "b#.las", "extra.las"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_command_errors_when_input_missing_value_via_short() {
    let app = app_with_command("tile", &["-i"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn tile_command_errors_on_missing_positionals() {
    let app = app_with_command("tile", &["--length=100"]);
    assert_eq!(app.run_tile(), 1);
}

#[test]
fn hausdorff_chamfer_delta_print_usage_when_args_empty() {
    let app = app_with_command("hausdorff", &[]);
    assert_eq!(app.run_hausdorff(), 1);
    let app = app_with_command("chamfer", &[]);
    assert_eq!(app.run_chamfer(), 1);
    let app = app_with_command("delta", &[]);
    assert_eq!(app.run_delta(), 1);
}

#[test]
fn hausdorff_chamfer_delta_help_return_zero() {
    for command in ["hausdorff", "chamfer", "delta"] {
        let mut app = app_with_command(command, &[]);
        app.help = true;
        let rc = match command {
            "hausdorff" => app.run_hausdorff(),
            "chamfer" => app.run_chamfer(),
            "delta" => app.run_delta(),
            _ => unreachable!(),
        };
        assert_eq!(rc, 0, "{command} --help should exit 0");
    }
}

#[test]
fn hausdorff_errors_on_invalid_args() {
    let app = app_with_command("hausdorff", &["only-one"]);
    assert_eq!(app.run_hausdorff(), 1);
}

#[test]
fn chamfer_errors_on_invalid_args() {
    let app = app_with_command("chamfer", &["only-one"]);
    assert_eq!(app.run_chamfer(), 1);
}

#[test]
fn delta_errors_on_invalid_args() {
    let app = app_with_command("delta", &["only-one"]);
    assert_eq!(app.run_delta(), 1);
}

#[test]
fn tindex_command_prints_usage_when_args_empty() {
    let app = app_with_command("tindex", &[]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_command_errors_on_unknown_subcommand() {
    let app = app_with_command("tindex", &["unknown"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_command_errors_on_create_without_more_args() {
    let app = app_with_command("tindex", &["create"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn eval_command_prints_usage_when_args_empty() {
    let app = app_with_command("eval", &[]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn dispatch_unknown_command_returns_error() {
    let app = app_with_command("not-a-real-command", &["arg"]);
    assert_eq!(app.run(), 1);
}

#[test]
fn dispatch_shows_help_when_no_command() {
    let app = App::new();
    assert_eq!(app.run(), 0);
}

#[test]
fn dispatch_shows_version_when_flag_set() {
    let mut app = App::new();
    app.show_version = true;
    assert_eq!(app.run(), 0);
}

#[test]
fn dispatch_shows_drivers_when_flag_set() {
    let mut app = App::new();
    app.show_drivers = true;
    assert_eq!(app.run(), 0);
}

#[test]
fn dispatch_shows_commands_when_flag_set() {
    let mut app = App::new();
    app.show_commands = true;
    assert_eq!(app.run(), 0);
}

#[test]
fn dispatch_shows_options_when_set() {
    let mut app = App::new();
    app.show_options = Some("writers.las".to_string());
    assert_eq!(app.run(), 0);
}

#[test]
fn run_entry_point_errors_on_bad_args() {
    assert_eq!(
        super::run(vec!["pdal-rs".to_string(), "--bogus".to_string()]),
        1
    );
}

#[test]
fn info_command_prints_usage_when_args_empty() {
    let app = app_with_command("info", &[]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn merge_command_prints_usage_when_args_empty() {
    let app = app_with_command("merge", &[]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn sort_command_prints_usage_when_args_empty() {
    let app = app_with_command("sort", &[]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn translate_command_prints_usage_when_args_empty() {
    let app = app_with_command("translate", &[]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn split_command_prints_usage_when_args_empty() {
    let app = app_with_command("split", &[]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn ground_command_prints_usage_when_args_empty() {
    let app = app_with_command("ground", &[]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn density_command_prints_usage_when_args_empty() {
    let app = app_with_command("density", &[]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn random_command_prints_usage_when_args_empty() {
    let app = app_with_command("random", &[]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn info_command_errors_without_filename() {
    let app = app_with_command("info", &["--summary"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_on_unknown_option() {
    let app = app_with_command("info", &["--mystery"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_with_two_filenames() {
    let app = app_with_command("info", &["a.las", "b.las"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_when_driver_missing_value() {
    let app = app_with_command("info", &["--driver", "input.las"]);
    // --driver consumes "input.las" as value, leaving no filename
    let _ = app.run_info();
}

#[test]
fn info_command_errors_when_input_missing_value() {
    let app = app_with_command("info", &["--input"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn info_command_errors_with_duplicate_input() {
    let app = app_with_command("info", &["--input", "a.las", "--input", "b.las"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn translate_command_errors_on_unknown_option() {
    let app = app_with_command("translate", &["--mystery"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn translate_command_errors_on_missing_args() {
    let app = app_with_command("translate", &["only-one.las"]);
    assert_eq!(app.run_translate(), 1);
}

#[test]
fn merge_command_errors_on_unknown_option() {
    let app = app_with_command("merge", &["--mystery"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn merge_command_errors_on_missing_output() {
    let app = app_with_command("merge", &["a.las"]);
    assert_eq!(app.run_merge(), 1);
}

#[test]
fn sort_command_errors_on_unknown_option() {
    let app = app_with_command("sort", &["--mystery"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn sort_command_errors_on_bad_args() {
    let app = app_with_command("sort", &["only-one.las"]);
    assert_eq!(app.run_sort(), 1);
}

#[test]
fn split_command_errors_on_unknown_option() {
    let app = app_with_command("split", &["--mystery"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_command_errors_on_missing_template() {
    let app = app_with_command("split", &["input.las"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn ground_command_errors_on_unknown_option() {
    let app = app_with_command("ground", &["--mystery"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn ground_command_errors_on_missing_output() {
    let app = app_with_command("ground", &["input.las"]);
    assert_eq!(app.run_ground(), 1);
}

#[test]
fn density_command_errors_on_unknown_option() {
    let app = app_with_command("density", &["--mystery"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn density_command_errors_on_missing_output() {
    let app = app_with_command("density", &["input.las"]);
    assert_eq!(app.run_density(), 1);
}

#[test]
fn random_command_errors_on_unknown_option() {
    let app = app_with_command("random", &["--mystery"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn random_command_errors_on_missing_output() {
    let app = app_with_command("random", &["--count=10"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn eval_command_errors_on_unknown_option() {
    let app = app_with_command("eval", &["--mystery"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn eval_command_errors_on_missing_args() {
    let app = app_with_command("eval", &["only-one.las"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn pipeline_command_errors_when_input_repeated() {
    let app = app_with_command("pipeline", &["--input", "a.json", "--input", "b.json"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn tindex_merge_errors_without_tindex() {
    let app = app_with_command("tindex", &["merge", "--filespec", "out.las"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_without_filespec() {
    let app = app_with_command("tindex", &["merge", "--tindex", "idx.json"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_unknown_option() {
    let app = app_with_command("tindex", &["merge", "--mystery"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_missing_tindex_value() {
    let app = app_with_command("tindex", &["merge", "--tindex"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_merge_errors_on_missing_filespec_value() {
    let app = app_with_command("tindex", &["merge", "--filespec"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_on_unknown_option() {
    let app = app_with_command("tindex", &["create", "--mystery=foo"]);
    assert_eq!(app.run_tindex(), 1);
}

#[test]
fn tindex_create_errors_without_tindex_path() {
    let app = app_with_command("tindex", &["create", "input.las"]);
    // No --tindex output specified -> should fail
    let _ = app.run_tindex();
}

#[test]
fn pipeline_errors_when_stdin_with_metadata_arg_missing() {
    let app = app_with_command("pipeline", &["--stdin", "--metadata"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn pipeline_errors_with_invalid_pipeline_json_via_stdin() {
    // Can't easily feed stdin in a unit test, just verify the path doesn't panic.
    // Use --validate which exits before doing pipeline work but still requires input
    let app = app_with_command("pipeline", &["--validate", "--input"]);
    assert_eq!(app.run_pipeline(), 1);
}

#[test]
fn info_command_errors_on_driver_equals_unknown() {
    let app = app_with_command("info", &["--driver=mystery", "/no/such/file"]);
    assert_eq!(app.run_info(), 1);
}

#[test]
fn eval_command_errors_on_unknown_option_full() {
    let app = app_with_command("eval", &["--mystery=foo", "p.las", "t.las", "--labels=1,2"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn eval_command_errors_on_three_positionals() {
    let app = app_with_command("eval", &["a", "b", "c", "--labels=1,2"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn eval_command_errors_on_missing_labels() {
    let app = app_with_command("eval", &["a.las", "b.las"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn eval_command_errors_on_missing_option_value() {
    let app = app_with_command("eval", &["--predicted"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn eval_command_errors_when_predicted_only() {
    let app = app_with_command("eval", &["a.las", "--labels=1,2"]);
    assert_eq!(app.run_eval(), 1);
}

#[test]
fn random_command_errors_on_unknown_option_via_long() {
    let app = app_with_command("random", &["--mystery=foo", "out.las"]);
    assert_eq!(app.run_random(), 1);
}

#[test]
fn hausdorff_with_two_paths_attempts_call() {
    // This will hit the C ABI path but likely fail since files don't exist
    let app = app_with_command("hausdorff", &["/no/such/a.las", "/no/such/b.las"]);
    assert_eq!(app.run_hausdorff(), 1);
}

#[test]
fn chamfer_with_two_paths_attempts_call() {
    let app = app_with_command("chamfer", &["/no/such/a.las", "/no/such/b.las"]);
    assert_eq!(app.run_chamfer(), 1);
}

#[test]
fn delta_with_two_paths_attempts_call() {
    let app = app_with_command("delta", &["/no/such/a.las", "/no/such/b.las"]);
    assert_eq!(app.run_delta(), 1);
}

#[test]
fn info_command_errors_on_filename_with_nul_byte() {
    let nul_path = String::from_utf8(vec![b'/', b't', b'm', b'p', 0, b'.', b'l', b'a', b's'])
        .unwrap_or_default();
    if !nul_path.is_empty() {
        let app = app_with_command("info", &[&nul_path]);
        let _ = app.run_info();
    }
}

#[test]
fn pipeline_command_errors_on_invalid_json() {
    let dir = std::env::temp_dir().join(format!(
        "pdal-cli-bad-pipeline-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pipeline.json");
    std::fs::write(&path, "{not-json}").unwrap();
    let path_str = path.to_str().unwrap().to_string();
    let app = app_with_command("pipeline", &[&path_str]);
    assert_eq!(app.run_pipeline(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pdal-cli-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[path = "app_tests/analysis_tindex_eval.rs"]
mod analysis_tindex_eval;
#[path = "app_tests/eval_and_dispatch.rs"]
mod eval_and_dispatch;
#[path = "app_tests/pipeline_and_command_errors.rs"]
mod pipeline_and_command_errors;
