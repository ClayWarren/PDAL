use super::*;
use crate::stage_abi::StageWrapper;
use pdal_core::options::Options;
use pdal_core::point::PointView;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

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

unsafe fn take_c_string(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null());
    let value = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    pdal_string_free(ptr);
    value
}

unsafe fn destroy_stage(stage: *mut StageWrapper) {
    assert!(!stage.is_null());
    pdal_stage_destroy(stage);
}

#[test]
fn csf_classify_rejects_negative_rigidness() {
    let xyz = [0.0, 0.0, 0.0];
    let mut ground = [0_u8];

    unsafe {
        assert_eq!(
            pdal_filter_csf_classify(
                xyz.as_ptr(),
                1,
                true,
                0.65,
                0.5,
                0.3,
                1.0,
                -1,
                500,
                ground.as_mut_ptr(),
            ),
            -1
        );
        assert_eq!(ground[0], 0);
    }
}

#[test]
fn assign_statements_apply_to_selected_view_points() {
    unsafe {
        let view = xyz_view(&[(0.0, 0.0, 1.0), (1.0, 0.0, 2.0), (2.0, 0.0, 3.0)]);
        let statement = cstring("Classification = Z + 10 WHERE Z >= 2");
        let statements = [statement.as_ptr()];
        let selected = [0_u64, 1, 2];
        let target = pdal_assign_statement_target_dim(statement.as_ptr());
        assert_eq!(take_c_string(target), "Classification");

        assert!(pdal_point_view_apply_assign_statements(
            view,
            statements.as_ptr(),
            statements.len() as u64,
            selected.as_ptr(),
            selected.len() as u64
        ));
        assert_eq!(get(view, 0, "Classification"), 0.0);
        assert_eq!(get(view, 1, "Classification"), 12.0);
        assert_eq!(get(view, 2, "Classification"), 13.0);

        pdal_point_view_destroy(view);
    }
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
            pdal_stage_create_colorinterp(
                x.as_ptr(),
                ramp.as_ptr(),
                0.0,
                10.0,
                true,
                false,
                false,
                1.4862,
                0.0,
            ),
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
            false,
            false,
            1.4862,
            0.0
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
                0.0,
                2,
                1,
                false,
                returns.as_ptr(),
                returns.len() as u64,
                std::ptr::null(),
                0,
                0,
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
            0.0,
            2,
            1,
            false,
            bad_returns.as_ptr(),
            1,
            std::ptr::null(),
            0,
            0,
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
        let norm = pdal_stage_create_normal(8, true, 1.5, true, 0.0, 0.0, 10.0, true, false);
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
        assert!(
            pdal_stage_create_geomdistance(std::ptr::null(), std::ptr::null(), false).is_null()
        );

        // 2. pdal_stage_create_projpipeline nulls
        assert!(
            pdal_stage_create_projpipeline(std::ptr::null(), std::ptr::null(), false).is_null()
        );

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
        assert!(
            !pdal_transformation_matrix_parse(std::ptr::null(), std::ptr::null_mut()).is_null()
        );
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

        let x_str = cstring("X");
        let valid_limit = pdal_range_limit_t {
            dim_name: x_str.as_ptr(),
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
            pdal_stage_create_neighborclassifier(std::ptr::null(), 1, 2, std::ptr::null())
                .is_null()
        );
        assert!(pdal_stage_create_neighborclassifier(&bad_limit, 1, 2, std::ptr::null()).is_null());
        let nc = pdal_stage_create_neighborclassifier(std::ptr::null(), 0, 2, std::ptr::null());
        assert!(!nc.is_null());
        pdal_stage_destroy(nc);
    }
}
mod errors;

mod execution;
