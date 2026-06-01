use super::*;
use command::validate_pipeline_for_kernel;
use pdal_core::metadata::MetadataNode;
use pdal_core::point::{PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use std::rc::Rc;

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

    let report = info::stac_report(&[view], &MetadataNode::new("root"), "sample.las", "sonar");
    let json: serde_json::Value = serde_json::from_str(&report).unwrap();

    assert_eq!(json["stac"]["properties"]["pc:type"], "sonar");
}
