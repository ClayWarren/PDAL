//! Stage registry -- construct implemented stages from PDAL driver names.
//!
//! This is the name-keyed slice of PDAL's `StageFactory`, restricted to the
//! reader/filter/writer drivers this Rust spike currently implements.

use crate::error::{clear_last_error, set_last_error};
use crate::pipeline_abi::PipelineHandle;
use pdal_core::bounds::{parse_bounds2d, parse_bounds3d};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline, Reader, Writer};
use pdal_core::point::DimId;
use pdal_core::stage::StageError;

use pdal_filters::approximate_coplanar::ApproximateCoplanarFilter;
use pdal_filters::transformation::{invert_affine, parse_transformation_matrix, TransformationFilter};
use pdal_filters::skewnessbalancing::SkewnessBalancingFilter;
use pdal_filters::sparse_surface::SparseSurfaceFilter;
use pdal_filters::farthestpointsampling::FarthestPointSamplingFilter;
use pdal_filters::expression::ExpressionFilter;
use pdal_filters::expressionstats::ExpressionStatsFilter;
use pdal_filters::ferry::FerryFilter;
use pdal_filters::mongo::MongoExpressionFilter;
use pdal_filters::neighborclassifier::NeighborClassifierFilter;
use pdal_filters::divider::{DividerFilter, DividerMode, DividerSizeMode};
use pdal_filters::chipper::ChipperFilter;
use pdal_filters::cluster::ClusterFilter;
use pdal_filters::covariancefeatures::{CovarianceFeaturesFilter, Mode as CovarianceMode};
use pdal_filters::crop::{CropCenter, CropFilter};
use pdal_filters::csf::CsfFilter;
use pdal_filters::dbscan::DbscanFilter;
use pdal_filters::decimation::DecimationFilter;
use pdal_filters::eigenvalues::EigenvaluesFilter;
use pdal_filters::elm::ElmFilter;
use pdal_filters::estimate_rank::EstimateRankFilter;
use pdal_filters::faceraster::FaceRasterFilter;
use pdal_filters::gpstimeconvert::GpsTimeConvert;
use pdal_filters::groupby::GroupByFilter;
use pdal_filters::hag_delaunay::HagDelaunayFilter;
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
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::nndistance::{NNDistanceFilter, NNDistanceMode};
use pdal_filters::normal::NormalFilter;
use pdal_filters::optimal_neighborhood::OptimalNeighborhoodFilter;
use pdal_filters::outlier::OutlierFilter;
use pdal_filters::planefit::PlaneFitFilter;
use pdal_filters::pmf::PmfFilter;
use pdal_filters::proj_pipeline::ProjPipelineFilter;
use pdal_filters::radialdensity::RadialDensityFilter;
use pdal_filters::randomize::RandomizeFilter;
use pdal_filters::assign::{AssignCondition, AssignFilter, AssignRange};
use pdal_filters::range::{parse_range_limit, RangeFilter, RangeLimit};
use pdal_filters::reciprocity::ReciprocityFilter;
use pdal_filters::relaxation_dart_throwing::RelaxationDartThrowingFilter;
use pdal_filters::reprojection::ReprojectionFilter;
use pdal_filters::returns::ReturnsFilter;
use pdal_filters::sample::SampleFilter;
use pdal_filters::separatescanline::SeparateScanLineFilter;
use pdal_filters::smrf::SmrfFilter;
use pdal_filters::sort::{SortAlgorithm, SortFilter, SortOrder};
use pdal_filters::splitter::SplitterFilter;
use pdal_filters::stats::StatsFilter;
use pdal_filters::straighten::StraightenFilter;
use pdal_filters::supervoxel::SupervoxelFilter;
use pdal_filters::tail::TailFilter;
use pdal_filters::voxel_center_nearest_neighbor::VoxelCenterNearestNeighborFilter;
use pdal_filters::voxel_centroid_nearest_neighbor::VoxelCentroidNearestNeighborFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;
use pdal_filters::zsmooth::ZsmoothFilter;

use serde_json::Value;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const READER_DRIVERS: &[&str] = &[
    "readers.faux",
    "readers.bpf",
    "readers.fbi",
    "readers.gdal",
    "readers.text",
    "readers.pcd",
    "readers.pts",
    "readers.ptx",
    "readers.ilvis2",
    "readers.obj",
    "readers.optech",
    "readers.qfit",
    "readers.sbet",
    "readers.smrmsg",
    "readers.terrasolid",
    "readers.copc",
    "readers.ept",
    "readers.las",
    "readers.laz",
    "readers.nitf",
    "readers.ply",
    "readers.spz",
    "readers.stac",
    "readers.tindex",
];

pub const FILTER_DRIVERS: &[&str] = &[
    "filters.approximatecoplanar",
    "filters.assign",
    "filters.chipper",
    "filters.cluster",
    "filters.covariancefeatures",
    "filters.crop",
    "filters.csf",
    "filters.dbscan",
    "filters.decimation",
    "filters.divider",
    "filters.eigenvalues",
    "filters.elm",
    "filters.estimaterank",
    "filters.expression",
    "filters.expressionstats",
    "filters.ferry",
    "filters.faceraster",
    "filters.gpstimeconvert",
    "filters.groupby",
    "filters.hag_delaunay",
    "filters.hag_nn",
    "filters.head",
    "filters.hexbin",
    "filters.iqr",
    "filters.label_duplicates",
    "filters.litree",
    "filters.lloydkmeans",
    "filters.m3c2",
    "filters.locate",
    "filters.lof",
    "filters.mad",
    "filters.merge",
    "filters.miniball",
    "filters.mongo",
    "filters.mortonorder",
    "filters.neighborclassifier",
    "filters.nndistance",
    "filters.normal",
    "filters.optimalneighborhood",
    "filters.outlier",
    "filters.planefit",
    "filters.pmf",
    "filters.projpipeline",
    "filters.radialdensity",
    "filters.randomize",
    "filters.range",
    "filters.reciprocity",
    "filters.relaxationdartthrowing",
    "filters.reprojection",
    "filters.returns",
    "filters.smrf",
    "filters.farthestpointsampling",
    "filters.sample",
    "filters.separatescanline",
    "filters.skewnessbalancing",
    "filters.sparsesurface",
    "filters.splitter",
    "filters.sort",
    "filters.stats",
    "filters.straighten",
    "filters.supervoxel",
    "filters.tail",
    "filters.transformation",
    "filters.voxelcenternearestneighbor",
    "filters.voxelcentroidnearestneighbor",
    "filters.voxeldownsize",
    "filters.zsmooth",
];

pub const WRITER_DRIVERS: &[&str] = &[
    "writers.null",
    "writers.bpf",
    "writers.fbi",
    "writers.gltf",
    "writers.text",
    "writers.pcd",
    "writers.sbet",
    "writers.las",
    "writers.laz",
    "writers.nitf",
    "writers.ply",
    "writers.ogr",
    "writers.gdal",
    "writers.raster",
    "writers.spz",
];

pub enum CreatedStage {
    Reader(Box<dyn Reader>),
    Filter(Box<dyn pdal_core::pipeline::StageWrapper>),
    Writer(Box<dyn Writer>),
}

pub fn create_reader(name: &str, options: &Options) -> Result<Box<dyn Reader>, StageError> {
    match name {
        "readers.faux" => pdal_io::faux::FauxReader::new(options)
            .map(|reader| Box::new(reader) as Box<dyn Reader>)
            .map_err(StageError),
        "readers.bpf" => Ok(Box::new(pdal_io::bpf::BpfReader::new(options))),
        "readers.fbi" => Ok(Box::new(pdal_io::fbi::FbiReader::new(options))),
        "readers.gdal" => Ok(Box::new(pdal_io::gdal_reader::GdalReader::new(options))),
        "readers.text" => Ok(Box::new(pdal_io::text::TextReader::new(options))),
        "readers.pcd" => Ok(Box::new(pdal_io::pcd::PcdReader::new(options))),
        "readers.pts" => Ok(Box::new(pdal_io::pts::PtsReader::new(options))),
        "readers.ptx" => Ok(Box::new(pdal_io::ptx::PtxReader::new(options))),
        "readers.ilvis2" => Ok(Box::new(pdal_io::ilvis2::Ilvis2Reader::new(options))),
        "readers.obj" => Ok(Box::new(pdal_io::obj::ObjReader::new(options))),
        "readers.optech" => Ok(Box::new(pdal_io::optech::OptechReader::new(options))),
        "readers.qfit" => Ok(Box::new(pdal_io::qfit::QfitReader::new(options))),
        "readers.sbet" => Ok(Box::new(pdal_io::sbet::SbetReader::new(options))),
        "readers.smrmsg" => Ok(Box::new(pdal_io::smrmsg::SmrmsgReader::new(options))),
        "readers.terrasolid" => Ok(Box::new(pdal_io::terrasolid::TerrasolidReader::new(
            options,
        ))),
        "readers.copc" => Ok(Box::new(pdal_io::copc::CopcReader::new(options))),
        "readers.las" | "readers.laz" => Ok(Box::new(pdal_io::las::LasReader::new(options))),
        "readers.nitf" => Ok(Box::new(pdal_io::nitf_reader::NitfReader::new(options))),
        "readers.ept" => Ok(Box::new(pdal_io::ept::EptReader::new(options))),
        "readers.ply" => Ok(Box::new(pdal_io::ply::PlyReader::new(options))),
        "readers.spz" => Ok(Box::new(pdal_io::spz::SpzReader::new(options))),
        "readers.stac" => Ok(Box::new(pdal_io::stac::StacReader::new(options))),
        "readers.tindex" => Ok(Box::new(pdal_io::tindex::TindexReader::new(options))),
        _ => Err(StageError(format!(
            "Reader driver '{name}' is not available in the Rust port."
        ))),
    }
}

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
            Ok(Box::new(FilterWrapper::new(HexBinFilter::with_options(
                edge,
                get_u64(options, "threshold", 15)? as u32,
                get_u64(options, "sample_size", 5000)? as usize,
                (!density.is_empty()).then_some(density),
                get_bool(options, "output_tesselation", false)?,
            ))))
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
            let classbits = parse_classbits(&options.get_str("classbits", "")).map_err(StageError)?;
            Ok(Box::new(FilterWrapper::new(SmrfFilter::with_segmentation(
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
            }))))
        }
        "filters.sample" => Ok(Box::new(FilterWrapper::new(
            SampleFilter::new(options).map_err(StageError)?,
        ))),
        "filters.separatescanline" => Ok(Box::new(FilterWrapper::new(
            SeparateScanLineFilter::new(get_u64(options, "groupby", 1)?),
        ))),
        "filters.skewnessbalancing" => Ok(Box::new(FilterWrapper::new(
            SkewnessBalancingFilter::new(
                get_u64(options, "ground_class", 2)? as u8,
                get_u64(options, "other_class", 1)? as u8,
                get_bool(options, "only_ground", false)?,
            ),
        ))),
        "filters.farthestpointsampling" => Ok(Box::new(FilterWrapper::new(
            FarthestPointSamplingFilter::new(get_u64(options, "count", 1000)?),
        ))),
        "filters.expression" => {
            // `expression` is the positional option; `limits` is its synonym.
            let mut sources: Vec<String> = options.values("expression").to_vec();
            if sources.is_empty() {
                sources = options.values("limits").to_vec();
            }
            Ok(Box::new(FilterWrapper::new(ExpressionFilter::new(&sources)?)))
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
            Ok(Box::new(FilterWrapper::new(MongoExpressionFilter::new(&expr)?)))
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
            Ok(Box::new(FilterWrapper::new(TransformationFilter::new(matrix))))
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

fn comma_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parse a simple `filters.assign` assignment statement of the form
/// `Dim[range]=value`, matching the C++ `AssignRange::parse`. The expression
/// form (the separate `value` option) is not handled here.
fn parse_assign_range(spec: &str) -> Result<AssignRange, String> {
    let limit = parse_range_limit(spec)?;
    let rest = spec[limit.consumed..].trim_start();
    let value_str = rest
        .strip_prefix('=')
        .ok_or_else(|| "filters.assign: Missing '=' assignment separator.".to_string())?
        .trim();
    let value: f64 = value_str
        .parse()
        .map_err(|_| "filters.assign: Missing value to assign following '='.".to_string())?;
    Ok(AssignRange {
        dim_name: limit.dim_name,
        value,
        lower_bound: limit.lower_bound,
        upper_bound: limit.upper_bound,
        inclusive_lower: limit.inclusive_lower,
        inclusive_upper: limit.inclusive_upper,
        negate: limit.negate,
    })
}

/// Parse a `filters.assign` `condition` DimRange (`Dim[range]`).
fn parse_assign_condition(spec: &str) -> Result<AssignCondition, String> {
    let limit = parse_range_limit(spec)?;
    if !spec[limit.consumed..].trim().is_empty() {
        return Err("filters.assign: Invalid characters following condition range.".to_string());
    }
    Ok(AssignCondition {
        dim_name: limit.dim_name,
        lower_bound: limit.lower_bound,
        upper_bound: limit.upper_bound,
        inclusive_lower: limit.inclusive_lower,
        inclusive_upper: limit.inclusive_upper,
        negate: limit.negate,
    })
}

/// Parse a `filters.smrf` `classbits` option (comma-separated
/// `synthetic|keypoint|withheld`) into the Classification-flag bit mask,
/// matching the C++ `Segmentation::PointClasses` stream operator.
fn parse_classbits(value: &str) -> Result<u8, String> {
    use pdal_filters::smrf::{CLASSBIT_KEYPOINT, CLASSBIT_SYNTHETIC, CLASSBIT_WITHHELD};
    let mut bits = 0u8;
    for token in value.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "keypoint" => bits |= CLASSBIT_KEYPOINT,
            "synthetic" => bits |= CLASSBIT_SYNTHETIC,
            "withheld" => bits |= CLASSBIT_WITHHELD,
            other => {
                return Err(format!("filters.smrf: Invalid 'classbits' value: '{other}'."));
            }
        }
    }
    Ok(bits)
}

fn crop_filter_from_options(options: &Options) -> Result<CropFilter, StageError> {
    let mut bounds = Vec::new();
    for value in options.values("bounds") {
        bounds.push(parse_crop_bounds(value)?);
    }

    let polygons = options.values("polygon").to_vec();

    let mut centers = Vec::new();
    for value in options.values("point") {
        let point = parse_wkt_point_coords(value)?;
        match point.as_slice() {
            [x, y] => centers.push(CropCenter::new_2d(*x, *y)),
            [x, y, z] => centers.push(CropCenter::new_3d(*x, *y, *z)),
            _ => unreachable!("parse_wkt_point_coords validates coordinate count"),
        }
    }

    CropFilter::new(
        get_bool(options, "outside", false)?,
        bounds,
        polygons,
        centers,
        get_f64(options, "distance", 0.0)?,
    )
}

fn parse_crop_bounds(value: &str) -> Result<(f64, f64, f64, f64, f64, f64), StageError> {
    if let Ok(parsed) = parse_bounds3d(value, 0) {
        let bounds = parsed.bounds;
        return Ok((
            bounds.minx,
            bounds.miny,
            bounds.minz,
            bounds.maxx,
            bounds.maxy,
            bounds.maxz,
        ));
    }

    let bounds = parse_bounds2d(value, 0).map_err(StageError)?.bounds;
    Ok((
        bounds.minx,
        bounds.miny,
        f64::MIN,
        bounds.maxx,
        bounds.maxy,
        f64::MAX,
    ))
}

fn get_f64(options: &Options, key: &str, default: f64) -> Result<f64, StageError> {
    options.try_get_f64(key, default).map_err(StageError)
}

fn get_u64(options: &Options, key: &str, default: u64) -> Result<u64, StageError> {
    options.try_get_u64(key, default).map_err(StageError)
}

fn get_bool(options: &Options, key: &str, default: bool) -> Result<bool, StageError> {
    options.try_get_bool(key, default).map_err(StageError)
}

/// Parse a WKT POINT string into `[x, y, z]`.
///
/// Accepts `"POINT Z (x y z)"`, `"POINT (x y z)"`, and `"POINT (x y)"` (z=0).
/// Returns an error if the string is not a valid WKT point.
fn parse_wkt_point(wkt: &str) -> Result<[f64; 3], StageError> {
    let parts = parse_wkt_point_coords(wkt)?;
    match parts.as_slice() {
        [x, y] => Ok([*x, *y, 0.0]),
        [x, y, z] => Ok([*x, *y, *z]),
        _ => unreachable!("parse_wkt_point_coords validates coordinate count"),
    }
}

fn parse_wkt_point_coords(wkt: &str) -> Result<Vec<f64>, StageError> {
    let s = wkt.trim();
    let s = s
        .strip_prefix("POINT")
        .ok_or_else(|| StageError(format!("viewpoint must be a WKT POINT string, got '{wkt}'")))?;
    // Skip optional Z / ZM dimensionality keyword.
    let s = s.trim_start();
    let s = s
        .strip_prefix("ZM")
        .or_else(|| s.strip_prefix("Z"))
        .or_else(|| s.strip_prefix("M"))
        .map(|s| s.trim_start())
        .unwrap_or(s);
    let s = s
        .strip_prefix('(')
        .ok_or_else(|| StageError(format!("viewpoint must be a WKT POINT string, got '{wkt}'")))?;
    let s = s
        .strip_suffix(')')
        .ok_or_else(|| StageError(format!("viewpoint must be a WKT POINT string, got '{wkt}'")))?;
    let parts: Vec<f64> = s
        .split_whitespace()
        .map(|p| {
            p.parse().map_err(|_| {
                StageError(format!(
                    "viewpoint must be a WKT POINT string with numeric coordinates, got '{wkt}'"
                ))
            })
        })
        .collect::<Result<Vec<f64>, StageError>>()?;
    match parts.len() {
        2 | 3 => Ok(parts),
        _ => Err(StageError(format!(
            "viewpoint must have 2 or 3 coordinates, got {} in '{wkt}'",
            parts.len()
        ))),
    }
}

fn sort_order(value: &str) -> Result<SortOrder, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "" | "asc" | "ascending" => Ok(SortOrder::Asc),
        "desc" | "descending" => Ok(SortOrder::Desc),
        _ => Err(StageError(format!(
            "filters.sort order must be 'asc' or 'desc', got '{value}'."
        ))),
    }
}

fn sort_algorithm(value: &str) -> Result<SortAlgorithm, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "" | "normal" => Ok(SortAlgorithm::Normal),
        "stable" => Ok(SortAlgorithm::Stable),
        _ => Err(StageError(format!(
            "filters.sort algorithm must be 'normal' or 'stable', got '{value}'."
        ))),
    }
}

fn covariance_mode(value: &str) -> CovarianceMode {
    match value.to_ascii_lowercase().as_str() {
        "raw" => CovarianceMode::Raw,
        "normalized" => CovarianceMode::Normalized,
        _ => CovarianceMode::Sqrt,
    }
}

fn nn_distance_mode(value: &str) -> Result<NNDistanceMode, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "kth" | "k" => Ok(NNDistanceMode::Kth),
        "avg" | "average" => Ok(NNDistanceMode::Average),
        _ => Err(StageError(format!(
            "filters.nndistance mode must be 'kth' or 'avg', got '{value}'."
        ))),
    }
}

fn m3c2_orientation(value: &str) -> Result<M3C2NormalOrientation, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "up" => Ok(M3C2NormalOrientation::Up),
        "down" => Ok(M3C2NormalOrientation::Down),
        "none" => Ok(M3C2NormalOrientation::None),
        _ => Err(StageError(format!(
            "filters.m3c2 orientation must be 'up', 'down', or 'none', got '{value}'."
        ))),
    }
}

pub fn create_writer(name: &str, options: &Options) -> Result<Box<dyn Writer>, StageError> {
    match name {
        "writers.null" => Ok(Box::new(pdal_io::nullwriter::NullWriter::new(options))),
        "writers.bpf" => Ok(Box::new(pdal_io::bpf::BpfWriter::new(options))),
        "writers.fbi" => Ok(Box::new(pdal_io::fbi_writer::FbiWriter::new(options))),
        "writers.gltf" => Ok(Box::new(pdal_io::gltf::GltfWriter::new(options))),
        "writers.text" => Ok(Box::new(pdal_io::text_writer::TextWriter::new(options))),
        "writers.pcd" => Ok(Box::new(pdal_io::pcd::PcdWriter::new(options))),
        "writers.sbet" => Ok(Box::new(pdal_io::sbet_writer::SbetWriter::new(options))),
        "writers.las" => Ok(Box::new(pdal_io::las_writer::LasWriter::new(options))),
        "writers.laz" => Ok(Box::new(pdal_io::las_writer::LasWriter::new_laz(options))),
        "writers.nitf" => Ok(Box::new(pdal_io::nitf_writer::NitfWriter::new(options)?)),
        "writers.ply" => Ok(Box::new(pdal_io::ply::PlyWriter::new(options)?)),
        "writers.ogr" => Ok(Box::new(pdal_io::ogr_writer::OgrWriter::new(options))),
        "writers.gdal" => Ok(Box::new(pdal_io::gdal_writer::GdalWriter::new(options))),
        "writers.raster" => Ok(Box::new(pdal_io::raster_writer::RasterWriter::new(options))),
        "writers.spz" => Ok(Box::new(pdal_io::spz::SpzWriter::new(options))),
        _ => Err(StageError(format!(
            "Writer driver '{name}' is not available in the Rust port."
        ))),
    }
}

pub fn create_stage(name: &str, options: &Options) -> Result<CreatedStage, StageError> {
    if name.starts_with("readers.") {
        create_reader(name, options).map(CreatedStage::Reader)
    } else if name.starts_with("writers.") {
        create_writer(name, options).map(CreatedStage::Writer)
    } else if name.starts_with("filters.") {
        create_filter(name, options).map(CreatedStage::Filter)
    } else {
        Err(StageError(format!(
            "Stage driver '{name}' is not available in the Rust port."
        )))
    }
}

pub fn pipeline_from_json(json: &str) -> Result<Pipeline, StageError> {
    let value: Value = serde_json::from_str(json)
        .map_err(|err| StageError(format!("Invalid pipeline JSON: {err}")))?;
    let stages = pipeline_stages(&value)?;

    let mut pipeline = Pipeline::new();
    let mut tags = HashMap::new();
    let mut previous: Option<usize> = None;

    for (position, stage_val) in stages.iter().enumerate() {
        let string_stage;
        let object = if let Some(object) = stage_val.as_object() {
            object
        } else if let Some(filename) = stage_val.as_str() {
            string_stage = filename_stage_object(filename);
            &string_stage
        } else {
            return Err(StageError(format!(
                "Pipeline stage {position} must be a JSON object or filename string."
            )));
        };

        let options = options_from_object(object)?;
        let driver_name = stage_name(object, position, stages.len(), &options)?;
        let stage = create_stage(&driver_name, &options)?;

        let idx = match stage {
            CreatedStage::Reader(r) => pipeline.add_reader(&driver_name, r, options),
            CreatedStage::Filter(f) => pipeline.add_stage(&driver_name, f, options),
            CreatedStage::Writer(w) => pipeline.add_writer(&driver_name, w, options),
        };

        if let Some(tag) = object.get("tag").and_then(Value::as_str) {
            pipeline.set_tag(idx, tag)?;
            tags.insert(tag.to_string(), idx);
        }

        let explicit_inputs = add_explicit_inputs(&mut pipeline, idx, object, &tags)?;
        if !explicit_inputs && position > 0 {
            if let Some(input) = previous {
                pipeline.add_dependency(idx, input)?;
            }
        }
        previous = Some(idx);
    }

    Ok(pipeline)
}

fn filename_stage_object(filename: &str) -> serde_json::Map<String, Value> {
    let mut object = serde_json::Map::new();
    object.insert("filename".to_string(), Value::String(filename.to_string()));
    object
}

fn pipeline_stages(value: &Value) -> Result<&Vec<Value>, StageError> {
    if let Some(stages) = value.as_array() {
        return Ok(stages);
    }
    let Some(object) = value.as_object() else {
        return Err(StageError(
            "Pipeline JSON must be an array or an object with a 'pipeline' array.".to_string(),
        ));
    };
    object
        .get("pipeline")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            StageError("Pipeline JSON object must contain a 'pipeline' array.".to_string())
        })
}

fn stage_name(
    object: &serde_json::Map<String, Value>,
    position: usize,
    len: usize,
    options: &Options,
) -> Result<String, StageError> {
    if let Some(name) = object.get("type").and_then(Value::as_str) {
        return Ok(name.to_string());
    }
    let filename = options.get_str("filename", "");
    if filename.is_empty() {
        return Err(StageError(format!(
            "Pipeline stage {position} is missing a 'type'."
        )));
    }
    if position == 0 {
        infer_reader_driver(&filename)
            .map(str::to_string)
            .ok_or_else(|| StageError(format!("Unable to infer reader for '{filename}'.")))
    } else if position + 1 == len {
        infer_writer_driver(&filename)
            .map(str::to_string)
            .ok_or_else(|| StageError(format!("Unable to infer writer for '{filename}'.")))
    } else {
        Err(StageError(format!(
            "Pipeline stage {position} needs an explicit 'type'."
        )))
    }
}

fn options_from_object(object: &serde_json::Map<String, Value>) -> Result<Options, StageError> {
    let mut options = Options::new();
    for (key, value) in object {
        if matches!(key.as_str(), "type" | "tag" | "inputs") {
            continue;
        }
        match value {
            Value::String(s) => {
                options.add(key, s);
            }
            Value::Bool(b) => {
                options.add(key, *b);
            }
            Value::Number(n) => {
                options.add(key, n);
            }
            Value::Array(items) => {
                let joined = items
                    .iter()
                    .map(option_value_to_string)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(",");
                options.add(key, joined);
            }
            // A nested object option value is stored as its JSON text, matching
            // C++ PDAL (e.g. filters.mongo's `expression` query object).
            Value::Object(_) => {
                options.add(key, value.to_string());
            }
            Value::Null => {
                return Err(StageError(format!(
                    "Option '{key}' must be a scalar, scalar array, or object."
                )));
            }
        }
    }
    Ok(options)
}

fn option_value_to_string(value: &Value) -> Result<String, StageError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(StageError(
            "Array options must contain only scalar values.".to_string(),
        )),
    }
}

fn add_explicit_inputs(
    pipeline: &mut Pipeline,
    idx: usize,
    object: &serde_json::Map<String, Value>,
    tags: &HashMap<String, usize>,
) -> Result<bool, StageError> {
    let Some(inputs) = object.get("inputs") else {
        return Ok(false);
    };
    let input_names = match inputs {
        Value::String(name) => vec![name.as_str()],
        Value::Array(values) => values
            .iter()
            .map(|value| {
                value.as_str().ok_or_else(|| {
                    StageError("Pipeline 'inputs' entries must be tag strings.".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(StageError(
                "Pipeline 'inputs' must be a tag string or array of tag strings.".to_string(),
            ));
        }
    };
    for name in input_names {
        let input = tags
            .get(name)
            .copied()
            .ok_or_else(|| StageError(format!("Unknown pipeline input tag '{name}'.")))?;
        pipeline.add_dependency(idx, input)?;
    }
    Ok(true)
}

/// Create a pipeline from JSON.
///
/// # Safety
/// `json` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_create_json(json: *const c_char) -> *mut PipelineHandle {
    clear_last_error();
    if json.is_null() {
        set_last_error("null json string");
        return std::ptr::null_mut();
    }
    let json_str = CStr::from_ptr(json).to_string_lossy();
    match pipeline_from_json(&json_str) {
        Ok(pipeline) => Box::into_raw(Box::new(PipelineHandle { pipeline })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

#[cfg(test)]
mod tests;
