use super::*;
use pdal_core::point::PointView;
use std::os::raw::c_char;

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn xyz_view(points: &[(f64, f64, f64, f64)]) -> *mut PointView {
    let layout = pdal_point_layout_create();
    for dim in ["X", "Y", "Z", "Classification"] {
        let name = cstring(dim);
        pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
    }
    let view = pdal_point_view_create(layout);
    for (x, y, z, class) in points {
        let idx = pdal_point_view_add_point(view);
        for (dim, value) in [("X", *x), ("Y", *y), ("Z", *z), ("Classification", *class)] {
            let name = cstring(dim);
            pdal_point_view_set_f64(view, idx, name.as_ptr(), value);
        }
    }
    view
}

unsafe fn take_u64s(ptr: *mut u64, len: u64) -> Vec<u64> {
    assert!(!ptr.is_null());
    let values = std::slice::from_raw_parts(ptr, len as usize).to_vec();
    pdal_u64_array_free(ptr, len);
    values
}

#[test]
fn options_abi_exposes_sorted_entries_and_command_line() {
    unsafe {
        let options = pdal_options_create();
        let z = cstring("z");
        let a = cstring("a");
        let value = cstring("text");
        pdal_options_add_f64(options, z.as_ptr(), 1.5);
        pdal_options_add_u64(options, a.as_ptr(), 7);
        pdal_options_add_str(options, a.as_ptr(), value.as_ptr());
        pdal_options_add_conditional_str(options, a.as_ptr(), cstring("ignored").as_ptr());
        pdal_options_add_conditional_str(options, cstring("b").as_ptr(), cstring("new").as_ptr());

        assert_eq!(pdal_options_count(options), 4);
        assert!(pdal_options_has(options, a.as_ptr()));
        assert_eq!(take_string(pdal_options_key(options, 0)), "a");
        assert_eq!(take_string(pdal_options_entry_value(options, 0)), "7");
        assert_eq!(take_string(pdal_options_value(options, a.as_ptr())), "text");
        let args: serde_json::Value =
            serde_json::from_str(&take_string(pdal_options_command_line_json(options))).unwrap();
        assert_eq!(
            args,
            serde_json::json!(["--a=7", "--a=text", "--b=new", "--z=1.5"])
        );
        pdal_options_replace_str(options, a.as_ptr(), cstring("replacement").as_ptr());
        assert_eq!(pdal_options_count(options), 3);
        assert_eq!(
            take_string(pdal_options_value(options, a.as_ptr())),
            "replacement"
        );

        let other = pdal_options_create();
        pdal_options_add_str(other, a.as_ptr(), cstring("from-other").as_ptr());
        pdal_options_add_str(other, cstring("c").as_ptr(), cstring("third").as_ptr());
        pdal_options_extend_conditional(options, other);
        assert_eq!(pdal_options_count(options), 4);
        assert_eq!(
            take_string(pdal_options_value(options, a.as_ptr())),
            "replacement"
        );
        assert_eq!(
            take_string(pdal_options_value(options, cstring("c").as_ptr())),
            "third"
        );
        pdal_options_extend(options, other);
        assert_eq!(pdal_options_count(options), 6);
        assert_eq!(
            take_string(pdal_options_value(options, a.as_ptr())),
            "from-other"
        );
        pdal_options_destroy(other);

        pdal_options_remove(options, a.as_ptr());
        assert_eq!(pdal_options_count(options), 4);
        assert!(!pdal_options_has(options, a.as_ptr()));
        assert!(pdal_options_key(options, 99).is_null());
        assert!(pdal_options_entry_value(options, 99).is_null());
        assert!(pdal_options_value(options, std::ptr::null()).is_null());
        assert!(!pdal_options_has(options, std::ptr::null()));
        assert_eq!(pdal_options_count(std::ptr::null()), 0);
        assert!(pdal_options_command_line_json(std::ptr::null()).is_null());

        pdal_options_destroy(options);
    }
}

#[test]
fn options_abi_parses_file_bodies() {
    unsafe {
        let json = cstring(
            r#"{
                // accepted by PDAL option files
                "count": 7,
                "flag": true,
                "name": "autzen"
            }"#,
        );
        let options = pdal_options_from_json_object_text(json.as_ptr());
        assert!(!options.is_null());
        assert_eq!(
            take_string(pdal_options_value(options, cstring("count").as_ptr())),
            "7"
        );
        assert_eq!(
            take_string(pdal_options_value(options, cstring("flag").as_ptr())),
            "true"
        );
        pdal_options_destroy(options);

        let text = cstring("--count=7 --name \"two words\"");
        let options = pdal_options_from_command_line_text(text.as_ptr());
        assert!(!options.is_null());
        assert_eq!(
            take_string(pdal_options_value(options, cstring("count").as_ptr())),
            "7"
        );
        assert_eq!(
            take_string(pdal_options_value(options, cstring("name").as_ptr())),
            "two words"
        );
        pdal_options_destroy(options);

        assert!(pdal_options_from_json_object_text(cstring("[1, 2]").as_ptr()).is_null());
        let error = CStr::from_ptr(pdal_last_error()).to_string_lossy();
        assert!(error.contains("object"));
    }
}

#[test]
fn utils_abi_roundtrips_strings_and_lists() {
    unsafe {
        let spaced = cstring("  Hello World  ");
        assert_eq!(
            take_string(pdal_utils_trim_leading(spaced.as_ptr())),
            "Hello World  "
        );
        assert_eq!(
            take_string(pdal_utils_trim_trailing(spaced.as_ptr())),
            "  Hello World"
        );
        assert_eq!(
            take_string(pdal_utils_replace_all(
                cstring("a-b-c").as_ptr(),
                cstring("-").as_ptr(),
                cstring(":").as_ptr()
            )),
            "a:b:c"
        );
        assert_eq!(
            take_string(pdal_utils_to_lower(cstring("AbC").as_ptr())),
            "abc"
        );
        assert_eq!(
            take_string(pdal_utils_to_upper(cstring("AbC").as_ptr())),
            "ABC"
        );
        assert!(pdal_utils_iequals(
            cstring("AbC").as_ptr(),
            cstring("abc").as_ptr()
        ));
        assert!(pdal_utils_starts_with(
            cstring("prefix-value").as_ptr(),
            cstring("prefix").as_ptr()
        ));

        let split: serde_json::Value = serde_json::from_str(&take_string(pdal_utils_split_char(
            cstring("a,b,c").as_ptr(),
            b',' as c_char,
        )))
        .unwrap();
        assert_eq!(split, serde_json::json!(["a", "b", "c"]));
        let split2: serde_json::Value = serde_json::from_str(&take_string(pdal_utils_split2_char(
            cstring("a=b=c").as_ptr(),
            b'=' as c_char,
        )))
        .unwrap();
        assert_eq!(split2, serde_json::json!(["a", "b", "c"]));
        assert_eq!(
            take_string(pdal_utils_escape_json(cstring("\"x\"").as_ptr())),
            "\\\"x\\\""
        );
        assert_eq!(
            take_string(pdal_utils_escape_nonprinting(cstring("a\n").as_ptr())),
            "a\\n"
        );
        assert_eq!(pdal_utils_normalize_longitude(181.0), -179.0);

        let wrapped: serde_json::Value = serde_json::from_str(&take_string(pdal_utils_word_wrap(
            cstring("one two three").as_ptr(),
            7,
            7,
        )))
        .unwrap();
        assert_eq!(wrapped, serde_json::json!(["one two", "three"]));
        let wrapped2: serde_json::Value = serde_json::from_str(&take_string(
            pdal_utils_word_wrap2(cstring("one two three").as_ptr(), 7, 7),
        ))
        .unwrap();
        assert!(wrapped2.as_array().unwrap().len() >= 2);
        let words: serde_json::Value = serde_json::from_str(&take_string(
            pdal_utils_simple_wordexp(cstring("a \"b c\"").as_ptr()),
        ))
        .unwrap();
        assert_eq!(words, serde_json::json!(["a", "b c"]));
    }
}

#[test]
fn utils_abi_roundtrips_bytes_and_charbuf_seeks() {
    unsafe {
        let bytes = b"PDAL";
        let encoded = take_string(pdal_utils_base64_encode(bytes.as_ptr(), bytes.len() as u64));
        assert_eq!(encoded, "UERBTA==");
        let mut out_len = 0;
        let decoded = pdal_utils_base64_decode(cstring(&encoded).as_ptr(), &mut out_len);
        assert_eq!(std::slice::from_raw_parts(decoded, out_len as usize), bytes);
        pdal_u8_array_free(decoded, out_len);
        assert!(pdal_utils_base64_decode(cstring("").as_ptr(), &mut out_len).is_null());
        assert_eq!(out_len, 0);

        let padded = [b'a', b'b', b'c', 0, 0];
        assert_eq!(
            take_string(pdal_utils_extract_c_string(
                padded.as_ptr(),
                padded.len() as u64,
                0,
                5
            )),
            "abc"
        );
        assert_eq!(
            take_string(pdal_utils_extract_c_string(
                padded.as_ptr(),
                padded.len() as u64,
                0,
                1
            )),
            "a"
        );
        assert_eq!(
            take_string(pdal_utils_extract_c_string(
                padded.as_ptr(),
                padded.len() as u64,
                0,
                0
            )),
            ""
        );

        assert_eq!(pdal_charbuf_seekpos(3, 0, 10, false), 3);
        assert_eq!(pdal_charbuf_seekpos(10, 0, 10, false), -1);
        assert_eq!(pdal_charbuf_seekpos(10, 0, 10, true), 10);
        assert_eq!(pdal_charbuf_seekpos(0, 5, 10, false), -1);
        assert_eq!(pdal_charbuf_seekoff(2, 0, 0, 10, 5), 2);
        assert_eq!(pdal_charbuf_seekoff(2, 1, 0, 10, 5), 7);
        assert_eq!(pdal_charbuf_seekoff(2, 2, 0, 10, 5), 8);
        assert_eq!(pdal_charbuf_seekoff(-2, 2, 0, 10, 5), -1);
    }
}

#[test]
fn utils_abi_roundtrips_paths_and_math_helpers() {
    unsafe {
        assert!(take_string(pdal_file_utils_getcwd()).ends_with('/'));
        assert!(take_string(pdal_file_utils_to_absolute_path(
            cstring("foo.txt").as_ptr()
        ))
        .ends_with("foo.txt"));
        assert!(take_string(pdal_file_utils_to_absolute_path_with_base(
            cstring("foo.txt").as_ptr(),
            cstring("/tmp/base").as_ptr()
        ))
        .ends_with("/tmp/base/foo.txt"));
        assert_eq!(
            take_string(pdal_file_utils_get_filename(
                cstring("/tmp/foo.txt").as_ptr()
            )),
            "foo.txt"
        );
        assert_eq!(
            take_string(pdal_file_utils_get_filename(cstring("/tmp/").as_ptr())),
            ""
        );
        assert!(take_string(pdal_file_utils_get_directory(
            cstring("/tmp/foo.txt").as_ptr()
        ))
        .ends_with('/'));
        assert_eq!(
            take_string(pdal_file_utils_stem(cstring("/tmp/foo.txt").as_ptr())),
            "foo"
        );
        assert_eq!(
            take_string(pdal_file_utils_extension(cstring("/tmp/foo.txt").as_ptr())),
            ".txt"
        );
        assert!(pdal_file_utils_is_absolute_path(
            cstring("/tmp/foo.txt").as_ptr()
        ));
        assert!(pdal_file_utils_is_absolute_path(
            cstring("s3://bucket/key").as_ptr()
        ));

        fn identity_matrix() -> pdal_rotation_matrix_t {
            pdal_rotation_matrix_t {
                m00: 1.0,
                m01: 0.0,
                m02: 0.0,
                m10: 0.0,
                m11: 1.0,
                m12: 0.0,
                m20: 0.0,
                m21: 0.0,
                m22: 1.0,
            }
        }
        let geo = pdal_georeference_wgs84(
            10.0,
            0.0,
            identity_matrix(),
            identity_matrix(),
            pdal_xyz_t {
                x: 1.0,
                y: 0.5,
                z: 100.0,
            },
        );
        assert_eq!(geo.x, 1.0);
        assert_eq!(geo.z, 90.0);
        assert_eq!(
            pdal_barycentric_interpolation(
                0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 0.0, 1.0, 20.0, 0.25, 0.25
            ),
            7.5
        );
        assert!(pdal_barycentric_interpolation(
            0.0, 0.0, 0.0, 1.0, 1.0, 10.0, 2.0, 2.0, 20.0, 0.25, 0.25
        )
        .is_infinite());
    }
}

#[test]
fn stats_and_expressionstats_abi_compute_values_and_metadata() {
    unsafe {
        let view = xyz_view(&[
            (1.0, 0.0, 0.0, 2.0),
            (2.0, 0.0, 0.0, 2.0),
            (5.0, 0.0, 0.0, 7.0),
        ]);
        let x = cstring("X");
        let class = cstring("Classification");
        let null_dim: *const c_char = std::ptr::null();
        let dims = [x.as_ptr(), class.as_ptr(), null_dim];
        let enums = [class.as_ptr()];
        let globals = [x.as_ptr()];
        let counts = [class.as_ptr()];
        let mut out = [
            pdal_dim_stats_t {
                count: 0,
                min: 0.0,
                max: 0.0,
                m1: 0.0,
                m2: 0.0,
                m3: 0.0,
                m4: 0.0,
                median: 0.0,
                mad: 0.0,
                unique_values: std::ptr::null_mut(),
                unique_counts: std::ptr::null_mut(),
                unique_len: 0,
            },
            pdal_dim_stats_t {
                count: 0,
                min: 0.0,
                max: 0.0,
                m1: 0.0,
                m2: 0.0,
                m3: 0.0,
                m4: 0.0,
                median: 0.0,
                mad: 0.0,
                unique_values: std::ptr::null_mut(),
                unique_counts: std::ptr::null_mut(),
                unique_len: 0,
            },
        ];

        pdal_stats_compute(
            view,
            dims.as_ptr(),
            dims.len() as u64,
            true,
            enums.as_ptr(),
            enums.len() as u64,
            counts.as_ptr(),
            counts.len() as u64,
            globals.as_ptr(),
            globals.len() as u64,
            out.as_mut_ptr(),
        );
        assert_eq!(out[0].count, 3);
        assert_eq!(out[0].min, 1.0);
        assert_eq!(out[0].max, 5.0);
        assert_eq!(out[0].median, 2.0);
        assert_eq!(out[0].mad, 1.0);
        assert_eq!(out[1].unique_len, 2);
        assert_eq!(std::slice::from_raw_parts(out[1].unique_counts, 2), &[2, 1]);
        pdal_free_stats_arrays(out.as_mut_ptr(), out.len() as u64);
        pdal_free_stats_arrays(std::ptr::null_mut(), 0);
        pdal_stats_compute(
            std::ptr::null_mut(),
            dims.as_ptr(),
            1,
            false,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            out.as_mut_ptr(),
        );

        let expr = cstring("X <= 2");
        let exprs = [expr.as_ptr()];
        let metadata = pdal_expressionstats_metadata(view, x.as_ptr(), exprs.as_ptr(), 1);
        assert!(!metadata.is_null());
        assert_eq!(take_string(pdal_metadata_node_name(metadata)), "metadata");
        pdal_metadata_node_destroy(metadata);
        assert!(pdal_expressionstats_metadata(view, std::ptr::null(), exprs.as_ptr(), 1).is_null());
        assert!(pdal_expressionstats_metadata(view, x.as_ptr(), std::ptr::null(), 1).is_null());
        let null_exprs: [*const c_char; 1] = [std::ptr::null()];
        assert!(pdal_expressionstats_metadata(view, x.as_ptr(), null_exprs.as_ptr(), 1).is_null());

        let srs = cstring("EPSG:3857");
        let in_srs = cstring("EPSG:4326");
        let reprojection = pdal_stage_create_reprojection(srs.as_ptr(), in_srs.as_ptr(), true);
        assert!(!reprojection.is_null());
        pdal_stage_destroy(reprojection);
        assert!(pdal_stage_create_reprojection(std::ptr::null(), in_srs.as_ptr(), true).is_null());

        pdal_point_view_destroy(view);
    }
}

#[test]
fn spatial_and_quad_index_abi_return_sorted_results() {
    unsafe {
        let view = xyz_view(&[
            (0.0, 0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0, 0.0),
            (0.0, 2.0, 0.0, 0.0),
            (5.0, 5.0, 0.0, 0.0),
        ]);
        let x = cstring("X");
        let y = cstring("Y");
        let dims = [x.as_ptr(), y.as_ptr()];
        let query = [0.0, 0.0];
        let mut knn = [pdal_spatial_result_t {
            id: 0,
            sqr_dist: 0.0,
        }; 3];
        assert_eq!(
            pdal_point_view_knn(
                view,
                dims.as_ptr(),
                query.as_ptr(),
                2,
                3,
                1,
                knn.as_mut_ptr(),
                3
            ),
            3
        );
        assert_eq!(knn[0].id, 0);
        assert_eq!(knn[1].id, 1);
        let mut len = 0;
        let radius = pdal_point_view_radius(view, dims.as_ptr(), query.as_ptr(), 2, 2.1, &mut len);
        assert_eq!(len, 3);
        pdal_spatial_results_free(radius, len);
        assert!(pdal_point_view_radius(
            std::ptr::null(),
            dims.as_ptr(),
            query.as_ptr(),
            2,
            1.0,
            &mut len
        )
        .is_null());

        let xs = [0.0, 1.0, 3.0, 7.0];
        let ys = [0.0, 1.0, 3.0, 7.0];
        let ids = [10, 11, 12, 13];
        let index = pdal_quad_index_create(
            xs.as_ptr(),
            ys.as_ptr(),
            ids.as_ptr(),
            4,
            0.0,
            0.0,
            8.0,
            8.0,
            3,
        );
        assert!(!index.is_null());
        assert_eq!(pdal_quad_index_depth(index), 3);
        let mut bounds = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        pdal_quad_index_bounds(index, &mut bounds);
        assert_eq!(bounds.maxx, 8.0);

        let fills = pdal_quad_index_fills(index, &mut len);
        assert_eq!(take_u64s(fills, len), vec![0, 0, 0, 4]);
        let all = pdal_quad_index_points_by_depth(index, 0, 0, &mut len);
        assert_eq!(take_u64s(all, len), ids);
        let subset = pdal_quad_index_points_in_bounds(index, 0.0, 0.0, 4.0, 4.0, 0, 0, &mut len);
        assert_eq!(take_u64s(subset, len), vec![10, 11, 12]);

        let mut x_begin = 0.0;
        let mut x_end = 0.0;
        let mut x_step = 0.0;
        let mut y_begin = 0.0;
        let mut y_end = 0.0;
        let mut y_step = 0.0;
        let raster = pdal_quad_index_points_raster_level(
            index,
            2,
            &mut x_begin,
            &mut x_end,
            &mut x_step,
            &mut y_begin,
            &mut y_end,
            &mut y_step,
            &mut len,
        );
        assert_eq!(len, 16);
        assert_eq!(x_step, 2.0);
        pdal_u64_array_free(raster, len);
        let raster =
            pdal_quad_index_points_raster_bounds(index, 0.0, 8.0, 2.0, 0.0, 8.0, 2.0, &mut len);
        assert_eq!(len, 16);
        pdal_u64_array_free(raster, len);
        let empty =
            pdal_quad_index_points_raster_bounds(index, 0.0, 8.0, 0.0, 0.0, 8.0, 2.0, &mut len);
        assert_eq!(take_u64s(empty, len), Vec::<u64>::new());
        pdal_quad_index_destroy(index);
        assert!(pdal_quad_index_create(
            std::ptr::null(),
            ys.as_ptr(),
            ids.as_ptr(),
            4,
            0.0,
            0.0,
            8.0,
            8.0,
            3
        )
        .is_null());

        pdal_point_view_destroy(view);
    }
}

#[test]
fn expression_filter_c_abi_constructs_and_reports_errors() {
    unsafe {
        let expr = cstring("X > 1");
        let exprs = [expr.as_ptr()];
        let stage = pdal_stage_create_expression(exprs.as_ptr(), exprs.len() as u64);
        assert!(!stage.is_null());
        pdal_stage_destroy(stage);

        let stats_expr = cstring("X > 1");
        let stats_exprs = [stats_expr.as_ptr()];
        let dim = cstring("Classification");
        let stage = pdal_stage_create_expressionstats(
            dim.as_ptr(),
            stats_exprs.as_ptr(),
            stats_exprs.len() as u64,
        );
        assert!(!stage.is_null());
        pdal_stage_destroy(stage);

        let mongo = cstring(r#"{"X":{"$gt":1}}"#);
        let stage = pdal_stage_create_mongoexpression(mongo.as_ptr());
        assert!(!stage.is_null());
        pdal_stage_destroy(stage);

        assert!(pdal_stage_create_expression(std::ptr::null(), 1).is_null());
        let null_exprs: [*const c_char; 1] = [std::ptr::null()];
        assert!(pdal_stage_create_expression(null_exprs.as_ptr(), 1).is_null());
        assert!(
            pdal_stage_create_expressionstats(std::ptr::null(), stats_exprs.as_ptr(), 1).is_null()
        );
        assert!(pdal_stage_create_mongoexpression(std::ptr::null()).is_null());
        assert!(pdal_stage_create_mongoexpression(cstring("{").as_ptr()).is_null());
    }
}

#[test]
fn metrics_c_abi_reports_distances_and_eval_errors() {
    unsafe {
        let path = cstring(&super::data_path("tile/tile.txt"));
        let mut hausdorff = -1.0;
        let mut modified = -1.0;
        assert_eq!(
            pdal_hausdorff(path.as_ptr(), path.as_ptr(), &mut hausdorff, &mut modified),
            0
        );
        assert_eq!(hausdorff, 0.0);
        assert_eq!(modified, 0.0);

        let mut chamfer = -1.0;
        assert_eq!(pdal_chamfer(path.as_ptr(), path.as_ptr(), &mut chamfer), 0);
        assert_eq!(chamfer, 0.0);

        let delta: serde_json::Value =
            serde_json::from_str(&take_string(pdal_delta(path.as_ptr(), path.as_ptr()))).unwrap();
        assert_eq!(delta["X"]["mean"], 0.0);
        assert_eq!(delta["Y"]["mean"], 0.0);
        assert_eq!(delta["Z"]["mean"], 0.0);

        let detail: serde_json::Value = serde_json::from_str(&take_string(pdal_delta_ex(
            path.as_ptr(),
            path.as_ptr(),
            true,
            false,
        )))
        .unwrap();
        assert_eq!(detail["delta"][0]["i"], 0);
        assert_eq!(detail["delta"][0]["X"], 0.0);

        assert_eq!(
            pdal_hausdorff(
                std::ptr::null(),
                path.as_ptr(),
                &mut hausdorff,
                &mut modified
            ),
            -1
        );
        assert!(pdal_delta(std::ptr::null(), path.as_ptr()).is_null());

        let labels = cstring("2");
        let classification = cstring("Classification");
        assert!(pdal_eval(
            path.as_ptr(),
            path.as_ptr(),
            labels.as_ptr(),
            classification.as_ptr(),
            classification.as_ptr(),
        )
        .is_null());
        assert!(pdal_eval(
            path.as_ptr(),
            path.as_ptr(),
            cstring("not-a-label").as_ptr(),
            classification.as_ptr(),
            classification.as_ptr(),
        )
        .is_null());
    }
}

#[test]
fn utility_abi_environment_and_random() {
    unsafe {
        let key = cstring("PDAL_RUST_ABI_TEST_VAR");
        let val1 = cstring("value_abi_1");
        let val2 = cstring("value_abi_2");

        let initial = pdal_utils_getenv(key.as_ptr());
        assert!(initial.is_null());

        assert_eq!(pdal_utils_setenv(key.as_ptr(), val1.as_ptr()), 0);
        let ret1 = pdal_utils_getenv(key.as_ptr());
        assert!(!ret1.is_null());
        assert_eq!(take_string(ret1), "value_abi_1");

        assert_eq!(pdal_utils_setenv(key.as_ptr(), val2.as_ptr()), 0);
        let ret2 = pdal_utils_getenv(key.as_ptr());
        assert!(!ret2.is_null());
        assert_eq!(take_string(ret2), "value_abi_2");

        assert_eq!(pdal_utils_unsetenv(key.as_ptr()), 0);
        let final_val = pdal_utils_getenv(key.as_ptr());
        assert!(final_val.is_null());

        pdal_utils_random_seed(12345);
        let r1 = pdal_utils_random(0.0, 100.0);
        assert!((0.0..=100.0).contains(&r1));

        pdal_utils_random_seed(12345);
        let r2 = pdal_utils_random(0.0, 100.0);
        assert_eq!(r1, r2);
    }
}
