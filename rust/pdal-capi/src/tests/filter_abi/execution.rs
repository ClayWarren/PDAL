use super::*;

#[test]
fn test_filter_abi_execution_coverage() {
    unsafe {
        let view = xyz_view(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0)]);

        // 1. HeadFilter
        let head = pdal_stage_create_head(options(&[("count", "2")]));
        let out_head = pdal_stage_run(head, view);
        assert_eq!(pdal_point_view_length(out_head), 2);
        pdal_point_view_destroy(out_head);
        pdal_stage_destroy(head);

        // 2. TailFilter
        let tail = pdal_stage_create_tail(options(&[("count", "2")]));
        let out_tail = pdal_stage_run(tail, view);
        assert_eq!(pdal_point_view_length(out_tail), 2);
        pdal_point_view_destroy(out_tail);
        pdal_stage_destroy(tail);

        // 3. LocateFilter
        let locate = pdal_stage_create_locate(options(&[("dimension", "X"), ("minmax", "max")]));
        let out_locate = pdal_stage_run(locate, view);
        assert_eq!(pdal_point_view_length(out_locate), 1);
        pdal_point_view_destroy(out_locate);
        pdal_stage_destroy(locate);

        // 4. RandomizeFilter
        let randomize = pdal_stage_create_randomize(options(&[("seed", "7")]));
        let out_random = pdal_stage_run(randomize, view);
        assert_eq!(pdal_point_view_length(out_random), 3);
        pdal_point_view_destroy(out_random);
        pdal_stage_destroy(randomize);

        // 5. VoxelDownsizeFilter
        let voxel = pdal_stage_create_voxeldownsize(options(&[("cell", "1.0")]));
        let out_voxel = pdal_stage_run(voxel, view);
        assert!(!out_voxel.is_null());
        pdal_point_view_destroy(out_voxel);
        pdal_stage_destroy(voxel);

        // 6. SampleFilter
        let sample = pdal_stage_create_sample(options(&[("radius", "1.0")]));
        let out_sample = pdal_stage_run(sample, view);
        assert!(!out_sample.is_null());
        pdal_point_view_destroy(out_sample);
        pdal_stage_destroy(sample);

        // 7. RangeFilter
        let limit_dim = cstring("X");
        let limit = pdal_range_limit_t {
            dim_name: limit_dim.as_ptr(),
            lower_bound: 0.5,
            upper_bound: 1.5,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        };
        let range = pdal_stage_create_range(&limit, 1);
        assert!(pdal_stage_range_point_passes(range, view, 1));
        let out_range = pdal_stage_run(range, view);
        assert_eq!(pdal_point_view_length(out_range), 1);
        assert_eq!(get(out_range, 0, "X"), 1.0);
        pdal_point_view_destroy(out_range);
        pdal_stage_destroy(range);

        // 8. SortFilter
        let sort_dim_str = cstring("X");
        let sort_dims = [sort_dim_str.as_ptr()];
        let sort_order = cstring("desc");
        let sort_alg = cstring("");
        let sort = pdal_stage_create_sort(
            sort_dims.as_ptr(),
            1,
            sort_order.as_ptr(),
            sort_alg.as_ptr(),
        );
        let out_sort = pdal_stage_run(sort, view);
        assert_eq!(pdal_point_view_length(out_sort), 3);
        assert_eq!(get(out_sort, 0, "X"), 2.0);
        pdal_point_view_destroy(out_sort);
        pdal_stage_destroy(sort);

        // 9. ReturnsFilter
        let only_str = cstring("only");
        let returns_groups = [only_str.as_ptr()];
        let returns = pdal_stage_create_returns(returns_groups.as_ptr(), 1);
        let out_returns = pdal_stage_run(returns, view);
        assert!(!out_returns.is_null());
        pdal_point_view_destroy(out_returns);
        pdal_stage_destroy(returns);

        // 10. DividerFilter
        let evals = [1u8, 0u8, 1u8];
        let divider = pdal_stage_create_divider(1, 0, 2, evals.as_ptr(), 3);
        let out_divider = pdal_stage_run(divider, view);
        assert!(!out_divider.is_null());
        pdal_point_view_destroy(out_divider);
        pdal_stage_destroy(divider);

        // 11. FerryFilter & specs
        let ferry_from_str = cstring("X");
        let ferry_to_str = cstring("Classification");
        let ferry_from = [ferry_from_str.as_ptr()];
        let ferry_to = [ferry_to_str.as_ptr()];
        let ferry = pdal_stage_create_ferry(ferry_from.as_ptr(), ferry_to.as_ptr(), 1);
        pdal_stage_ferry_point(ferry, view, 1);
        let out_ferry = pdal_stage_run(ferry, view);
        assert_eq!(get(out_ferry, 1, "Classification"), 1.0);
        pdal_point_view_destroy(out_ferry);
        pdal_stage_destroy(ferry);

        // 12. GeomDistanceFilter
        let wkt = cstring("POINT(0.5 0.0)");
        let geom_dim = cstring("Classification");
        let geom = pdal_stage_create_geomdistance(wkt.as_ptr(), geom_dim.as_ptr(), false);
        let out_geom = pdal_stage_run(geom, view);
        assert!(!out_geom.is_null());
        pdal_point_view_destroy(out_geom);
        pdal_stage_destroy(geom);

        // 13. ProjPipelineFilter
        let out_srs = cstring("EPSG:4326");
        let coord_op = cstring("+proj=pipeline +step +proj=axisswap +order=2,1");
        let proj = pdal_stage_create_projpipeline(out_srs.as_ptr(), coord_op.as_ptr(), false);
        let out_proj = pdal_stage_run(proj, view);
        assert!(!out_proj.is_null());
        pdal_point_view_destroy(out_proj);
        pdal_stage_destroy(proj);

        // 14. GroupByFilter
        let groupby_dim = cstring("X");
        let groupby = pdal_stage_create_groupby(groupby_dim.as_ptr());
        let out_groupby = pdal_stage_run(groupby, view);
        assert!(!out_groupby.is_null());
        pdal_point_view_destroy(out_groupby);
        pdal_stage_destroy(groupby);

        // 15. LabelDuplicatesFilter
        let label_dim_str = cstring("X");
        let label_dims = [label_dim_str.as_ptr()];
        let label = pdal_stage_create_labelduplicates(label_dims.as_ptr(), 1);
        let out_label = pdal_stage_run(label, view);
        assert!(!out_label.is_null());
        pdal_point_view_destroy(out_label);
        pdal_stage_destroy(label);

        // 16. TransformationFilter
        let matrix = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 2.0, 3.0, 1.0,
        ];
        let xform = pdal_stage_create_transformation(matrix.as_ptr());
        pdal_stage_transformation_point(xform, view, 1);
        let out_xform = pdal_stage_run(xform, view);
        assert!(!out_xform.is_null());
        pdal_point_view_destroy(out_xform);
        pdal_stage_destroy(xform);

        // 17. GpsTimeConvert
        let gps_ops = options(&[("conversion", "gws2gt"), ("start_date", "2020-01-08")]);
        let gps = pdal_stage_create_gpstimeconvert(gps_ops);
        let out_gps = pdal_stage_run(gps, view);
        assert!(!out_gps.is_null());
        pdal_point_view_destroy(out_gps);
        pdal_stage_destroy(gps);
        pdal_options_destroy(gps_ops);

        // 18. NeighborClassifierFilter
        let nc_dim_str = cstring("X");
        let nc_limit = pdal_range_limit_t {
            dim_name: nc_dim_str.as_ptr(),
            lower_bound: 0.5,
            upper_bound: 1.5,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        };
        let class_name = cstring("Classification");
        let nc = pdal_stage_create_neighborclassifier(&nc_limit, 1, 2, class_name.as_ptr());
        let out_nc = pdal_stage_run(nc, view);
        assert!(!out_nc.is_null());
        pdal_point_view_destroy(out_nc);
        pdal_stage_destroy(nc);

        pdal_point_view_destroy(view);
    }
}

#[test]
fn test_geo_filters_execution() {
    unsafe {
        pdal_core::gdal::register_drivers();

        // --- 1. ColorizationFilter ---
        // Setup point layout registering X, Y, Z, Red
        let layout1 = pdal_point_layout_create();
        for dim in ["X", "Y", "Z", "Red"] {
            let name = cstring(dim);
            pdal_point_layout_register_dim(layout1, name.as_ptr(), 9);
        }
        let view1 = pdal_point_view_create(layout1);
        let idx1 = pdal_point_view_add_point(view1);
        pdal_point_view_set_f64(view1, idx1, cstring("X").as_ptr(), 440750.0);
        pdal_point_view_set_f64(view1, idx1, cstring("Y").as_ptr(), 3751290.0);
        pdal_point_view_set_f64(view1, idx1, cstring("Z").as_ptr(), 200.0);
        pdal_point_view_set_f64(view1, idx1, cstring("Red").as_ptr(), 0.0);

        let raster_path = cstring(&data_path("gdal/int32.tif"));
        let red_dim = cstring("Red");
        let bands = [pdal_band_info_t {
            name: red_dim.as_ptr(),
            band: 1,
            scale: 1.0,
        }];
        let colorization = pdal_stage_create_colorization(raster_path.as_ptr(), bands.as_ptr(), 1);
        assert!(!colorization.is_null());

        let out_colorization = pdal_stage_run(colorization, view1);
        assert!(!out_colorization.is_null());
        assert_eq!(pdal_point_view_length(out_colorization), 1);
        let red_val = get(out_colorization, 0, "Red");
        assert_eq!(red_val, 107.0);

        pdal_point_view_destroy(out_colorization);
        pdal_stage_destroy(colorization);
        pdal_point_view_destroy(view1);

        // --- 2. OverlayFilter ---
        // Setup point layout registering X, Y, Z, Classification
        let layout2 = pdal_point_layout_create();
        for dim in ["X", "Y", "Z", "Classification"] {
            let name = cstring(dim);
            pdal_point_layout_register_dim(layout2, name.as_ptr(), 9);
        }
        let view2 = pdal_point_view_create(layout2);
        let idx2 = pdal_point_view_add_point(view2);
        // A point inside the first polygon feature of attributes.json:
        // X = -123.065, Y = 44.058, Z = 0.0
        pdal_point_view_set_f64(view2, idx2, cstring("X").as_ptr(), -123.065);
        pdal_point_view_set_f64(view2, idx2, cstring("Y").as_ptr(), 44.058);
        pdal_point_view_set_f64(view2, idx2, cstring("Z").as_ptr(), 0.0);
        pdal_point_view_set_f64(view2, idx2, cstring("Classification").as_ptr(), 0.0);

        let overlay_ds = cstring(&data_path("autzen/attributes.json"));
        let class_dim = cstring("Classification");
        let cls_col = cstring("cls");
        let overlay =
            pdal_stage_create_overlay(class_dim.as_ptr(), overlay_ds.as_ptr(), cls_col.as_ptr());
        assert!(!overlay.is_null());

        let out_overlay = pdal_stage_run(overlay, view2);
        assert!(!out_overlay.is_null());
        assert_eq!(pdal_point_view_length(out_overlay), 1);
        let class_val = get(out_overlay, 0, "Classification");
        assert_eq!(class_val, 2.0);

        pdal_point_view_destroy(out_overlay);
        pdal_stage_destroy(overlay);
        pdal_point_view_destroy(view2);

        check_overlay_bounds_filter_excludes_point(
            class_dim.as_ptr(),
            overlay_ds.as_ptr(),
            cls_col.as_ptr(),
        );

        // --- 3. HagDemFilter ---
        // Setup point layout registering X, Y, Z, HeightAboveGround
        let layout3 = pdal_point_layout_create();
        for dim in ["X", "Y", "Z", "HeightAboveGround"] {
            let name = cstring(dim);
            pdal_point_layout_register_dim(layout3, name.as_ptr(), 9);
        }
        let view3 = pdal_point_view_create(layout3);
        let idx3 = pdal_point_view_add_point(view3);
        pdal_point_view_set_f64(view3, idx3, cstring("X").as_ptr(), 440750.0);
        pdal_point_view_set_f64(view3, idx3, cstring("Y").as_ptr(), 3751290.0);
        pdal_point_view_set_f64(view3, idx3, cstring("Z").as_ptr(), 200.0);
        pdal_point_view_set_f64(view3, idx3, cstring("HeightAboveGround").as_ptr(), 0.0);

        let hag_dem =
            pdal_stage_create_hag_dem(raster_path.as_ptr(), 1, false, 0.0, 1000.0, -9999.0, 2);
        assert!(!hag_dem.is_null());

        let out_hag = pdal_stage_run(hag_dem, view3);
        assert!(!out_hag.is_null());
        assert_eq!(pdal_point_view_length(out_hag), 1);
        let hag_val = get(out_hag, 0, "HeightAboveGround");
        assert_eq!(hag_val, 93.0); // 200.0 - 107.0 = 93.0

        pdal_point_view_destroy(out_hag);
        pdal_stage_destroy(hag_dem);
        pdal_point_view_destroy(view3);

        // HagDemFilter zero_ground path:
        let layout4 = pdal_point_layout_create();
        for dim in ["X", "Y", "Z", "Classification", "HeightAboveGround"] {
            let name = cstring(dim);
            pdal_point_layout_register_dim(layout4, name.as_ptr(), 9);
        }
        let view4 = pdal_point_view_create(layout4);
        let idx4 = pdal_point_view_add_point(view4);
        pdal_point_view_set_f64(view4, idx4, cstring("X").as_ptr(), 440750.0);
        pdal_point_view_set_f64(view4, idx4, cstring("Y").as_ptr(), 3751290.0);
        pdal_point_view_set_f64(view4, idx4, cstring("Z").as_ptr(), 200.0);
        pdal_point_view_set_f64(view4, idx4, cstring("Classification").as_ptr(), 2.0);
        pdal_point_view_set_f64(view4, idx4, cstring("HeightAboveGround").as_ptr(), -1.0);

        let hag_dem_zero =
            pdal_stage_create_hag_dem(raster_path.as_ptr(), 1, true, 0.0, 1000.0, -9999.0, 2);
        assert!(!hag_dem_zero.is_null());

        let out_hag_zero = pdal_stage_run(hag_dem_zero, view4);
        assert!(!out_hag_zero.is_null());
        assert_eq!(pdal_point_view_length(out_hag_zero), 1);
        let hag_val_zero = get(out_hag_zero, 0, "HeightAboveGround");
        assert_eq!(hag_val_zero, 0.0);

        pdal_point_view_destroy(out_hag_zero);
        pdal_stage_destroy(hag_dem_zero);
        pdal_point_view_destroy(view4);

        // HagDemFilter failed raster load path:
        let layout5 = pdal_point_layout_create();
        for dim in ["X", "Y", "Z", "HeightAboveGround"] {
            let name = cstring(dim);
            pdal_point_layout_register_dim(layout5, name.as_ptr(), 9);
        }
        let view5 = pdal_point_view_create(layout5);
        let idx5 = pdal_point_view_add_point(view5);
        pdal_point_view_set_f64(view5, idx5, cstring("HeightAboveGround").as_ptr(), -42.0);

        let bad_raster = cstring("nonexistent.tif");
        let hag_dem_bad =
            pdal_stage_create_hag_dem(bad_raster.as_ptr(), 1, false, 0.0, 1000.0, -9999.0, 2);
        assert!(!hag_dem_bad.is_null());
        let out_hag_bad = pdal_stage_run(hag_dem_bad, view5);
        // nonexistent.tif load failure will result in pipeline execution error and returns null
        assert!(out_hag_bad.is_null());

        pdal_point_view_destroy(view5);
        pdal_stage_destroy(hag_dem_bad);
    }
}

unsafe fn check_overlay_bounds_filter_excludes_point(
    class_dim: *const std::os::raw::c_char,
    overlay_ds: *const std::os::raw::c_char,
    cls_col: *const std::os::raw::c_char,
) {
    let bounded_layout = pdal_point_layout_create();
    for dim in ["X", "Y", "Z", "Classification"] {
        let name = cstring(dim);
        pdal_point_layout_register_dim(bounded_layout, name.as_ptr(), 9);
    }
    let bounded_view = pdal_point_view_create(bounded_layout);
    let idx = pdal_point_view_add_point(bounded_view);
    pdal_point_view_set_f64(bounded_view, idx, cstring("X").as_ptr(), -123.065);
    pdal_point_view_set_f64(bounded_view, idx, cstring("Y").as_ptr(), 44.058);
    pdal_point_view_set_f64(bounded_view, idx, cstring("Z").as_ptr(), 0.0);
    pdal_point_view_set_f64(bounded_view, idx, cstring("Classification").as_ptr(), 0.0);

    let far_bounds = cstring("POLYGON((0 0,0 1,1 1,1 0,0 0))");
    let bounded_overlay = pdal_stage_create_overlay_with_options(
        class_dim,
        overlay_ds,
        cls_col,
        std::ptr::null(),
        std::ptr::null(),
        far_bounds.as_ptr(),
    );
    assert!(!bounded_overlay.is_null());
    let out = pdal_stage_run(bounded_overlay, bounded_view);
    assert!(!out.is_null());
    assert_eq!(get(out, 0, "Classification"), 0.0);
    pdal_point_view_destroy(out);
    pdal_stage_destroy(bounded_overlay);
    pdal_point_view_destroy(bounded_view);
}
