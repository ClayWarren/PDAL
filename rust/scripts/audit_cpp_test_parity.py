#!/usr/bin/env python3
"""Audit Rust C ABI coverage of built C++ GoogleTest cases."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
from pathlib import Path


ALL = object()
DEFAULT_BASELINE_REF = "3df1668e0^"
TEST_RE = re.compile(r"\bTEST(?:_F|_P)?\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)")

COVERED: dict[str, object] = {
    "pdal_artifact_test": ALL,
    "pdal_kdindex_test": ALL,
    "pdal_spatial_reference_test": {
        "test_ctor",
        "calcZone",
        "wgs84FromZone",
        "test_proj4_roundtrip",
        "test_userstring_roundtrip",
        "test_get_utmzone",
        "test_read_srs",
        "test_io",
        "test_vertical_and_horizontal",
        "readerOptions",
        "test_bounds",
        "identifyEPSG",
        "issue_1989",
        "test_writing_vlr",
        "merge",
        "axis_ordering",
        "badGeoTIFFkeys",
    },
    "pdal_point_view_test": {
        "getSet",
        "getAsUint8",
        "getAsInt32",
        "getFloat",
        "calculateBounds",
        "pointRef",
        "order",
        "issue1264",
        "bigfile",
        "getFloatNan",
    },
    "pdal_eigen_test": {
        "PointViewToEigen",
        "RoundtripString",
        "calcBounds",
        "ComputeValues",
        "Morphological",
        "computeCentroid",
        "demeanTest",
    },
    "pdal_bounds_test": {
        "test_ctor",
        "test_equals",
        "test_accessor",
        "test_clip",
        "test_intersect",
        "test_grow",
        "test_bounds_grow_2_3_args",
        "test_static",
        "test_invalid",
        "test_output",
        "test_input",
        "test_parse",
        "test_parse2",
        "test_parse_geojson",
        "test_parse_object",
        "test_wkt",
        "test_json",
        "test_2d_input",
        "test_precisionloss",
        "fromstring",
        "b1",
        "b2",
        "bounds_insertion",
        "test_copy",
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
        "split",
        "splitChar",
        "split2",
        "split2Char",
        "case",
        "starts",
        "iequals",
        "test_random",
        "test_env",
        "test_comparators",
        "naninf",
        "toString",
        "fromString",
        "StringStreamClassicLocale",
        "OStringStreamClassicLocale",
        "IStringStreamClassicLocale",
        "numeric_cast",
        "extractor",
    },
    "pdal_file_utils_test": ALL,
    "pdal_georeference_test": ALL,
    "pdal_charbuf_test": ALL,
    "pdal_deflate_test": ALL,
    "pdal_zstd_test": ALL,
    "pdal_io_vsi_test": ALL,
    "pdal_i3s_obb_test": ALL,
    "pdal_math_utils_test": ALL,
    "pdal_scaling_test": ALL,
    "pdal_support_test": ALL,
    "pdal_segmentation_test": ALL,
    "pdal_filespec_test": ALL,
    "pdal_dimension_test": ALL,
    "pdal_point_table_test": {
        "resolveType",
        "layoutLimit",
        "userView",
        "srs",
        "simple",
        "typedStorage",
    },
    "pdal_kernel_test": ALL,
    "pdal_app_plugin_test": ALL,
    "pdal_app_test": {
        "option_file",
        "load",
        "log",
        "listCommands",
        "pipeline_dims_limits_metadata_dimensions",
        "pipeline_progress_reports_writer_file",
    },
    "chamfer_test": ALL,
    "hausdorff_test": ALL,
    "pdal_eval_test": ALL,
    "pdal_info_test": ALL,
    "pdal_merge_test": ALL,
    "pdal_tile_test": ALL,
    "pdal_tindex_test": ALL,
    "pdal_config_test": ALL,
    "pdal_log_test": ALL,
    "pdal_where_test": {"filter", "empty", "writer"},
    "pdal_stage_factory_test": {
        "Load",
        "extensionTest",
        "stageExtensionsLoadPerInstance",
        "stageExtensionsCustomMappingsOverrideDefaults",
    },
    "pdal_plugin_manager_test": {"MissingPlugin", "validnames", "CreateObject"},
    "pdal_program_arg_test": ALL,
    "pdal_options_test": {
        "valid",
        "programargs",
        "nan",
        "doublepreicison",
        "issue_4751",
        "conditional",
        "test_option_writing",
        "json",
        "json_file_allows_leading_comments",
        "rust_mirror_tracks_lifecycle_mutations",
    },
    "pdal_polygon_test": {
        "test_wkt_in",
        "test_wkt_out",
        "test_json_in",
        "test_json_out",
        "simplify",
        "smooth",
        "covers",
        "valid",
        "bounds",
        "bounds2d",
        "bounds3d",
        "streams",
    },
    "pdal_quad_index_test": ALL,
    "pdal_xml_schema_test": {"legacyNames", "roundTrip"},
    "pdal_uuid_test": ALL,
    "pdal_ogr_arg_test": {"parseErrors", "createFromFile"},
    "pdal_oldpclblock_test": {
        "StatisticalOutliers1",
        "StatisticalOutliers2",
        "RadiusOutliers1",
        "RadiusOutliers2",
        "PMF",
    },
    "pdal_thread_pool_test": ALL,
    "pdal_filters_crop_test": {
        "create",
        "test_crop",
        "test_crop_3d",
        "test_crop_polygon",
        "test_crop_polygon_reprojection",
        "test_crop_ogr",
        "multibounds",
        "circle",
        "sphere",
        "test_crop_on_edge",
        "issue_3114",
        "stream",
        "bounds_inside_outside",
    },
    "pdal_filters_colorinterp_test": {
        "minmax",
        "badramp",
        "autorange",
        "k",
        "mad",
        "missingz",
        "cantstream",
    },
    "pdal_filters_colorization_test": {
        "test1",
        "test2",
        "test3",
        "test4",
        "test5",
    },
    "pdal_filters_covariancefeatures_test": ALL,
    "pdal_filters_csf_test": ALL,
    "pdal_filters_normal_test": ALL,
    "pdal_filters_relaxation_dart_throwing_test": ALL,
    "pdal_filters_lloydkmeans_test": ALL,
    "pdal_filters_delaunay_test": ALL,
    "pdal_filters_icp_test": ALL,
    "pdal_filters_straighten_test": ALL,
    "pdal_filters_shell_test": ALL,
    "pdal_filters_additional_merge_test": ALL,
    "pdal_filters_hag_test": {"dem", "dem_clamps", "neighbors", "closest", "delaunay"},
    "pdal_filters_h3_test": {"createStage", "stream_test_2"},
    "pdal_filters_geomdistance_test": {"create", "test_polygon"},
    "pdal_filters_hexbin_test": {
        "HexbinFilterTest_test_1",
        "ogr_density_boundary_output",
        "HexbinFilterTest_test_2",
        "HexGrid_issue_2507",
        "H3Grid_issue_2507",
        "issue_4899",
    },
    "pdal_filters_info_test": ALL,
    "pdal_filters_faceraster_test": ALL,
    "pdal_filters_overlay_test": ALL,
    "pdal_filters_reprojection_test": ALL,
    "pdal_filters_ht_test": ALL,
    "pdal_filters_divider_test": {
        "partition_count",
        "partition_capacity",
        "round_robin_count",
        "round_robin_capacity",
        "break_on_expression",
        "break_on_userdata",
        "zero_capacity",
    },
    "pdal_filters_sparsesurface_test": {
        "create",
        "lowest_is_ground_rest_low_noise",
        "equal_classes_throw",
    },
    "pdal_filters_gpstimeconvert_test": ALL,
    "pdal_filters_expression_test": {
        "createStage",
        "noLimits",
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
        "stream",
        "dimset",
        "metadata",
        "enum",
        "global",
        "counts",
        "merge",
    },
    "pdal_filters_sample_test": {
        "create",
        "culls_close_points",
        "keeps_distant_points",
        "cell_mode",
        "requires_cell_or_radius",
        "rejects_cell_and_radius",
        "culls_across_voxels",
        "radius_boundary",
        "repeated_execute_resets_voxels",
        "dimension_mode_flags_points",
        "dense_grid_radius_spacing",
        "dense_grid_cell_spacing",
    },
    "pdal_filters_decimation_test": {
        "create",
        "test1",
        "preservesSpatialReference",
        "fpstep",
        "stream",
        "stream_fpstep",
    },
    "pdal_filters_dem_test": {
        "KeepsPointsInsideRasterRelativeLimits",
    },
    "pdal_filters_ferry_test": {
        "create",
        "stream",
        "test_ferry_copy_json",
        "test_ferry_invalid",
    },
    "pdal_filters_range_test": {
        "createStage",
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
        "wrongStringFormat",
    },
    "pdal_filters_randomize_test": ALL,
    "pdal_filters_locate_test": ALL,
    "pdal_filters_cluster_test": {
        "create",
        "two_clusters",
        "min_points_threshold",
        "is3d_toggle",
    },
    "pdal_filters_dbscan_test": {
        "create",
        "two_clusters_and_noise",
        "min_points_threshold",
        "dimensions_restrict_clustering",
        "propagates_along_chain",
    },
    "pdal_filters_approximatecoplanar_test": {
        "create",
        "labels_plane",
        "rejects_volume",
    },
    "pdal_filters_eigenvalues_test": {
        "create",
        "planar_neighborhood",
        "normalized_eigenvalues_sum_to_one",
    },
    "pdal_filters_elm_test": ALL,
    "pdal_filters_estimaterank_test": {"create", "planar", "linear"},
    "pdal_filters_expressionstats_test": {
        "create",
        "metadata_bins_by_expression",
    },
    "pdal_filters_iqr_test": {
        "create",
        "drops_high_outlier",
        "drops_low_outlier",
        "fence_boundary_is_strict",
        "multiplier_widens_fence",
        "missing_dimension_throws",
    },
    "pdal_filters_mad_test": {
        "create",
        "drops_outlier",
        "keeps_clean_data",
        "missing_dimension_throws",
        "parameters_affect_fence",
    },
    "pdal_filters_miniball_test": ALL,
    "pdal_filters_lof_test": {
        "create",
        "flags_outlier",
        "minpts_controls_k_distance",
    },
    "pdal_filters_nndistance_test": ALL,
    "pdal_filters_outlier_test": ALL,
    "pdal_filters_planefit_test": ALL,
    "pdal_filters_pmf_test": ALL,
    "pdal_filters_radius_assign_test": {
        "basic_usage",
        "with_z_limit",
        "with_src_domain",
        "update_expression_uses_math_and_condition",
        "missing_param",
    },
    "pdal_filters_radialdensity_test": {"create", "density"},
    "pdal_filters_reciprocity_test": ALL,
    "pdal_filters_skewness_test": ALL,
    "pdal_filters_zsmooth_test": {
        "create",
        "medianpercent_selects_neighbor_z",
        "z_output_throws",
    },
    "pdal_filters_chipper_test": {"issue_2479", "empty_buffer", "test_construction"},
    "pdal_filters_groupby_test": ALL,
    "pdal_filters_merge_test": ALL,
    "pdal_filters_sort_test": {
        "create",
        "simple",
        "partial",
        "pipelineJSON",
        "issue1382",
        "issue1121_simpleSortOrderDesc",
        "testUnknownOptions",
    },
    "pdal_filters_splitter_test": ALL,
    "pdal_filters_voxel_downsize_test": ALL,
    "pdal_filters_voxel_center_nearest_neighbor_test": ALL,
    "pdal_filters_voxel_centroid_nearest_neighbor_test": ALL,
    "pdal_filters_grid_decimation_test": {
        "create",
        "GridDecimationFilterTest_test_empty",
        "GridDecimationFilterTest_test1",
    },
    "pdal_filters_mongoexpression_test": {
        "createStage",
        "singleComparisons",
        "multiComparisons",
        "logicalOperators",
        "noExpression",
        "missingDimension",
        "invalidSingleComparisons",
        "invalidMultiComparisons",
        "invalidLogicalOperators",
    },
    "pdal_filters_georeference_test": {
        "MissingBeamDimensionsThrows",
        "InvalidCoordinateSystemThrows",
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
        "test_creation",
        "test_errors",
    },
    "pdal_filters_transformation_test": {
        "create",
        "init",
        "TooShort",
        "TooLong",
        "init_file_oneline",
        "init_file_multiline",
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
    "pdal_filters_optimalneighborhood_test": {
        "create",
        "k_within_bounds",
        "custom_k_window",
    },
    "pdal_filters_fps_test": {
        "create",
        "samples_to_count",
        "fewer_points_than_count",
    },
    "pdal_filters_smrf_test": ALL,
    "pdal_filters_labelduplicates_test": ALL,
    "pdal_filters_litree_test": ALL,
    "pdal_filters_m3c2_test": ALL,
    "pdal_filters_neighborclassifier_test": ALL,
    "pdal_filters_separatescanline_test": ALL,
    "pdal_filters_supervoxel_test": ALL,
    "pdal_metadata_test": ALL,
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
    "pdal_io_pts_reader_test": {
        "Constructor",
        "ReadPtsExtraDims",
        "ReadPtsThreeDims",
        "ReadPtsFourDims",
    },
    "pdal_io_ptx_reader_test": {
        "Basic",
        "DiscardMissingPointsWithComplexTransform",
        "MultipleClouds",
        "NoColor",
    },
    "pdal_io_qfit_test": {"test_10_word", "test_14_word"},
    "pdal_io_obj_reader_test": {
        "Constructor",
        "NoFace",
        "NoVertex",
        "Read",
        "FourDimensionRead",
        "TexturesAndNormals",
        "LargeFile",
    },
    "pdal_io_gltf_writer_test": ALL,
    "pdal_io_nitf_reader_test": ALL,
    "pdal_io_nitf_writer_test": ALL,
    "pdal_io_memoryview_reader_test": {
        "readsFieldsFromMemory",
        "rejectsMalformedShape",
        "synthesizesRowMajorShapeCoordinates",
    },
    "pdal_io_buffer_test": {"test_basic"},
    "pdal_io_faux_test": {
        "test_constant_mode_sequential_iter",
        "test_random_mode",
        "test_ramp_mode_1",
        "test_ramp_mode_2",
        "test_return_number",
        "one_point",
        "grid",
        "uniform",
        "normal",
        "badseed",
    },
    "pdal_io_fbi_test": {
        "Constructor",
        "Header",
        "ReadingPoints",
        "RoundtripBasicDimensions",
    },
    "pdal_io_gdal_reader_test": {
        "simple",
        "byte",
        "int16",
        "int32",
        "float32",
        "float64",
        "badfile",
    },
    "pdal_io_gdal_writer_test": {
        "min",
        "min2",
        "minWindow",
        "max",
        "maxWindow",
        "mean",
        "meanWindow",
        "idw",
        "idwWindow",
        "count",
        "percentile",
        "stdev",
        "stdevWindow",
        "bounds",
        "issue_2074",
        "issue_2545",
        "alternate_grid",
        "additionalDim",
        "no_points",
        "btbad",
        "testMetadata",
        "srs",
        "btint",
        "floatDataType",
    },
    "pdal_io_copc_reader_test": {
        "inspect",
        "fullRead",
        "boundedRead2d",
        "boundedRead3d",
        "stream",
        "multipleInputs",
        "boundedpreview",
        "boundedCrop",
        "boundedCropGeoJSON",
        "polygonAndBoundsCrop",
        "boundedCropReprojection",
        "ogrCrop",
        "resolutionLimit",
    },
    "pdal_io_copc_writer_test": {
        "scaling",
        "extradim",
        "srsUTM",
        "srsWkt2",
    },
    "pdal_io_copc_remote_reader_test": {
        "vsi",
    },
    "pdal_filters_poisson_test": {
        "partialNormalsThrow",
        "registersMissingNormalDimensions",
    },
    "pdal_filters_greedyprojection_test": {
        "invalidOptionsThrow",
        "planarPointsProduceMesh",
    },
    "pdal_filters_groundfilter_test": {
        "CustomGroundClass/0",
        "CustomGroundClass/1",
        "CustomGroundClass/2",
        "CustomGroundClass/3",
    },
    "pdal_io_ept_reader_test": {
        "inspect",
        "inspectBounds",
        "fullReadLaszip",
        "fullReadBinary",
        "fullReadZstandard",
        "boundedRead2d",
        "boundedRead3d",
        "resolutionLimit",
        "originReadVersion1_0_0",
        "originRead",
        "badOriginQuery",
        "unreadableDataFailure",
        "unreadableDataIgnored",
        "unreadableDataIgnoredStreaming",
        "unreadableTileFailure",
        "unreadableTileFailureStreaming",
        "badTilePointCountLaszip",
        "badTilePointCountBinary",
        "boundedCrop",
        "polygonAndBoundsCrop",
        "boundedCropReprojection",
        "ogrCrop",
        "bcbfToLonLat",
        "bcbfToLonLat2dBounds",
        "duplicateInputs",
        "binaryStream",
        "laszipStream",
        "zstandardStream",
    },
    "pdal_io_ept_addon_writer_test": {
        "fullLoop",
        "boundedWrite",
        "boundedRead",
        "mustDescendFromEptReader",
    },
    "pdal_io_las_reader_test": ALL,
    "pdal_io_las_writer_test": {
        "srs",
        "srs2",
        "srsWkt2",
        "flex",
        "flex2",
        "forward",
        "header_bbox",
        "issue2235",
        "issue2320",
        "issue3288",
        "issue3652",
        "issue3964",
        "lazperf",
        "stream",
        "compressed1_4",
        "auto_offset",
        "auto_offset2",
        "auto_scale_with_auto_offset",
        "issue1940",
        "forwardvlr",
        "forward_spec_3",
        "issue2663",
        "las10_classification_from_las10_classification",
        "las10_classification_from_las14_classflags",
        "las14_classflags_from_las10_classification",
        "las14_classflags_from_las14_classflags",
        "pdal_metadata",
        "flex_vlr",
        "pdal_add_vlr",
        "pdal_wkt2_vlr",
        "pdal_wkt2_with_derivedprojcrs_vlr",
        "pdal_wkt2_read_as_projjson",
        "extra_dims",
        "all_extra_dims",
        "evlrOffset",
        "read_srs_order",
        "streamhashwrite",
        "fix1063_1064_1065",
        "issue2937",
        "badVlr",
        "oversize_vlr",
    },
    "pdal_io_ilvis2_test": ALL,
    "pdal_io_ilvis2_metadata_test": ALL,
    "pdal_io_ilvis2_reader_metadata_test": {
        "testValidMetadataFile",
        "testNoMetadataFile",
        "testInvalidMetadataFile",
    },
    "pdal_io_ogr_writer_test": {
        "shapefile",
        "shapefile_measure",
        "json",
        "geopackage",
        "creation_options",
        "attrs_all",
        "geopackage_attrs_all",
        "attrs",
        "attrs_measure",
        "multicount",
        "multicount_2",
        "multicount_3",
        "error_multicount_attrs",
        "error_unknown_attr",
        "error_ogr",
    },
    "pdal_io_optech_test": {
        "Constructor",
        "Header",
        "ReadingPoints",
        "Spatialreference",
    },
    "pdal_io_pcd_reader_test": ALL,
    "pdal_io_pcd_writer_test": ALL,
    "pdal_io_ply_reader_test": {
        "Constructor",
        "ReadText",
        "ReadTextExtraDims",
        "ReadBinary",
        "ReadBinaryStream",
        "NoVertex",
        "inspect",
    },
    "pdal_io_ply_writer_test": {
        "Constructor",
        "Write",
        "mesh",
        "issue_2421",
        "dimtypes",
        "flex",
        "flex2",
        "precisionException",
    },
    "pdal_io_sbet_reader_test": ALL,
    "pdal_io_sbet_writer_test": {"testConstructor", "testWrite"},
    "pdal_io_smrmsg_reader_test": ALL,
    "pdal_io_slpk_reader_test": ALL,
    "pdal_io_stac_reader_test": {
        "local_catalog_test",
        "item_collection_test",
        "date_validate_test",
        "date_prune_accept_test",
        "date_start_end_time_accept_test",
        "date_prune_reject_test",
        "bounds_prune_accept_test",
        "bounds_prune_reject_test",
        "ogr_bounds_accept_test",
        "ogr_bounds_reject_test",
        "ogr_bounds_invalid_test",
        "remote_item_test",
        "catalog_test",
        "nested_catalog_test",
        "id_prune_test",
        "local_data_test",
        "collection_filter_test",
        "collection_test",
        "wrench_test",
        "schema_validate_test",
        "multiple_readers_test",
    },
    "pdal_io_terrasolid_test": {
        "Constructor",
        "Header",
        "ReadingPoints",
    },
    "pdal_io_bpf_base_test": ALL,
    "pdal_io_bpf_zlib_test": ALL,
    "pdal_io_writer_test": {
        "nullWriterConsumesInput",
        "issue4261",
        "filenameTemplate",
    },
    "pdal_pipeline_writer_test": {
        "issue_2458",
        "serialize",
        "serializeTerminalBranches",
    },
    "pdal_pipeline_manager_test": {
        "basic",
        "OptionOrder",
        "progress",
        "InputGlobbing",
        "objects",
        "arrayPipeline",
        "jsonPipelineAllowsComments",
        "jsonPipelineRejectsInvalidStageMetadata",
        "replace",
    },
    "pdal_streaming_test": ALL,
    "nitfwrap_test": ALL,
    "pc2pc_test": ALL,
    "random_test": ALL,
    "translate_test": ALL,
}


def list_tests(binary: Path, env: dict[str, str]) -> list[str]:
    result = subprocess.run(
        [str(binary), "--gtest_list_tests"],
        check=True,
        capture_output=True,
        env=env,
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


def git(args: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        check=check,
        capture_output=True,
        text=True,
    )


def baseline_tests(ref: str) -> set[str]:
    exists = git(["rev-parse", "--verify", "--quiet", ref], check=False)
    if exists.returncode != 0:
        raise SystemExit(f"baseline ref not found: {ref}")

    listed = git(["ls-tree", "-r", "--name-only", ref, "test"]).stdout
    tests: set[str] = set()
    for path in listed.splitlines():
        if not path.endswith((".cpp", ".hpp")):
            continue
        blob = git(["show", f"{ref}:{path}"], check=False)
        if blob.returncode != 0:
            continue
        for suite, name in TEST_RE.findall(blob.stdout):
            tests.add(f"{suite}.{name}")
    return tests


def default_build_dir() -> str:
    env_dir = Path.cwd() / ".build"
    plain_dir = Path.cwd() / "build"
    for candidate in (env_dir, plain_dir):
        if (candidate / "bin").is_dir():
            return str(candidate)
    return "build"


def test_env(build_dir: Path) -> dict[str, str]:
    env = os.environ.copy()
    lib_dirs = [build_dir / "lib", Path.cwd() / ".pixi" / "envs" / "dev" / "lib"]
    for name in ("DYLD_LIBRARY_PATH", "LD_LIBRARY_PATH"):
        existing = env.get(name, "")
        parts = [str(path) for path in lib_dirs if path.is_dir()]
        if existing:
            parts.append(existing)
        if parts:
            env[name] = os.pathsep.join(parts)
    return env


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--build-dir",
        default=default_build_dir(),
        help="CMake build directory. Defaults to .build when present, then build.",
    )
    parser.add_argument(
        "--baseline-ref",
        default=DEFAULT_BASELINE_REF,
        help="Git ref used for the C++ test baseline before local guard tests and Rust port work",
    )
    parser.add_argument(
        "--include-added-tests",
        action="store_true",
        help="Use every currently built C++ GoogleTest case as the denominator",
    )
    args = parser.parse_args()

    build_dir = Path(args.build_dir)
    bin_dir = build_dir / "bin"
    binaries = sorted(bin_dir.glob("*_test"))
    if not binaries:
        raise SystemExit(f"no built test binaries found under {bin_dir}")

    baseline = None if args.include_added_tests else baseline_tests(args.baseline_ref)
    current_total = 0
    ignored_added = 0
    total = 0
    covered = 0
    missing_binaries = []
    missing_tests: dict[str, list[str]] = {}
    rows: list[tuple[str, int, int]] = []

    env = test_env(build_dir)
    for binary in binaries:
        all_tests = list_tests(binary, env)
        current_total += len(all_tests)
        tests = all_tests
        if baseline is not None:
            tests = [test for test in all_tests if test in baseline]
            ignored_added += len(all_tests) - len(tests)
            if not tests:
                continue
        total += len(tests)
        rule = COVERED.get(binary.name)
        if rule is ALL:
            count = len(tests)
        elif isinstance(rule, set):
            by_short_name = {test.rsplit(".", 1)[-1]: test for test in tests}
            all_by_short_name = {test.rsplit(".", 1)[-1]: test for test in all_tests}
            found = {name for name in rule if name in by_short_name}
            missing = sorted(name for name in rule - found if name not in all_by_short_name)
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
    scope = "all currently built tests" if args.include_added_tests else f"pre-port baseline ({args.baseline_ref})"
    print(f"Scope: {scope}")
    print(f"Built C++ GoogleTest binaries: {len(binaries)}")
    if baseline is None:
        print(f"Denominator C++ GoogleTest cases: {total}")
    else:
        print(f"Currently built C++ GoogleTest cases: {current_total}")
        print(f"Pre-port denominator C++ GoogleTest cases: {total}")
        print(f"Added local C++ GoogleTest cases ignored: {ignored_added}")
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
