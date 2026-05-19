//! C ABI for the PDAL Rust port spike.
//!
//! Every function in this crate is `extern "C"` and intended to be called from
//! C or C++ through the header `include/pdal_capi.h`.

mod error;
mod filter_abi;
mod filter_expression_abi;
mod filter_grid_abi;
mod filter_runtime;
mod io_abi;
mod metadata_abi;
mod metrics_abi;
mod native_abi;
mod options;
mod pipeline_abi;
mod point_abi;
mod registry;
mod srs;
mod stage_abi;
mod stats_abi;
mod tile_abi;

pub use error::*;
pub use filter_abi::*;
pub use filter_expression_abi::*;
pub use filter_grid_abi::*;
pub use filter_runtime::*;
pub use io_abi::*;
pub use metadata_abi::*;
pub use metrics_abi::*;
pub use native_abi::*;
pub use options::*;
pub use pipeline_abi::*;
pub use point_abi::*;
pub use registry::*;
pub use srs::*;
pub use stage_abi::*;
pub use stats_abi::*;
pub use tile_abi::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    unsafe fn take_string(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null());
        let value = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        pdal_string_free(ptr);
        value
    }

    #[test]
    fn spatial_reference_roundtrips_through_c_abi() {
        unsafe {
            let text = CString::new("EPSG:4326").unwrap();
            let srs = pdal_spatial_reference_create_with_epoch(text.as_ptr(), 2020.0);
            assert!(!pdal_spatial_reference_empty(srs));
            assert_eq!(pdal_spatial_reference_epoch(srs), 2020.0);
            assert_eq!(take_string(pdal_spatial_reference_text(srs)), "EPSG:4326");

            pdal_spatial_reference_set_epoch(srs, 2021.5);
            assert_eq!(pdal_spatial_reference_epoch(srs), 2021.5);
            pdal_spatial_reference_destroy(srs);
        }
    }

    #[test]
    fn native_dependencies_serialize_through_c_abi() {
        unsafe {
            let json: serde_json::Value =
                serde_json::from_str(&take_string(pdal_native_dependencies_json())).unwrap();

            assert!(json
                .as_array()
                .unwrap()
                .iter()
                .any(|dependency| dependency["name"] == "GDAL"));
            assert!(json
                .as_array()
                .unwrap()
                .iter()
                .all(|dependency| dependency["version"]
                    .as_str()
                    .is_some_and(|v| !v.is_empty())));
        }
    }

    #[test]
    fn point_view_carries_spatial_reference() {
        unsafe {
            let layout = pdal_point_layout_create();
            let x = CString::new("X").unwrap();
            pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
            let view = pdal_point_view_create(layout);

            let text = CString::new("EPSG:4978").unwrap();
            let srs = pdal_spatial_reference_create(text.as_ptr());
            pdal_point_view_set_spatial_reference(view, srs);

            let copied = pdal_point_view_spatial_reference(view);
            assert_eq!(
                take_string(pdal_spatial_reference_text(copied)),
                "EPSG:4978"
            );

            pdal_spatial_reference_destroy(copied);
            pdal_spatial_reference_destroy(srs);
            pdal_point_view_destroy(view);
        }
    }

    #[test]
    fn point_view_exposes_layout_dimensions() {
        unsafe {
            let layout = pdal_point_layout_create();
            let x = CString::new("X").unwrap();
            let classification = CString::new("Classification").unwrap();
            pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
            pdal_point_layout_register_dim(layout, classification.as_ptr(), 0);
            let view = pdal_point_view_create(layout);

            assert_eq!(pdal_point_view_dim_count(view), 2);
            assert_eq!(take_string(pdal_point_view_dim_name(view, 0)), "X");
            assert_eq!(pdal_point_view_dim_type(view, 0), 9);
            assert_eq!(
                take_string(pdal_point_view_dim_name(view, 1)),
                "Classification"
            );
            assert_eq!(pdal_point_view_dim_type(view, 1), 0);

            pdal_point_view_destroy(view);
        }
    }

    #[test]
    fn point_view_bounds_roundtrip_through_c_abi() {
        unsafe {
            let layout = pdal_point_layout_create();
            for dim in ["X", "Y", "Z"] {
                let name = CString::new(dim).unwrap();
                pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
            }
            let view = pdal_point_view_create(layout);

            for (x, y, z) in [(-10.0, 5.0, 100.0), (20.0, -15.0, -50.0), (3.0, 7.0, 25.0)] {
                let point = pdal_point_view_add_point(view);
                for (dim, value) in [("X", x), ("Y", y), ("Z", z)] {
                    let name = CString::new(dim).unwrap();
                    pdal_point_view_set_f64(view, point, name.as_ptr(), value);
                }
            }

            let mut bounds2d = pdal_bounds2d_t {
                minx: 0.0,
                maxx: 0.0,
                miny: 0.0,
                maxy: 0.0,
            };
            assert!(pdal_point_view_calculate_bounds_2d(view, &mut bounds2d));
            assert_eq!(
                bounds2d,
                pdal_bounds2d_t {
                    minx: -10.0,
                    maxx: 20.0,
                    miny: -15.0,
                    maxy: 7.0,
                }
            );

            let mut bounds3d = pdal_bounds3d_t {
                minx: 0.0,
                maxx: 0.0,
                miny: 0.0,
                maxy: 0.0,
                minz: 0.0,
                maxz: 0.0,
            };
            assert!(pdal_point_view_calculate_bounds_3d(view, &mut bounds3d));
            assert_eq!(
                bounds3d,
                pdal_bounds3d_t {
                    minx: -10.0,
                    maxx: 20.0,
                    miny: -15.0,
                    maxy: 7.0,
                    minz: -50.0,
                    maxz: 100.0,
                }
            );

            pdal_point_view_destroy(view);
        }
    }

    #[test]
    fn point_view_bounds_c_abi_reports_unavailable_bounds() {
        unsafe {
            let layout = pdal_point_layout_create();
            let x = CString::new("X").unwrap();
            let y = CString::new("Y").unwrap();
            pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
            pdal_point_layout_register_dim(layout, y.as_ptr(), 9);
            let view = pdal_point_view_create(layout);

            let mut bounds2d = pdal_bounds2d_t {
                minx: 0.0,
                maxx: 0.0,
                miny: 0.0,
                maxy: 0.0,
            };
            assert!(!pdal_point_view_calculate_bounds_2d(view, &mut bounds2d));

            let point = pdal_point_view_add_point(view);
            pdal_point_view_set_f64(view, point, x.as_ptr(), 1.0);
            pdal_point_view_set_f64(view, point, y.as_ptr(), 2.0);
            assert!(pdal_point_view_calculate_bounds_2d(view, &mut bounds2d));

            let mut bounds3d = pdal_bounds3d_t {
                minx: 0.0,
                maxx: 0.0,
                miny: 0.0,
                maxy: 0.0,
                minz: 0.0,
                maxz: 0.0,
            };
            assert!(!pdal_point_view_calculate_bounds_3d(view, &mut bounds3d));
            assert!(!pdal_point_view_calculate_bounds_2d(
                std::ptr::null(),
                &mut bounds2d
            ));
            assert!(!pdal_point_view_calculate_bounds_2d(
                view,
                std::ptr::null_mut()
            ));

            pdal_point_view_destroy(view);
        }
    }

    #[test]
    fn point_view_dimension_summaries_serialize_through_c_abi() {
        unsafe {
            let layout = pdal_point_layout_create();
            let x = CString::new("X").unwrap();
            let intensity = CString::new("Intensity").unwrap();
            pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
            pdal_point_layout_register_dim(layout, intensity.as_ptr(), 1);
            let view = pdal_point_view_create(layout);

            for (x_value, intensity_value) in [(-10.0, 7.0), (20.0, 3.0), (2.0, 5.0)] {
                let point = pdal_point_view_add_point(view);
                pdal_point_view_set_f64(view, point, x.as_ptr(), x_value);
                pdal_point_view_set_f64(view, point, intensity.as_ptr(), intensity_value);
            }

            let json: serde_json::Value =
                serde_json::from_str(&take_string(pdal_point_view_dimension_summaries_json(view)))
                    .unwrap();
            assert_eq!(json[0]["name"], "X");
            assert_eq!(json[0]["count"], 3);
            assert_eq!(json[0]["minimum"], -10.0);
            assert_eq!(json[0]["maximum"], 20.0);
            assert_eq!(json[0]["mean"], 4.0);
            assert_eq!(json[1]["name"], "Intensity");
            assert_eq!(json[1]["minimum"], 3.0);
            assert_eq!(json[1]["maximum"], 7.0);
            assert_eq!(json[1]["mean"], 5.0);

            pdal_point_view_destroy(view);
        }
    }

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

            let root = pdal_metadata_node_create(root_name.as_ptr());
            let child = pdal_metadata_node_create(child_name.as_ptr());
            pdal_metadata_node_set_string(child, child_value.as_ptr());
            pdal_metadata_node_add_child(root, child);

            assert_eq!(pdal_metadata_node_child_count(root), 1);
            let copied = pdal_metadata_node_child(root, 0);
            assert_eq!(take_string(pdal_metadata_node_name(copied)), "child");
            assert_eq!(take_string(pdal_metadata_node_value(copied)), "value");

            pdal_metadata_node_destroy(copied);
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

    fn empty_pipeline_result() -> pdal_pipeline_result_t {
        pdal_pipeline_result_t {
            point_count: 0,
            view_count: 0,
            has_bounds_2d: false,
            bounds_2d: pdal_bounds2d_t {
                minx: 0.0,
                maxx: 0.0,
                miny: 0.0,
                maxy: 0.0,
            },
            has_bounds_3d: false,
            bounds_3d: pdal_bounds3d_t {
                minx: 0.0,
                maxx: 0.0,
                miny: 0.0,
                maxy: 0.0,
                minz: 0.0,
                maxz: 0.0,
            },
        }
    }
}
