#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    typedef struct pdal_options pdal_options_t;
    typedef struct pdal_point_layout pdal_point_layout_t;
    typedef struct pdal_point_view pdal_point_view_t;
    typedef struct pdal_spatial_reference pdal_spatial_reference_t;
    typedef struct pdal_metadata_node pdal_metadata_node_t;
    typedef struct pdal_stage pdal_stage_t;
    typedef struct pdal_pipeline pdal_pipeline_t;
    typedef struct pdal_reader pdal_reader_t;
    typedef struct pdal_writer pdal_writer_t;
    typedef struct pdal_quad_index pdal_quad_index_t;
    typedef struct pdal_deflate_compressor pdal_deflate_compressor_t;
    typedef struct pdal_deflate_decompressor pdal_deflate_decompressor_t;

    const char* pdal_last_error();
    void pdal_clear_error();
    void pdal_string_free(char* ptr);

    // Options
    pdal_options_t* pdal_options_create();
    void pdal_options_add_f64(pdal_options_t* ops, const char* key,
                              double value);
    void pdal_options_add_u64(pdal_options_t* ops, const char* key,
                              uint64_t value);
    void pdal_options_add_str(pdal_options_t* ops, const char* key,
                              const char* value);
    bool pdal_options_has(const pdal_options_t* ops, const char* key);
    uint64_t pdal_options_count(const pdal_options_t* ops);
    char* pdal_options_key(const pdal_options_t* ops, uint64_t index);
    char* pdal_options_entry_value(const pdal_options_t* ops, uint64_t index);
    char* pdal_options_value(const pdal_options_t* ops, const char* key);
    char* pdal_options_command_line_json(const pdal_options_t* ops);
    bool pdal_option_name_valid(const char* name);
    void pdal_options_destroy(pdal_options_t* ops);

    // Driver inference
    char* pdal_infer_reader_driver(const char* filename);
    char* pdal_infer_writer_driver(const char* filename);

    // Config
    int32_t pdal_config_version_integer(int32_t major, int32_t minor,
                                        int32_t patch);
    char* pdal_config_full_version_string(const char* version, const char* sha);

    // Log
    const char* pdal_log_level_string(int32_t level);

    // FileSpec
    char* pdal_file_spec_parse_json(const char* input);

    // Utilities
    bool pdal_utils_is_json(const char* value);
    char* pdal_utils_trim_leading(const char* value);
    char* pdal_utils_trim_trailing(const char* value);
    char* pdal_utils_replace_all(const char* value, const char* replace_what,
                                 const char* replace_with);
    int pdal_utils_run_shell_command(const char* command, char** out_output);
    char* pdal_utils_to_lower(const char* value);
    char* pdal_utils_to_upper(const char* value);
    bool pdal_utils_iequals(const char* left, const char* right);
    bool pdal_utils_starts_with(const char* value, const char* prefix);
    char* pdal_utils_split_char(const char* value, char split);
    char* pdal_utils_split2_char(const char* value, char split);
    char* pdal_utils_escape_json(const char* value);
    char* pdal_utils_escape_nonprinting(const char* value);
    double pdal_utils_normalize_longitude(double longitude);
    char* pdal_utils_word_wrap(const char* value, uint64_t line_length,
                               uint64_t first_length);
    char* pdal_utils_word_wrap2(const char* value, uint64_t line_length,
                                uint64_t first_length);
    char* pdal_utils_simple_wordexp(const char* value);
    char* pdal_utils_base64_encode(const uint8_t* bytes, uint64_t len);
    uint8_t* pdal_utils_base64_decode(const char* value, uint64_t* out_len);
    void pdal_u8_array_free(uint8_t* ptr, uint64_t len);
    bool pdal_uuid_parse(const char* input, uint8_t* out_bytes);
    char* pdal_uuid_unparse(const uint8_t* bytes);
    bool pdal_uuid_random(uint8_t* out_bytes);
    bool pdal_uuid_is_null(const uint8_t* bytes);
    int64_t pdal_charbuf_seekpos(int64_t pos, int64_t offset, int64_t len,
                                 bool for_output);
    int64_t pdal_charbuf_seekoff(int64_t off, uint8_t dir, int64_t offset,
                                 int64_t len, int64_t current);
    char* pdal_file_utils_getcwd();
    char* pdal_file_utils_to_absolute_path(const char* filename);
    char* pdal_file_utils_to_absolute_path_with_base(const char* filename,
                                                     const char* base);
    char* pdal_file_utils_get_filename(const char* path);
    char* pdal_file_utils_get_directory(const char* path);
    char* pdal_file_utils_stem(const char* path);
    char* pdal_file_utils_extension(const char* path);
    bool pdal_file_utils_is_absolute_path(const char* path);
    bool pdal_file_utils_directory_exists(const char* dirname);
    int32_t pdal_file_utils_create_directory(const char* dirname);
    int32_t pdal_file_utils_create_directories(const char* path);
    void pdal_file_utils_delete_directory(const char* dirname);
    bool pdal_file_utils_delete_file(const char* filename);
    void pdal_file_utils_rename_file(const char* dest, const char* src);
    bool pdal_file_utils_file_exists(const char* filename);
    uint64_t pdal_file_utils_file_size(const char* filename);
    char* pdal_file_utils_read_file_into_string(const char* filename);
    char* pdal_file_utils_directory_list(const char* dirname);
    char* pdal_file_utils_glob(const char* pattern);
    char* pdal_utils_getenv(const char* name);
    int32_t pdal_utils_setenv(const char* name, const char* value);
    int32_t pdal_utils_unsetenv(const char* name);
    void pdal_utils_random_seed(uint32_t seed);
    double pdal_utils_random(double minimum, double maximum);
    bool pdal_utils_compare_approx(double v1, double v2, double tolerance);
    char* pdal_utils_to_string_f64(double value, uint32_t precision);
    char* pdal_utils_to_string_i32(int32_t value);
    int32_t pdal_utils_from_string_i32(const char* value, int32_t* out);
    int32_t pdal_utils_from_string_f64(const char* value, double* out);
    bool pdal_utils_numeric_cast_f32_to_f64(float value, double* out);
    bool pdal_utils_numeric_cast_f64_to_f32(double value, float* out);

    // Writer filename templates
    bool pdal_writer_handle_filename_template(const char* filename,
                                              size_t* out_pos);
    char* pdal_writer_replace_tags(const char* filename);

    // Support utils
    uint32_t pdal_support_diff_files(
        const char* file1, const char* file2,
        const uint32_t* ignorable_start, const uint32_t* ignorable_length,
        uint32_t num_ignorables);
    uint32_t pdal_support_diff_text_files(const char* file1, const char* file2,
                                          int32_t ignore_line);

    // Streaming zlib (DEFLATE) compression
    pdal_deflate_compressor_t* pdal_deflate_compressor_create();
    bool pdal_deflate_compressor_update(pdal_deflate_compressor_t* compressor,
                                        const char* buf, size_t len,
                                        uint8_t** out_buf, size_t* out_len);
    bool pdal_deflate_compressor_finish(pdal_deflate_compressor_t* compressor,
                                        uint8_t** out_buf, size_t* out_len);
    void pdal_deflate_compressor_destroy(pdal_deflate_compressor_t* compressor);
    pdal_deflate_decompressor_t* pdal_deflate_decompressor_create();
    bool
    pdal_deflate_decompressor_update(pdal_deflate_decompressor_t* decompressor,
                                     const char* buf, size_t len,
                                     uint8_t** out_buf, size_t* out_len);
    bool
    pdal_deflate_decompressor_finish(pdal_deflate_decompressor_t* decompressor,
                                     uint8_t** out_buf, size_t* out_len);
    void pdal_deflate_decompressor_destroy(
        pdal_deflate_decompressor_t* decompressor);

    typedef struct
    {
        double x;
        double y;
        double z;
    } pdal_xyz_t;

    typedef struct
    {
        double m00;
        double m01;
        double m02;
        double m10;
        double m11;
        double m12;
        double m20;
        double m21;
        double m22;
    } pdal_rotation_matrix_t;

    pdal_xyz_t pdal_georeference_wgs84(double range, double scan_angle,
                                       pdal_rotation_matrix_t boresight,
                                       pdal_rotation_matrix_t imu,
                                       pdal_xyz_t gps);
    double pdal_barycentric_interpolation(double x1, double y1, double z1,
                                          double x2, double y2, double z2,
                                          double x3, double y3, double z3,
                                          double x, double y);

    // Raster math (column-major buffers)
    void pdal_math_grad_x(const double* data, size_t rows, size_t cols,
                          double* out);
    void pdal_math_grad_y(const double* data, size_t rows, size_t cols,
                          double* out);
    void pdal_math_dilate_diamond(double* data, size_t rows, size_t cols,
                                  int32_t iterations);
    void pdal_math_erode_diamond(double* data, size_t rows, size_t cols,
                                 int32_t iterations);
    void pdal_math_compute_centroid(const double* xyz, size_t count,
                                    double* out_xyz);

    // Oriented bounding box intersection
    bool pdal_obb_intersect(const double* center_a, const double* half_a,
                            const double* quat_a, const double* center_b,
                            const double* half_b, const double* quat_b);

    // Point segmentation
    bool pdal_segmentation_extract_clusters(
        const double* xyz, size_t count, uint64_t min_points,
        uint64_t max_points, double tolerance, bool is_3d,
        uint64_t** out_cluster_sizes, uint64_t* out_cluster_count,
        uint64_t** out_point_ids, uint64_t* out_point_count);
    void pdal_segmentation_segment_returns(const uint8_t* return_number,
                                           const uint8_t* number_of_returns,
                                           size_t count, bool want_first,
                                           bool want_intermediate,
                                           bool want_last, bool want_only,
                                           uint8_t* out_to_first);

    // OGRSpec
    char* pdal_ogr_spec_parse_json(const char* input);

    // Kernel
    int pdal_kernel_parse_stage_option(const char* input,
                                       bool allow_stage_prefix, char** stage,
                                       char** option, char** value);

    // Pipeline
    char* pdal_pipeline_generate_stage_tag(const char* stage_name,
                                           const char* explicit_tag,
                                           const char* const* existing_tags,
                                           uint64_t existing_count);

    // Plugin
    char* pdal_plugin_valid_name(const char* path, const char** types,
                                 uint64_t type_count,
                                 const char* dynamic_lib_extension);

    // PointLayout
    pdal_point_layout_t* pdal_point_layout_create();
    void pdal_point_layout_register_dim(pdal_point_layout_t* layout,
                                        const char* name, int type_id);
    int pdal_dimension_resolve_type(int type1, int type2);
    char* pdal_dimension_interpretation_name(int type_id);
    int pdal_dimension_type_from_name(const char* name);
    int pdal_dimension_type_from_base_and_size(const char* base, uint64_t size);
    char* pdal_dimension_fix_name(const char* name);
    void pdal_point_layout_destroy(pdal_point_layout_t* layout);

    // PointView
    typedef struct
    {
        double minx;
        double maxx;
        double miny;
        double maxy;
    } pdal_bounds2d_t;

    typedef struct
    {
        double minx;
        double maxx;
        double miny;
        double maxy;
        double minz;
        double maxz;
    } pdal_bounds3d_t;

    void pdal_bounds2d_clear(pdal_bounds2d_t* bounds);
    bool pdal_bounds2d_empty(const pdal_bounds2d_t* bounds);
    void pdal_bounds2d_grow_point(pdal_bounds2d_t* bounds, double x, double y);
    void pdal_bounds2d_grow_distance(pdal_bounds2d_t* bounds, double distance);
    void pdal_bounds2d_grow_bounds(pdal_bounds2d_t* bounds,
                                   const pdal_bounds2d_t* other);
    void pdal_bounds2d_clip(pdal_bounds2d_t* bounds,
                            const pdal_bounds2d_t* other);
    bool pdal_bounds2d_contains_point(const pdal_bounds2d_t* bounds, double x,
                                      double y);
    bool pdal_bounds2d_contains_bounds(const pdal_bounds2d_t* bounds,
                                       const pdal_bounds2d_t* other);
    bool pdal_bounds2d_overlaps(const pdal_bounds2d_t* bounds,
                                const pdal_bounds2d_t* other);
    char* pdal_bounds2d_parse(const char* input, uint64_t pos,
                              pdal_bounds2d_t* out_bounds, char** out_wkt,
                              uint64_t* out_pos);
    void pdal_bounds3d_clear(pdal_bounds3d_t* bounds);
    bool pdal_bounds3d_empty(const pdal_bounds3d_t* bounds);
    void pdal_bounds3d_grow_point(pdal_bounds3d_t* bounds, double x, double y,
                                  double z);
    void pdal_bounds3d_grow_bounds(pdal_bounds3d_t* bounds,
                                   const pdal_bounds3d_t* other);
    void pdal_bounds3d_grow_distance(pdal_bounds3d_t* bounds, double distance);
    void pdal_bounds3d_clip(pdal_bounds3d_t* bounds,
                            const pdal_bounds3d_t* other);
    bool pdal_bounds3d_contains_point(const pdal_bounds3d_t* bounds, double x,
                                      double y, double z);
    bool pdal_bounds3d_contains_bounds(const pdal_bounds3d_t* bounds,
                                       const pdal_bounds3d_t* other);
    bool pdal_bounds3d_overlaps(const pdal_bounds3d_t* bounds,
                                const pdal_bounds3d_t* other);
    char* pdal_bounds3d_parse(const char* input, uint64_t pos,
                              pdal_bounds3d_t* out_bounds, char** out_wkt,
                              uint64_t* out_pos);
    bool pdal_bounds2d_equal(const pdal_bounds2d_t* left,
                             const pdal_bounds2d_t* right);
    bool pdal_bounds3d_equal(const pdal_bounds3d_t* left,
                             const pdal_bounds3d_t* right);
    void pdal_bounds2d_default(pdal_bounds2d_t* out_bounds);
    void pdal_bounds3d_default(pdal_bounds3d_t* out_bounds);
    char* pdal_bounds2d_format(const pdal_bounds2d_t* bounds,
                               uint32_t precision);
    char* pdal_bounds3d_format(const pdal_bounds3d_t* bounds,
                               uint32_t precision);
    char* pdal_bounds2d_to_wkt(const pdal_bounds2d_t* bounds,
                               uint32_t precision);
    char* pdal_bounds3d_to_wkt(const pdal_bounds3d_t* bounds,
                               uint32_t precision);
    char* pdal_bounds2d_to_geojson(const pdal_bounds2d_t* bounds,
                                   uint32_t precision);

    typedef struct
    {
        uint64_t id;
        double sqr_dist;
    } pdal_spatial_result_t;

    typedef struct
    {
        double x_origin;
        double y_origin;
        uint64_t width;
        uint64_t height;
        double edge_length;
    } pdal_raster_limits_t;

    pdal_point_view_t* pdal_point_view_create(pdal_point_layout_t* layout);
    uint64_t pdal_point_view_add_point(pdal_point_view_t* view);
    void pdal_point_view_set_f64(pdal_point_view_t* view, uint64_t idx,
                                 const char* dim_name, double val);
    double pdal_point_view_get_f64(pdal_point_view_t* view, uint64_t idx,
                                   const char* dim_name);
    uint64_t pdal_point_view_dim_count(const pdal_point_view_t* view);
    char* pdal_point_view_dim_name(const pdal_point_view_t* view, uint64_t idx);
    int pdal_point_view_dim_type(const pdal_point_view_t* view, uint64_t idx);
    void
    pdal_point_view_set_spatial_reference(pdal_point_view_t* view,
                                          const pdal_spatial_reference_t* srs);
    pdal_spatial_reference_t*
    pdal_point_view_spatial_reference(const pdal_point_view_t* view);
    uint64_t pdal_point_view_length(pdal_point_view_t* view);
    uint64_t pdal_point_view_source_index(pdal_point_view_t* view,
                                          uint64_t idx);
    bool pdal_point_view_calculate_bounds_2d(const pdal_point_view_t* view,
                                             pdal_bounds2d_t* out_bounds);
    bool pdal_point_view_calculate_bounds_3d(const pdal_point_view_t* view,
                                             pdal_bounds3d_t* out_bounds);
    uint64_t pdal_point_view_mesh_triangle_count(const pdal_point_view_t* view);
    uint64_t
    pdal_point_view_named_mesh_triangle_count(const pdal_point_view_t* view,
                                              const char* name);
    bool pdal_point_view_mesh_triangle(const pdal_point_view_t* view,
                                       uint64_t idx, uint64_t* a, uint64_t* b,
                                       uint64_t* c);
    bool pdal_point_view_named_mesh_triangle(const pdal_point_view_t* view,
                                             const char* name, uint64_t idx,
                                             uint64_t* a, uint64_t* b,
                                             uint64_t* c);
    bool pdal_point_view_add_mesh_triangle(pdal_point_view_t* view, uint64_t a,
                                           uint64_t b, uint64_t c);
    bool pdal_point_view_add_named_mesh_triangle(pdal_point_view_t* view,
                                                 const char* name, uint64_t a,
                                                 uint64_t b, uint64_t c);
    uint64_t pdal_point_view_raster_count(const pdal_point_view_t* view);
    char* pdal_point_view_raster_name(const pdal_point_view_t* view,
                                      uint64_t idx);
    bool pdal_point_view_create_raster(pdal_point_view_t* view,
                                       const char* name,
                                       const pdal_raster_limits_t* limits,
                                       double initializer);
    bool pdal_point_view_raster_limits(const pdal_point_view_t* view,
                                       const char* name,
                                       pdal_raster_limits_t* out_limits);
    double pdal_point_view_raster_initializer(const pdal_point_view_t* view,
                                              const char* name);
    bool pdal_point_view_raster_cell(const pdal_point_view_t* view,
                                     const char* name, uint64_t x, uint64_t y,
                                     double* out_value);
    bool pdal_point_view_set_raster_cell(pdal_point_view_t* view,
                                         const char* name, uint64_t x,
                                         uint64_t y, double value);
    uint64_t pdal_point_view_knn(const pdal_point_view_t* view,
                                 const char* const* dim_names,
                                 const double* query, uint64_t dim_count,
                                 uint64_t k, uint64_t stride,
                                 pdal_spatial_result_t* out_results,
                                 uint64_t max_results);
    pdal_spatial_result_t* pdal_point_view_radius(const pdal_point_view_t* view,
                                                  const char* const* dim_names,
                                                  const double* query,
                                                  uint64_t dim_count,
                                                  double radius,
                                                  uint64_t* out_len);
    void pdal_spatial_results_free(pdal_spatial_result_t* ptr, uint64_t len);
    char*
    pdal_point_view_dimension_summaries_json(const pdal_point_view_t* view);
    bool pdal_point_view_split_where(const pdal_point_view_t* view,
                                     const char* expression,
                                     pdal_point_view_t** out_keep,
                                     pdal_point_view_t** out_skip);
    void pdal_point_view_destroy(pdal_point_view_t* view);

    // QuadIndex
    pdal_quad_index_t* pdal_quad_index_create(const double* xs,
                                              const double* ys,
                                              const uint64_t* ids,
                                              uint64_t count, double x_min,
                                              double y_min, double x_max,
                                              double y_max, uint64_t top_level);
    void pdal_quad_index_bounds(const pdal_quad_index_t* index,
                                pdal_bounds2d_t* out_bounds);
    uint64_t pdal_quad_index_depth(const pdal_quad_index_t* index);
    uint64_t* pdal_quad_index_fills(const pdal_quad_index_t* index,
                                    uint64_t* out_len);
    uint64_t* pdal_quad_index_points_by_depth(const pdal_quad_index_t* index,
                                              uint64_t depth_begin,
                                              uint64_t depth_end,
                                              uint64_t* out_len);
    uint64_t* pdal_quad_index_points_in_bounds(const pdal_quad_index_t* index,
                                               double x_min, double y_min,
                                               double x_max, double y_max,
                                               uint64_t depth_begin,
                                               uint64_t depth_end,
                                               uint64_t* out_len);
    uint64_t* pdal_quad_index_points_raster_level(
        const pdal_quad_index_t* index, uint64_t rasterize, double* x_begin,
        double* x_end, double* x_step, double* y_begin, double* y_end,
        double* y_step, uint64_t* out_len);
    uint64_t* pdal_quad_index_points_raster_bounds(
        const pdal_quad_index_t* index, double x_begin, double x_end,
        double x_step, double y_begin, double y_end, double y_step,
        uint64_t* out_len);
    void pdal_u64_array_free(uint64_t* ptr, uint64_t len);
    void pdal_quad_index_destroy(pdal_quad_index_t* index);

    // SpatialReference
    pdal_spatial_reference_t* pdal_spatial_reference_create(const char* text);
    pdal_spatial_reference_t*
    pdal_spatial_reference_create_with_epoch(const char* text, double epoch);
    bool pdal_spatial_reference_empty(const pdal_spatial_reference_t* srs);
    char* pdal_spatial_reference_text(const pdal_spatial_reference_t* srs);
    double pdal_spatial_reference_epoch(const pdal_spatial_reference_t* srs);
    void pdal_spatial_reference_set_epoch(pdal_spatial_reference_t* srs,
                                          double epoch);
    pdal_metadata_node_t*
    pdal_spatial_reference_to_metadata(const pdal_spatial_reference_t* srs);
    int32_t pdal_spatial_reference_calculate_zone(double lon, double lat);
    char* pdal_spatial_reference_wgs84_code_from_zone(int32_t zone);
    void pdal_spatial_reference_destroy(pdal_spatial_reference_t* srs);

    // Scaling
    typedef struct
    {
        bool is_auto;
        double value;
    } pdal_xform_component_t;

    typedef struct
    {
        pdal_xform_component_t offset;
        pdal_xform_component_t scale;
    } pdal_xform_t;

    typedef struct
    {
        pdal_xform_t x;
        pdal_xform_t y;
        pdal_xform_t z;
    } pdal_scaling_t;

    bool pdal_scaling_set_auto_xform(const double* xs, const double* ys,
                                     const double* zs, uint64_t count,
                                     pdal_scaling_t* scaling);

    // Geometry
    bool pdal_geometry_wkt_is_valid(const char* wkt, bool* out_value);
    bool pdal_geometry_wkt_distance_to_point(const char* wkt, double x,
                                             double y, double z,
                                             double* out_value);
    bool pdal_geometry_wkt_contains_point(const char* wkt, double x, double y,
                                          bool* out_value);

    // XML schema
    char* pdal_xml_schema_remap_old_name(const char* name);

    // Metadata
    pdal_metadata_node_t* pdal_metadata_node_create(const char* name);
    pdal_metadata_node_t*
    pdal_metadata_node_clone(const pdal_metadata_node_t* node);
    char* pdal_metadata_node_name(const pdal_metadata_node_t* node);
    char* pdal_metadata_node_type(const pdal_metadata_node_t* node);
    char* pdal_metadata_node_description(const pdal_metadata_node_t* node);
    void pdal_metadata_node_set_string(pdal_metadata_node_t* node,
                                       const char* value);
    void pdal_metadata_node_set_type(pdal_metadata_node_t* node,
                                     const char* type_name);
    void pdal_metadata_node_set_description(pdal_metadata_node_t* node,
                                            const char* description);
    void pdal_metadata_node_set_i64(pdal_metadata_node_t* node, int64_t value);
    void pdal_metadata_node_set_u64(pdal_metadata_node_t* node, uint64_t value);
    void pdal_metadata_node_set_f64(pdal_metadata_node_t* node, double value);
    void pdal_metadata_node_set_bool(pdal_metadata_node_t* node, bool value);
    uint8_t pdal_metadata_node_value_kind(const pdal_metadata_node_t* node);
    char* pdal_metadata_node_value(const pdal_metadata_node_t* node);
    int64_t pdal_metadata_node_value_i64(const pdal_metadata_node_t* node);
    uint64_t pdal_metadata_node_value_u64(const pdal_metadata_node_t* node);
    double pdal_metadata_node_value_f64(const pdal_metadata_node_t* node);
    bool pdal_metadata_node_value_bool(const pdal_metadata_node_t* node);
    char* pdal_metadata_json_value(const char* type_name, const char* value);
    bool pdal_metadata_value_as_i64(const char* type_name, const char* value,
                                    int64_t* out_value);
    bool pdal_metadata_value_as_u64(const char* type_name, const char* value,
                                    uint64_t* out_value);
    bool pdal_metadata_value_as_f64(const char* type_name, const char* value,
                                    double* out_value);
    bool pdal_metadata_value_as_bool(const char* type_name, const char* value,
                                     bool* out_value);
    void pdal_metadata_node_add_child(pdal_metadata_node_t* node,
                                      pdal_metadata_node_t* child);
    void pdal_metadata_node_add_child_clone(pdal_metadata_node_t* node,
                                            const pdal_metadata_node_t* child);
    void pdal_metadata_node_add_or_update_child(pdal_metadata_node_t* node,
                                                pdal_metadata_node_t* child);
    void pdal_metadata_node_add_or_update_child_clone(
        pdal_metadata_node_t* node, const pdal_metadata_node_t* child);
    uint64_t pdal_metadata_node_child_count(const pdal_metadata_node_t* node);
    pdal_metadata_node_t*
    pdal_metadata_node_child(const pdal_metadata_node_t* node, uint64_t idx);
    uint64_t
    pdal_metadata_node_child_named_count(const pdal_metadata_node_t* node,
                                         const char* name);
    pdal_metadata_node_t*
    pdal_metadata_node_child_named(const pdal_metadata_node_t* node,
                                   const char* name, uint64_t idx);
    void pdal_metadata_node_destroy(pdal_metadata_node_t* node);

    // Stage
    pdal_stage_t* pdal_stage_create_decimation(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_head(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_tail(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_locate(const pdal_options_t* ops);

    pdal_stage_t* pdal_stage_create_ferry(const char* const* from_dims,
                                          const char* const* to_dims,
                                          uint64_t count);
    pdal_stage_t* pdal_stage_create_ferry_specs(const char* const* specs,
                                                uint64_t count);
    bool pdal_stage_validate_assign_statement(const char* statement);
    void pdal_stage_ferry_point(pdal_stage_t* stage, pdal_point_view_t* view,
                                uint64_t idx);

    pdal_stage_t* pdal_stage_create_randomize(const pdal_options_t* ops);

    typedef struct
    {
        const char* dim_name;
        double lower_bound;
        double upper_bound;
        bool inclusive_lower;
        bool inclusive_upper;
        bool negate;
    } pdal_range_limit_t;

    char* pdal_range_limit_parse(const char* input, char** out_dim_name,
                                 double* lower_bound, double* upper_bound,
                                 bool* inclusive_lower, bool* inclusive_upper,
                                 bool* negate, uint64_t* consumed);
    pdal_stage_t* pdal_stage_create_range(const pdal_range_limit_t* limits,
                                          uint64_t count);
    bool pdal_stage_range_point_passes(pdal_stage_t* stage,
                                       pdal_point_view_t* view, uint64_t idx);

    void pdal_stage_destroy(pdal_stage_t* stage);
    void pdal_stage_reset(pdal_stage_t* stage);
    pdal_metadata_node_t* pdal_stage_metadata(const pdal_stage_t* stage);
    bool pdal_stage_process_one(pdal_stage_t* stage);
    bool pdal_stage_process_one_at(pdal_stage_t* stage, pdal_point_view_t* view,
                                   uint64_t idx);
    pdal_point_view_t* pdal_stage_run(pdal_stage_t* stage,
                                      pdal_point_view_t* input);
    pdal_point_view_t*
    pdal_stage_run_with_reference(pdal_stage_t* stage, pdal_point_view_t* input,
                                  pdal_point_view_t* reference);
    uint64_t pdal_stage_run_multi(pdal_stage_t* stage, pdal_point_view_t* input,
                                  pdal_point_view_t** outputs,
                                  uint64_t max_outputs);

    pdal_stage_t* pdal_stage_create_sort(const char* const* dims,
                                         uint64_t count, const char* order,
                                         const char* algorithm);
    pdal_stage_t* pdal_stage_create_mongoexpression(const char* expr);
    pdal_stage_t* pdal_stage_create_expression(const char* const* exprs,
                                               uint64_t count);
    pdal_stage_t* pdal_stage_create_expressionstats(const char* dim_name,
                                                    const char* const* sources,
                                                    uint64_t count);
    pdal_stage_t* pdal_stage_create_returns(const char* const* groups,
                                            uint64_t count);
    pdal_stage_t* pdal_stage_create_separatescanline(uint64_t groupby);
    pdal_stage_t* pdal_stage_create_groupby(const char* dim_name);
    pdal_stage_t* pdal_stage_create_labelduplicates(const char* const* dims,
                                                    uint64_t count);
    pdal_stage_t* pdal_stage_create_merge();
    void pdal_stage_merge_append(pdal_stage_t* stage, pdal_point_view_t* view);
    pdal_stage_t* pdal_stage_create_mortonorder(bool reverse);
    pdal_stage_t* pdal_stage_create_transformation(const double* matrix);
    void pdal_stage_transformation_point(pdal_stage_t* stage,
                                         pdal_point_view_t* view, uint64_t idx);
    char* pdal_transformation_matrix_parse(const char* input,
                                           double* out_matrix);
    char* pdal_transformation_matrix_format(const double* matrix);

    pdal_stage_t* pdal_stage_create_voxeldownsize(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_sample(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_hexbin(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_faceraster(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_radialdensity(double radius);
    pdal_stage_t* pdal_stage_create_nndistance(uint64_t k, const char* mode);
    pdal_stage_t* pdal_stage_create_zsmooth(double radius, double position,
                                            const char* dim_name);
    pdal_stage_t* pdal_stage_create_outlier(const char* method, uint64_t min_k,
                                            double radius, uint64_t mean_k,
                                            double multiplier,
                                            uint8_t class_label);
    pdal_stage_t* pdal_stage_create_dbscan(uint64_t min_points, double eps,
                                           const char* const* dims,
                                           uint64_t count);
    pdal_stage_t* pdal_stage_create_covariancefeatures(
        uint64_t knn, bool has_radius, double radius, uint64_t min_k,
        uint64_t stride, uint8_t mode, bool optimal, const char* const* dims,
        uint64_t dim_count);
    pdal_stage_t* pdal_stage_create_lof(uint64_t minpts);
    pdal_stage_t* pdal_stage_create_elm(double cell, uint8_t class_label,
                                        double threshold);
    pdal_stage_t* pdal_stage_create_smrf(double cell, double slope,
                                         bool has_window, double window,
                                         double scalar, double threshold,
                                         uint8_t ground_class,
                                         uint8_t other_class, bool only_ground,
                                         const char* const* returns,
                                         uint64_t count);
    pdal_stage_t* pdal_stage_create_skewnessbalancing(uint8_t ground_class,
                                                      uint8_t other_class,
                                                      bool only_ground);
    pdal_stage_t* pdal_stage_create_iqr(double multiplier,
                                        const char* dim_name);
    pdal_stage_t* pdal_stage_create_mad(double multiplier, const char* dim_name,
                                        double mad_multiplier);
    pdal_stage_t* pdal_stage_create_hagnn(uint64_t count, double max_distance,
                                          bool allow_extrapolation,
                                          uint8_t class_label);
    pdal_stage_t* pdal_stage_create_cluster(uint64_t min_points,
                                            uint64_t max_points,
                                            double tolerance, bool is_3d);
    pdal_stage_t* pdal_stage_create_sparsesurface(double radius,
                                                  uint8_t ground_class,
                                                  uint8_t low_point_class);
    pdal_stage_t* pdal_stage_create_voxelcenternearestneighbor(double cell);
    pdal_stage_t* pdal_stage_create_voxelcentroidnearestneighbor(double cell);
    pdal_stage_t* pdal_stage_create_reciprocity(uint64_t knn);
    pdal_stage_t* pdal_stage_create_estimaterank(uint64_t knn,
                                                 double threshold);
    pdal_stage_t* pdal_stage_create_approximatecoplanar(uint64_t knn,
                                                        double threshold1,
                                                        double threshold2);
    pdal_stage_t* pdal_stage_create_planefit(uint64_t knn);
    pdal_stage_t* pdal_stage_create_miniball(uint64_t knn);
    pdal_stage_t* pdal_stage_create_eigenvalues(uint64_t knn, bool normalize,
                                                uint64_t stride,
                                                bool has_radius, double radius,
                                                uint64_t min_k);
    pdal_stage_t* pdal_stage_create_optimalneighborhood(uint64_t min_k,
                                                        uint64_t max_k);
    pdal_stage_t* pdal_stage_create_normal(uint64_t knn, bool has_radius,
                                           double radius, bool has_viewpoint,
                                           double viewpoint_x,
                                           double viewpoint_y,
                                           double viewpoint_z, bool always_up);
    pdal_stage_t* pdal_stage_create_relaxationdartthrowing(
        double decay, double radius, double terminal_radius, uint64_t count,
        bool shuffle, bool has_seed, uint32_t seed);
    pdal_stage_t* pdal_stage_create_lloydkmeans(uint64_t k, uint64_t maxiters,
                                                const char* const* dims,
                                                uint64_t dim_count);
    pdal_stage_t* pdal_stage_create_straighten(const char* polyline,
                                               bool reverse, double offset);

    char* pdal_grid_decimation_validate(double resolution,
                                        const char* output_type);
    uint64_t* pdal_grid_decimation_get_kept_indices(
        const pdal_point_view_t* view, double resolution,
        const char* output_type, uint64_t* out_len);
    void pdal_free_u64_array(uint64_t* ptr, uint64_t len);
    uint64_t* pdal_delaunay_triangulate(const pdal_point_view_t* view,
                                        uint64_t* out_len);
    pdal_point_view_t* pdal_icp_register(
        const pdal_point_view_t* fixed, const pdal_point_view_t* moving,
        int32_t max_iters, int32_t max_similar, double rotation_threshold,
        double translation_threshold, double mse_abs, bool has_maxdist,
        double maxdist, bool has_init, const double* init,
        double* out_transform, double* out_centroid, bool* out_converged,
        double* out_mse);

    pdal_stage_t* pdal_stage_create_divider(int32_t mode, int32_t size_mode,
                                            uint64_t size, const uint8_t* evals,
                                            uint64_t evals_count);
    pdal_stage_t* pdal_stage_create_splitter(double length, double origin_x,
                                             double origin_y, double buffer);
    pdal_stage_t* pdal_stage_create_gpstimeconvert(const pdal_options_t* ops);
    pdal_stage_t* pdal_stage_create_chipper(uint64_t capacity);
    pdal_stage_t* pdal_stage_create_farthestpointsampling(uint64_t count);

    typedef struct
    {
        const char* dim_name;
        double value;
        double lower_bound;
        double upper_bound;
        bool inclusive_lower;
        bool inclusive_upper;
        bool negate;
    } pdal_assign_range_t;

    pdal_stage_t* pdal_stage_create_assign(
        bool has_condition, const char* cond_dim, double cond_lower,
        double cond_upper, bool cond_inclusive_lower, bool cond_inclusive_upper,
        bool cond_negate, const pdal_assign_range_t* assignments,
        uint64_t count);
    pdal_stage_t* pdal_stage_create_radiusassign(
        const pdal_range_limit_t* src_limits, uint64_t src_count,
        const pdal_range_limit_t* reference_limits, uint64_t reference_count,
        const pdal_assign_range_t* assignments, uint64_t assignment_count,
        double radius, bool search_3d, double max_2d_above,
        double max_2d_below);
    pdal_stage_t*
    pdal_stage_create_neighborclassifier(const pdal_range_limit_t* domain,
                                         uint64_t domain_count, uint64_t k,
                                         const char* dim_name);

    typedef struct
    {
        uint64_t count;
        double min;
        double max;
        double m1;
        double m2;
        double m3;
        double m4;
        double median;
        double mad;
        double* unique_values;
        uint64_t* unique_counts;
        uint64_t unique_len;
    } pdal_dim_stats_t;

    void pdal_stats_compute(pdal_point_view_t* view, const char* const* dims,
                            uint64_t dims_count, bool advanced,
                            const char* const* enums, uint64_t enums_count,
                            const char* const* counts, uint64_t counts_count,
                            const char* const* globals, uint64_t globals_count,
                            pdal_dim_stats_t* out_stats);

    void pdal_free_stats_arrays(pdal_dim_stats_t* ptr, uint64_t dims_count);

    typedef struct
    {
        double value;
        uint64_t count;
    } pdal_summary_merge_entry_t;

    typedef struct
    {
        const char* name;
        uint32_t enumerate;
        bool advanced;
        uint64_t count;
        double min;
        double max;
        double m1;
        double m2;
        double m3;
        double m4;
        double median;
        double mad;
        pdal_summary_merge_entry_t* values;
        uint64_t values_len;
        uint64_t values_capacity;
        double* data;
        uint64_t data_len;
        uint64_t data_capacity;
    } pdal_summary_merge_state_t;

    bool pdal_stats_summary_merge(pdal_summary_merge_state_t* target,
                                  const pdal_summary_merge_state_t* other);

    pdal_metadata_node_t*
    pdal_expressionstats_metadata(pdal_point_view_t* view, const char* dim_name,
                                  const char* const* expressions,
                                  uint64_t count);

    // GDAL/PROJ Family Additions
    pdal_stage_t* pdal_stage_create_h3(uint64_t resolution);
    pdal_stage_t* pdal_stage_create_reprojection(const char* out_srs,
                                                 const char* in_srs,
                                                 bool error_on_failure);
    pdal_stage_t* pdal_stage_create_geomdistance(const char* wkt,
                                                 const char* dim_name,
                                                 bool ring);
    pdal_stage_t* pdal_stage_create_overlay(const char* dim_name,
                                            const char* datasource,
                                            const char* column);
    pdal_stage_t* pdal_stage_create_georeference(const char* out_srs);
    char*
    pdal_georeference_validate_coordinate_system(const char* coordinate_system);
    char*
    pdal_georeference_validate_transform_beam(const pdal_point_layout_t* layout,
                                              bool transform_beam);
    pdal_stage_t* pdal_stage_create_projpipeline(const char* out_srs,
                                                 const char* coord_op,
                                                 bool reverse);

    typedef struct
    {
        const char* name;
        uint32_t band;
        double scale;
    } pdal_band_info_t;

    pdal_stage_t* pdal_stage_create_colorinterp(const char* dim_name,
                                                const char* ramp, double min,
                                                double max, bool clamp,
                                                bool invert);
    char* pdal_colorinterp_validate_prepared(const pdal_point_layout_t* layout,
                                             const char* dim_name, double min,
                                             double max);
    bool pdal_colorinterp_pipeline_streamable(double min, double max);
    pdal_stage_t* pdal_stage_create_colorization(const char* raster_path,
                                                 const pdal_band_info_t* bands,
                                                 uint64_t count);
    pdal_stage_t* pdal_stage_create_dem(const char* dim_name,
                                        const char* raster_path, int32_t band,
                                        double lower_bound, double upper_bound);
    pdal_stage_t* pdal_stage_create_hag_dem(const char* raster_path,
                                            int32_t band, bool zero_ground,
                                            double min_clamp, double max_clamp,
                                            double nodata_height,
                                            uint8_t ground_class);

    typedef struct
    {
        double minx;
        double miny;
        double minz;
        double maxx;
        double maxy;
        double maxz;
    } pdal_box3d_t;

    typedef struct
    {
        double x;
        double y;
        double z;
    } pdal_point3d_t;

    pdal_stage_t*
    pdal_stage_create_crop(bool outside, const pdal_box3d_t* bounds,
                           uint64_t bounds_count, const char* const* polygons,
                           uint64_t poly_count, const pdal_point3d_t* centers,
                           uint64_t center_count, double distance);

    // Reader
    pdal_reader_t* pdal_reader_create_faux(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_text(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_pcd(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_pts(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_ptx(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_ilvis2(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_obj(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_ply(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_qfit(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_sbet(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_smrmsg(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_optech(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_terrasolid(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_fbi(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_bpf(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_gdal(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_las(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_laz(const pdal_options_t* ops);
    bool pdal_las_detect_copc(const char* path);
    pdal_reader_t* pdal_reader_create_spz(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_stac(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_copc(const pdal_options_t* ops);
    pdal_reader_t* pdal_reader_create_ept(const pdal_options_t* ops);
    pdal_point_view_t* pdal_reader_read_first(pdal_reader_t* reader);
    pdal_metadata_node_t* pdal_reader_metadata(const pdal_reader_t* reader);
    pdal_metadata_node_t* pdal_ilvis2_metadata_read(const char* filename);
    void pdal_reader_destroy(pdal_reader_t* reader);

    typedef const unsigned char* (*pdal_memoryview_incrementer_t)(
        uint64_t point_id, void* user_data);

    typedef struct
    {
        const char* name;
        int type_id;
        uint64_t offset;
    } pdal_memoryview_field_t;

    pdal_point_view_t* pdal_memoryview_read(
        const pdal_memoryview_field_t* fields, uint64_t field_count,
        pdal_memoryview_incrementer_t incrementer, void* user_data,
        uint64_t depth, uint64_t rows, uint64_t columns, bool column_major);
    char* pdal_memoryview_shape_parse(const char* input, uint64_t* out_depth,
                                      uint64_t* out_rows,
                                      uint64_t* out_columns);

    // Stage registry: construct implemented stages from PDAL driver names.
    pdal_reader_t* pdal_create_reader(const char* name,
                                      const pdal_options_t* ops);
    pdal_writer_t* pdal_create_writer(const char* name,
                                      const pdal_options_t* ops);

    // Metrics: compare two point cloud files.
    int pdal_hausdorff(const char* path_a, const char* path_b,
                       double* hausdorff, double* modified_hausdorff);
    int pdal_chamfer(const char* path_a, const char* path_b, double* chamfer);
    char* pdal_delta(const char* path_a, const char* path_b);

    // Writer
    pdal_writer_t* pdal_writer_create_null(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_fbi(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_bpf(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_text(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_pcd(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_ply(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_gltf(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_sbet(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_las(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_laz(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_spz(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_ogr(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_gdal(const pdal_options_t* ops);
    pdal_writer_t* pdal_writer_create_raster(const pdal_options_t* ops);
    bool pdal_writer_write_view(pdal_writer_t* writer,
                                const pdal_point_view_t* view);
    bool pdal_writer_write_views(pdal_writer_t* writer,
                                 const pdal_point_view_t* const* views,
                                 uint64_t count);
    void pdal_writer_destroy(pdal_writer_t* writer);

    // Pipeline
    typedef struct
    {
        uint64_t point_count;
        uint64_t view_count;
        bool has_bounds_2d;
        pdal_bounds2d_t bounds_2d;
        bool has_bounds_3d;
        pdal_bounds3d_t bounds_3d;
    } pdal_pipeline_result_t;

    pdal_pipeline_t* pdal_pipeline_create();
    pdal_pipeline_t* pdal_pipeline_create_json(const char* json);
    void pdal_pipeline_destroy(pdal_pipeline_t* pipeline);
    int64_t pdal_pipeline_add_stage(pdal_pipeline_t* pipeline,
                                    pdal_stage_t* stage);
    int64_t pdal_pipeline_add_stage_tagged(pdal_pipeline_t* pipeline,
                                           pdal_stage_t* stage,
                                           const char* tag);
    int64_t pdal_pipeline_add_reader(pdal_pipeline_t* pipeline,
                                     pdal_reader_t* reader);
    int64_t pdal_pipeline_add_writer(pdal_pipeline_t* pipeline,
                                     pdal_writer_t* writer);
    int64_t pdal_pipeline_add_dependency(pdal_pipeline_t* pipeline,
                                         uint64_t target, uint64_t input);
    pdal_point_view_t* pdal_pipeline_execute(pdal_pipeline_t* pipeline,
                                             pdal_point_view_t* input_view);
    int64_t pdal_pipeline_execute_count(pdal_pipeline_t* pipeline,
                                        pdal_point_view_t* input_view);
    int64_t pdal_pipeline_execute_result(pdal_pipeline_t* pipeline,
                                         pdal_point_view_t* input_view,
                                         pdal_pipeline_result_t* out_result);
    char* pdal_pipeline_execute_summary_json(pdal_pipeline_t* pipeline,
                                             pdal_point_view_t* input_view);
    uint64_t pdal_pipeline_stage_count(const pdal_pipeline_t* pipeline);
    pdal_metadata_node_t*
    pdal_pipeline_metadata(const pdal_pipeline_t* pipeline);
    int64_t pdal_pipeline_find_by_tag(const pdal_pipeline_t* pipeline,
                                      const char* tag);

    // CLI / Kernel dispatch
    const char* pdal_version_string(void);
    char* pdal_kernel_list_json(void);
    char* pdal_stage_list_json(void);
    char* pdal_stage_options_json(const char* stage_name);
    int pdal_kernel_run(const char* kernel_name, int argc,
                        const char* const* argv);
    void pdal_capi_free(void* ptr);

#ifdef __cplusplus
}
#endif
