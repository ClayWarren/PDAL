use super::*;
use crate::kernel_abi::CliStageOption;
use command::{
    apply_stage_options_to_pipeline_json, validate_pipeline_for_kernel,
    validate_pipeline_json_shape,
};
use pdal_core::point::PointLayout;
use pdal_core::srs::SpatialReference;
use std::rc::Rc;

#[test]
fn applies_cli_stage_options_to_object_pipeline() {
    let json = r#"{"pipeline":[{"type":"readers.faux"},{"type":"filters.sort","dimension":"X"},{"type":"writers.las"}]}"#;
    let options = vec![CliStageOption {
        stage: "filters.sort".to_string(),
        key: "dimension".to_string(),
        value: "Y".to_string(),
    }];

    let updated = apply_stage_options_to_pipeline_json(json, &options).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&updated).unwrap();

    assert_eq!(parsed["pipeline"][1]["dimension"][0], "X");
    assert_eq!(parsed["pipeline"][1]["dimension"][1], "Y");
}

#[test]
fn applies_cli_stage_options_to_array_pipeline() {
    let json =
        r#"[{"type":"readers.faux"},{"type":"sort","dimension":"X"},{"type":"writers.las"}]"#;
    let options = vec![CliStageOption {
        stage: "sort".to_string(),
        key: "dimension".to_string(),
        value: "Y".to_string(),
    }];

    let updated = apply_stage_options_to_pipeline_json(json, &options).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&updated).unwrap();

    assert_eq!(parsed[1]["dimension"][0], "X");
    assert_eq!(parsed[1]["dimension"][1], "Y");
}

#[test]
fn validate_shape_accepts_object_valued_options() {
    let json = r#"[{"type":"readers.ept","filename":"ept.json"},{"type":"writers.ept_addon","addons":{"Z":"Z"}}]"#;

    assert!(validate_pipeline_json_shape(json).is_ok());
}

#[test]
fn validate_shape_rejects_non_stage_entries() {
    let json = r#"[{"type":"readers.faux"}, 7]"#;

    assert!(validate_pipeline_json_shape(json).is_err());
}

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
fn stac_report_uses_requested_pointcloud_type() {
    let layout = Rc::new(PointLayout::new());
    let mut view = PointView::new(layout);
    view.set_spatial_reference(SpatialReference::new("EPSG:4326"));
    view.add_point();

    let report = stac_report(&[view], &MetadataNode::new("root"), "sample.las", "sonar");
    let json: serde_json::Value = serde_json::from_str(&report).unwrap();

    assert_eq!(json["stac"]["properties"]["pc:type"], "sonar");
}
