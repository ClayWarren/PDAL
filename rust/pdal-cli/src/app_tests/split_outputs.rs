use super::*;

// SplitArgs::parse error branches via run_split

#[test]
fn split_errors_on_length_no_value() {
    let app = app_with_command("split", &["--length"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_length_eq_bad_value() {
    let app = app_with_command("split", &["--length=notanumber", "in.las", "out.las"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_length_bad_value_space() {
    let app = app_with_command("split", &["--length", "notanumber", "in.las", "out.las"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_capacity_no_value() {
    let app = app_with_command("split", &["--capacity"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_capacity_bad_value() {
    let app = app_with_command("split", &["--capacity", "notanumber", "in.las", "out.las"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_origin_x_no_value() {
    let app = app_with_command("split", &["--origin_x"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_origin_x_bad_value() {
    let app = app_with_command("split", &["--origin_x", "notanumber", "in.las", "out.las"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_origin_y_no_value() {
    let app = app_with_command("split", &["--origin_y"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_origin_y_bad_value() {
    let app = app_with_command("split", &["--origin_y", "notanumber", "in.las", "out.las"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_input_no_value() {
    let app = app_with_command("split", &["--input"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_output_no_value() {
    let app = app_with_command("split", &["--output"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_driver_no_value() {
    let app = app_with_command("split", &["--driver"]);
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_accepts_driver_equals_form() {
    let app = app_with_command(
        "split",
        &["--driver=readers.las", "/no/such/a.las", "/tmp/out.las"],
    );
    // Reader infer would fail but driver is provided; pipeline will fail.
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_length_and_capacity_combined() {
    let app = app_with_command(
        "split",
        &["--length=10", "--capacity=100", "in.las", "out.las"],
    );
    assert_eq!(app.run_split(), 1);
}

#[test]
fn split_errors_on_origin_without_length() {
    let app = app_with_command("split", &["--origin_x=0", "in.las", "out.las"]);
    assert_eq!(app.run_split(), 1);
}

// numbered_output: extension-less path
#[test]
fn numbered_output_handles_no_extension() {
    let p = std::path::Path::new("/tmp/foo");
    let result = numbered_output(p, 3);
    assert_eq!(result, std::path::PathBuf::from("/tmp/foo_3"));
}

#[test]
fn numbered_output_handles_extension() {
    let p = std::path::Path::new("/tmp/foo.las");
    let result = numbered_output(p, 7);
    assert_eq!(result, std::path::PathBuf::from("/tmp/foo_7.las"));
}

#[test]
fn split_output_path_keeps_filename_when_input_has_no_separator() {
    // Trailing separator -> use input file_name appended to dir
    let sep = std::path::MAIN_SEPARATOR.to_string();
    let out = format!("/tmp/dirout{sep}");
    let result = split_output_path("/data/in.las", &out);
    let expected = std::path::PathBuf::from(format!("/tmp/dirout{sep}in.las"));
    assert_eq!(result, expected);
}

#[test]
fn parse_stage_option_arg_rejects_no_dash() {
    assert!(parse_stage_option_arg("filters.foo.bar=baz").is_err());
}

#[test]
fn parse_stage_option_arg_rejects_no_dot() {
    assert!(parse_stage_option_arg("--noequals").is_err());
}

#[test]
fn run_entry_point_parse_error_returns_1() {
    // Pass an unexpected root argument to trigger parse_args error in run.
    assert_eq!(
        super::run(vec!["pdal-rs".to_string(), "--unknown".to_string()]),
        1
    );
}
