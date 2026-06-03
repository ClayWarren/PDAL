use super::*;

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_filter_abi_nulls_and_errors() {
    unsafe {
        assert!(
            pdal_stage_create_geomdistance(std::ptr::null(), std::ptr::null(), false).is_null()
        );
        assert!(pdal_stage_create_geomdistance(
            CString::new("POINT(0 0)").unwrap().as_ptr(),
            std::ptr::null(),
            false
        )
        .is_null());
        assert!(pdal_stage_create_geomdistance(
            std::ptr::null(),
            CString::new("X").unwrap().as_ptr(),
            false
        )
        .is_null());

        assert!(
            pdal_stage_create_projpipeline(std::ptr::null(), std::ptr::null(), false).is_null()
        );
        assert!(pdal_stage_create_projpipeline(
            CString::new("EPSG:4326").unwrap().as_ptr(),
            std::ptr::null(),
            false
        )
        .is_null());
        assert!(pdal_stage_create_projpipeline(
            std::ptr::null(),
            CString::new("+proj=utm").unwrap().as_ptr(),
            false
        )
        .is_null());

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

        assert!(pdal_stage_create_radiusassign(
            std::ptr::null(),
            1,
            std::ptr::null(),
            1,
            std::ptr::null(),
            1,
            1.0,
            false,
            0.0,
            0.0
        )
        .is_null());
        assert!(pdal_stage_create_head(std::ptr::null()).is_null());
        assert!(pdal_stage_create_tail(std::ptr::null()).is_null());
        assert!(pdal_stage_create_locate(std::ptr::null()).is_null());
        assert!(pdal_stage_create_ferry(std::ptr::null(), std::ptr::null(), 0).is_null());
        assert!(pdal_stage_create_ferry_specs(std::ptr::null(), 0).is_null());
        assert!(!pdal_stage_validate_assign_statement(std::ptr::null()));
        assert!(!pdal_stage_validate_assign_statement_with_layout(
            std::ptr::null(),
            std::ptr::null()
        ));
        let statement = cstring("Classification = Z + 10 WHERE Z == 5");
        assert!(!pdal_stage_validate_assign_statement_with_layout(
            statement.as_ptr(),
            std::ptr::null()
        ));
        let layout = pdal_point_layout_create();
        for dim in ["Classification", "Z"] {
            let name = cstring(dim);
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        assert!(pdal_stage_validate_assign_statement_with_layout(
            statement.as_ptr(),
            layout
        ));
        pdal_point_layout_destroy(layout);
        assert!(!pdal_point_view_apply_assign_statements(
            std::ptr::null_mut(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            0
        ));
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
        assert!(!pdal_stage_range_point_passes(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0
        ));
        assert!(
            pdal_stage_create_sort(std::ptr::null(), 0, std::ptr::null(), std::ptr::null())
                .is_null()
        );
        assert!(pdal_stage_create_returns(std::ptr::null(), 0).is_null());
        assert!(pdal_stage_create_decimation(std::ptr::null()).is_null());
        let crop = pdal_stage_create_crop(
            false,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            0.0,
        );
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
        )
        .is_null());
        assert!(
            pdal_stage_create_overlay(std::ptr::null(), std::ptr::null(), std::ptr::null())
                .is_null()
        );
        assert!(pdal_stage_create_colorinterp(
            std::ptr::null(),
            std::ptr::null(),
            0.0,
            0.0,
            false,
            false,
            false,
            1.4862,
            0.0
        )
        .is_null());
        assert_eq!(
            take_string(pdal_colorinterp_validate_prepared(
                std::ptr::null(),
                std::ptr::null(),
                0.0,
                0.0
            )),
            "Missing colorinterp layout."
        );
        assert!(pdal_colorinterp_pipeline_streamable(0.0, 0.0));
        let ramp_name = cstring("pestel_shades");
        let mut ramp_data = std::ptr::null();
        let mut ramp_len = 0;
        assert!(pdal_colorinterp_default_ramp(
            ramp_name.as_ptr(),
            &mut ramp_data,
            &mut ramp_len
        ));
        assert!(!ramp_data.is_null());
        assert!(ramp_len > 8);
        let ramp_bytes = std::slice::from_raw_parts(ramp_data, ramp_len as usize);
        assert_eq!(&ramp_bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        let missing_ramp = cstring("not_a_ramp");
        assert!(!pdal_colorinterp_default_ramp(
            missing_ramp.as_ptr(),
            &mut ramp_data,
            &mut ramp_len
        ));
        assert!(pdal_stage_create_colorization(std::ptr::null(), std::ptr::null(), 0).is_null());
        assert!(pdal_stage_create_hag_dem(std::ptr::null(), 0, false, 0.0, 0.0, 0.0, 0).is_null());
        assert!(pdal_stage_create_voxeldownsize(std::ptr::null()).is_null());
        assert!(pdal_stage_create_sample(std::ptr::null()).is_null());
        assert!(pdal_stage_create_hexbin(std::ptr::null()).is_null());
        let hexes: Vec<c_int> = [(0, 0), (1, 0), (0, 1)]
            .into_iter()
            .flat_map(|(i, j)| [i, j])
            .collect();
        let wkt = take_string(pdal_hexgrid_wkt(
            1.0,
            1,
            hexes.as_ptr(),
            (hexes.len() / 2) as u64,
            6,
        ));
        assert!(wkt.starts_with("MULTIPOLYGON "));
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

        let smrf = pdal_stage_create_smrf(
            0.0,
            0.0,
            false,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            0,
            false,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            0,
        );
        assert!(!smrf.is_null());
        pdal_stage_destroy(smrf);

        assert!(pdal_stage_create_iqr(0.0, std::ptr::null()).is_null());
        assert!(pdal_stage_create_mad(0.0, std::ptr::null(), 0.0).is_null());

        let cov = pdal_stage_create_covariancefeatures(
            0,
            false,
            0.0,
            0,
            0,
            0,
            false,
            std::ptr::null(),
            0,
        );
        assert!(!cov.is_null());
        pdal_stage_destroy(cov);

        assert!(pdal_stage_create_straighten(std::ptr::null(), false, 0.0).is_null());

        let lk = pdal_stage_create_lloydkmeans(0, 0, std::ptr::null(), 0);
        assert!(!lk.is_null());
        pdal_stage_destroy(lk);
    }
}
