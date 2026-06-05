#
# Rust C ABI integration.
#

set(RUST_CAPI_DIR "${ROOT_DIR}/rust")
set(RUST_CAPI_HEADER_DIR "${RUST_CAPI_DIR}/pdal-capi/include")
if(MSVC)
    set(RUST_CAPI_LIB "${RUST_CAPI_DIR}/target/release/pdal_capi.lib")
    set(RUST_CAPI_MSVC_EXPORTS
        "/EXPORT:pdal_artifact_manager_create"
        "/EXPORT:pdal_artifact_manager_destroy"
        "/EXPORT:pdal_artifact_manager_erase"
        "/EXPORT:pdal_artifact_manager_exists"
        "/EXPORT:pdal_artifact_manager_get"
        "/EXPORT:pdal_artifact_manager_keys_json"
        "/EXPORT:pdal_artifact_manager_put"
        "/EXPORT:pdal_artifact_manager_replace"
        "/EXPORT:pdal_artifact_manager_replace_or_put"
        "/EXPORT:pdal_chamfer"
        "/EXPORT:pdal_clear_error"
        "/EXPORT:pdal_dimension_fix_name"
        "/EXPORT:pdal_dimension_interpretation_name"
        "/EXPORT:pdal_dimension_resolve_type"
        "/EXPORT:pdal_dimension_type_from_base_and_size"
        "/EXPORT:pdal_dimension_type_from_name"
        "/EXPORT:pdal_ept_addon_validate_input"
        "/EXPORT:pdal_eval"
        "/EXPORT:pdal_file_spec_parse_json"
        "/EXPORT:pdal_filter_greedyprojection_validate_options"
        "/EXPORT:pdal_filter_poisson_needs_normal_dims"
        "/EXPORT:pdal_filter_poisson_validate_normals"
        "/EXPORT:pdal_geometry_json_is_valid"
        "/EXPORT:pdal_geometry_wkt_area"
        "/EXPORT:pdal_geometry_wkt_bounds"
        "/EXPORT:pdal_geometry_wkt_contains_point"
        "/EXPORT:pdal_geometry_wkt_covers_point"
        "/EXPORT:pdal_geometry_wkt_distance_to_point"
        "/EXPORT:pdal_geometry_wkt_is_valid"
        "/EXPORT:pdal_geometry_wkt_simplify"
        "/EXPORT:pdal_geometry_wkt_to_json"
        "/EXPORT:pdal_geometry_wkt_to_wkt"
        "/EXPORT:pdal_geometry_wkt_to_wkt_precision"
        "/EXPORT:pdal_h3grid_wkt"
        "/EXPORT:pdal_hausdorff"
        "/EXPORT:pdal_hexgrid_wkt"
        "/EXPORT:pdal_info_summary_json"
        "/EXPORT:pdal_last_error"
        "/EXPORT:pdal_barycentric_interpolation"
        "/EXPORT:pdal_math_compute_centroid"
        "/EXPORT:pdal_math_dilate_diamond"
        "/EXPORT:pdal_math_erode_diamond"
        "/EXPORT:pdal_math_grad_x"
        "/EXPORT:pdal_math_grad_y"
        "/EXPORT:pdal_math_point_view_to_xyz"
        "/EXPORT:pdal_metadata_json_value"
        "/EXPORT:pdal_metadata_node_add_child"
        "/EXPORT:pdal_metadata_node_add_child_clone"
        "/EXPORT:pdal_metadata_node_add_list_child"
        "/EXPORT:pdal_metadata_node_add_list_child_clone"
        "/EXPORT:pdal_metadata_node_add_or_update_child"
        "/EXPORT:pdal_metadata_node_add_or_update_child_clone"
        "/EXPORT:pdal_metadata_node_child"
        "/EXPORT:pdal_metadata_node_child_count"
        "/EXPORT:pdal_metadata_node_child_named"
        "/EXPORT:pdal_metadata_node_child_named_count"
        "/EXPORT:pdal_metadata_node_clone"
        "/EXPORT:pdal_metadata_node_create"
        "/EXPORT:pdal_metadata_node_description"
        "/EXPORT:pdal_metadata_node_destroy"
        "/EXPORT:pdal_metadata_node_find_child_path"
        "/EXPORT:pdal_metadata_node_kind"
        "/EXPORT:pdal_metadata_node_name"
        "/EXPORT:pdal_metadata_node_set_bool"
        "/EXPORT:pdal_metadata_node_set_description"
        "/EXPORT:pdal_metadata_node_set_f64"
        "/EXPORT:pdal_metadata_node_set_i64"
        "/EXPORT:pdal_metadata_node_set_pointer"
        "/EXPORT:pdal_metadata_node_set_string"
        "/EXPORT:pdal_metadata_node_set_type"
        "/EXPORT:pdal_metadata_node_set_u64"
        "/EXPORT:pdal_metadata_node_to_json"
        "/EXPORT:pdal_metadata_node_type"
        "/EXPORT:pdal_metadata_node_value"
        "/EXPORT:pdal_metadata_node_value_bool"
        "/EXPORT:pdal_metadata_node_value_f64"
        "/EXPORT:pdal_metadata_node_value_i64"
        "/EXPORT:pdal_metadata_node_value_kind"
        "/EXPORT:pdal_metadata_node_value_pointer"
        "/EXPORT:pdal_metadata_node_value_u64"
        "/EXPORT:pdal_metadata_value_as_bool"
        "/EXPORT:pdal_metadata_value_as_f64"
        "/EXPORT:pdal_metadata_value_as_i64"
        "/EXPORT:pdal_metadata_value_as_u64"
        "/EXPORT:pdal_nitf_lidar_segment"
        "/EXPORT:pdal_nitf_read_metadata"
        "/EXPORT:pdal_nitf_write"
        "/EXPORT:pdal_options_add_str"
        "/EXPORT:pdal_options_add_f64"
        "/EXPORT:pdal_options_add_u64"
        "/EXPORT:pdal_options_create"
        "/EXPORT:pdal_options_destroy"
        "/EXPORT:pdal_point_layout_create"
        "/EXPORT:pdal_point_layout_destroy"
        "/EXPORT:pdal_point_layout_register_dim"
        "/EXPORT:pdal_point_view_add_named_mesh_triangle"
        "/EXPORT:pdal_point_view_add_mesh_triangle"
        "/EXPORT:pdal_point_view_add_point"
        "/EXPORT:pdal_point_view_calculate_bounds_2d"
        "/EXPORT:pdal_point_view_calculate_bounds_3d"
        "/EXPORT:pdal_point_view_create"
        "/EXPORT:pdal_point_view_create_raster"
        "/EXPORT:pdal_point_view_destroy"
        "/EXPORT:pdal_point_view_dim_count"
        "/EXPORT:pdal_point_view_dim_name"
        "/EXPORT:pdal_point_view_dim_type"
        "/EXPORT:pdal_point_view_expression_mask"
        "/EXPORT:pdal_point_view_get_f32"
        "/EXPORT:pdal_point_view_get_f64"
        "/EXPORT:pdal_point_view_get_i32"
        "/EXPORT:pdal_point_view_get_u8"
        "/EXPORT:pdal_point_view_get_u64"
        "/EXPORT:pdal_point_view_id"
        "/EXPORT:pdal_point_view_length"
        "/EXPORT:pdal_point_view_mesh_triangle"
        "/EXPORT:pdal_point_view_mesh_triangle_count"
        "/EXPORT:pdal_point_view_named_mesh_triangle"
        "/EXPORT:pdal_point_view_named_mesh_triangle_count"
        "/EXPORT:pdal_point_view_raster_cell"
        "/EXPORT:pdal_point_view_raster_count"
        "/EXPORT:pdal_point_view_raster_initializer"
        "/EXPORT:pdal_point_view_raster_limits"
        "/EXPORT:pdal_point_view_raster_name"
        "/EXPORT:pdal_point_view_set_f64"
        "/EXPORT:pdal_point_view_set_raster_cell"
        "/EXPORT:pdal_point_view_set_spatial_reference"
        "/EXPORT:pdal_point_view_set_u64"
        "/EXPORT:pdal_point_view_source_index"
        "/EXPORT:pdal_point_view_spatial_reference"
        "/EXPORT:pdal_point_view_split_where"
        "/EXPORT:pdal_point_view_swap_points"
        "/EXPORT:pdal_point_view_try_set_f64"
        "/EXPORT:pdal_ogr_spec_parse_json"
        "/EXPORT:pdal_plugin_valid_name"
        "/EXPORT:pdal_program_args_parse_json"
        "/EXPORT:pdal_pipeline_add_dependency"
        "/EXPORT:pdal_pipeline_add_reader"
        "/EXPORT:pdal_pipeline_add_stage"
        "/EXPORT:pdal_pipeline_add_stage_tagged"
        "/EXPORT:pdal_pipeline_add_writer"
        "/EXPORT:pdal_pipeline_create"
        "/EXPORT:pdal_pipeline_create_json"
        "/EXPORT:pdal_pipeline_destroy"
        "/EXPORT:pdal_pipeline_execute"
        "/EXPORT:pdal_pipeline_execute_count"
        "/EXPORT:pdal_pipeline_execute_result"
        "/EXPORT:pdal_pipeline_execute_streaming"
        "/EXPORT:pdal_pipeline_execute_summary_json"
        "/EXPORT:pdal_pipeline_find_by_tag"
        "/EXPORT:pdal_pipeline_generate_stage_tag"
        "/EXPORT:pdal_pipeline_input"
        "/EXPORT:pdal_pipeline_input_count"
        "/EXPORT:pdal_pipeline_metadata"
        "/EXPORT:pdal_pipeline_reader_parse_json"
        "/EXPORT:pdal_pipeline_replace_stage"
        "/EXPORT:pdal_pipeline_serialize_json"
        "/EXPORT:pdal_pipeline_stage_count"
        "/EXPORT:pdal_pipeline_streamable"
        "/EXPORT:pdal_reader_create_faux"
        "/EXPORT:pdal_reader_create_spz"
        "/EXPORT:pdal_reader_destroy"
        "/EXPORT:pdal_reader_read_first"
        "/EXPORT:pdal_runtime_plugin_description"
        "/EXPORT:pdal_runtime_plugin_has"
        "/EXPORT:pdal_runtime_plugin_link"
        "/EXPORT:pdal_runtime_plugin_lookup_creator"
        "/EXPORT:pdal_runtime_plugin_names_json"
        "/EXPORT:pdal_runtime_plugin_register"
        "/EXPORT:pdal_rust_kernel_run"
        "/EXPORT:pdal_rust_stage_list_json"
        "/EXPORT:pdal_raster_limits_valid"
        "/EXPORT:pdal_raster_limits_x_cell"
        "/EXPORT:pdal_raster_limits_x_cell_pos"
        "/EXPORT:pdal_raster_limits_y_cell"
        "/EXPORT:pdal_raster_limits_y_cell_pos"
        "/EXPORT:pdal_spatial_reference_create"
        "/EXPORT:pdal_spatial_reference_create_with_epoch"
        "/EXPORT:pdal_spatial_reference_calculate_zone"
        "/EXPORT:pdal_spatial_reference_destroy"
        "/EXPORT:pdal_spatial_reference_empty"
        "/EXPORT:pdal_spatial_reference_epoch"
        "/EXPORT:pdal_spatial_reference_list_add"
        "/EXPORT:pdal_spatial_reference_list_any"
        "/EXPORT:pdal_spatial_reference_list_clear"
        "/EXPORT:pdal_spatial_reference_list_create"
        "/EXPORT:pdal_spatial_reference_list_destroy"
        "/EXPORT:pdal_spatial_reference_list_size"
        "/EXPORT:pdal_spatial_reference_list_unique"
        "/EXPORT:pdal_spatial_reference_set_epoch"
        "/EXPORT:pdal_spatial_reference_text"
        "/EXPORT:pdal_spatial_reference_to_metadata"
        "/EXPORT:pdal_spatial_reference_wgs84_code_from_zone"
        "/EXPORT:pdal_slpk_summary_json"
        "/EXPORT:pdal_srs_transform_create"
        "/EXPORT:pdal_srs_transform_destroy"
        "/EXPORT:pdal_srs_transform_xyz_array"
        "/EXPORT:pdal_stage_create_csf"
        "/EXPORT:pdal_stage_create_decimation"
        "/EXPORT:pdal_stage_create_head"
        "/EXPORT:pdal_stage_create_merge"
        "/EXPORT:pdal_stage_create_tail"
        "/EXPORT:pdal_stage_destroy"
        "/EXPORT:pdal_stage_extensions_create"
        "/EXPORT:pdal_stage_extensions_default_reader"
        "/EXPORT:pdal_stage_extensions_default_writer"
        "/EXPORT:pdal_stage_extensions_destroy"
        "/EXPORT:pdal_stage_extensions_set"
        "/EXPORT:pdal_stage_registry_has"
        "/EXPORT:pdal_stage_run"
        "/EXPORT:pdal_stage_run_multi"
        "/EXPORT:pdal_string_free"
        "/EXPORT:pdal_support_diff_files"
        "/EXPORT:pdal_support_diff_text_files"
        "/EXPORT:pdal_transformation_matrix_format"
        "/EXPORT:pdal_transformation_matrix_parse"
        "/EXPORT:pdal_utils_base64_encode"
        "/EXPORT:pdal_utils_canonical_json"
        "/EXPORT:pdal_utils_extract_c_string"
        "/EXPORT:pdal_utils_from_string_f64"
        "/EXPORT:pdal_utils_from_string_i32"
        "/EXPORT:pdal_utils_to_string_f64"
        "/EXPORT:pdal_utils_to_string_i32"
        "/EXPORT:pdal_uuid_is_null"
        "/EXPORT:pdal_uuid_parse"
        "/EXPORT:pdal_uuid_random"
        "/EXPORT:pdal_uuid_unparse"
        "/EXPORT:pdal_vsi_local_io_scenario_json"
        "/EXPORT:pdal_writer_create_copc"
        "/EXPORT:pdal_writer_create_null"
        "/EXPORT:pdal_writer_create_spz"
        "/EXPORT:pdal_writer_destroy"
        "/EXPORT:pdal_writer_write_view"
    )
else()
    set(RUST_CAPI_LIB "${RUST_CAPI_DIR}/target/release/libpdal_capi.a")
endif()

file(GLOB_RECURSE RUST_CAPI_SOURCES CONFIGURE_DEPENDS
    "${RUST_CAPI_DIR}/Cargo.toml"
    "${RUST_CAPI_DIR}/Cargo.lock"
    "${RUST_CAPI_DIR}/pdal-capi/*"
    "${RUST_CAPI_DIR}/pdal-cli/*"
    "${RUST_CAPI_DIR}/pdal-core/*"
    "${RUST_CAPI_DIR}/pdal-filters/*"
    "${RUST_CAPI_DIR}/pdal-io/*"
    "${RUST_CAPI_DIR}/pdal-kernels/*"
    "${RUST_CAPI_DIR}/pdal-native/*"
    "${RUST_CAPI_DIR}/pdal-plugins/*"
)

set(RUST_MACOSX_DEPLOYMENT_TARGET "${CMAKE_OSX_DEPLOYMENT_TARGET}")
if(APPLE AND NOT RUST_MACOSX_DEPLOYMENT_TARGET)
    set(RUST_MACOSX_DEPLOYMENT_TARGET "16.0")
endif()

find_program(CARGO_EXECUTABLE cargo)
if(NOT CARGO_EXECUTABLE)
    message(FATAL_ERROR "cargo (Rust) is required to build the PDAL Rust C ABI layer.")
endif()

# Build the pdal-capi static library with cargo and make the given PDAL library
# target depend on it. Call once, after the library target exists (the archive is
# then linked in alongside the other C++ dependencies). Keeping the cargo
# invocation here groups it with the cargo discovery and source globbing above.
macro(pdal_build_rust_capi _pdal_target)
    set(RUST_CAPI_BUILD_ENV
        MACOSX_DEPLOYMENT_TARGET=${RUST_MACOSX_DEPLOYMENT_TARGET}
    )
    if (MSVC)
        list(APPEND RUST_CAPI_BUILD_ENV
            CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=${CMAKE_LINKER}
            CXXSTDLIB=
        )
        if(CMAKE_LIBRARY_PATH)
            list(GET CMAKE_LIBRARY_PATH 0 RUST_CONDA_LIBRARY_DIR)
            file(TO_CMAKE_PATH "${RUST_CONDA_LIBRARY_DIR}" RUST_CONDA_LIBRARY_DIR)
            list(APPEND RUST_CAPI_BUILD_ENV
                PKG_CONFIG_PATH=${RUST_CONDA_LIBRARY_DIR}/pkgconfig
                GDAL_DYNAMIC=1
            )
        elseif(DEFINED ENV{CONDA_PREFIX})
            file(TO_CMAKE_PATH "$ENV{CONDA_PREFIX}" RUST_CONDA_PREFIX)
            list(APPEND RUST_CAPI_BUILD_ENV
                PKG_CONFIG_PATH=${RUST_CONDA_PREFIX}/Library/lib/pkgconfig
                GDAL_DYNAMIC=1
            )
        endif()
    endif()
    add_custom_command(
        OUTPUT ${RUST_CAPI_LIB}
        COMMAND ${CMAKE_COMMAND} -E env
            ${RUST_CAPI_BUILD_ENV}
            ${CARGO_EXECUTABLE} build --release -p pdal-capi ${RUST_CAPI_FEATURE_ARGS}
        DEPENDS ${RUST_CAPI_SOURCES}
        WORKING_DIRECTORY ${RUST_CAPI_DIR}
        COMMENT "Building Rust C ABI (pdal-capi) with cargo"
    )
    add_custom_target(pdal_rust_capi DEPENDS ${RUST_CAPI_LIB})
    add_dependencies(${_pdal_target} pdal_rust_capi)
endmacro()

find_library(GEOS_C_LIBRARY NAMES geos_c)
find_library(SQLITE3_LIBRARY NAMES sqlite3)
find_package(SQLite3 QUIET)

# When Nitro is available, the Rust pdal-native crate builds a NITF bridge
# (used by tools.nitfwrap, readers.nitf, and writers.nitf). The conda-forge
# Nitro headers still include POSIX headers on MSVC, so keep that bridge off for
# the MSVC build until the native shim has a Windows-compatible include path.
# Locate Nitro without polluting the global include path (nitro.cmake calls
# include_directories which would shadow vendor/nlohmann/json.hpp with a stale
# copy from the pixi env).
find_package(Nitro 2.6 QUIET MODULE)
if (NITRO_FOUND AND NOT MSVC)
    set(RUST_CAPI_FEATURE_ARGS "--features" "nitf")
else()
    set(RUST_CAPI_FEATURE_ARGS "--no-default-features")
endif()
add_definitions("-D_REENTRANT")
if (WIN32)
    add_definitions("-DSIZEOF_SIZE_T=4")
    add_definitions("-DIMPORT_NITRO_API")
else()
    add_definitions("-D__POSIX")
endif()

# Link a target against the Rust C ABI archive and the native libraries that
# archive embeds: GEOS (via the `geos` crate), the Nitro NITF bridge, and
# CoreFoundation on Apple. Use this for every target that links
# pdal-capi archive directly -- the main pdalcpp library and the standalone
# Rust-backed tools (lasdump, nitfwrap) -- so the Nitro/GEOS link details live
# here instead of being repeated at each call site. The archive is listed
# first so its references resolve against the native libraries that follow.
# Call after the target exists.
macro(pdal_link_rust_capi _target)
    target_link_libraries(${_target}
        PRIVATE
            ${RUST_CAPI_LIB}
            ${GEOS_C_LIBRARY}
    )
    if (TARGET SQLite3::SQLite3)
        target_link_libraries(${_target}
            PRIVATE
                SQLite3::SQLite3
        )
    elseif (TARGET SQLite::SQLite3)
        target_link_libraries(${_target}
            PRIVATE
                SQLite::SQLite3
        )
    elseif (SQLITE3_LIBRARY)
        target_link_libraries(${_target}
            PRIVATE
                ${SQLITE3_LIBRARY}
        )
    endif()
    if (APPLE)
        target_link_options(${_target} PRIVATE "SHELL:-framework CoreFoundation")
    endif()
    if (UNIX AND NOT APPLE)
        target_link_libraries(${_target}
            PRIVATE
                pthread
        )
        target_link_options(${_target} PRIVATE "-pthread")
    endif()
    if (MSVC)
        get_target_property(_target_type ${_target} TYPE)
        if (_target_type STREQUAL "SHARED_LIBRARY")
            target_link_options(${_target}
                PRIVATE
                    ${RUST_CAPI_MSVC_EXPORTS}
            )
        endif()
        target_link_libraries(${_target}
            PRIVATE
                userenv
                ntdll
        )
    endif()
    if (NITRO_FOUND AND NOT MSVC AND NITRO_LIBRARIES)
        target_link_libraries(${_target}
            PRIVATE
                ${NITRO_LIBRARIES}
        )
    endif()
endmacro()
