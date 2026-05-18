#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct pdal_options pdal_options_t;
typedef struct pdal_point_layout pdal_point_layout_t;
typedef struct pdal_point_view pdal_point_view_t;
typedef struct pdal_spatial_reference pdal_spatial_reference_t;
typedef struct pdal_metadata_node pdal_metadata_node_t;
typedef struct pdal_stage pdal_stage_t;

const char* pdal_last_error();
void pdal_clear_error();
void pdal_string_free(char* ptr);

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
uint64_t pdal_point_view_dim_count(const pdal_point_view_t* view);
char* pdal_point_view_dim_name(const pdal_point_view_t* view, uint64_t idx);
int pdal_point_view_dim_type(const pdal_point_view_t* view, uint64_t idx);
void pdal_point_view_set_spatial_reference(pdal_point_view_t* view, const pdal_spatial_reference_t* srs);
pdal_spatial_reference_t* pdal_point_view_spatial_reference(const pdal_point_view_t* view);
uint64_t pdal_point_view_length(pdal_point_view_t* view);
uint64_t pdal_point_view_source_index(pdal_point_view_t* view, uint64_t idx);
void pdal_point_view_destroy(pdal_point_view_t* view);

// SpatialReference
pdal_spatial_reference_t* pdal_spatial_reference_create(const char* text);
pdal_spatial_reference_t* pdal_spatial_reference_create_with_epoch(const char* text, double epoch);
bool pdal_spatial_reference_empty(const pdal_spatial_reference_t* srs);
char* pdal_spatial_reference_text(const pdal_spatial_reference_t* srs);
double pdal_spatial_reference_epoch(const pdal_spatial_reference_t* srs);
void pdal_spatial_reference_set_epoch(pdal_spatial_reference_t* srs, double epoch);
pdal_metadata_node_t* pdal_spatial_reference_to_metadata(const pdal_spatial_reference_t* srs);
void pdal_spatial_reference_destroy(pdal_spatial_reference_t* srs);

// Metadata
pdal_metadata_node_t* pdal_metadata_node_create(const char* name);
char* pdal_metadata_node_name(const pdal_metadata_node_t* node);
void pdal_metadata_node_set_string(pdal_metadata_node_t* node, const char* value);
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
void pdal_metadata_node_add_child(pdal_metadata_node_t* node, pdal_metadata_node_t* child);
uint64_t pdal_metadata_node_child_count(const pdal_metadata_node_t* node);
pdal_metadata_node_t* pdal_metadata_node_child(const pdal_metadata_node_t* node, uint64_t idx);
void pdal_metadata_node_destroy(pdal_metadata_node_t* node);

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
bool pdal_stage_range_point_passes(pdal_stage_t* stage, pdal_point_view_t* view, uint64_t idx);

void pdal_stage_destroy(pdal_stage_t* stage);
void pdal_stage_reset(pdal_stage_t* stage);
bool pdal_stage_process_one(pdal_stage_t* stage);
bool pdal_stage_process_one_at(pdal_stage_t* stage, const pdal_point_view_t* view, uint64_t idx);
pdal_point_view_t* pdal_stage_run(pdal_stage_t* stage, pdal_point_view_t* input);
uint64_t pdal_stage_run_multi(pdal_stage_t* stage, pdal_point_view_t* input, pdal_point_view_t** outputs, uint64_t max_outputs);

pdal_stage_t* pdal_stage_create_sort(const char* const* dims, uint64_t count, const char* order, const char* algorithm);
pdal_stage_t* pdal_stage_create_expression(const char* const* exprs, uint64_t count);
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
pdal_stage_t* pdal_stage_create_lof(uint64_t minpts);
pdal_stage_t* pdal_stage_create_elm(double cell, uint8_t class_label, double threshold);
pdal_stage_t* pdal_stage_create_skewnessbalancing(uint8_t ground_class, uint8_t other_class, bool only_ground);
pdal_stage_t* pdal_stage_create_iqr(double multiplier, const char* dim_name);
pdal_stage_t* pdal_stage_create_mad(double multiplier, const char* dim_name, double mad_multiplier);
pdal_stage_t* pdal_stage_create_hagnn(uint64_t count, double max_distance, bool allow_extrapolation, uint8_t class_label);
pdal_stage_t* pdal_stage_create_cluster(uint64_t min_points, uint64_t max_points, double tolerance, bool is_3d);
pdal_stage_t* pdal_stage_create_sparsesurface(double radius, uint8_t ground_class, uint8_t low_point_class);
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

pdal_metadata_node_t* pdal_expressionstats_metadata(
    pdal_point_view_t* view,
    const char* dim_name,
    const char* const* expressions,
    uint64_t count
);


#ifdef __cplusplus
}
#endif
