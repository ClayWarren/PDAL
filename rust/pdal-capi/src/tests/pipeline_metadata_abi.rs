use super::*;

#[test]
fn pipeline_result_roundtrips_through_c_abi() {
    unsafe {
        let json = CString::new(
            r#"[
                    {
                        "type":"readers.faux",
                        "count":3,
                        "mode":"ramp",
                        "minx":-10,
                        "maxx":20,
                        "miny":-15,
                        "maxy":7,
                        "minz":-50,
                        "maxz":100
                    }
                ]"#,
        )
        .unwrap();
        let pipeline = pdal_pipeline_create_json(json.as_ptr());
        assert!(!pipeline.is_null());

        let mut result = empty_pipeline_result();
        assert_eq!(
            pdal_pipeline_execute_result(pipeline, std::ptr::null_mut(), &mut result),
            0
        );

        assert_eq!(result.point_count, 3);
        assert_eq!(result.view_count, 1);
        assert!(result.has_bounds_2d);
        assert_eq!(
            result.bounds_2d,
            pdal_bounds2d_t {
                minx: -10.0,
                maxx: 20.0,
                miny: -15.0,
                maxy: 7.0,
            }
        );
        assert!(result.has_bounds_3d);
        assert_eq!(
            result.bounds_3d,
            pdal_bounds3d_t {
                minx: -10.0,
                maxx: 20.0,
                miny: -15.0,
                maxy: 7.0,
                minz: -50.0,
                maxz: 100.0,
            }
        );

        let summary: serde_json::Value = serde_json::from_str(&take_string(
            pdal_pipeline_execute_summary_json(pipeline, std::ptr::null_mut()),
        ))
        .unwrap();
        assert_eq!(summary["point_count"], 3);
        assert_eq!(summary["view_count"], 1);
        assert_eq!(summary["dimension_summaries"][0]["name"], "X");
        assert_eq!(summary["dimension_summaries"][0]["count"], 3);
        assert_eq!(summary["dimension_summaries"][0]["minimum"], -10.0);
        assert_eq!(summary["dimension_summaries"][0]["maximum"], 20.0);
        assert_eq!(summary["dimension_summaries"][0]["mean"], 5.0);
        // Flat metadata summary doesn't include the root "pipeline" node name as a field
        assert!(summary["metadata"].is_object());

        pdal_pipeline_destroy(pipeline);
    }
}

#[test]
fn pipeline_result_c_abi_rejects_missing_output() {
    unsafe {
        let pipeline = pdal_pipeline_create();
        assert_eq!(
            pdal_pipeline_execute_result(pipeline, std::ptr::null_mut(), std::ptr::null_mut()),
            -1
        );
        pdal_pipeline_destroy(pipeline);
    }
}

#[test]
fn metadata_tree_roundtrips_through_c_abi() {
    unsafe {
        let root_name = CString::new("root").unwrap();
        let child_name = CString::new("child").unwrap();
        let child_value = CString::new("value").unwrap();
        let child_type = CString::new("string").unwrap();
        let child_description = CString::new("child description").unwrap();

        let root = pdal_metadata_node_create(root_name.as_ptr());
        let child = pdal_metadata_node_create(child_name.as_ptr());
        pdal_metadata_node_set_string(child, child_value.as_ptr());
        pdal_metadata_node_set_type(child, child_type.as_ptr());
        pdal_metadata_node_set_description(child, child_description.as_ptr());
        pdal_metadata_node_add_child(root, child);

        assert_eq!(pdal_metadata_node_child_count(root), 1);
        assert_eq!(
            pdal_metadata_node_child_named_count(root, child_name.as_ptr()),
            1
        );
        let copied = pdal_metadata_node_child(root, 0);
        assert_eq!(take_string(pdal_metadata_node_name(copied)), "child");
        assert_eq!(take_string(pdal_metadata_node_type(copied)), "string");
        assert_eq!(
            take_string(pdal_metadata_node_description(copied)),
            "child description"
        );
        assert_eq!(take_string(pdal_metadata_node_value(copied)), "value");

        pdal_metadata_node_destroy(copied);

        let cloned_root = pdal_metadata_node_clone(root);
        let cloned_child = pdal_metadata_node_child(cloned_root, 0);
        assert_eq!(take_string(pdal_metadata_node_name(cloned_child)), "child");
        pdal_metadata_node_destroy(cloned_child);
        pdal_metadata_node_destroy(cloned_root);

        let named = pdal_metadata_node_child_named(root, child_name.as_ptr(), 0);
        assert_eq!(take_string(pdal_metadata_node_name(named)), "child");
        pdal_metadata_node_destroy(named);

        pdal_metadata_node_destroy(root);
    }
}

#[test]
fn metadata_add_or_update_replaces_child_subtree_through_c_abi() {
    unsafe {
        let root_name = CString::new("root").unwrap();
        let child_name = CString::new("child").unwrap();
        let old_name = CString::new("old").unwrap();
        let new_name = CString::new("new").unwrap();

        let root = pdal_metadata_node_create(root_name.as_ptr());
        let child = pdal_metadata_node_create(child_name.as_ptr());
        let old = pdal_metadata_node_create(old_name.as_ptr());
        pdal_metadata_node_set_u64(old, 1);
        pdal_metadata_node_add_child(child, old);
        pdal_metadata_node_add_child_clone(root, child);

        let replacement = pdal_metadata_node_create(child_name.as_ptr());
        let new_child = pdal_metadata_node_create(new_name.as_ptr());
        pdal_metadata_node_set_u64(new_child, 2);
        pdal_metadata_node_add_child(replacement, new_child);
        pdal_metadata_node_add_or_update_child_clone(root, replacement);

        assert_eq!(pdal_metadata_node_child_count(root), 1);
        let copied = pdal_metadata_node_child_named(root, child_name.as_ptr(), 0);
        assert_eq!(pdal_metadata_node_child_count(copied), 1);
        let grandchild = pdal_metadata_node_child(copied, 0);
        assert_eq!(take_string(pdal_metadata_node_name(grandchild)), "new");
        assert_eq!(pdal_metadata_node_value_u64(grandchild), 2);

        pdal_metadata_node_destroy(grandchild);
        pdal_metadata_node_destroy(copied);
        pdal_metadata_node_destroy(replacement);
        pdal_metadata_node_destroy(child);
        pdal_metadata_node_destroy(root);
    }
}

#[test]
fn metadata_numeric_values_roundtrip_through_c_abi() {
    unsafe {
        let node_name = CString::new("count").unwrap();
        let node = pdal_metadata_node_create(node_name.as_ptr());
        pdal_metadata_node_set_u64(node, 42);

        assert_eq!(pdal_metadata_node_value_kind(node), 2);
        assert_eq!(pdal_metadata_node_value_u64(node), 42);
        assert_eq!(take_string(pdal_metadata_node_value(node)), "42");

        pdal_metadata_node_destroy(node);
    }
}

#[test]
fn metadata_tree_serializes_to_json_through_c_abi() {
    unsafe {
        let root_name = CString::new("root").unwrap();
        let child_name = CString::new("count").unwrap();
        let valid_name = CString::new("valid").unwrap();

        let root = pdal_metadata_node_create(root_name.as_ptr());
        let child = pdal_metadata_node_create(child_name.as_ptr());
        pdal_metadata_node_set_u64(child, 42);
        pdal_metadata_node_add_child(root, child);

        let valid = pdal_metadata_node_create(valid_name.as_ptr());
        pdal_metadata_node_set_bool(valid, true);
        pdal_metadata_node_add_child(root, valid);

        let json: serde_json::Value =
            serde_json::from_str(&take_string(pdal_metadata_node_to_json(root))).unwrap();
        assert_eq!(json["name"], "root");
        assert_eq!(json["children"][0]["name"], "count");
        assert_eq!(json["children"][0]["value"], 42);
        assert_eq!(json["children"][0]["value_type"], "u64");
        assert_eq!(json["children"][1]["name"], "valid");
        assert_eq!(json["children"][1]["value"], true);
        assert_eq!(json["children"][1]["value_type"], "bool");

        pdal_metadata_node_destroy(root);
    }
}

#[test]
fn spatial_reference_exports_metadata() {
    unsafe {
        let text = CString::new("EPSG:4326").unwrap();
        let srs = pdal_spatial_reference_create_with_epoch(text.as_ptr(), 2020.0);
        let metadata = pdal_spatial_reference_to_metadata(srs);

        assert_eq!(take_string(pdal_metadata_node_name(metadata)), "srs");
        assert_eq!(pdal_metadata_node_child_count(metadata), 2);

        let wkt = pdal_metadata_node_child(metadata, 0);
        assert_eq!(take_string(pdal_metadata_node_name(wkt)), "wkt");
        assert_eq!(take_string(pdal_metadata_node_value(wkt)), "EPSG:4326");

        pdal_metadata_node_destroy(wkt);
        pdal_metadata_node_destroy(metadata);
        pdal_spatial_reference_destroy(srs);
    }
}
