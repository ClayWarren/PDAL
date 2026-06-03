use super::*;
use command::{progress_file_targets, validate_pipeline_for_kernel};

#[test]
fn validate_pipeline_reports_actual_streamability() {
    let streamable = r#"{"pipeline":[
        {"type":"readers.faux","count":10,"mode":"ramp"},
        {"type":"filters.range","limits":"X[0:5]"},
        {"type":"writers.null"}
    ]}"#;
    let nonstreamable = r#"{"pipeline":[
        {"type":"readers.faux","count":10,"mode":"ramp"},
        {"type":"filters.sort","dimension":"X"},
        {"type":"writers.null"}
    ]}"#;

    assert_eq!(validate_pipeline_for_kernel(streamable)["streamable"], true);
    assert_eq!(
        validate_pipeline_for_kernel(nonstreamable)["streamable"],
        false
    );
}

#[test]
fn validate_pipeline_reports_writerless_streamable_chain() {
    let writerless = r#"{"pipeline":[
        {"type":"readers.faux","count":10,"mode":"ramp"},
        {"type":"filters.range","limits":"X[0:5]"}
    ]}"#;

    let validation = validate_pipeline_for_kernel(writerless);
    assert_eq!(validation["valid"], true);
    assert_eq!(validation["streamable"], true);
}

#[test]
fn validate_pipeline_rejects_non_reader_roots_even_when_a_reader_exists() {
    let invalid = r#"{"pipeline":[
        {"type":"writers.null", "tag":"W"},
        {"type":"readers.faux", "tag":"R", "count":1}
    ]}"#;

    let validation = validate_pipeline_for_kernel(invalid);
    assert_eq!(validation["valid"], false);
    assert!(validation["error_detail"]
        .as_str()
        .unwrap()
        .contains("start with a reader"));
}

#[test]
fn validate_pipeline_rejects_prepare_layout_errors() {
    let invalid = r#"{"pipeline":[
        {"type":"readers.faux","count":5,"mode":"ramp"},
        {"type":"filters.assign","assignment":"Classification[:]=2"},
        {"type":"writers.null"}
    ]}"#;

    let validation = validate_pipeline_for_kernel(invalid);
    assert_eq!(validation["valid"], false);
    assert_eq!(validation["streamable"], false);
    assert!(validation["error_detail"]
        .as_str()
        .unwrap()
        .contains("Invalid dimension name"));
}

#[test]
fn validate_pipeline_allows_unknown_reader_layouts() {
    let unknown_layout = r#"{"pipeline":[
        {"type":"readers.text","filename":"missing.txt","header":"X,Y,Z"},
        {"type":"filters.assign","assignment":"Classification[:]=2"},
        {"type":"writers.null"}
    ]}"#;

    let validation = validate_pipeline_for_kernel(unknown_layout);
    assert_eq!(validation["valid"], true);
}

#[test]
fn progress_targets_are_writer_filenames() {
    let json = r#"{"pipeline":[
        "input.las",
        {"type":"filters.sort", "dimension":"X"},
        "output.laz"
    ]}"#;
    assert_eq!(progress_file_targets(json), vec!["output.laz".to_string()]);

    let typed_writers = r#"{"pipeline":[
        {"type":"readers.faux", "count":1},
        {"type":"writers.las", "filename":"output.laz"},
        {"type":"writers.text", "filename":"summary.txt"}
    ]}"#;
    assert_eq!(
        progress_file_targets(typed_writers),
        vec!["output.laz".to_string(), "summary.txt".to_string()]
    );
}

#[test]
fn progress_targets_ignore_writerless_pipelines() {
    let json = r#"{"pipeline":[
        {"type":"readers.faux", "count":1},
        {"type":"filters.decimation", "step":2}
    ]}"#;

    assert!(progress_file_targets(json).is_empty());
}
