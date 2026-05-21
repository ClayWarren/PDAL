#!/usr/bin/env python3
"""Audit Rust C ABI coverage of built C++ GoogleTest cases."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


ALL = object()

COVERED: dict[str, object] = {
    "pdal_kdindex_test": ALL,
    "pdal_spatial_reference_test": {"calcZone", "wgs84FromZone"},
    "pdal_point_view_test": {"calculateBounds"},
    "pdal_eigen_test": {"calcBounds"},
    "pdal_bounds_test": {
        "test_ctor",
        "test_clip",
        "test_intersect",
        "test_grow",
        "test_bounds_grow_2_3_args",
        "test_invalid",
        "test_input",
        "test_parse",
        "test_parse2",
        "test_parse_geojson",
        "test_2d_input",
        "test_precisionloss",
        "fromstring",
    },
    "pdal_utils_test": {
        "test_base64",
        "blanks",
        "replaceAll",
        "escapeNonprinting",
        "escapeJSON",
        "wordWrap",
        "wordWrap2",
        "simpleWordexpTest",
        "splitChar",
        "split2Char",
        "case",
        "starts",
        "iequals",
    },
    "pdal_file_utils_test": {
        "test_toAbsolutePath",
        "test_getDirectory",
        "test_isAbsolute",
        "filename",
        "extension",
        "stem",
    },
    "pdal_georeference_test": ALL,
    "pdal_charbuf_test": ALL,
    "pdal_math_utils_test": ALL,
    "pdal_scaling_test": ALL,
    "pdal_filespec_test": ALL,
    "pdal_dimension_test": ALL,
    "pdal_point_table_test": {"resolveType"},
    "pdal_kernel_test": ALL,
    "pdal_config_test": ALL,
    "pdal_log_test": {"t1"},
    "pdal_stage_factory_test": {"extensionTest", "stageExtensionsLoadPerInstance"},
    "pdal_plugin_manager_test": {"validnames"},
    "pdal_options_test": {
        "valid",
        "programargs",
        "nan",
        "doublepreicison",
        "issue_4751",
    },
    "pdal_polygon_test": {"valid"},
    "pdal_quad_index_test": ALL,
    "pdal_xml_schema_test": {"legacyNames"},
    "pdal_uuid_test": ALL,
    "pdal_ogr_arg_test": {"parseErrors"},
    "pdal_filters_crop_test": {
        "test_crop",
        "test_crop_3d",
        "test_crop_polygon",
        "multibounds",
        "circle",
        "sphere",
        "test_crop_on_edge",
    },
    "pdal_filters_colorinterp_test": {"minmax", "badramp", "autorange", "k", "mad"},
    "pdal_filters_colorization_test": {"test1", "test2", "test3", "test5"},
    "pdal_filters_hag_test": {"dem", "dem_clamps"},
    "pdal_filters_h3_test": {"stream_test_2"},
    "pdal_filters_geomdistance_test": {"test_polygon"},
    "pdal_filters_faceraster_test": ALL,
    "pdal_filters_overlay_test": ALL,
    "pdal_filters_reprojection_test": ALL,
    "pdal_filters_divider_test": {
        "partition_count",
        "partition_capacity",
        "round_robin_count",
        "round_robin_capacity",
        "break_on_expression",
        "break_on_userdata",
    },
    "pdal_filters_sparsesurface_test": {"lowest_is_ground_rest_low_noise"},
    "pdal_filters_gpstimeconvert_test": ALL,
    "pdal_filters_expression_test": {
        "singleDimension",
        "multipleDimensions",
        "onlyMin",
        "onlyMax",
        "negation",
        "equals",
        "negativeValues",
        "simple_logic",
        "issue_4920",
        "extrachars",
        "issue_1659",
        "stream_logic",
        "nan",
        "nan2",
        "multipleExpressions",
    },
    "pdal_filters_stats_test": {
        "handcalc",
        "baseline",
        "simple",
        "advanced",
        "dimset",
        "metadata",
        "enum",
        "global",
        "counts",
    },
    "pdal_filters_sample_test": {
        "culls_close_points",
        "keeps_distant_points",
        "cell_mode",
        "culls_across_voxels",
        "radius_boundary",
        "repeated_execute_resets_voxels",
    },
    "pdal_filters_decimation_test": {
        "test1",
        "preservesSpatialReference",
        "fpstep",
        "stream",
        "stream_fpstep",
    },
    "pdal_filters_ferry_test": {"stream", "test_ferry_copy_json"},
    "pdal_filters_range_test": {
        "noLimits",
        "singleDimension",
        "multipleDimensions",
        "multipleDimsBusted",
        "onlyMin",
        "onlyMax",
        "negation",
        "equals",
        "negativeValues",
        "simple_logic",
        "case_1659",
        "stream_logic",
        "nan",
    },
    "pdal_filters_randomize_test": ALL,
    "pdal_filters_locate_test": ALL,
    "pdal_filters_cluster_test": {
        "two_clusters",
        "min_points_threshold",
        "is3d_toggle",
    },
    "pdal_filters_dbscan_test": {
        "two_clusters_and_noise",
        "min_points_threshold",
        "dimensions_restrict_clustering",
        "propagates_along_chain",
    },
    "pdal_filters_approximatecoplanar_test": {"labels_plane", "rejects_volume"},
    "pdal_filters_eigenvalues_test": {
        "planar_neighborhood",
        "normalized_eigenvalues_sum_to_one",
    },
    "pdal_filters_elm_test": ALL,
    "pdal_filters_estimaterank_test": {"planar", "linear"},
    "pdal_filters_expressionstats_test": {"metadata_bins_by_expression"},
    "pdal_filters_iqr_test": {
        "drops_high_outlier",
        "drops_low_outlier",
        "fence_boundary_is_strict",
        "multiplier_widens_fence",
        "missing_dimension_throws",
    },
    "pdal_filters_mad_test": {
        "drops_outlier",
        "keeps_clean_data",
        "missing_dimension_throws",
        "parameters_affect_fence",
    },
    "pdal_filters_lof_test": {"flags_outlier", "minpts_controls_k_distance"},
    "pdal_filters_nndistance_test": ALL,
    "pdal_filters_outlier_test": ALL,
    "pdal_filters_planefit_test": ALL,
    "pdal_filters_radialdensity_test": {"density"},
    "pdal_filters_reciprocity_test": ALL,
    "pdal_filters_skewness_test": ALL,
    "pdal_filters_zsmooth_test": {"medianpercent_selects_neighbor_z"},
    "pdal_filters_chipper_test": {"issue_2479", "empty_buffer"},
    "pdal_filters_groupby_test": ALL,
    "pdal_filters_merge_test": ALL,
    "pdal_filters_sort_test": {
        "simple",
        "partial",
        "pipelineJSON",
        "issue1382",
        "issue1121_simpleSortOrderDesc",
    },
    "pdal_filters_splitter_test": ALL,
    "pdal_filters_voxel_downsize_test": ALL,
    "pdal_filters_voxel_center_nearest_neighbor_test": ALL,
    "pdal_filters_voxel_centroid_nearest_neighbor_test": ALL,
    "pdal_filters_grid_decimation_test": {
        "GridDecimationFilterTest_test_empty",
        "GridDecimationFilterTest_test1",
    },
    "pdal_filters_mongoexpression_test": {
        "singleComparisons",
        "multiComparisons",
        "logicalOperators",
    },
    "pdal_filters_georeference_test": {
        "TransformsPointAndBeamDirection",
        "PreservesDistancesBetweenPoints",
        "ForwardAndReverseRoundtrip",
        "ENUCoordinateSystem",
        "WithTimeOffset",
        "WithCustomScan2ImuTransform",
    },
    "pdal_filters_assign_test": {
        "value",
        "t2",
        "assignment_parse",
        "test_condition",
    },
    "pdal_filters_transformation_test": {
        "NoChange",
        "Translation",
        "InvertTranslation",
        "Rotation",
        "InvertRotation",
        "SrsReset",
        "StreamsTest",
    },
    "pdal_filters_returns_test": ALL,
    "pdal_morton_order_test": ALL,
    "pdal_filters_rust_pipeline_test": ALL,
    "pdal_filters_optimalneighborhood_test": {"k_within_bounds", "custom_k_window"},
    "pdal_filters_fps_test": {"samples_to_count", "fewer_points_than_count"},
    "pdal_filters_smrf_test": ALL,
    "pdal_filters_labelduplicates_test": ALL,
    "pdal_filters_separatescanline_test": ALL,
    "pdal_metadata_test": {"typed_value", "test_float", "infnan"},
    "pdal_io_text_reader_test": {
        "t1",
        "t1a",
        "t2",
        "t3",
        "badheader",
        "s1",
        "strip_whitespace_from_dimension_names",
        "issue3859",
        "issue1939",
        "warnMissingHeader",
        "overrideHeader",
        "insertHeader",
        "quotedHeader",
    },
    "pdal_io_text_writer_test": {"t1", "t2", "t2stream", "precision", "geojson"},
    "pdal_io_pts_reader_test": {"ReadPtsExtraDims", "ReadPtsThreeDims", "ReadPtsFourDims"},
    "pdal_io_ptx_reader_test": {
        "Basic",
        "DiscardMissingPointsWithComplexTransform",
        "MultipleClouds",
        "NoColor",
    },
    "pdal_io_qfit_test": {"test_10_word", "test_14_word"},
    "pdal_io_obj_reader_test": {
        "NoFace",
        "NoVertex",
        "Read",
        "FourDimensionRead",
        "TexturesAndNormals",
        "LargeFile",
    },
    "pdal_io_gltf_writer_test": ALL,
    "pdal_io_faux_test": {
        "test_constant_mode_sequential_iter",
        "test_random_mode",
        "test_ramp_mode_1",
        "test_ramp_mode_2",
        "test_return_number",
        "one_point",
        "grid",
    },
    "pdal_io_fbi_test": {"Header", "ReadingPoints", "RoundtripBasicDimensions"},
    "pdal_io_gdal_reader_test": {
        "simple",
        "byte",
        "int16",
        "int32",
        "float32",
        "float64",
    },
    "pdal_io_ilvis2_test": ALL,
    "pdal_io_ilvis2_metadata_test": ALL,
    "pdal_io_ilvis2_reader_metadata_test": {
        "testValidMetadataFile",
        "testNoMetadataFile",
    },
    "pdal_io_optech_test": {"Header", "ReadingPoints", "Spatialreference"},
    "pdal_io_pcd_reader_test": ALL,
    "pdal_io_pcd_writer_test": ALL,
    "pdal_io_ply_reader_test": {
        "ReadText",
        "ReadTextExtraDims",
        "ReadBinary",
        "ReadBinaryStream",
        "NoVertex",
        "inspect",
    },
    "pdal_io_ply_writer_test": {
        "Write",
        "mesh",
        "issue_2421",
        "dimtypes",
        "flex",
        "flex2",
    },
    "pdal_io_sbet_reader_test": ALL,
    "pdal_io_sbet_writer_test": {"testWrite"},
    "pdal_io_smrmsg_reader_test": ALL,
    "pdal_io_terrasolid_test": {"Header", "ReadingPoints"},
    "pdal_io_bpf_base_test": ALL,
    "pdal_io_bpf_zlib_test": ALL,
    "pdal_io_writer_test": {"nullWriterConsumesInput"},
    "pdal_pipeline_writer_test": {"issue_2458"},
}


def list_tests(binary: Path) -> list[str]:
    result = subprocess.run(
        [str(binary), "--gtest_list_tests"],
        check=True,
        capture_output=True,
        text=True,
    )
    tests: list[str] = []
    suite = ""
    for raw_line in result.stdout.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        if raw_line.startswith("  "):
            name = line.split("#", 1)[0].strip()
            if name:
                tests.append(f"{suite}{name}")
        elif line.endswith("."):
            suite = line
    return tests


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-dir", default="build")
    args = parser.parse_args()

    bin_dir = Path(args.build_dir) / "bin"
    binaries = sorted(bin_dir.glob("pdal*_test"))
    if not binaries:
        raise SystemExit(f"no built test binaries found under {bin_dir}")

    total = 0
    covered = 0
    missing_binaries = []
    missing_tests: dict[str, list[str]] = {}
    rows: list[tuple[str, int, int]] = []

    for binary in binaries:
        tests = list_tests(binary)
        total += len(tests)
        rule = COVERED.get(binary.name)
        if rule is ALL:
            count = len(tests)
        elif isinstance(rule, set):
            by_short_name = {test.rsplit(".", 1)[-1]: test for test in tests}
            found = {name for name in rule if name in by_short_name}
            missing = sorted(rule - found)
            if missing:
                missing_tests[binary.name] = missing
            count = len(found)
        else:
            count = 0
            missing_binaries.append(binary.name)
        covered += count
        if count or rule is not None:
            rows.append((binary.name, count, len(tests)))

    percent = covered / total * 100 if total else 0.0
    print(f"Built C++ GoogleTest binaries: {len(binaries)}")
    print(f"Built C++ GoogleTest cases: {total}")
    print(f"Rust C ABI-backed cases: {covered}")
    print(f"Progress: {percent:.2f}%")
    print()
    print("Counted binaries:")
    for name, count, subtotal in rows:
        print(f"  {name}: {count}/{subtotal}")

    if missing_tests:
        print()
        print("Configured Rust-backed tests not found in built binaries:")
        for name, tests in sorted(missing_tests.items()):
            print(f"  {name}: {', '.join(tests)}")

    print()
    print(f"Uncounted built binaries: {len(missing_binaries)}")
    for name in missing_binaries:
        print(f"  {name}")

    return 1 if missing_tests else 0


if __name__ == "__main__":
    raise SystemExit(main())
