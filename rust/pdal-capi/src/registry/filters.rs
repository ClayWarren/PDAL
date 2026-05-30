//! `filters.*` stage construction for the registry.
//!
//! Split out of `registry.rs` to keep modules under ~1k LOC. The driver
//! tables and shared numeric option helpers stay in the parent `registry`
//! module; the filter-only option-string parsers live in `filter_parse`.

use pdal_core::bounds::{parse_bounds2d, parse_bounds3d};
use pdal_core::options::Options;
use pdal_core::pipeline::FilterWrapper;
use pdal_core::point::DimId;
use pdal_core::stage::StageError;

use pdal_core::point::{DimType, PointLayout};
use pdal_filters::approximate_coplanar::ApproximateCoplanarFilter;
use pdal_filters::assign::{AssignCondition, AssignFilter, AssignRange};
use pdal_filters::chipper::ChipperFilter;
use pdal_filters::cluster::ClusterFilter;
use pdal_filters::colorization::{parse_band_spec, ColorizationFilter};
use pdal_filters::covariancefeatures::{CovarianceFeaturesFilter, Mode as CovarianceMode};
use pdal_filters::crop::{CropCenter, CropFilter};
use pdal_filters::csf::CsfFilter;
use pdal_filters::dbscan::DbscanFilter;
use pdal_filters::decimation::DecimationFilter;
use pdal_filters::dem::DEMFilter;
use pdal_filters::divider::{DividerFilter, DividerMode, DividerSizeMode};
use pdal_filters::eigenvalues::EigenvaluesFilter;
use pdal_filters::elm::ElmFilter;
use pdal_filters::estimate_rank::EstimateRankFilter;
use pdal_filters::expression::ExpressionFilter;
use pdal_filters::expressionstats::ExpressionStatsFilter;
use pdal_filters::faceraster::FaceRasterFilter;
use pdal_filters::farthestpointsampling::FarthestPointSamplingFilter;
use pdal_filters::ferry::FerryFilter;
use pdal_filters::geom_distance::GeomDistanceFilter;
use pdal_filters::gpstimeconvert::GpsTimeConvert;
use pdal_filters::groupby::GroupByFilter;
use pdal_filters::hag_delaunay::HagDelaunayFilter;
use pdal_filters::hag_dem::HagDemFilter;
use pdal_filters::hagnn::HagNnFilter;
use pdal_filters::head::HeadFilter;
use pdal_filters::hexbin::HexBinFilter;
use pdal_filters::iqr::IqrFilter;
use pdal_filters::labelduplicates::LabelDuplicatesFilter;
use pdal_filters::litree::LiTreeFilter;
use pdal_filters::lloydkmeans::LloydKMeansFilter;
use pdal_filters::locate::LocateFilter;
use pdal_filters::lof::LofFilter;
use pdal_filters::m3c2::{M3C2Filter, NormalOrientation as M3C2NormalOrientation};
use pdal_filters::mad::MadFilter;
use pdal_filters::merge::MergeFilter;
use pdal_filters::miniball::MiniballFilter;
use pdal_filters::mongo::MongoExpressionFilter;
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::neighborclassifier::NeighborClassifierFilter;
use pdal_filters::nndistance::{NNDistanceFilter, NNDistanceMode};
use pdal_filters::normal::NormalFilter;
use pdal_filters::optimal_neighborhood::OptimalNeighborhoodFilter;
use pdal_filters::outlier::OutlierFilter;
use pdal_filters::overlay::OverlayFilter;
use pdal_filters::planefit::PlaneFitFilter;
use pdal_filters::pmf::PmfFilter;
use pdal_filters::proj_pipeline::ProjPipelineFilter;
use pdal_filters::radialdensity::RadialDensityFilter;
use pdal_filters::radiusassign::{parse_assignments, RadiusAssignFilter};
use pdal_filters::randomize::RandomizeFilter;
use pdal_filters::range::{parse_range_limit, RangeFilter, RangeLimit};
use pdal_filters::reciprocity::ReciprocityFilter;
use pdal_filters::relaxation_dart_throwing::RelaxationDartThrowingFilter;
use pdal_filters::reprojection::ReprojectionFilter;
use pdal_filters::returns::ReturnsFilter;
use pdal_filters::sample::SampleFilter;
use pdal_filters::separatescanline::SeparateScanLineFilter;
use pdal_filters::skewnessbalancing::SkewnessBalancingFilter;
use pdal_filters::smrf::SmrfFilter;
use pdal_filters::sort::{SortAlgorithm, SortFilter, SortOrder};
use pdal_filters::sparse_surface::SparseSurfaceFilter;
use pdal_filters::splitter::SplitterFilter;
use pdal_filters::stats::StatsFilter;
use pdal_filters::straighten::StraightenFilter;
use pdal_filters::supervoxel::SupervoxelFilter;
use pdal_filters::tail::TailFilter;
use pdal_filters::transformation::{
    invert_affine, parse_transformation_matrix, TransformationFilter,
};
use pdal_filters::voxel_center_nearest_neighbor::VoxelCenterNearestNeighborFilter;
use pdal_filters::voxel_centroid_nearest_neighbor::VoxelCentroidNearestNeighborFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;
use pdal_filters::zsmooth::ZsmoothFilter;

use crate::registry::{get_bool, get_f64, get_u64};

mod filter_parse;
use filter_parse::*;

pub fn create_filter(
    name: &str,
    options: &Options,
) -> Result<Box<dyn pdal_core::pipeline::StageWrapper>, StageError> {
    match name {
        "filters.approximatecoplanar" => Ok(Box::new(FilterWrapper::new(
            ApproximateCoplanarFilter::new(
                get_u64(options, "knn", 8)? as usize,
                get_f64(options, "thresh1", 25.0)?,
                get_f64(options, "thresh2", 6.0)?,
            ),
        ))),
        "filters.chipper" => Ok(Box::new(FilterWrapper::new(ChipperFilter::new(get_u64(
            options,
            "capacity",
            get_u64(options, "threshold", 5000)?,
        )?)))),
        "filters.cluster" => Ok(Box::new(FilterWrapper::new(ClusterFilter::new(
            get_u64(options, "min_points", 1)? as usize,
            get_u64(options, "max_points", u64::MAX)? as usize,
            get_f64(options, "tolerance", 1.0)?,
            get_bool(options, "is3d", true)?,
        )))),
        "filters.colorization" => {
            // C++ ColorizationFilter parses the `dimensions` list into
            // name:band:scale specs (defaulting to Red/Green/Blue) before
            // sampling the raster; replicate that parse here.
            let raster = options.get_str("raster", "");
            if raster.trim().is_empty() {
                return Err(StageError(
                    "filters.colorization: missing 'raster' option.".to_string(),
                ));
            }
            let bands = parse_band_spec(&options.get_str("dimensions", "")).map_err(StageError)?;
            Ok(Box::new(FilterWrapper::new(ColorizationFilter::new(
                &raster, bands,
            ))))
        }
        "filters.covariancefeatures" => {
            Ok(Box::new(FilterWrapper::new(CovarianceFeaturesFilter::new(
                get_u64(options, "knn", 10)? as usize,
                get_u64(options, "stride", 1)? as usize,
                options
                    .has("radius")
                    .then(|| get_f64(options, "radius", 0.0))
                    .transpose()?,
                get_u64(options, "min_k", 3)? as usize,
                covariance_mode(&options.get_str("mode", "sqrt")),
                get_bool(options, "optimized", false)?,
                &options.get_str("feature_set", "dimensionality"),
            ))))
        }
        "filters.crop" => Ok(Box::new(FilterWrapper::new(crop_filter_from_options(
            options,
        )?))),
        "filters.csf" => Ok(Box::new(FilterWrapper::new(CsfFilter::new(
            get_u64(options, "ground_class", 2)? as u8,
            get_u64(options, "other_class", 1)? as u8,
            get_bool(options, "only_ground", false)?,
            comma_list(&options.get_str("ignore", ""))
                .into_iter()
                .map(|dim| DimId::from_name(dim.split('[').next().unwrap_or(&dim)))
                .collect(),
        )?))),
        "filters.dbscan" => Ok(Box::new(FilterWrapper::new(DbscanFilter::new(
            get_u64(options, "min_points", 6)? as usize,
            get_f64(options, "eps", 1.0)?,
            comma_list(&options.get_str("dimensions", "X,Y,Z")),
        )))),
        "filters.decimation" => Ok(Box::new(FilterWrapper::new(DecimationFilter::new(options)))),
        "filters.divider" => {
            // `expression` mode (implicit when the `expression` option is set)
            // splits on a per-point conditional whose delegation is not wired
            // through the registry; reject it explicitly.
            let mode_str = options.get_str("mode", "partition");
            if mode_str == "expression" || options.has("expression") {
                return Err(StageError(
                    "filters.divider: 'expression' mode is not supported in the Rust \
                     pipeline registry."
                        .to_string(),
                ));
            }
            let mode = match mode_str.as_str() {
                "partition" => DividerMode::Partition,
                "round_robin" => DividerMode::RoundRobin,
                other => {
                    return Err(StageError(format!(
                        "filters.divider: invalid 'mode' '{other}'. Valid options are \
                         'partition' and 'round_robin'."
                    )));
                }
            };
            // `capacity` (split every N points) vs `count` (N output views).
            let (size_mode, size) = if options.has("capacity") {
                (DividerSizeMode::Capacity, get_u64(options, "capacity", 0)?)
            } else {
                (DividerSizeMode::Count, get_u64(options, "count", 1)?)
            };
            if size_mode == DividerSizeMode::Capacity && size == 0 {
                return Err(StageError(
                    "filters.divider: option 'capacity' must be greater than 0.".to_string(),
                ));
            }
            Ok(Box::new(FilterWrapper::new(DividerFilter::new(
                mode,
                size_mode,
                size,
                Vec::new(),
            ))))
        }
        "filters.radiusassign" => {
            if !options.has("radius") {
                return Err(StageError(
                    "filters.radiusassign: missing required option 'radius'.".to_string(),
                ));
            }
            let parse_domain = |key: &str| -> Result<Vec<RangeLimit>, StageError> {
                options
                    .values(key)
                    .iter()
                    .flat_map(|value| value.split(','))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| {
                        parse_range_limit(value).map(|limit| RangeLimit {
                            dim_name: limit.dim_name,
                            lower_bound: limit.lower_bound,
                            upper_bound: limit.upper_bound,
                            inclusive_lower: limit.inclusive_lower,
                            inclusive_upper: limit.inclusive_upper,
                            negate: limit.negate,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(StageError)
            };
            let src_domain = parse_domain("src_domain")?;
            let reference_domain = parse_domain("reference_domain")?;
            let update_exprs: Vec<String> = options.values("update_expression").to_vec();
            if update_exprs.is_empty() {
                return Err(StageError(
                    "filters.radiusassign: missing required option 'update_expression'."
                        .to_string(),
                ));
            }
            // Build a synthetic layout from the assignment LHS dimensions so the
            // statements can be prepared at build time (matching the C ABI
            // pdal_stage_create_radiusassign). Expressions referencing other
            // dimensions error here rather than silently misbehaving.
            let mut layout = PointLayout::default();
            for expr in &update_exprs {
                if let Some((dim, _)) = expr.split_once('=') {
                    layout.register(DimId::from_name(dim.trim()), DimType::F64);
                }
            }
            let assignments = parse_assignments(&update_exprs, &layout).map_err(|e| {
                StageError(format!(
                    "{} (expressions referencing other dimensions are not \
                    supported in the Rust pipeline registry)",
                    e.0
                ))
            })?;
            Ok(Box::new(FilterWrapper::new(RadiusAssignFilter::new(
                src_domain,
                reference_domain,
                assignments,
                get_f64(options, "radius", 0.0)?,
                get_bool(options, "is3d", false)?,
                get_f64(options, "max2d_above", -1.0)?,
                get_f64(options, "max2d_below", -1.0)?,
            ))))
        }
        "filters.dem" => {
            // C++ DEMFilter parses `limits` (a DimRange like "Z[0:100]") into a
            // dimension name plus lower/upper bounds, reads `raster`/`band`, then
            // keeps points whose dim value lies within [v - lower, v + upper] of
            // the raster sample. Parse the same way here.
            let raster = options.get_str("raster", "");
            if raster.trim().is_empty() {
                return Err(StageError(
                    "filters.dem: missing 'raster' option.".to_string(),
                ));
            }
            let limits = options.get_str("limits", "");
            if limits.trim().is_empty() {
                return Err(StageError(
                    "filters.dem: missing 'limits' option.".to_string(),
                ));
            }
            let limit = parse_range_limit(&limits).map_err(StageError)?;
            Ok(Box::new(FilterWrapper::new(DEMFilter::new(
                &limit.dim_name,
                &raster,
                get_u64(options, "band", 1)? as i32,
                limit.lower_bound,
                limit.upper_bound,
            ))))
        }
        "filters.eigenvalues" => Ok(Box::new(FilterWrapper::new(EigenvaluesFilter::new(
            get_u64(options, "knn", 8)? as usize,
            get_bool(options, "normalize", false)?,
            get_u64(options, "stride", 1)? as usize,
            options
                .has("radius")
                .then(|| get_f64(options, "radius", 0.0))
                .transpose()?,
            get_u64(options, "min_k", 3)? as usize,
        )))),
        "filters.elm" => Ok(Box::new(FilterWrapper::new(ElmFilter::new(
            get_f64(options, "cell", 10.0)?,
            get_u64(options, "class", 7)? as u8,
            get_f64(options, "threshold", 1.0)?,
        )))),
        "filters.estimaterank" => Ok(Box::new(FilterWrapper::new(EstimateRankFilter::new(
            get_u64(options, "knn", 8)? as usize,
            get_f64(options, "threshold", 0.01)?,
        )))),
        "filters.faceraster" => Ok(Box::new(FilterWrapper::new(FaceRasterFilter::new(options)))),
        "filters.geomdistance" => {
            // The Rust filter takes an inline WKT/GeoJSON geometry; the OGR
            // vector-source option needs a reader the registry can't drive.
            if !options.get_str("ogr", "").trim().is_empty() {
                return Err(StageError(
                    "filters.geomdistance: the 'ogr' geometry source is not supported in \
                     the Rust pipeline registry."
                        .to_string(),
                ));
            }
            let geometry = options.get_str("geometry", "");
            if geometry.trim().is_empty() {
                return Err(StageError(
                    "filters.geomdistance: missing 'geometry' option.".to_string(),
                ));
            }
            Ok(Box::new(FilterWrapper::new(GeomDistanceFilter::new(
                &geometry,
                &options.get_str("dimension", "distance"),
                get_bool(options, "ring", false)?,
            )?)))
        }
        "filters.gpstimeconvert" => Ok(Box::new(FilterWrapper::new(GpsTimeConvert::from_options(
            options,
        )?))),
        "filters.groupby" => Ok(Box::new(FilterWrapper::new(GroupByFilter::new(
            options.get_str("dimension", ""),
        )))),
        "filters.hag_delaunay" => Ok(Box::new(FilterWrapper::new(HagDelaunayFilter::new(
            get_u64(options, "count", 10)? as usize,
            get_bool(options, "allow_extrapolation", true)?,
            get_u64(options, "class", 2)? as u8,
        )))),
        "filters.hag_dem" => {
            // C++ HagDemFilter clamp defaults are numeric_limits<double>::min()
            // (smallest positive, f64::MIN_POSITIVE) and ::max() (f64::MAX); the
            // ground class defaults to 2.
            let raster = options.get_str("raster", "");
            if raster.trim().is_empty() {
                return Err(StageError(
                    "filters.hag_dem: missing 'raster' option.".to_string(),
                ));
            }
            Ok(Box::new(FilterWrapper::new(HagDemFilter::new(
                &raster,
                get_u64(options, "band", 1)? as i32,
                get_bool(options, "zero_ground", true)?,
                get_f64(options, "min_clamp", f64::MIN_POSITIVE)?,
                get_f64(options, "max_clamp", f64::MAX)?,
                get_f64(options, "nodata_hag", 0.0)?,
                get_u64(options, "class", 2)? as u8,
            ))))
        }
        "filters.hag_nn" => Ok(Box::new(FilterWrapper::new(HagNnFilter::new(
            get_u64(options, "count", 1)? as usize,
            get_f64(options, "max_distance", 0.0)?,
            get_bool(options, "allow_extrapolation", false)?,
            get_u64(options, "class", 2)? as u8,
        )))),
        "filters.head" => Ok(Box::new(FilterWrapper::new(HeadFilter::new(
            get_u64(options, "count", 10)?,
            get_bool(options, "invert", false)?,
        )))),
        "filters.hexbin" => {
            let edge = if options.has("edge_length") {
                Some(get_f64(options, "edge_length", 0.0)?)
            } else if options.has("edge_size") {
                Some(get_f64(options, "edge_size", 0.0)?)
            } else {
                None
            };
            let density = options.get_str("density", "");
            let mut filter = HexBinFilter::with_options(
                edge,
                get_u64(options, "threshold", 15)? as u32,
                get_u64(options, "sample_size", 5000)? as usize,
                (!density.is_empty()).then_some(density),
                get_bool(options, "output_tesselation", false)?,
            );
            if get_bool(options, "h3_grid", false)? {
                let resolution = options
                    .has("h3_resolution")
                    .then(|| get_u64(options, "h3_resolution", 0).map(|r| r as u8))
                    .transpose()?;
                filter.set_h3(resolution);
            }
            Ok(Box::new(FilterWrapper::new(filter)))
        }
        "filters.iqr" => Ok(Box::new(FilterWrapper::new(IqrFilter::new(
            get_f64(options, "multiplier", 1.5)?,
            DimId::from_name(&options.get_str("dimension", "Z")),
        )))),
        "filters.lloydkmeans" => Ok(Box::new(FilterWrapper::new(LloydKMeansFilter::new(
            get_u64(options, "k", 10)? as usize,
            get_u64(options, "maxiters", 10)? as usize,
            comma_list(&options.get_str("dimensions", "X,Y,Z"))
                .iter()
                .map(|s| DimId::from_name(s))
                .collect(),
        )))),
        "filters.m3c2" => Ok(Box::new(FilterWrapper::new(M3C2Filter::new(
            get_f64(options, "normal_radius", 2.0)?,
            get_f64(options, "cyl_radius", 2.0)?,
            get_f64(options, "cyl_halflen", 5.0)?,
            get_f64(options, "reg_error", 0.0)?,
            m3c2_orientation(&options.get_str("orientation", "up"))?,
            get_u64(options, "min_points", 1)? as usize,
        )))),
        "filters.locate" => Ok(Box::new(FilterWrapper::new(LocateFilter::new(
            options.get_str("dimension", ""),
            options.get_str("minmax", "max"),
        )))),
        "filters.label_duplicates" => Ok(Box::new(FilterWrapper::new(LabelDuplicatesFilter::new(
            comma_list(&options.get_str("dimensions", "X,Y,Z")),
        )))),
        "filters.litree" => Ok(Box::new(FilterWrapper::new(LiTreeFilter::new(
            get_u64(options, "min_points", 10)? as usize,
            get_f64(options, "min_height", 3.0)?,
            get_f64(options, "radius", 100.0)?,
        )))),
        "filters.lof" => Ok(Box::new(FilterWrapper::new(LofFilter::new(get_u64(
            options, "minpts", 10,
        )?
            as usize)))),
        "filters.mad" => Ok(Box::new(FilterWrapper::new(MadFilter::new(
            get_f64(options, "multiplier", 1.4826)?,
            DimId::from_name(&options.get_str("dimension", "Z")),
            get_f64(options, "mad_multiplier", 2.0)?,
        )))),
        "filters.merge" => Ok(Box::new(FilterWrapper::new(MergeFilter::new()))),
        "filters.miniball" => Ok(Box::new(FilterWrapper::new(MiniballFilter::new(get_u64(
            options, "knn", 8,
        )?)))),
        "filters.mortonorder" => Ok(Box::new(FilterWrapper::new(MortonOrderFilter::new(
            get_bool(options, "reverse", false)?,
        )))),
        "filters.neighborclassifier" => {
            // The `candidate` mode loads neighbors from another file; that
            // requires a reader the Rust pipeline registry can't drive here, so
            // reject it explicitly rather than silently classify from self.
            if !options.get_str("candidate", "").trim().is_empty() {
                return Err(StageError(
                    "filters.neighborclassifier: the 'candidate' file option is not \
                     supported in the Rust pipeline registry."
                        .to_string(),
                ));
            }
            if !options.has("k") {
                return Err(StageError(
                    "filters.neighborclassifier: missing required option 'k'.".to_string(),
                ));
            }
            let domain = options
                .values("domain")
                .iter()
                .flat_map(|value| value.split(','))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    parse_range_limit(value).map(|limit| RangeLimit {
                        dim_name: limit.dim_name,
                        lower_bound: limit.lower_bound,
                        upper_bound: limit.upper_bound,
                        inclusive_lower: limit.inclusive_lower,
                        inclusive_upper: limit.inclusive_upper,
                        negate: limit.negate,
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(StageError)?;
            Ok(Box::new(FilterWrapper::new(NeighborClassifierFilter::new(
                domain,
                get_u64(options, "k", 8)? as usize,
                options.get_str("dimension", "Classification"),
            ))))
        }
        "filters.nndistance" => Ok(Box::new(FilterWrapper::new(NNDistanceFilter::new(
            get_u64(options, "knn", get_u64(options, "k", 8)?)? as usize,
            nn_distance_mode(&options.get_str("mode", "kth"))?,
        )))),
        "filters.normal" => {
            let viewpoint = if options.has("viewpoint") {
                Some(parse_wkt_point(&options.get_str("viewpoint", ""))?)
            } else {
                None
            };
            Ok(Box::new(FilterWrapper::new(NormalFilter::new(
                get_u64(options, "knn", 8)? as usize + 1,
                options
                    .has("radius")
                    .then(|| get_f64(options, "radius", 0.0))
                    .transpose()?,
                viewpoint,
                get_bool(options, "always_up", true)?,
                get_bool(options, "refine", false)?,
            ))))
        }
        "filters.optimalneighborhood" => Ok(Box::new(FilterWrapper::new(
            OptimalNeighborhoodFilter::new(
                get_u64(options, "min_k", 3)? as usize,
                get_u64(options, "max_k", 8)? as usize,
            ),
        ))),
        "filters.assign" => {
            // The expression-based `value` option needs the full assign-statement
            // parser, which is not yet ported; reject it explicitly rather than
            // silently ignore it.
            if !options.values("value").is_empty() {
                return Err(StageError(
                    "filters.assign: the expression-based 'value' option is not \
                     supported in the Rust pipeline registry."
                        .to_string(),
                ));
            }
            let assignments = options
                .values("assignment")
                .iter()
                .flat_map(|value| value.split(','))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(parse_assign_range)
                .collect::<Result<Vec<_>, _>>()
                .map_err(StageError)?;
            let condition = match options.get_str("condition", "") {
                ref c if c.trim().is_empty() => None,
                c => Some(parse_assign_condition(c.trim()).map_err(StageError)?),
            };
            if assignments.is_empty() && condition.is_none() {
                return Err(StageError(
                    "filters.assign: no 'assignment' provided.".to_string(),
                ));
            }
            Ok(Box::new(FilterWrapper::new(AssignFilter::new(
                condition,
                assignments,
            ))))
        }
        "filters.outlier" => Ok(Box::new(FilterWrapper::new(OutlierFilter::new(
            options.get_str("method", "statistical"),
            get_u64(options, "min_k", 2)? as usize,
            get_f64(options, "radius", 1.0)?,
            get_u64(options, "mean_k", 8)? as usize,
            get_f64(options, "multiplier", 2.0)?,
            get_u64(options, "class", 7)? as u8,
        )))),
        "filters.overlay" => {
            // The Rust OverlayFilter reads polygons from layer 0 of the OGR
            // datasource with the given attribute column. The OGR SQL `query`,
            // explicit `layer`/`lyr_name`, and spatial `bounds` options need
            // OGR query/layer plumbing the registry path does not drive; error
            // explicitly on them. `threads` is accepted but ignored (the Rust
            // filter runs single-threaded).
            let dimension = options.get_str("dimension", "");
            if dimension.trim().is_empty() {
                return Err(StageError(
                    "filters.overlay: missing 'dimension' option.".to_string(),
                ));
            }
            let datasource = options.get_str("datasource", "");
            if datasource.trim().is_empty() {
                return Err(StageError(
                    "filters.overlay: missing 'datasource' option.".to_string(),
                ));
            }
            for opt in ["query", "layer", "lyr_name", "bounds"] {
                if !options.get_str(opt, "").trim().is_empty() {
                    return Err(StageError(format!(
                        "filters.overlay: the '{opt}' option is not supported in \
                         the Rust pipeline registry (uses layer 0, no OGR query/\
                         spatial filter)."
                    )));
                }
            }
            Ok(Box::new(FilterWrapper::new(OverlayFilter::new(
                &dimension,
                &datasource,
                &options.get_str("column", ""),
            ))))
        }
        "filters.planefit" => Ok(Box::new(FilterWrapper::new(PlaneFitFilter::new(get_u64(
            options, "knn", 8,
        )?
            as usize)))),
        "filters.pmf" => Ok(Box::new(FilterWrapper::new(PmfFilter::new(
            get_f64(options, "cell_size", 1.0)?,
            get_bool(options, "exponential", true)?,
            get_f64(options, "initial_distance", 0.15)?,
            comma_list(&options.get_str("returns", "last,only")),
            get_f64(options, "max_distance", 2.5)?,
            get_f64(options, "max_window_size", 33.0)?,
            get_f64(options, "slope", 1.0)?,
            get_u64(options, "ground_class", 2)? as u8,
            get_u64(options, "other_class", 1)? as u8,
            get_bool(options, "only_ground", false)?,
        )?))),
        "filters.projpipeline" => Ok(Box::new(FilterWrapper::new(ProjPipelineFilter::new(
            &options.get_str("out_srs", ""),
            &options.get_str("coord_op", ""),
            get_bool(options, "reverse_transfo", false)?,
        )))),
        "filters.radialdensity" => Ok(Box::new(FilterWrapper::new(RadialDensityFilter::new(
            get_f64(options, "radius", 1.0)?,
        )))),
        "filters.randomize" => {
            let seed = options
                .has("seed")
                .then(|| get_u64(options, "seed", 0).map(|seed| seed as u32))
                .transpose()?;
            Ok(Box::new(FilterWrapper::new(RandomizeFilter::new(seed))))
        }
        "filters.range" => {
            let limits = options
                .values("limits")
                .iter()
                .flat_map(|value| value.split(','))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    parse_range_limit(value).map(|limit| RangeLimit {
                        dim_name: limit.dim_name,
                        lower_bound: limit.lower_bound,
                        upper_bound: limit.upper_bound,
                        inclusive_lower: limit.inclusive_lower,
                        inclusive_upper: limit.inclusive_upper,
                        negate: limit.negate,
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(StageError)?;
            if limits.is_empty() {
                return Err(StageError(
                    "Missing value for positional argument 'limits'.".to_string(),
                ));
            }
            Ok(Box::new(FilterWrapper::new(RangeFilter::new(limits))))
        }
        "filters.reciprocity" => Ok(Box::new(FilterWrapper::new(ReciprocityFilter::new(
            get_u64(options, "knn", 8)? as usize,
        )))),
        "filters.relaxationdartthrowing" => Ok(Box::new(FilterWrapper::new(
            RelaxationDartThrowingFilter::new(
                get_f64(options, "decay", 0.9)?,
                get_f64(options, "radius", 1.0)?,
                get_f64(options, "terminal_radius", 0.001)?,
                get_u64(options, "count", 1000)?,
                get_bool(options, "shuffle", true)?,
                options
                    .has("seed")
                    .then(|| get_u64(options, "seed", 0))
                    .transpose()?
                    .map(|s| s as u32),
            ),
        ))),
        "filters.reprojection" => Ok(Box::new(FilterWrapper::new(ReprojectionFilter::new(
            &options.get_str("out_srs", ""),
            options.has("in_srs").then(|| options.get_str("in_srs", "")),
            get_bool(options, "error_on_failure", false)?,
        )))),
        "filters.returns" => Ok(Box::new(FilterWrapper::new(ReturnsFilter::new(
            comma_list(&options.get_str("groups", "last")),
        )))),
        "filters.smrf" => {
            let ignore = options
                .values("ignore")
                .iter()
                .flat_map(|value| value.split(','))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    parse_range_limit(value).map(|limit| RangeLimit {
                        dim_name: limit.dim_name,
                        lower_bound: limit.lower_bound,
                        upper_bound: limit.upper_bound,
                        inclusive_lower: limit.inclusive_lower,
                        inclusive_upper: limit.inclusive_upper,
                        negate: limit.negate,
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(StageError)?;
            let classbits =
                parse_classbits(&options.get_str("classbits", "")).map_err(StageError)?;
            Ok(Box::new(FilterWrapper::new(
                SmrfFilter::with_segmentation(
                    get_f64(options, "cell", 1.0)?,
                    get_f64(options, "slope", 0.15)?,
                    options
                        .has("window")
                        .then(|| get_f64(options, "window", 18.0))
                        .transpose()?,
                    get_f64(options, "scalar", 1.25)?,
                    get_f64(options, "threshold", 0.5)?,
                    get_f64(options, "cut", 0.0)?,
                    get_u64(options, "ground_class", 2)? as u8,
                    get_u64(options, "other_class", 1)? as u8,
                    get_bool(options, "only_ground", false)?,
                    comma_list(&options.get_str("returns", "last,only")),
                    ignore,
                    classbits,
                )
                .with_dir(match options.get_str("dir", "") {
                    ref d if d.is_empty() => None,
                    d => Some(d),
                }),
            )))
        }
        "filters.sample" => Ok(Box::new(FilterWrapper::new(
            SampleFilter::new(options).map_err(StageError)?,
        ))),
        "filters.separatescanline" => Ok(Box::new(FilterWrapper::new(
            SeparateScanLineFilter::new(get_u64(options, "groupby", 1)?),
        ))),
        "filters.skewnessbalancing" => {
            Ok(Box::new(FilterWrapper::new(SkewnessBalancingFilter::new(
                get_u64(options, "ground_class", 2)? as u8,
                get_u64(options, "other_class", 1)? as u8,
                get_bool(options, "only_ground", false)?,
            ))))
        }
        "filters.farthestpointsampling" => Ok(Box::new(FilterWrapper::new(
            FarthestPointSamplingFilter::new(get_u64(options, "count", 1000)?),
        ))),
        "filters.expression" => {
            // `expression` is the positional option; `limits` is its synonym.
            let mut sources: Vec<String> = options.values("expression").to_vec();
            if sources.is_empty() {
                sources = options.values("limits").to_vec();
            }
            Ok(Box::new(FilterWrapper::new(ExpressionFilter::new(
                &sources,
            )?)))
        }
        "filters.expressionstats" => {
            let dim = options.get_str("dimension", "");
            if dim.trim().is_empty() {
                return Err(StageError(
                    "filters.expressionstats: missing 'dimension' option.".to_string(),
                ));
            }
            let sources: Vec<String> = options.values("expressions").to_vec();
            Ok(Box::new(FilterWrapper::new(ExpressionStatsFilter::new(
                &dim, &sources,
            )?)))
        }
        "filters.mongo" => {
            let expr = options.get_str("expression", "");
            if expr.trim().is_empty() {
                return Err(StageError(
                    "filters.mongo: missing 'expression' option.".to_string(),
                ));
            }
            Ok(Box::new(FilterWrapper::new(MongoExpressionFilter::new(
                &expr,
            )?)))
        }
        "filters.ferry" => {
            let specs: Vec<String> = options
                .values("dimensions")
                .iter()
                .flat_map(|value| value.split(','))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let dims = FerryFilter::parse_specs(&specs).map_err(StageError)?;
            Ok(Box::new(FilterWrapper::new(FerryFilter::new(dims))))
        }
        "filters.sparsesurface" => {
            let ground = get_u64(options, "ground_class", 2)? as u8;
            let low = get_u64(options, "low_point_class", 7)? as u8;
            if ground == low {
                return Err(StageError(
                    "filters.sparsesurface: Ground and low point class cannot be equal."
                        .to_string(),
                ));
            }
            Ok(Box::new(FilterWrapper::new(SparseSurfaceFilter::new(
                get_f64(options, "radius", 1.0)?,
                ground,
                low,
            ))))
        }
        "filters.splitter" => Ok(Box::new(FilterWrapper::new(SplitterFilter::new(
            get_f64(options, "length", 1000.0)?,
            if options.has("origin_x") {
                get_f64(options, "origin_x", f64::NAN)?
            } else {
                f64::NAN
            },
            if options.has("origin_y") {
                get_f64(options, "origin_y", f64::NAN)?
            } else {
                f64::NAN
            },
            get_f64(options, "buffer", 0.0)?,
        )))),
        "filters.sort" => Ok(Box::new(FilterWrapper::new(SortFilter::new(
            comma_list(&options.get_str("dimensions", &options.get_str("dimension", ""))),
            sort_order(&options.get_str("order", "asc"))?,
            sort_algorithm(&options.get_str("algorithm", "normal"))?,
        )))),
        "filters.straighten" => Ok(Box::new(FilterWrapper::new(
            StraightenFilter::new(
                &options.get_str("polyline", ""),
                get_bool(options, "reverse", false)?,
                get_f64(options, "offset", 0.0)?,
            )
            .ok_or_else(|| {
                StageError("Invalid polyline specification in option 'polyline'.".to_string())
            })?,
        ))),
        "filters.stats" => Ok(Box::new(FilterWrapper::new(StatsFilter::from_options(
            options,
        )))),
        "filters.supervoxel" => Ok(Box::new(FilterWrapper::new(SupervoxelFilter::new(
            get_u64(options, "knn", 32)? as usize,
            get_f64(options, "resolution", 1.0)?,
        )))),
        "filters.tail" => Ok(Box::new(FilterWrapper::new(TailFilter::new(
            get_u64(options, "count", 10)?,
            get_bool(options, "invert", false)?,
        )))),
        "filters.transformation" => {
            let matrix_str = options.get_str("matrix", "");
            if matrix_str.trim().is_empty() {
                return Err(StageError(
                    "filters.transformation: missing 'matrix' option.".to_string(),
                ));
            }
            let mut matrix = parse_transformation_matrix(&matrix_str).map_err(StageError)?;
            // The C++ TransformationFilter inverts the matrix (Affine3d::inverse)
            // up front when `invert` is set, then applies it; mirror that here.
            if get_bool(options, "invert", false)? {
                matrix = invert_affine(&matrix).map_err(StageError)?;
            }
            Ok(Box::new(FilterWrapper::new(TransformationFilter::new(
                matrix,
            ))))
        }
        "filters.voxelcenternearestneighbor" => Ok(Box::new(FilterWrapper::new(
            VoxelCenterNearestNeighborFilter::new(get_f64(options, "cell", 1.0)?),
        ))),
        "filters.voxelcentroidnearestneighbor" => Ok(Box::new(FilterWrapper::new(
            VoxelCentroidNearestNeighborFilter::new(get_f64(options, "cell", 1.0)?),
        ))),
        "filters.voxeldownsize" => Ok(Box::new(FilterWrapper::new(VoxelDownsizeFilter::new(
            options,
        )))),
        "filters.zsmooth" => Ok(Box::new(FilterWrapper::new(ZsmoothFilter::new(
            get_f64(options, "radius", 1.0)?,
            get_f64(options, "position", 0.5)?,
            options.get_str("dimension", "Z"),
        )))),
        _ => Err(StageError(format!(
            "Filter driver '{name}' is not available in the Rust port registry."
        ))),
    }
}
