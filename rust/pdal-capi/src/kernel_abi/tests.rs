use super::*;
use std::ffi::CString;

#[test]
fn rust_kernel_run_reports_unsupported_kernels() {
    let name = CString::new("kernels.missing").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, -1);
}

#[test]
fn cli_stage_options_preserve_repeated_values() {
    let mut stages = vec![serde_json::json!({ "type": "filters.returns" })];
    let options = vec![
        CliStageOption {
            stage: "filters.returns".to_string(),
            key: "groups".to_string(),
            value: "last".to_string(),
        },
        CliStageOption {
            stage: "filters.returns".to_string(),
            key: "groups".to_string(),
            value: "first".to_string(),
        },
    ];

    assert!(apply_cli_stage_options(&mut stages, &options));
    assert_eq!(stages[0]["groups"], serde_json::json!(["last", "first"]));
}

#[test]
fn rust_kernel_run_dispatches_fauxplugin() {
    let name = CString::new("fauxplugin").unwrap();
    let arg = CString::new("7").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, 0);
}

#[test]
fn rust_kernel_run_requires_fauxplugin_arg() {
    let name = CString::new("kernels.fauxplugin").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_density_missing_input() {
    let name = CString::new("density").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_metric_missing_inputs() {
    for command in ["hausdorff", "chamfer", "delta", "eval"] {
        let name = CString::new(command).unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_eq!(result, 1);
    }
}

#[test]
fn rust_kernel_run_accepts_metric_help() {
    for command in ["hausdorff", "chamfer", "delta", "eval"] {
        let name = CString::new(command).unwrap();
        let arg = CString::new("--help").unwrap();
        let argv = [arg.as_ptr()];

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

        assert_eq!(result, 0);
    }
}

#[test]
fn rust_kernel_run_rejects_metric_unknown_option() {
    let name = CString::new("chamfer").unwrap();
    let arg = CString::new("--mystery").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_rejects_eval_without_labels() {
    let name = CString::new("eval").unwrap();
    let predicted = CString::new("predicted.las").unwrap();
    let truth = CString::new("truth.las").unwrap();
    let argv = [predicted.as_ptr(), truth.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 2, argv.as_ptr()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_ground_missing_input() {
    let name = CString::new("ground").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_sort_missing_input() {
    let name = CString::new("sort").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_split_missing_input() {
    let name = CString::new("split").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_merge_missing_files() {
    let name = CString::new("merge").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_tile_missing_input() {
    let name = CString::new("tile").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_translate_missing_input() {
    let name = CString::new("translate").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_declines_translate_option_file() {
    let name = CString::new("translate").unwrap();
    let arg = CString::new("--filters.range.option_file=opts.json").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, -1);
}

#[test]
fn rust_kernel_run_declines_translate_range_filter() {
    let name = CString::new("translate").unwrap();
    let arg = CString::new("filters.range").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, -1);
}

#[test]
fn rust_kernel_run_reports_random_missing_output() {
    let name = CString::new("random").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_pipeline_missing_input() {
    let name = CString::new("pipeline").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_accepts_info_help() {
    let name = CString::new("info").unwrap();
    let arg = CString::new("--help").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, 0);
}

#[test]
fn rust_kernel_run_declines_default_info_mode() {
    let name = CString::new("info").unwrap();
    let arg = CString::new("input.las").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, -1);
}

#[test]
fn rust_kernel_run_declines_rich_info_options() {
    let name = CString::new("info").unwrap();
    let arg = CString::new("--schema").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, -1);
}
