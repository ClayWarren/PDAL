#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct pdal_options pdal_options_t;
typedef struct pdal_point_layout pdal_point_layout_t;
typedef struct pdal_point_view pdal_point_view_t;
typedef struct pdal_stage pdal_stage_t;

const char* pdal_last_error();
void pdal_clear_error();

// Options
pdal_options_t* pdal_options_create();
void pdal_options_add_f64(pdal_options_t* ops, const char* key, double value);
void pdal_options_add_u64(pdal_options_t* ops, const char* key, uint64_t value);
void pdal_options_add_str(pdal_options_t* ops, const char* key, const char* value);
void pdal_options_destroy(pdal_options_t* ops);

// PointLayout
pdal_point_layout_t* pdal_point_layout_create();
void pdal_point_layout_register_dim(pdal_point_layout_t* layout, const char* name, int type_id);
void pdal_point_layout_destroy(pdal_point_layout_t* layout);

// PointView
pdal_point_view_t* pdal_point_view_create(pdal_point_layout_t* layout);
uint64_t pdal_point_view_add_point(pdal_point_view_t* view);
void pdal_point_view_set_f64(pdal_point_view_t* view, uint64_t idx, const char* dim_name, double val);
double pdal_point_view_get_f64(pdal_point_view_t* view, uint64_t idx, const char* dim_name);
void pdal_point_view_set_spatial_reference(pdal_point_view_t* view, const char* wkt);
uint64_t pdal_point_view_length(pdal_point_view_t* view);
uint64_t pdal_point_view_source_index(pdal_point_view_t* view, uint64_t idx);
void pdal_point_view_destroy(pdal_point_view_t* view);

// Stage
pdal_stage_t* pdal_stage_create_decimation(const pdal_options_t* ops);
pdal_stage_t* pdal_stage_create_head(const pdal_options_t* ops);
pdal_stage_t* pdal_stage_create_tail(const pdal_options_t* ops);
pdal_stage_t* pdal_stage_create_locate(const pdal_options_t* ops);

pdal_stage_t* pdal_stage_create_ferry(const char* const* from_dims, const char* const* to_dims, uint64_t count);
void pdal_stage_ferry_point(pdal_stage_t* stage, pdal_point_view_t* view, uint64_t idx);

pdal_stage_t* pdal_stage_create_randomize(const pdal_options_t* ops);

typedef struct {
    const char* dim_name;
    double lower_bound;
    double upper_bound;
    bool inclusive_lower;
    bool inclusive_upper;
    bool negate;
} pdal_range_limit_t;

pdal_stage_t* pdal_stage_create_range(const pdal_range_limit_t* limits, uint64_t count);

pdal_stage_t* pdal_stage_create_expression(const char* const* sources, uint64_t count);
pdal_stage_t* pdal_stage_create_expressionstats(const char* dim_name, const char* const* sources, uint64_t count);
pdal_stage_t* pdal_stage_create_h3(uint64_t resolution);
pdal_stage_t* pdal_stage_create_mongoexpression(const char* json);
pdal_stage_t* pdal_stage_create_reprojection(const char* out_srs, const char* in_srs, bool error_on_failure);
pdal_stage_t* pdal_stage_create_geomdistance(const char* wkt, const char* dim_name);
pdal_stage_t* pdal_stage_create_overlay(const char* dim_name, const char* datasource, const char* column);
pdal_stage_t* pdal_stage_create_georeference(const char* out_srs);
pdal_stage_t* pdal_stage_create_projpipeline(const char* out_srs, const char* coord_op, bool reverse);
pdal_stage_t* pdal_stage_create_colorinterp(const char* dim_name, const char* ramp, double min, double max, bool clamp, bool invert);

typedef struct {
    const char* name;
    uint32_t band;
    double scale;
} pdal_band_info_t;

pdal_stage_t* pdal_stage_create_colorization(const char* raster_path, const pdal_band_info_t* bands, uint64_t count);
pdal_stage_t* pdal_stage_create_dem(const char* dim_name, const char* raster_path, int32_t band, double lower_bound, double upper_bound);
pdal_stage_t* pdal_stage_create_hag_dem(const char* raster_path, int32_t band, bool zero_ground, double min_clamp, double max_clamp, double nodata_height, uint8_t ground_class);

typedef struct {
    double minx;
    double miny;
    double minz;
    double maxx;
    double maxy;
    double maxz;
} pdal_box3d_t;

typedef struct {
    double x;
    double y;
    double z;
} pdal_point3d_t;

pdal_stage_t* pdal_stage_create_crop(bool outside, const pdal_box3d_t* bounds, uint64_t bounds_count, const char* const* polygons, uint64_t poly_count, const pdal_point3d_t* centers, uint64_t center_count, double distance);

void pdal_stage_destroy(pdal_stage_t* stage);
void pdal_stage_reset(pdal_stage_t* stage);
bool pdal_stage_process_one(pdal_stage_t* stage, pdal_point_view_t* view, uint64_t idx);
pdal_point_view_t* pdal_stage_run(pdal_stage_t* stage, pdal_point_view_t* input);
uint64_t pdal_stage_run_multi(pdal_stage_t* stage, pdal_point_view_t* input, pdal_point_view_t** outputs, uint64_t max_outputs);

pdal_stage_t* pdal_stage_create_sort(const char* const* dims, uint64_t count, const char* order, const char* algorithm);
pdal_stage_t* pdal_stage_create_returns(const char* const* groups, uint64_t count);
pdal_stage_t* pdal_stage_create_separatescanline(uint64_t groupby);
pdal_stage_t* pdal_stage_create_groupby(const char* dim_name);
pdal_stage_t* pdal_stage_create_labelduplicates(const char* const* dims, uint64_t count);
pdal_stage_t* pdal_stage_create_merge();
void pdal_stage_merge_append(pdal_stage_t* stage, pdal_point_view_t* view);
pdal_stage_t* pdal_stage_create_mortonorder(bool reverse);
pdal_stage_t* pdal_stage_create_transformation(const double* matrix);
void pdal_stage_transformation_point(pdal_stage_t* stage, pdal_point_view_t* view, uint64_t idx);

pdal_stage_t* pdal_stage_create_voxeldownsize(const pdal_options_t* ops);
pdal_stage_t* pdal_stage_create_sample(const pdal_options_t* ops);
pdal_stage_t* pdal_stage_create_radialdensity(double radius);
pdal_stage_t* pdal_stage_create_nndistance(uint64_t k, const char* mode);
pdal_stage_t* pdal_stage_create_zsmooth(double radius, double position, const char* dim_name);
pdal_stage_t* pdal_stage_create_outlier(const char* method, uint64_t min_k, double radius, uint64_t mean_k, double multiplier, uint8_t class_label);
pdal_stage_t* pdal_stage_create_dbscan(uint64_t min_points, double eps, const char* const* dims, uint64_t count);
pdal_stage_t* pdal_stage_create_covariancefeatures(uint64_t knn, bool has_radius, double radius, uint64_t min_k, uint64_t stride, uint8_t mode, bool optimal, const char* const* dims, uint64_t dim_count);
pdal_stage_t* pdal_stage_create_lof(uint64_t minpts);
pdal_stage_t* pdal_stage_create_miniball(uint64_t knn);
pdal_stage_t* pdal_stage_create_elm(double cell, uint8_t class_label, double threshold);
pdal_stage_t* pdal_stage_create_skewnessbalancing(uint8_t ground_class, uint8_t other_class, bool only_ground);
pdal_stage_t* pdal_stage_create_splitter(double length, double x_origin, double y_origin, double buffer);
pdal_stage_t* pdal_stage_create_iqr(double multiplier, const char* dim_name);
pdal_stage_t* pdal_stage_create_mad(double multiplier, const char* dim_name, double mad_multiplier);
pdal_stage_t* pdal_stage_create_hagnn(uint64_t count, double max_distance, bool allow_extrapolation, uint8_t class_label);
pdal_stage_t* pdal_stage_create_cluster(uint64_t min_points, uint64_t max_points, double tolerance, bool is_3d);
pdal_stage_t* pdal_stage_create_sparsesurface(double radius, uint8_t ground_class, uint8_t low_point_class);
pdal_stage_t* pdal_stage_create_litree(uint64_t min_size, double min_hag, double dummy_radius);
pdal_stage_t* pdal_stage_create_neighborclassifier(uint64_t k, const char* dim_name, const pdal_range_limit_t* domain, uint64_t domain_count);
uint64_t* pdal_radiusassign_get_update_indices(pdal_point_view_t* view, const pdal_range_limit_t* src_domain, uint64_t src_count, const pdal_range_limit_t* reference_domain, uint64_t reference_count, double radius, bool search_3d, double max_2d_above, double max_2d_below, uint64_t* out_len);
pdal_stage_t* pdal_stage_create_normal(uint64_t knn, bool has_radius, double radius, bool always_up, bool has_viewpoint, double viewpoint_x, double viewpoint_y, double viewpoint_z, bool refine);
bool pdal_straighten_transform_point(bool reverse, double x, double y, double z, double segment_x, double segment_y, double segment_z, double segment_m, double segment_azimuth, double segment_offset, double offset, double* out_xyz);

typedef struct {
    double distance;
    double uncertainty;
    bool significant;
    double std_dev1;
    double std_dev2;
    uint64_t n1;
    uint64_t n2;
} pdal_m3c2_stats_t;

bool pdal_m3c2_compute_stats(const double* pts1, uint64_t pts1_count, bool skip_first1, const double* pts2, uint64_t pts2_count, bool skip_first2, const double* center, const double* normal, double cyl_radius2, double cyl_half_len, uint64_t min_points, double reg_error, pdal_m3c2_stats_t* out_stats);
pdal_stage_t* pdal_stage_create_voxelcenternearestneighbor(double cell);
pdal_stage_t* pdal_stage_create_voxelcentroidnearestneighbor(double cell);
pdal_stage_t* pdal_stage_create_reciprocity(uint64_t knn);
pdal_stage_t* pdal_stage_create_estimaterank(uint64_t knn, double threshold);
pdal_stage_t* pdal_stage_create_approximatecoplanar(uint64_t knn, double threshold1, double threshold2);
pdal_stage_t* pdal_stage_create_planefit(uint64_t knn);
pdal_stage_t* pdal_stage_create_eigenvalues(uint64_t knn, bool normalize, uint64_t stride, bool has_radius, double radius, uint64_t min_k);
pdal_stage_t* pdal_stage_create_optimalneighborhood(uint64_t min_k, uint64_t max_k);

uint64_t* pdal_grid_decimation_get_kept_indices(const pdal_point_view_t* view, double resolution, const char* output_type, uint64_t* out_len);
void pdal_free_u64_array(uint64_t* ptr, uint64_t len);

pdal_stage_t* pdal_stage_create_divider(int32_t mode, int32_t size_mode, uint64_t size, const uint8_t* evals, uint64_t evals_count);
pdal_stage_t* pdal_stage_create_chipper(uint64_t capacity);
pdal_stage_t* pdal_stage_create_lloydkmeans(uint64_t k, uint64_t maxiters, const char* const* dims, uint64_t dim_count);
pdal_stage_t* pdal_stage_create_farthestpointsampling(uint64_t count);

typedef struct {
    const char* dim_name;
    double value;
    double lower_bound;
    double upper_bound;
    bool inclusive_lower;
    bool inclusive_upper;
    bool negate;
} pdal_assign_range_t;

pdal_stage_t* pdal_stage_create_assign(
    bool has_condition,
    const char* cond_dim,
    double cond_lower,
    double cond_upper,
    bool cond_inclusive_lower,
    bool cond_inclusive_upper,
    bool cond_negate,
    const pdal_assign_range_t* assignments,
    uint64_t count
);

typedef struct {
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

void pdal_stats_compute(
    pdal_point_view_t* view,
    const char* const* dims,
    uint64_t dims_count,
    bool advanced,
    const char* const* enums,
    uint64_t enums_count,
    const char* const* counts,
    uint64_t counts_count,
    const char* const* globals,
    uint64_t globals_count,
    pdal_dim_stats_t* out_stats
);

void pdal_free_stats_arrays(pdal_dim_stats_t* ptr, uint64_t dims_count);


#ifdef __cplusplus
}
#endif
