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
