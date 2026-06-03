use super::*;
use command::validate_pipeline_for_kernel;

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
