use super::translate::{expand_translate_option_files, parse_option_file, translate_json_stages};
use super::*;
use std::ffi::{CStr, CString};

#[test]
fn rust_kernel_run_reports_unsupported_kernels() {
    let name = CString::new("kernels.missing").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, -1);
}

#[test]
fn rust_kernel_list_json_lists_command_metadata() {
    let ptr = pdal_rust_kernel_list_json();
    assert!(!ptr.is_null());

    let text = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    let kernels: serde_json::Value = serde_json::from_str(&text).unwrap();
    let names: Vec<_> = kernels
        .as_array()
        .unwrap()
        .iter()
        .map(|kernel| kernel["name"].as_str().unwrap())
        .collect();

    assert_eq!(
        names,
        vec![
            "chamfer",
            "delta",
            "density",
            "eval",
            "fauxplugin",
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
        ]
    );
}

#[test]
fn rust_kernel_dispatch_recognizes_all_cpp_kernel_names() {
    for kernel in [
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
        let name = CString::new(kernel).unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_ne!(result, -1, "{kernel} should be Rust-dispatchable");
    }
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
fn rust_kernel_run_enforces_pipeline_stream_options() {
    let dir = std::env::temp_dir().join(format!(
        "pdal-rs-kernel-pipeline-stream-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let streamable = dir.join("streamable.json");
    let nonstreamable = dir.join("nonstreamable.json");
    std::fs::write(
        &streamable,
        r#"{"pipeline":[
            {"type":"readers.faux","count":10,"mode":"ramp"},
            {"type":"filters.range","limits":"X[0:9]"},
            {"type":"writers.null"}
        ]}"#,
    )
    .unwrap();
    std::fs::write(
        &nonstreamable,
        r#"{"pipeline":[
            {"type":"readers.faux","count":10,"mode":"ramp"},
            {"type":"filters.sort","dimension":"X"},
            {"type":"writers.null"}
        ]}"#,
    )
    .unwrap();

    let run = |path: &std::path::Path, extra: &[&str]| -> i32 {
        let name = CString::new("pipeline").unwrap();
        let mut owned = vec![CString::new(path.to_str().unwrap()).unwrap()];
        for arg in extra {
            owned.push(CString::new(*arg).unwrap());
        }
        let argv: Vec<_> = owned.iter().map(|arg| arg.as_ptr()).collect();
        unsafe { pdal_rust_kernel_run(name.as_ptr(), argv.len() as i32, argv.as_ptr()) }
    };

    assert_eq!(run(&streamable, &["--stream"]), 0);
    assert_eq!(run(&streamable, &["--nostream"]), 0);
    assert_eq!(run(&nonstreamable, &["--stream"]), 1);
    assert_eq!(run(&streamable, &["--stream", "--nostream"]), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn density_pipeline_json_appends_hexbin_stage() {
    let pipeline = r#"{"pipeline":[{"type":"readers.faux","count":5}]}"#;
    let value = append_stage_to_pipeline_json(
        pipeline,
        serde_json::json!({
            "type": "filters.hexbin",
            "density": "density.geojson",
        }),
    )
    .unwrap();

    let stages = value["pipeline"].as_array().unwrap();
    assert_eq!(stages.len(), 2);
    assert_eq!(stages[1]["type"], "filters.hexbin");
    assert_eq!(stages[1]["density"], "density.geojson");
}

#[test]
fn density_pipeline_json_rejects_missing_pipeline_array() {
    let err = append_stage_to_pipeline_json(
        r#"{"type":"readers.faux"}"#,
        serde_json::json!({"type":"filters.hexbin"}),
    )
    .err()
    .unwrap();

    assert!(err.contains("pipeline"));
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
fn rust_kernel_run_rejects_ground_unknown_option() {
    let name = CString::new("ground").unwrap();
    let input = CString::new("in.las").unwrap();
    let output = CString::new("out.las").unwrap();
    let bogus = CString::new("--bogus").unwrap();
    let argv = [input.as_ptr(), output.as_ptr(), bogus.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), argv.len() as i32, argv.as_ptr()) };

    // Unknown options now error in the Rust runner instead of returning the -1
    // C++ fallback sentinel.
    assert_eq!(result, 1);
}

/// End-to-end check of the GroundKernel option mapping the Rust runner now owns.
/// A plain run keeps every point; `--extract` inserts the filters.range
/// "Classification[2:2]" stage and yields only the ground subset.
#[test]
fn ground_kernel_basic_and_extract_option_mapping() {
    use crate::metrics_abi::read_cloud;
    use pdal_core::point::DimId;

    let input = "../../test/data/las/interesting.las";
    let dir = std::env::temp_dir();
    let basic_out = dir.join(format!("pdal-rs-ground-basic-{}.las", std::process::id()));
    let extract_out = dir.join(format!("pdal-rs-ground-extract-{}.las", std::process::id()));
    let _ = std::fs::remove_file(&basic_out);
    let _ = std::fs::remove_file(&extract_out);

    let run = |out: &std::path::Path, extra: &[&str]| -> i32 {
        let name = CString::new("ground").unwrap();
        let mut owned = vec![
            CString::new(input).unwrap(),
            CString::new(out.to_str().unwrap()).unwrap(),
            CString::new("--cell_size=10").unwrap(),
        ];
        for arg in extra {
            owned.push(CString::new(*arg).unwrap());
        }
        let argv: Vec<_> = owned.iter().map(|c| c.as_ptr()).collect();
        unsafe { pdal_rust_kernel_run(name.as_ptr(), argv.len() as i32, argv.as_ptr()) }
    };

    assert_eq!(run(&basic_out, &[]), 0);
    assert_eq!(run(&extract_out, &["--extract"]), 0);

    let basic = read_cloud(basic_out.to_str().unwrap()).unwrap();
    let extract = read_cloud(extract_out.to_str().unwrap()).unwrap();

    // interesting.las has 1065 points; a plain ground run keeps them all.
    assert_eq!(basic.len(), 1065);
    // --extract keeps only the ground (Classification 2) subset: fewer points,
    // and every surviving point is classified ground.
    assert!(!extract.is_empty() && extract.len() < basic.len());
    for i in 0..extract.len() {
        assert_eq!(extract.get_f64(i, &DimId::Classification), 2.0);
    }

    let _ = std::fs::remove_file(&basic_out);
    let _ = std::fs::remove_file(&extract_out);
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
fn translate_json_replaces_pipeline_reader_and_writer() {
    let json = r#"{
        "pipeline": [
            { "type": "readers.las", "filename": "old-input.las" },
            { "type": "filters.range", "limits": "Z[0:100]" },
            { "type": "writers.las", "filename": "old-output.las" }
        ]
    }"#;

    let stages = translate_json_stages(
        json,
        "new-input.las",
        "new-output.las",
        "readers.las",
        "writers.las",
    )
    .unwrap();

    assert_eq!(stages[0]["filename"], "new-input.las");
    assert_eq!(stages[1]["type"], "filters.range");
    assert_eq!(stages[2]["filename"], "new-output.las");
}

#[test]
fn translate_json_wraps_filter_only_pipeline() {
    let json = r#"{"pipeline":[{"type":"filters.stats"}]}"#;

    let stages =
        translate_json_stages(json, "in.las", "out.las", "readers.las", "writers.las").unwrap();

    assert_eq!(stages[0]["type"], "readers.las");
    assert_eq!(stages[1]["type"], "filters.stats");
    assert_eq!(stages[2]["type"], "writers.las");
}

#[test]
fn translate_option_file_expands_command_options() {
    let options = vec![CliStageOption {
        stage: "filters.range".to_string(),
        key: "option_file".to_string(),
        value: "../../test/data/apps/good_cmd_opt".to_string(),
    }];

    let expanded = expand_translate_option_files(options).unwrap();

    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].stage, "filters.range");
    assert_eq!(expanded[0].key, "limits");
    assert_eq!(expanded[0].value, "Classification[0:3]");
}

#[test]
fn translate_option_file_expands_json_options() {
    let options = vec![CliStageOption {
        stage: "filters.range".to_string(),
        key: "option_file".to_string(),
        value: "../../test/data/apps/good_json_opt".to_string(),
    }];

    let expanded = expand_translate_option_files(options).unwrap();

    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].stage, "filters.range");
    assert_eq!(expanded[0].key, "limits");
    assert_eq!(expanded[0].value, "Classification[0:3]");
}

#[test]
fn translate_option_file_rejects_unknown_option() {
    let err = parse_option_file("filters.range", "--foobar=Classification[0:3]").unwrap_err();

    assert_eq!(err, "Unexpected argument");
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
fn rust_kernel_run_accepts_tindex_help() {
    let name = CString::new("tindex").unwrap();
    let arg = CString::new("--help").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, 0);
}

#[test]
fn rust_kernel_run_reports_tindex_missing_subcommand() {
    let name = CString::new("tindex").unwrap();

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

    assert_eq!(result, 1);
}

/// Rich-boundary tindex options used to fall back to C++. The Rust hexer
/// port now handles them, so we expect a real attempt (and a 1 from the
/// missing input file) rather than the legacy -1 Unsupported sentinel.
#[test]
fn rust_kernel_run_handles_tindex_rich_boundary_options() {
    let name = CString::new("tindex").unwrap();
    let create = CString::new("create").unwrap();
    let tindex = CString::new("--tindex").unwrap();
    let output = CString::new("out.geojson").unwrap();
    let threshold = CString::new("--threshold=1").unwrap();
    let file = CString::new("input.las").unwrap();
    let argv = [
        create.as_ptr(),
        tindex.as_ptr(),
        output.as_ptr(),
        threshold.as_ptr(),
        file.as_ptr(),
    ];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), argv.len() as i32, argv.as_ptr()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_handles_tindex_srs_options() {
    let name = CString::new("tindex").unwrap();
    let create = CString::new("create").unwrap();
    let tindex = CString::new("--tindex").unwrap();
    let output = CString::new("out.geojson").unwrap();
    let target_srs = CString::new("--t_srs=EPSG:3857").unwrap();
    let assign_srs = CString::new("--a_srs").unwrap();
    let assign_value = CString::new("EPSG:26915").unwrap();
    let file = CString::new("input.las").unwrap();
    let argv = [
        create.as_ptr(),
        tindex.as_ptr(),
        output.as_ptr(),
        target_srs.as_ptr(),
        assign_srs.as_ptr(),
        assign_value.as_ptr(),
        file.as_ptr(),
    ];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), argv.len() as i32, argv.as_ptr()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_handles_tindex_merge_polygon_options() {
    let name = CString::new("tindex").unwrap();
    let merge = CString::new("merge").unwrap();
    let tindex = CString::new("--tindex").unwrap();
    let index = CString::new("missing.geojson").unwrap();
    let filespec = CString::new("--filespec").unwrap();
    let output = CString::new("out.las").unwrap();
    let polygon = CString::new("--polygon").unwrap();
    let wkt = CString::new("POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))").unwrap();
    let target_srs = CString::new("--t_srs=EPSG:3857").unwrap();
    let argv = [
        merge.as_ptr(),
        tindex.as_ptr(),
        index.as_ptr(),
        filespec.as_ptr(),
        output.as_ptr(),
        polygon.as_ptr(),
        wkt.as_ptr(),
        target_srs.as_ptr(),
    ];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), argv.len() as i32, argv.as_ptr()) };

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
fn rust_kernel_run_reports_default_info_missing_file() {
    let name = CString::new("info").unwrap();
    let arg = CString::new("input.las").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, 1);
}

#[test]
fn rust_kernel_run_reports_rich_info_missing_input() {
    let name = CString::new("info").unwrap();
    let arg = CString::new("--schema").unwrap();
    let argv = [arg.as_ptr()];

    let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

    assert_eq!(result, 1);
}
