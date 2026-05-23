use super::*;
use crate::stage_abi::StageWrapper;
use pdal_core::options::Options;
use pdal_core::point::PointView;

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn options(pairs: &[(&str, &str)]) -> *mut Options {
    let options = pdal_options_create();
    for (key, value) in pairs {
        let key = cstring(key);
        let value = cstring(value);
        pdal_options_add_str(options, key.as_ptr(), value.as_ptr());
    }
    options
}

unsafe fn xyz_view(points: &[(f64, f64, f64)]) -> *mut PointView {
    let layout = pdal_point_layout_create();
    for dim in [
        "X",
        "Y",
        "Z",
        "Classification",
        "Intensity",
        "GpsTime",
        "Flag",
        "ReturnNumber",
        "NumberOfReturns",
    ] {
        let name = cstring(dim);
        pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
    }
    let view = pdal_point_view_create(layout);
    for (x, y, z) in points {
        let idx = pdal_point_view_add_point(view);
        for (dim, value) in [
            ("X", *x),
            ("Y", *y),
            ("Z", *z),
            ("ReturnNumber", 1.0),
            ("NumberOfReturns", 1.0),
        ] {
            let name = cstring(dim);
            pdal_point_view_set_f64(view, idx, name.as_ptr(), value);
        }
    }
    view
}

unsafe fn get(view: *mut PointView, idx: u64, dim: &str) -> f64 {
    let dim = cstring(dim);
    pdal_point_view_get_f64(view, idx, dim.as_ptr())
}

unsafe fn destroy_stage(stage: *mut StageWrapper) {
    assert!(!stage.is_null());
    pdal_stage_destroy(stage);
}

#[test]
fn option_backed_filter_stages_construct_and_run_through_c_abi() {
    unsafe {
        let view = xyz_view(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0)]);

        let decimation_options = options(&[("step", "2")]);
        let decimation = pdal_stage_create_decimation(decimation_options);
        let decimated = pdal_stage_run(decimation, view);
        assert_eq!(pdal_point_view_length(decimated), 2);
        assert_eq!(get(decimated, 1, "X"), 2.0);

        pdal_point_view_destroy(decimated);
        pdal_stage_destroy(decimation);
        pdal_options_destroy(decimation_options);

        for stage in [
            pdal_stage_create_head(options(&[("count", "2")])),
            pdal_stage_create_tail(options(&[("count", "2")])),
            pdal_stage_create_locate(options(&[("dimension", "X"), ("minmax", "max")])),
            pdal_stage_create_randomize(options(&[("seed", "7")])),
            pdal_stage_create_voxeldownsize(options(&[("cell", "1.0")])),
            pdal_stage_create_sample(options(&[("radius", "1.0")])),
            pdal_stage_create_faceraster(options(&[])),
            pdal_stage_create_gpstimeconvert(options(&[
                ("conversion", "gws2gt"),
                ("start_date", "2020-01-08"),
            ])),
        ] {
            destroy_stage(stage);
        }

        assert!(pdal_stage_create_decimation(std::ptr::null()).is_null());
        assert!(pdal_stage_create_head(std::ptr::null()).is_null());
        assert!(pdal_stage_create_tail(std::ptr::null()).is_null());
        assert!(pdal_stage_create_locate(std::ptr::null()).is_null());
        assert!(pdal_stage_create_randomize(std::ptr::null()).is_null());
        assert!(pdal_stage_create_voxeldownsize(std::ptr::null()).is_null());
        assert!(pdal_stage_create_sample(std::ptr::null()).is_null());

        pdal_point_view_destroy(view);
    }
}

#[test]
fn string_and_array_filter_stages_construct_through_c_abi() {
    unsafe {
        let x = cstring("X");
        let y = cstring("Y");
        let z = cstring("Z");
        let class = cstring("Classification");
        let flag = cstring("Flag");
        let asc = cstring("asc");
        let stable = cstring("stable");
        let first = cstring("first");
        let polygon = cstring("POLYGON ((0 0, 0 10, 10 10, 10 0, 0 0))");
        let point = cstring("POINT (0 0)");
        let datasource = cstring("attributes.json");
        let ramp = cstring("pestel_shades");
        let raster = cstring("raster.tif");
        let coord_op = cstring("+proj=noop");
        let red = cstring("Red");

        let dims = [x.as_ptr(), y.as_ptr()];
        let groups = [first.as_ptr()];
        let ranges = [pdal_range_limit_t {
            dim_name: x.as_ptr(),
            lower_bound: 0.0,
            upper_bound: 10.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
        let assignments = [pdal_assign_range_t {
            dim_name: flag.as_ptr(),
            value: 9.0,
            lower_bound: 0.0,
            upper_bound: 10.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
        let bands = [pdal_band_info_t {
            name: red.as_ptr(),
            band: 1,
            scale: 1.0,
        }];
        let bounds = [pdal_box3d_t {
            minx: -1.0,
            miny: -1.0,
            minz: -1.0,
            maxx: 1.0,
            maxy: 1.0,
            maxz: 1.0,
        }];
        let centers = [pdal_point3d_t {
            x: 0.0,
            y: 0.0,
            z: f64::NAN,
        }];
        let matrix = [
            1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 3.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let evals = [true as u8, false as u8, true as u8];

        for stage in [
            pdal_stage_create_crop(
                false,
                bounds.as_ptr(),
                bounds.len() as u64,
                std::ptr::null(),
                0,
                centers.as_ptr(),
                centers.len() as u64,
                1.0,
            ),
            pdal_stage_create_overlay(class.as_ptr(), datasource.as_ptr(), std::ptr::null()),
            pdal_stage_create_colorinterp(x.as_ptr(), ramp.as_ptr(), 0.0, 10.0, true, false),
            pdal_stage_create_colorization(raster.as_ptr(), bands.as_ptr(), bands.len() as u64),
            pdal_stage_create_hag_dem(raster.as_ptr(), 1, true, 0.0, 10.0, -9999.0, 2),
            pdal_stage_create_ferry(dims.as_ptr(), [z.as_ptr(), flag.as_ptr()].as_ptr(), 2),
            pdal_stage_create_range(ranges.as_ptr(), ranges.len() as u64),
            pdal_stage_create_sort(
                dims.as_ptr(),
                dims.len() as u64,
                asc.as_ptr(),
                stable.as_ptr(),
            ),
            pdal_stage_create_returns(groups.as_ptr(), groups.len() as u64),
            pdal_stage_create_separatescanline(2),
            pdal_stage_create_geomdistance(point.as_ptr(), flag.as_ptr(), false),
            pdal_stage_create_projpipeline(cstring("EPSG:4326").as_ptr(), coord_op.as_ptr(), false),
            pdal_stage_create_groupby(class.as_ptr()),
            pdal_stage_create_labelduplicates(dims.as_ptr(), dims.len() as u64),
            pdal_stage_create_transformation(matrix.as_ptr()),
            pdal_stage_create_divider(2, 1, 2, evals.as_ptr(), evals.len() as u64),
            pdal_stage_create_assign(
                true,
                x.as_ptr(),
                0.0,
                10.0,
                true,
                true,
                false,
                assignments.as_ptr(),
                assignments.len() as u64,
            ),
            pdal_stage_create_radiusassign(
                ranges.as_ptr(),
                ranges.len() as u64,
                ranges.as_ptr(),
                ranges.len() as u64,
                assignments.as_ptr(),
                assignments.len() as u64,
                1.0,
                true,
                0.0,
                0.0,
            ),
            pdal_stage_create_neighborclassifier(
                ranges.as_ptr(),
                ranges.len() as u64,
                2,
                class.as_ptr(),
            ),
        ] {
            destroy_stage(stage);
        }

        let polygon_stage = pdal_stage_create_crop(
            false,
            std::ptr::null(),
            0,
            &polygon.as_ptr(),
            1,
            std::ptr::null(),
            0,
            1.0,
        );
        destroy_stage(polygon_stage);
        assert!(
            pdal_stage_create_overlay(std::ptr::null(), datasource.as_ptr(), std::ptr::null())
                .is_null()
        );
        assert!(pdal_stage_create_colorinterp(
            std::ptr::null(),
            ramp.as_ptr(),
            0.0,
            1.0,
            false,
            false
        )
        .is_null());
        assert!(pdal_stage_create_colorization(std::ptr::null(), bands.as_ptr(), 1).is_null());
        assert!(
            pdal_stage_create_hag_dem(std::ptr::null(), 1, false, 0.0, 0.0, -9999.0, 2).is_null()
        );
        assert!(pdal_stage_create_ferry(std::ptr::null(), dims.as_ptr(), 1).is_null());
        assert!(pdal_stage_create_range(std::ptr::null(), 1).is_null());
        assert!(
            pdal_stage_create_sort(std::ptr::null(), 0, asc.as_ptr(), stable.as_ptr()).is_null()
        );
        assert!(pdal_stage_create_returns(std::ptr::null(), 1).is_null());
        assert!(pdal_stage_create_groupby(std::ptr::null()).is_null());
        assert!(pdal_stage_create_labelduplicates(std::ptr::null(), 1).is_null());
        assert!(pdal_stage_create_transformation(std::ptr::null()).is_null());
        assert!(pdal_stage_create_zsmooth(1.0, 0.5, std::ptr::null()).is_null());
        assert!(pdal_stage_create_outlier(std::ptr::null(), 2, 1.0, 2, 1.0, 7).is_null());
        assert!(pdal_stage_create_dbscan(2, 1.0, std::ptr::null(), 1).is_null());
    }
}

#[test]
fn spatial_and_statistical_filter_stages_construct_through_c_abi() {
    unsafe {
        let x = cstring("X");
        let dims = [x.as_ptr()];
        let method = cstring("radius");
        let last = cstring("last");
        let returns = [last.as_ptr()];

        for stage in [
            pdal_stage_create_h3(5),
            pdal_stage_create_merge(),
            pdal_stage_create_mortonorder(false),
            pdal_stage_create_radialdensity(1.0),
            pdal_stage_create_nndistance(3, cstring("avg").as_ptr()),
            pdal_stage_create_zsmooth(1.0, 0.5, x.as_ptr()),
            pdal_stage_create_outlier(method.as_ptr(), 2, 1.0, 8, 2.0, 7),
            pdal_stage_create_dbscan(2, 1.0, dims.as_ptr(), dims.len() as u64),
            pdal_stage_create_lof(3),
            pdal_stage_create_elm(10.0, 7, 1.0),
            pdal_stage_create_smrf(
                1.0,
                0.2,
                true,
                16.0,
                1.25,
                0.45,
                2,
                1,
                false,
                returns.as_ptr(),
                returns.len() as u64,
            ),
            pdal_stage_create_skewnessbalancing(2, 1, false),
            pdal_stage_create_iqr(1.5, x.as_ptr()),
            pdal_stage_create_mad(2.5, x.as_ptr(), 1.4826),
            pdal_stage_create_hagnn(4, 0.0, false, 2),
            pdal_stage_create_cluster(1, u64::MAX, 1.0, true),
            pdal_stage_create_sparsesurface(1.0, 2, 7),
            pdal_stage_create_voxelcenternearestneighbor(1.0),
            pdal_stage_create_voxelcentroidnearestneighbor(1.0),
            pdal_stage_create_reciprocity(3),
            pdal_stage_create_estimaterank(4, 0.1),
            pdal_stage_create_approximatecoplanar(4, 0.1, 0.1),
            pdal_stage_create_planefit(4),
            pdal_stage_create_eigenvalues(4, true, 1, true, 1.0, 3),
            pdal_stage_create_optimalneighborhood(3, 8),
            pdal_stage_create_splitter(10.0, 0.0, 0.0, 0.0),
            pdal_stage_create_chipper(15),
            pdal_stage_create_farthestpointsampling(2),
        ] {
            destroy_stage(stage);
        }

        let bad_returns = [std::ptr::null()];
        assert!(pdal_stage_create_smrf(
            1.0,
            0.2,
            false,
            0.0,
            1.25,
            0.45,
            2,
            1,
            false,
            bad_returns.as_ptr(),
            1
        )
        .is_null());
        assert!(pdal_stage_create_iqr(1.5, std::ptr::null()).is_null());
        assert!(pdal_stage_create_mad(2.5, std::ptr::null(), 1.4826).is_null());
    }
}

#[test]
fn stage_runtime_helpers_cover_error_and_multi_output_paths() {
    unsafe {
        assert!(!pdal_stage_process_one(std::ptr::null_mut()));
        assert!(pdal_stage_run(std::ptr::null_mut(), std::ptr::null_mut()).is_null());

        let view = xyz_view(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0)]);
        let stage = pdal_stage_create_head(options(&[("count", "2")]));
        assert!(pdal_stage_process_one(stage));
        pdal_stage_reset(stage);
        assert!(pdal_stage_process_one_at(stage, view, 0));

        let mut outputs = [std::ptr::null_mut(); 4];
        let count = pdal_stage_run_multi(stage, view, outputs.as_mut_ptr(), outputs.len() as u64);
        assert_eq!(count, 1);
        assert_eq!(pdal_point_view_length(outputs[0]), 2);

        let metadata = pdal_stage_metadata(stage);
        assert!(!metadata.is_null());
        pdal_metadata_node_destroy(metadata);

        pdal_point_view_destroy(outputs[0]);
        pdal_stage_destroy(stage);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn more_spatial_stats_filters_constructors_through_c_abi() {
    unsafe {
        let x = cstring("X");
        let y = cstring("Y");
        let dims = [x.as_ptr(), y.as_ptr()];

        // 1. Covariance Features
        let cov =
            pdal_stage_create_covariancefeatures(10, true, 2.0, 3, 1, 0, true, dims.as_ptr(), 2);
        destroy_stage(cov);

        let cov_null = pdal_stage_create_covariancefeatures(
            10,
            false,
            0.0,
            3,
            1,
            0,
            false,
            std::ptr::null(),
            0,
        );
        destroy_stage(cov_null);

        // 2. Normal
        let norm = pdal_stage_create_normal(8, true, 1.5, true, 0.0, 0.0, 10.0, true);
        destroy_stage(norm);

        // 3. Relaxation Dart Throwing
        let rdt = pdal_stage_create_relaxationdartthrowing(0.5, 2.0, 0.5, 100, true, true, 12345);
        destroy_stage(rdt);

        // 4. Straighten
        let line_wkt = cstring("LINESTRING ZM (0 0 0 0, 10 10 10 10)");
        let str_filt = pdal_stage_create_straighten(line_wkt.as_ptr(), false, 0.0);
        destroy_stage(str_filt);

        let str_bad = pdal_stage_create_straighten(cstring("invalid wkt").as_ptr(), false, 0.0);
        assert!(str_bad.is_null());

        assert!(pdal_stage_create_straighten(std::ptr::null(), false, 0.0).is_null());

        // 5. Lloyd KMeans
        let lloyd = pdal_stage_create_lloydkmeans(3, 100, dims.as_ptr(), 2);
        destroy_stage(lloyd);

        let lloyd_null = pdal_stage_create_lloydkmeans(3, 100, std::ptr::null(), 0);
        destroy_stage(lloyd_null);

        // 6. Miniball
        let mb = pdal_stage_create_miniball(5);
        destroy_stage(mb);

        // 7. DBSCAN dims null/invalid elements
        assert!(pdal_stage_create_dbscan(2, 1.0, std::ptr::null(), 0).is_null());
        let bad_dims = [std::ptr::null()];
        assert!(pdal_stage_create_dbscan(2, 1.0, bad_dims.as_ptr(), 1).is_null());
    }
}

#[test]
fn test_filters_abi_error_and_invalid_paths() {
    unsafe {
        // 1. pdal_stage_create_geomdistance nulls
        assert!(pdal_stage_create_geomdistance(std::ptr::null(), std::ptr::null(), false).is_null());

        // 2. pdal_stage_create_projpipeline nulls
        assert!(pdal_stage_create_projpipeline(std::ptr::null(), std::ptr::null(), false).is_null());

        // 3. pdal_stage_create_groupby null
        assert!(pdal_stage_create_groupby(std::ptr::null()).is_null());

        // 4. pdal_stage_create_labelduplicates nulls
        assert!(pdal_stage_create_labelduplicates(std::ptr::null(), 1).is_null());
        let bad_ptrs = [std::ptr::null()];
        assert!(pdal_stage_create_labelduplicates(bad_ptrs.as_ptr(), 1).is_null());

        // 5. pdal_stage_merge_append null noop
        pdal_stage_merge_append(std::ptr::null_mut(), std::ptr::null_mut());

        // 6. pdal_stage_create_transformation null
        assert!(pdal_stage_create_transformation(std::ptr::null()).is_null());

        // 7. pdal_stage_transformation_point null noop
        pdal_stage_transformation_point(std::ptr::null_mut(), std::ptr::null_mut(), 0);

        // 8. pdal_transformation_matrix_parse null and bad
        assert!(!pdal_transformation_matrix_parse(std::ptr::null(), std::ptr::null_mut()).is_null());
        let mut mat = [0.0f64; 16];
        let err = take_string(pdal_transformation_matrix_parse(
            cstring("1 2 3").as_ptr(),
            mat.as_mut_ptr(),
        ));
        assert!(!err.is_empty());

        // 9. pdal_transformation_matrix_format null
        let formatted = pdal_transformation_matrix_format(std::ptr::null());
        assert!(!formatted.is_null());
        assert_eq!(take_string(formatted), "");

        // 10. pdal_georeference_validate_coordinate_system null & bad
        assert!(!pdal_georeference_validate_coordinate_system(std::ptr::null()).is_null());
        let bad_sys = take_string(pdal_georeference_validate_coordinate_system(
            cstring("invalid-srs").as_ptr(),
        ));
        assert!(!bad_sys.is_empty());

        // 11. pdal_georeference_validate_transform_beam null
        assert!(!pdal_georeference_validate_transform_beam(std::ptr::null(), true).is_null());

        // 12. pdal_stage_create_divider capacity=0 with Capacity mode
        assert!(pdal_stage_create_divider(0, 1, 0, std::ptr::null(), 0).is_null());

        // 13. pdal_stage_create_gpstimeconvert null
        let gps = pdal_stage_create_gpstimeconvert(std::ptr::null());
        assert!(gps.is_null());

        // 14. pdal_stage_create_assign null cond_dim & null assignments
        let assign_null_cond = pdal_stage_create_assign(
            true,
            std::ptr::null(),
            0.0,
            10.0,
            true,
            true,
            false,
            std::ptr::null(),
            0,
        );
        assert!(!assign_null_cond.is_null());
        pdal_stage_destroy(assign_null_cond);

        // 15. pdal_stage_create_radiusassign invalid domains and update_expression=0
        assert!(pdal_stage_create_radiusassign(
            std::ptr::null(),
            1,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            1.0,
            true,
            0.0,
            0.0
        )
        .is_null());

        let valid_limit = pdal_range_limit_t {
            dim_name: cstring("X").as_ptr(),
            lower_bound: 0.0,
            upper_bound: 1.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        };
        assert!(pdal_stage_create_radiusassign(
            &valid_limit,
            1,
            std::ptr::null(),
            1,
            std::ptr::null(),
            0,
            1.0,
            true,
            0.0,
            0.0
        )
        .is_null());
        assert!(pdal_stage_create_radiusassign(
            &valid_limit,
            1,
            &valid_limit,
            1,
            std::ptr::null(),
            0,
            1.0,
            true,
            0.0,
            0.0
        )
        .is_null());

        let bad_limit = pdal_range_limit_t {
            dim_name: std::ptr::null(),
            lower_bound: 0.0,
            upper_bound: 1.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        };
        assert!(pdal_stage_create_radiusassign(
            &bad_limit,
            1,
            &valid_limit,
            1,
            std::ptr::null(),
            0,
            1.0,
            true,
            0.0,
            0.0
        )
        .is_null());

        let bad_assign = pdal_assign_range_t {
            dim_name: std::ptr::null(),
            value: 1.0,
            lower_bound: 0.0,
            upper_bound: 1.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        };
        assert!(pdal_stage_create_radiusassign(
            &valid_limit,
            1,
            &valid_limit,
            1,
            &bad_assign,
            1,
            1.0,
            true,
            0.0,
            0.0
        )
        .is_null());

        // 16. pdal_stage_create_neighborclassifier errors
        assert!(
            pdal_stage_create_neighborclassifier(std::ptr::null(), 1, 2, std::ptr::null()).is_null()
        );
        assert!(pdal_stage_create_neighborclassifier(&bad_limit, 1, 2, std::ptr::null()).is_null());
        let nc = pdal_stage_create_neighborclassifier(std::ptr::null(), 0, 2, std::ptr::null());
        assert!(!nc.is_null());
        pdal_stage_destroy(nc);
    }
}

#[test]
fn test_filter_abi_nulls_and_errors() {
    unsafe {
        // --- filter_abi.rs ---
        assert!(pdal_stage_create_geomdistance(std::ptr::null(), std::ptr::null(), false).is_null());
        assert!(pdal_stage_create_geomdistance(CString::new("POINT(0 0)").unwrap().as_ptr(), std::ptr::null(), false).is_null());
        assert!(pdal_stage_create_geomdistance(std::ptr::null(), CString::new("X").unwrap().as_ptr(), false).is_null());
        
        assert!(pdal_stage_create_projpipeline(std::ptr::null(), std::ptr::null(), false).is_null());
        assert!(pdal_stage_create_projpipeline(CString::new("EPSG:4326").unwrap().as_ptr(), std::ptr::null(), false).is_null());
        assert!(pdal_stage_create_projpipeline(std::ptr::null(), CString::new("+proj=utm").unwrap().as_ptr(), false).is_null());
        
        assert!(pdal_stage_create_groupby(std::ptr::null()).is_null());
        
        assert!(pdal_stage_create_labelduplicates(std::ptr::null(), 0).is_null());
        let bad_dims = [std::ptr::null()];
        assert!(pdal_stage_create_labelduplicates(bad_dims.as_ptr(), 1).is_null());
        
        pdal_stage_merge_append(std::ptr::null_mut(), std::ptr::null_mut());
        
        assert!(pdal_stage_create_transformation(std::ptr::null()).is_null());
        pdal_stage_transformation_point(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        
        let mut matrix_out = [0.0; 16];
        let parse_err = pdal_transformation_matrix_parse(std::ptr::null(), matrix_out.as_mut_ptr());
        assert!(!parse_err.is_null());
        pdal_string_free(parse_err);
        
        let format_err = pdal_transformation_matrix_format(std::ptr::null());
        assert!(!format_err.is_null());
        assert_eq!(take_string(format_err), "");
        
        let geo_err = pdal_georeference_validate_coordinate_system(std::ptr::null());
        assert!(!geo_err.is_null());
        assert_eq!(take_string(geo_err), "Missing coordinate system.");
        let beam_err = pdal_georeference_validate_transform_beam(std::ptr::null(), false);
        assert!(!beam_err.is_null());
        assert_eq!(take_string(beam_err), "Missing point layout.");
        
        let div = pdal_stage_create_divider(0, 0, 0, std::ptr::null(), 0);
        assert!(!div.is_null());
        pdal_stage_destroy(div);
        
        let bad_evals = [0u8];
        let div2 = pdal_stage_create_divider(0, 0, 0, bad_evals.as_ptr(), 1);
        assert!(!div2.is_null());
        pdal_stage_destroy(div2);

        assert!(pdal_stage_create_divider(0, 1, 0, std::ptr::null(), 0).is_null());
        
        assert!(pdal_stage_create_gpstimeconvert(std::ptr::null()).is_null());
        
        assert!(pdal_stage_create_radiusassign(std::ptr::null(), 1, std::ptr::null(), 1, std::ptr::null(), 1, 1.0, false, 0.0, 0.0).is_null());

        // --- filter_abi_basic.rs ---
        assert!(pdal_stage_create_head(std::ptr::null()).is_null());
        assert!(pdal_stage_create_tail(std::ptr::null()).is_null());
        assert!(pdal_stage_create_locate(std::ptr::null()).is_null());
        assert!(pdal_stage_create_ferry(std::ptr::null(), std::ptr::null(), 0).is_null());
        assert!(pdal_stage_create_ferry_specs(std::ptr::null(), 0).is_null());
        assert!(!pdal_stage_validate_assign_statement(std::ptr::null()));
        pdal_stage_ferry_point(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        assert!(pdal_stage_create_randomize(std::ptr::null()).is_null());
        
        let limit_err = pdal_range_limit_parse(
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert!(!limit_err.is_null());
        pdal_string_free(limit_err);
        
        assert!(pdal_stage_create_range(std::ptr::null(), 0).is_null());
        assert!(!pdal_stage_range_point_passes(std::ptr::null_mut(), std::ptr::null_mut(), 0));
        assert!(pdal_stage_create_sort(std::ptr::null(), 0, std::ptr::null(), std::ptr::null()).is_null());
        assert!(pdal_stage_create_returns(std::ptr::null(), 0).is_null());
        
        // --- filter_abi_geo.rs ---
        assert!(pdal_stage_create_decimation(std::ptr::null()).is_null());
        let crop = pdal_stage_create_crop(false, std::ptr::null(), 0, std::ptr::null(), 0, std::ptr::null(), 0, 0.0);
        assert!(!crop.is_null());
        pdal_stage_destroy(crop);
        
        let invalid_wkt = CString::new("INVALID WKT").unwrap();
        let bad_poly = [invalid_wkt.as_ptr()];
        assert!(pdal_stage_create_crop(
            false,
            std::ptr::null(),
            0,
            bad_poly.as_ptr(),
            1,
            std::ptr::null(),
            0,
            0.0
        ).is_null());
        assert!(pdal_stage_create_overlay(std::ptr::null(), std::ptr::null(), std::ptr::null()).is_null());
        assert!(pdal_stage_create_colorinterp(std::ptr::null(), std::ptr::null(), 0.0, 0.0, false, false).is_null());
        assert_eq!(
            take_string(pdal_colorinterp_validate_prepared(std::ptr::null(), std::ptr::null(), 0.0, 0.0)),
            "Missing colorinterp layout."
        );
        assert!(pdal_colorinterp_pipeline_streamable(0.0, 0.0));
        assert!(pdal_stage_create_colorization(std::ptr::null(), std::ptr::null(), 0).is_null());
        assert!(pdal_stage_create_hag_dem(std::ptr::null(), 0, false, 0.0, 0.0, 0.0, 0).is_null());
        
        // --- filter_abi_spatial_stats.rs ---
        assert!(pdal_stage_create_voxeldownsize(std::ptr::null()).is_null());
        assert!(pdal_stage_create_sample(std::ptr::null()).is_null());
        assert!(pdal_stage_create_hexbin(std::ptr::null()).is_null());
        assert!(pdal_stage_create_faceraster(std::ptr::null()).is_null());
        let rd = pdal_stage_create_radialdensity(0.0);
        assert!(!rd.is_null());
        pdal_stage_destroy(rd);
        
        let nnd = pdal_stage_create_nndistance(0, std::ptr::null());
        assert!(!nnd.is_null());
        pdal_stage_destroy(nnd);
        
        assert!(pdal_stage_create_zsmooth(0.0, 0.0, std::ptr::null()).is_null());
        assert!(pdal_stage_create_outlier(std::ptr::null(), 0, 0.0, 0, 0.0, 0).is_null());
        assert!(pdal_stage_create_dbscan(0, 0.0, std::ptr::null(), 0).is_null());
        
        let smrf = pdal_stage_create_smrf(0.0, 0.0, false, 0.0, 0.0, 0.0, 0, 0, false, std::ptr::null(), 0);
        assert!(!smrf.is_null());
        pdal_stage_destroy(smrf);
        
        assert!(pdal_stage_create_iqr(0.0, std::ptr::null()).is_null());
        assert!(pdal_stage_create_mad(0.0, std::ptr::null(), 0.0).is_null());
        
        let cov = pdal_stage_create_covariancefeatures(0, false, 0.0, 0, 0, 0, false, std::ptr::null(), 0);
        assert!(!cov.is_null());
        pdal_stage_destroy(cov);
        
        assert!(pdal_stage_create_straighten(std::ptr::null(), false, 0.0).is_null());
        
        let lk = pdal_stage_create_lloydkmeans(0, 0, std::ptr::null(), 0);
        assert!(!lk.is_null());
        pdal_stage_destroy(lk);
    }
}

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
        let sort = pdal_stage_create_sort(sort_dims.as_ptr(), 1, sort_order.as_ptr(), sort_alg.as_ptr());
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
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            1.0, 2.0, 3.0, 1.0,
        ];
        let xform = pdal_stage_create_transformation(matrix.as_ptr());
        pdal_stage_transformation_point(xform, view, 1);
        let out_xform = pdal_stage_run(xform, view);
        assert!(!out_xform.is_null());
        pdal_point_view_destroy(out_xform);
        pdal_stage_destroy(xform);

        // 17. GpsTimeConvert
        let gps_ops = options(&[
            ("conversion", "gws2gt"),
            ("start_date", "2020-01-08"),
        ]);
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

