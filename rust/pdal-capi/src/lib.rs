//! C ABI for the PDAL Rust port spike.
//!
//! Every function in this crate is `extern "C"` and intended to be called from
//! C or C++ through the header `include/pdal_capi.h`.

mod config_abi;
mod driver_abi;
mod error;
mod file_spec_abi;
mod filter_abi;
mod filter_expression_abi;
mod filter_grid_abi;
mod filter_runtime;
mod io_abi;
mod kernel_abi;
mod log_abi;
mod metadata_abi;
mod metrics_abi;
mod native_abi;
mod ogr_spec_abi;
mod options;
mod pipeline_abi;
mod plugin_abi;
mod point_abi;
mod registry;
mod scaling_abi;
mod srs;
mod stage_abi;
mod stats_abi;
mod tile_abi;
mod utils_abi;
mod xml_schema_abi;

pub use config_abi::*;
pub use driver_abi::*;
pub use error::*;
pub use file_spec_abi::*;
pub use filter_abi::*;
pub use filter_expression_abi::*;
pub use filter_grid_abi::*;
pub use filter_runtime::*;
pub use io_abi::*;
pub use kernel_abi::*;
pub use log_abi::*;
pub use metadata_abi::*;
pub use metrics_abi::*;
pub use native_abi::*;
pub use ogr_spec_abi::*;
pub use options::*;
pub use pipeline_abi::*;
pub use plugin_abi::*;
pub use point_abi::*;
pub use registry::*;
pub use scaling_abi::*;
pub use srs::*;
pub use stage_abi::*;
pub use stats_abi::*;
pub use tile_abi::*;
pub use utils_abi::*;
pub use xml_schema_abi::*;

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
    fn spatial_reference_wgs84_zone_code_roundtrips_through_c_abi() {
        unsafe {
            assert_eq!(
                take_string(pdal_spatial_reference_wgs84_code_from_zone(17)),
                "EPSG:32617"
            );
            assert_eq!(
                take_string(pdal_spatial_reference_wgs84_code_from_zone(-17)),
                "EPSG:32717"
            );
            assert_eq!(
                take_string(pdal_spatial_reference_wgs84_code_from_zone(0)),
                ""
            );
        }
    }

    #[test]
    fn kernel_stage_option_parser_roundtrips_through_c_abi() {
        unsafe {
            let input = CString::new("--readers.p2g.foobar=baz").unwrap();
            let mut stage = std::ptr::null_mut();
            let mut option = std::ptr::null_mut();
            let mut value = std::ptr::null_mut();

            let result = pdal_kernel_parse_stage_option(
                input.as_ptr(),
                false,
                &mut stage,
                &mut option,
                &mut value,
            );
            assert_eq!(result, 0);
            assert_eq!(take_string(stage), "readers.p2g");
            assert_eq!(take_string(option), "foobar");
            assert_eq!(take_string(value), "baz");

            let input = CString::new("--stage.tag.option=value").unwrap();
            assert_eq!(
                pdal_kernel_parse_stage_option(
                    input.as_ptr(),
                    false,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ),
                2
            );
            assert_eq!(
                pdal_kernel_parse_stage_option(
                    input.as_ptr(),
                    true,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ),
                0
            );
        }
    }

    #[test]
    fn plugin_name_validation_roundtrips_through_c_abi() {
        unsafe {
            let path = CString::new("libpdal_plugin_reader_a_b.dylib").unwrap();
            let reader = CString::new("reader").unwrap();
            let writer = CString::new("writer").unwrap();
            let ext = CString::new(".dylib").unwrap();
            let types = [reader.as_ptr(), writer.as_ptr()];

            assert_eq!(
                take_string(pdal_plugin_valid_name(
                    path.as_ptr(),
                    types.as_ptr(),
                    types.len() as u64,
                    ext.as_ptr(),
                )),
                "readers.a_b"
            );

            let bad = CString::new("libpdal_plugin_reader_1a_b.dylib").unwrap();
            assert_eq!(
                take_string(pdal_plugin_valid_name(
                    bad.as_ptr(),
                    types.as_ptr(),
                    types.len() as u64,
                    ext.as_ptr(),
                )),
                ""
            );
        }
    }

    #[test]
    fn file_spec_parser_roundtrips_through_c_abi() {
        unsafe {
            let input = CString::new(r#"{"path":"foo.laz","headers":{"h":"v"},"query":{"q":"v"}}"#)
                .unwrap();
            let parsed: serde_json::Value =
                serde_json::from_str(&take_string(pdal_file_spec_parse_json(input.as_ptr())))
                    .unwrap();

            assert_eq!(parsed["ok"], true);
            assert_eq!(parsed["path"], "foo.laz");
            assert_eq!(parsed["headers"]["h"], "v");
            assert_eq!(parsed["query"]["q"], "v");

            let input = CString::new(r#"{"query":[]}"#).unwrap();
            let parsed: serde_json::Value =
                serde_json::from_str(&take_string(pdal_file_spec_parse_json(input.as_ptr())))
                    .unwrap();
            assert_eq!(parsed["ok"], false);
            assert_eq!(
                parsed["error"],
                "'filename' object must contain 'path' member."
            );
        }
    }

    #[test]
    fn option_name_validation_roundtrips_through_c_abi() {
        unsafe {
            let valid = CString::new("foo_123_bar_baz").unwrap();
            let bad = CString::new("foo_123_bar-baz").unwrap();

            assert!(pdal_option_name_valid(valid.as_ptr()));
            assert!(!pdal_option_name_valid(bad.as_ptr()));
            assert!(!pdal_option_name_valid(std::ptr::null()));
        }
    }

    #[test]
    fn driver_inference_roundtrips_through_c_abi() {
        unsafe {
            let reader = CString::new("foo.laz").unwrap();
            let writer = CString::new("foo.tif").unwrap();
            let unknown = CString::new("foo.unknown").unwrap();

            assert_eq!(
                take_string(pdal_infer_reader_driver(reader.as_ptr())),
                "readers.las"
            );
            assert_eq!(
                take_string(pdal_infer_writer_driver(writer.as_ptr())),
                "writers.gdal"
            );
            assert_eq!(take_string(pdal_infer_reader_driver(unknown.as_ptr())), "");
        }
    }

    #[test]
    fn config_helpers_roundtrip_through_c_abi() {
        unsafe {
            assert_eq!(pdal_config_version_integer(2, 10, 1), 21001);

            let version = CString::new("2.10.1").unwrap();
            let sha = CString::new("abcdef123456").unwrap();
            assert_eq!(
                take_string(pdal_config_full_version_string(
                    version.as_ptr(),
                    sha.as_ptr()
                )),
                "2.10.1 (git-version: abcdef)"
            );
        }
    }

    #[test]
    fn log_level_strings_roundtrip_through_c_abi() {
        unsafe {
            assert_eq!(
                CStr::from_ptr(pdal_log_level_string(0)).to_string_lossy(),
                "Error"
            );
            assert_eq!(
                CStr::from_ptr(pdal_log_level_string(1)).to_string_lossy(),
                "Warning"
            );
            assert_eq!(
                CStr::from_ptr(pdal_log_level_string(2)).to_string_lossy(),
                "Info"
            );
            assert_eq!(
                CStr::from_ptr(pdal_log_level_string(7)).to_string_lossy(),
                "Debug"
            );
        }
    }

    #[test]
    fn ogr_spec_roundtrips_through_c_abi() {
        unsafe {
            let input = CString::new(
                r#"{"type":"OGR","datasource":"attributes.json","drivers":["GeoJSON"],"options":{"dialect":"OGRSQL"}}"#,
            )
            .unwrap();
            let json: serde_json::Value =
                serde_json::from_str(&take_string(pdal_ogr_spec_parse_json(input.as_ptr())))
                    .unwrap();

            assert_eq!(json["ok"], true);
            assert_eq!(json["datasource"], "attributes.json");
            assert_eq!(json["drivers"][0], "GeoJSON");
            assert_eq!(json["dialect"], "OGRSQL");
        }
    }

    #[test]
    fn pipeline_stage_tag_generation_roundtrips_through_c_abi() {
        unsafe {
            let stage = CString::new("readers.las").unwrap();
            assert_eq!(
                take_string(pdal_pipeline_generate_stage_tag(
                    stage.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                )),
                "readers_las1"
            );

            let existing = CString::new("readers_las1").unwrap();
            let tags = [existing.as_ptr()];
            assert_eq!(
                take_string(pdal_pipeline_generate_stage_tag(
                    stage.as_ptr(),
                    std::ptr::null(),
                    tags.as_ptr(),
                    tags.len() as u64,
                )),
                "readers_las2"
            );
        }
    }

    #[test]
    fn utility_json_detection_roundtrips_through_c_abi() {
        unsafe {
            let object = CString::new(r#" {"path":"file.laz"} "#).unwrap();
            let plain = CString::new("file.laz").unwrap();

            assert!(pdal_utils_is_json(object.as_ptr()));
            assert!(!pdal_utils_is_json(plain.as_ptr()));
            assert!(!pdal_utils_is_json(std::ptr::null()));
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
    fn geometry_predicates_roundtrip_through_c_abi() {
        unsafe {
            let polygon = CString::new("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();

            let mut valid = false;
            assert!(pdal_geometry_wkt_is_valid(polygon.as_ptr(), &mut valid));
            assert!(valid);

            let mut contains = false;
            assert!(pdal_geometry_wkt_contains_point(
                polygon.as_ptr(),
                5.0,
                5.0,
                &mut contains
            ));
            assert!(contains);

            assert!(pdal_geometry_wkt_contains_point(
                polygon.as_ptr(),
                15.0,
                5.0,
                &mut contains
            ));
            assert!(!contains);
        }
    }

    #[test]
    fn geometry_distance_roundtrips_through_c_abi() {
        unsafe {
            let point = CString::new("POINT(0 0 0)").unwrap();
            let mut distance = 0.0;

            assert!(pdal_geometry_wkt_distance_to_point(
                point.as_ptr(),
                3.0,
                4.0,
                0.0,
                &mut distance
            ));
            assert_eq!(distance, 5.0);
        }
    }

    #[test]
    fn xml_schema_legacy_names_roundtrip_through_c_abi() {
        unsafe {
            let point_id = CString::new("Chipper Point ID").unwrap();
            let block_id = CString::new("Unnamed field 513").unwrap();
            let unchanged = CString::new("Intensity").unwrap();

            assert_eq!(
                take_string(pdal_xml_schema_remap_old_name(point_id.as_ptr())),
                "Chipper:PointID"
            );
            assert_eq!(
                take_string(pdal_xml_schema_remap_old_name(block_id.as_ptr())),
                "Chipper:BlockID"
            );
            assert_eq!(
                take_string(pdal_xml_schema_remap_old_name(unchanged.as_ptr())),
                "Intensity"
            );
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
    fn dimension_type_helpers_roundtrip_through_c_abi() {
        unsafe {
            let signed = CString::new("signed").unwrap();
            let int32 = CString::new("INT32_T").unwrap();
            let bad = CString::new("unknown").unwrap();

            assert_eq!(
                take_string(pdal_dimension_interpretation_name(0x200 | 2)),
                "uint16_t"
            );
            assert_eq!(pdal_dimension_type_from_name(int32.as_ptr()), 0x100 | 4);
            assert_eq!(
                pdal_dimension_type_from_base_and_size(signed.as_ptr(), 8),
                0x100 | 8
            );
            assert_eq!(pdal_dimension_type_from_name(bad.as_ptr()), 0);
            assert_eq!(
                pdal_dimension_type_from_base_and_size(std::ptr::null(), 8),
                0
            );
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
    fn reader_read_first_returns_point_view_through_c_abi() {
        unsafe {
            let options = pdal_options_create();
            for (key, value) in [
                ("mode", "ramp"),
                ("count", "3"),
                ("minx", "10"),
                ("maxx", "12"),
                ("miny", "20"),
                ("maxy", "22"),
                ("minz", "30"),
                ("maxz", "32"),
            ] {
                let key = CString::new(key).unwrap();
                let value = CString::new(value).unwrap();
                pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
            }

            let reader = pdal_reader_create_faux(options);
            assert!(!reader.is_null());
            let view = pdal_reader_read_first(reader);
            assert!(!view.is_null());
            assert_eq!(pdal_point_view_length(view), 3);

            let x = CString::new("X").unwrap();
            let y = CString::new("Y").unwrap();
            let z = CString::new("Z").unwrap();
            assert_eq!(pdal_point_view_get_f64(view, 0, x.as_ptr()), 10.0);
            assert_eq!(pdal_point_view_get_f64(view, 1, y.as_ptr()), 21.0);
            assert_eq!(pdal_point_view_get_f64(view, 2, z.as_ptr()), 32.0);

            pdal_point_view_destroy(view);
            pdal_reader_destroy(reader);
            pdal_options_destroy(options);
        }
    }

    #[test]
    fn writer_write_view_consumes_point_view_through_c_abi() {
        unsafe {
            let mut filename = std::env::temp_dir();
            filename.push(format!(
                "pdal-capi-writer-write-view-{}-{}.csv",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let filename_text = filename.display().to_string();

            let options = pdal_options_create();
            for (key, value) in [
                ("filename", filename_text.as_str()),
                ("order", "X:1,Y:1,Z:1"),
                ("keep_unspecified", "false"),
            ] {
                let key = CString::new(key).unwrap();
                let value = CString::new(value).unwrap();
                pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
            }

            let layout = pdal_point_layout_create();
            for dim in ["X", "Y", "Z"] {
                let name = CString::new(dim).unwrap();
                pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
            }
            let view = pdal_point_view_create(layout);
            let point = pdal_point_view_add_point(view);
            for (dim, value) in [("X", 1.25), ("Y", 2.5), ("Z", 3.75)] {
                let name = CString::new(dim).unwrap();
                pdal_point_view_set_f64(view, point, name.as_ptr(), value);
            }

            let writer = pdal_writer_create_text(options);
            assert!(!writer.is_null());
            assert!(pdal_writer_write_view(writer, view));
            assert_eq!(
                std::fs::read_to_string(&filename).unwrap(),
                "\"X\",\"Y\",\"Z\"\n1.2,2.5,3.8\n"
            );

            let _ = std::fs::remove_file(&filename);
            pdal_writer_destroy(writer);
            pdal_point_view_destroy(view);
            pdal_options_destroy(options);
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
