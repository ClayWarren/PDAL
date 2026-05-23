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
fn pipeline_management_c_abi_covers_tags_and_empty_execution() {
    unsafe {
        let stage_name = CString::new("filters.decimation").unwrap();
        let explicit = CString::new("").unwrap();
        let existing = CString::new("decimation").unwrap();
        let existing_tags = [existing.as_ptr()];
        assert_eq!(
            take_string(pdal_pipeline_generate_stage_tag(
                stage_name.as_ptr(),
                explicit.as_ptr(),
                existing_tags.as_ptr(),
                existing_tags.len() as u64,
            )),
            "filters_decimation1"
        );
        assert!(pdal_pipeline_generate_stage_tag(
            stage_name.as_ptr(),
            explicit.as_ptr(),
            std::ptr::null(),
            1,
        )
        .is_null());

        let pipeline = pdal_pipeline_create();
        assert_eq!(pdal_pipeline_stage_count(pipeline), 0);
        assert_eq!(pdal_pipeline_stage_count(std::ptr::null()), 0);
        assert!(pdal_pipeline_execute(std::ptr::null_mut(), std::ptr::null_mut()).is_null());
        assert_eq!(
            pdal_pipeline_execute_count(std::ptr::null_mut(), std::ptr::null_mut()),
            -1
        );
        assert!(
            pdal_pipeline_execute_summary_json(std::ptr::null_mut(), std::ptr::null_mut())
                .is_null()
        );
        assert!(pdal_pipeline_metadata(std::ptr::null()).is_null());
        assert_eq!(
            pdal_pipeline_find_by_tag(std::ptr::null(), existing.as_ptr()),
            -1
        );
        assert_eq!(pdal_pipeline_find_by_tag(pipeline, std::ptr::null()), -1);
        assert_eq!(pdal_pipeline_add_dependency(std::ptr::null_mut(), 0, 0), -1);
        assert_eq!(pdal_pipeline_add_dependency(pipeline, 1, 0), -1);
        assert_eq!(
            pdal_pipeline_add_stage(std::ptr::null_mut(), std::ptr::null_mut()),
            -1
        );
        assert_eq!(
            pdal_pipeline_add_reader(std::ptr::null_mut(), std::ptr::null_mut()),
            -1
        );
        assert_eq!(
            pdal_pipeline_add_writer(std::ptr::null_mut(), std::ptr::null_mut()),
            -1
        );

        let metadata = pdal_pipeline_metadata(pipeline);
        assert!(!metadata.is_null());
        assert_eq!(take_string(pdal_metadata_node_name(metadata)), "pipeline");
        pdal_metadata_node_destroy(metadata);

        let options = pdal_options_create();
        let count = CString::new("count").unwrap();
        let three = CString::new("3").unwrap();
        pdal_options_add_str(options, count.as_ptr(), three.as_ptr());
        let stage = pdal_stage_create_decimation(options);
        let tag = CString::new("keep").unwrap();
        assert_eq!(
            pdal_pipeline_add_stage_tagged(pipeline, stage, tag.as_ptr()),
            0
        );
        assert_eq!(pdal_pipeline_find_by_tag(pipeline, tag.as_ptr()), 0);
        assert_eq!(
            pdal_pipeline_add_stage_tagged(pipeline, std::ptr::null_mut(), tag.as_ptr()),
            -1
        );

        pdal_options_destroy(options);
        pdal_pipeline_destroy(pipeline);
        pdal_pipeline_destroy(std::ptr::null_mut());
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

        let child_path = CString::new("child").unwrap();
        let path_child = pdal_metadata_node_find_child_path(root, child_path.as_ptr());
        assert_eq!(take_string(pdal_metadata_node_name(path_child)), "child");
        pdal_metadata_node_destroy(path_child);
        assert!(pdal_metadata_node_find_child_path(root, std::ptr::null()).is_null());

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
fn metadata_scalar_conversions_and_nulls_follow_c_abi_contract() {
    unsafe {
        let empty = pdal_metadata_node_create(std::ptr::null());
        assert_eq!(take_string(pdal_metadata_node_name(empty)), "");
        assert_eq!(take_string(pdal_metadata_node_type(empty)), "");
        assert_eq!(take_string(pdal_metadata_node_description(empty)), "");
        assert_eq!(pdal_metadata_node_value_kind(empty), 255);
        assert_eq!(pdal_metadata_node_child_count(std::ptr::null()), 0);
        assert!(pdal_metadata_node_clone(std::ptr::null()).is_null());
        assert!(pdal_metadata_node_child(empty, 0).is_null());
        assert!(pdal_metadata_node_child_named(empty, std::ptr::null(), 0).is_null());

        let type_name = CString::new("integer").unwrap();
        let value = CString::new("-7").unwrap();
        let mut out_i64 = 0;
        assert!(pdal_metadata_value_as_i64(
            type_name.as_ptr(),
            value.as_ptr(),
            &mut out_i64
        ));
        assert_eq!(out_i64, -7);

        let type_name = CString::new("unsigned").unwrap();
        let value = CString::new("7").unwrap();
        let mut out_u64 = 0;
        assert!(pdal_metadata_value_as_u64(
            type_name.as_ptr(),
            value.as_ptr(),
            &mut out_u64
        ));
        assert_eq!(out_u64, 7);

        let type_name = CString::new("double").unwrap();
        let value = CString::new("3.5").unwrap();
        let mut out_f64 = 0.0;
        assert!(pdal_metadata_value_as_f64(
            type_name.as_ptr(),
            value.as_ptr(),
            &mut out_f64
        ));
        assert_eq!(out_f64, 3.5);

        let type_name = CString::new("boolean").unwrap();
        let value = CString::new("true").unwrap();
        let mut out_bool = false;
        assert!(pdal_metadata_value_as_bool(
            type_name.as_ptr(),
            value.as_ptr(),
            &mut out_bool
        ));
        assert!(out_bool);
        assert_eq!(
            take_string(pdal_metadata_json_value(type_name.as_ptr(), value.as_ptr())),
            "true"
        );

        pdal_metadata_node_set_i64(empty, -2);
        assert_eq!(pdal_metadata_node_value_i64(empty), -2);
        pdal_metadata_node_set_f64(empty, 2.5);
        assert_eq!(pdal_metadata_node_value_f64(empty), 2.5);
        pdal_metadata_node_set_bool(empty, true);
        assert!(pdal_metadata_node_value_bool(empty));
        pdal_metadata_node_set_type(empty, std::ptr::null());
        pdal_metadata_node_set_description(empty, std::ptr::null());

        assert!(!pdal_metadata_value_as_i64(
            CString::new("integer").unwrap().as_ptr(),
            CString::new("nope").unwrap().as_ptr(),
            std::ptr::null_mut(),
        ));
        pdal_metadata_node_add_child_clone(std::ptr::null_mut(), empty);
        pdal_metadata_node_add_or_update_child_clone(std::ptr::null_mut(), empty);
        pdal_metadata_node_add_child(std::ptr::null_mut(), std::ptr::null_mut());
        pdal_metadata_node_add_or_update_child(std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(
            take_string(pdal_metadata_node_to_json(std::ptr::null())),
            "null"
        );

        pdal_metadata_node_destroy(empty);
        pdal_metadata_node_destroy(std::ptr::null_mut());
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

#[test]
fn test_metadata_abi_nulls_and_errors() {
    unsafe {
        // 1. Check null handling on metadata functions
        assert_eq!(take_string(pdal_metadata_node_name(std::ptr::null())), "");
        assert_eq!(take_string(pdal_metadata_node_type(std::ptr::null())), "");
        assert_eq!(
            take_string(pdal_metadata_node_description(std::ptr::null())),
            ""
        );

        pdal_metadata_node_set_string(std::ptr::null_mut(), std::ptr::null());
        pdal_metadata_node_set_type(std::ptr::null_mut(), std::ptr::null());
        pdal_metadata_node_set_description(std::ptr::null_mut(), std::ptr::null());
        pdal_metadata_node_set_i64(std::ptr::null_mut(), 0);
        pdal_metadata_node_set_u64(std::ptr::null_mut(), 0);
        pdal_metadata_node_set_f64(std::ptr::null_mut(), 0.0);
        pdal_metadata_node_set_bool(std::ptr::null_mut(), false);

        assert_eq!(pdal_metadata_node_value_kind(std::ptr::null()), 255);
        assert_eq!(take_string(pdal_metadata_node_value(std::ptr::null())), "");
        assert_eq!(pdal_metadata_node_value_i64(std::ptr::null()), 0);
        assert_eq!(pdal_metadata_node_value_u64(std::ptr::null()), 0);
        assert_eq!(pdal_metadata_node_value_f64(std::ptr::null()), 0.0);
        assert!(!pdal_metadata_node_value_bool(std::ptr::null()));

        assert!(pdal_metadata_node_child(std::ptr::null(), 0).is_null());
        assert_eq!(
            pdal_metadata_node_child_named_count(std::ptr::null(), std::ptr::null()),
            0
        );
        assert!(pdal_metadata_node_child_named(std::ptr::null(), std::ptr::null(), 0).is_null());

        pdal_metadata_node_add_child(std::ptr::null_mut(), std::ptr::null_mut());
        pdal_metadata_node_add_or_update_child(std::ptr::null_mut(), std::ptr::null_mut());

        // 2. Test valid value but null out_value pointer for converters
        let int_type = CString::new("integer").unwrap();
        let int_val = CString::new("123").unwrap();
        assert!(pdal_metadata_value_as_i64(
            int_type.as_ptr(),
            int_val.as_ptr(),
            std::ptr::null_mut()
        ));

        let uint_type = CString::new("unsigned").unwrap();
        let uint_val = CString::new("123").unwrap();
        assert!(pdal_metadata_value_as_u64(
            uint_type.as_ptr(),
            uint_val.as_ptr(),
            std::ptr::null_mut()
        ));

        let double_type = CString::new("double").unwrap();
        let double_val = CString::new("1.23").unwrap();
        assert!(pdal_metadata_value_as_f64(
            double_type.as_ptr(),
            double_val.as_ptr(),
            std::ptr::null_mut()
        ));

        let bool_type = CString::new("boolean").unwrap();
        let bool_val = CString::new("true").unwrap();
        assert!(pdal_metadata_value_as_bool(
            bool_type.as_ptr(),
            bool_val.as_ptr(),
            std::ptr::null_mut()
        ));

        // 3. Test non-cloned child adding and non-cloned updating
        let root = pdal_metadata_node_create(CString::new("root").unwrap().as_ptr());
        let child1 = pdal_metadata_node_create(CString::new("child").unwrap().as_ptr());
        pdal_metadata_node_set_string(child1, CString::new("one").unwrap().as_ptr());

        // Test non-cloned add_child (takes ownership)
        pdal_metadata_node_add_child(root, child1);
        assert_eq!(pdal_metadata_node_child_count(root), 1);
        assert_eq!(
            pdal_metadata_node_child_named_count(root, CString::new("child").unwrap().as_ptr()),
            1
        );

        // Test non-cloned add_or_update_child (takes ownership and replaces)
        let child2 = pdal_metadata_node_create(CString::new("child").unwrap().as_ptr());
        pdal_metadata_node_set_string(child2, CString::new("two").unwrap().as_ptr());
        pdal_metadata_node_add_or_update_child(root, child2);

        assert_eq!(pdal_metadata_node_child_count(root), 1);
        let replaced_child = pdal_metadata_node_child(root, 0);
        assert_eq!(take_string(pdal_metadata_node_value(replaced_child)), "two");
        pdal_metadata_node_destroy(replaced_child);

        // Test add_child / add_or_update_child with null child but valid parent (should do nothing and not crash)
        pdal_metadata_node_add_child(root, std::ptr::null_mut());
        pdal_metadata_node_add_or_update_child(root, std::ptr::null_mut());

        pdal_metadata_node_destroy(root);
    }
}
