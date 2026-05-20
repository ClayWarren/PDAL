//! Stage registry -- construct implemented stages from PDAL driver names.
//!
//! This is the name-keyed slice of PDAL's `StageFactory`, restricted to the
//! reader/filter/writer drivers this Rust spike currently implements.

use crate::error::{clear_last_error, set_last_error};
use crate::pipeline_abi::PipelineHandle;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline, Reader, Writer};
use pdal_core::point::DimId;
use pdal_core::stage::StageError;

use pdal_filters::approximate_coplanar::ApproximateCoplanarFilter;
use pdal_filters::chipper::ChipperFilter;
use pdal_filters::cluster::ClusterFilter;
use pdal_filters::dbscan::DbscanFilter;
use pdal_filters::decimation::DecimationFilter;
use pdal_filters::eigenvalues::EigenvaluesFilter;
use pdal_filters::elm::ElmFilter;
use pdal_filters::estimate_rank::EstimateRankFilter;
use pdal_filters::gpstimeconvert::GpsTimeConvert;
use pdal_filters::groupby::GroupByFilter;
use pdal_filters::hagnn::HagNnFilter;
use pdal_filters::head::HeadFilter;
use pdal_filters::hexbin::HexBinFilter;
use pdal_filters::iqr::IqrFilter;
use pdal_filters::labelduplicates::LabelDuplicatesFilter;
use pdal_filters::locate::LocateFilter;
use pdal_filters::lof::LofFilter;
use pdal_filters::mad::MadFilter;
use pdal_filters::merge::MergeFilter;
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::nndistance::{NNDistanceFilter, NNDistanceMode};
use pdal_filters::optimal_neighborhood::OptimalNeighborhoodFilter;
use pdal_filters::outlier::OutlierFilter;
use pdal_filters::planefit::PlaneFitFilter;
use pdal_filters::radialdensity::RadialDensityFilter;
use pdal_filters::randomize::RandomizeFilter;
use pdal_filters::reciprocity::ReciprocityFilter;
use pdal_filters::reprojection::ReprojectionFilter;
use pdal_filters::returns::ReturnsFilter;
use pdal_filters::sample::SampleFilter;
use pdal_filters::separatescanline::SeparateScanLineFilter;
use pdal_filters::smrf::SmrfFilter;
use pdal_filters::sort::{SortAlgorithm, SortFilter, SortOrder};
use pdal_filters::splitter::SplitterFilter;
use pdal_filters::stats::StatsFilter;
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
    "readers.las",
    "readers.laz",
    "readers.ply",
    "readers.stac",
    "readers.tindex",
];

pub const FILTER_DRIVERS: &[&str] = &[
    "filters.approximatecoplanar",
    "filters.chipper",
    "filters.cluster",
    "filters.dbscan",
    "filters.decimation",
    "filters.eigenvalues",
    "filters.elm",
    "filters.estimaterank",
    "filters.gpstimeconvert",
    "filters.groupby",
    "filters.hag_nn",
    "filters.head",
    "filters.hexbin",
    "filters.iqr",
    "filters.label_duplicates",
    "filters.locate",
    "filters.lof",
    "filters.mad",
    "filters.merge",
    "filters.mortonorder",
    "filters.nndistance",
    "filters.optimalneighborhood",
    "filters.outlier",
    "filters.planefit",
    "filters.radialdensity",
    "filters.randomize",
    "filters.reciprocity",
    "filters.reprojection",
    "filters.returns",
    "filters.smrf",
    "filters.sample",
    "filters.separatescanline",
    "filters.splitter",
    "filters.sort",
    "filters.stats",
    "filters.tail",
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
    "writers.ply",
];

pub enum CreatedStage {
    Reader(Box<dyn Reader>),
    Filter(Box<dyn pdal_core::pipeline::StageWrapper>),
    Writer(Box<dyn Writer>),
}

pub fn create_reader(name: &str, options: &Options) -> Result<Box<dyn Reader>, StageError> {
    match name {
        "readers.faux" => Ok(Box::new(pdal_io::faux::FauxReader::new(options))),
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
        "readers.las" | "readers.laz" => Ok(Box::new(pdal_io::las::LasReader::new(options))),
        "readers.ply" => Ok(Box::new(pdal_io::ply::PlyReader::new(options))),
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
        "filters.dbscan" => Ok(Box::new(FilterWrapper::new(DbscanFilter::new(
            get_u64(options, "min_points", 6)? as usize,
            get_f64(options, "eps", 1.0)?,
            comma_list(&options.get_str("dimensions", "X,Y,Z")),
        )))),
        "filters.decimation" => Ok(Box::new(FilterWrapper::new(DecimationFilter::new(options)))),
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
        "filters.gpstimeconvert" => Ok(Box::new(FilterWrapper::new(GpsTimeConvert::from_options(
            options,
        )?))),
        "filters.groupby" => Ok(Box::new(FilterWrapper::new(GroupByFilter::new(
            options.get_str("dimension", ""),
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
            Ok(Box::new(FilterWrapper::new(HexBinFilter::new(
                edge,
                get_u64(options, "threshold", 15)? as u32,
                get_u64(options, "sample_size", 5000)? as usize,
                (!density.is_empty()).then_some(density),
            ))))
        }
        "filters.iqr" => Ok(Box::new(FilterWrapper::new(IqrFilter::new(
            get_f64(options, "multiplier", 1.5)?,
            DimId::from_name(&options.get_str("dimension", "Z")),
        )))),
        "filters.locate" => Ok(Box::new(FilterWrapper::new(LocateFilter::new(
            options.get_str("dimension", ""),
            options.get_str("minmax", "max"),
        )))),
        "filters.label_duplicates" => Ok(Box::new(FilterWrapper::new(LabelDuplicatesFilter::new(
            comma_list(&options.get_str("dimensions", "X,Y,Z")),
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
        "filters.mortonorder" => Ok(Box::new(FilterWrapper::new(MortonOrderFilter::new(
            get_bool(options, "reverse", false)?,
        )))),
        "filters.nndistance" => Ok(Box::new(FilterWrapper::new(NNDistanceFilter::new(
            get_u64(options, "knn", get_u64(options, "k", 8)?)? as usize,
            nn_distance_mode(&options.get_str("mode", "kth"))?,
        )))),
        "filters.optimalneighborhood" => Ok(Box::new(FilterWrapper::new(
            OptimalNeighborhoodFilter::new(
                get_u64(options, "min_k", 3)? as usize,
                get_u64(options, "max_k", 8)? as usize,
            ),
        ))),
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
        "filters.reciprocity" => Ok(Box::new(FilterWrapper::new(ReciprocityFilter::new(
            get_u64(options, "knn", 8)? as usize,
        )))),
        "filters.reprojection" => Ok(Box::new(FilterWrapper::new(ReprojectionFilter::new(
            &options.get_str("out_srs", ""),
            options.has("in_srs").then(|| options.get_str("in_srs", "")),
            get_bool(options, "error_on_failure", false)?,
        )))),
        "filters.returns" => Ok(Box::new(FilterWrapper::new(ReturnsFilter::new(
            comma_list(&options.get_str("groups", "last")),
        )))),
        "filters.smrf" => Ok(Box::new(FilterWrapper::new(SmrfFilter::new(
            get_f64(options, "cell", 1.0)?,
            get_f64(options, "slope", 0.15)?,
            options
                .has("window")
                .then(|| get_f64(options, "window", 18.0))
                .transpose()?,
            get_f64(options, "scalar", 1.25)?,
            get_f64(options, "threshold", 0.5)?,
            get_u64(options, "ground_class", 2)? as u8,
            get_u64(options, "other_class", 1)? as u8,
            get_bool(options, "only_ground", true)?,
            comma_list(&options.get_str("returns", "last,only")),
        )))),
        "filters.sample" => Ok(Box::new(FilterWrapper::new(SampleFilter::new(options)))),
        "filters.separatescanline" => Ok(Box::new(FilterWrapper::new(
            SeparateScanLineFilter::new(get_u64(options, "groupby", 1)?),
        ))),
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
        "filters.stats" => Ok(Box::new(FilterWrapper::new(StatsFilter::from_options(
            options,
        )))),
        "filters.tail" => Ok(Box::new(FilterWrapper::new(TailFilter::new(
            get_u64(options, "count", 10)?,
            get_bool(options, "invert", false)?,
        )))),
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

fn get_f64(options: &Options, key: &str, default: f64) -> Result<f64, StageError> {
    options.try_get_f64(key, default).map_err(StageError)
}

fn get_u64(options: &Options, key: &str, default: u64) -> Result<u64, StageError> {
    options.try_get_u64(key, default).map_err(StageError)
}

fn get_bool(options: &Options, key: &str, default: bool) -> Result<bool, StageError> {
    options.try_get_bool(key, default).map_err(StageError)
}

fn sort_order(value: &str) -> Result<SortOrder, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "asc" | "ascending" => Ok(SortOrder::Asc),
        "desc" | "descending" => Ok(SortOrder::Desc),
        _ => Err(StageError(format!(
            "filters.sort order must be 'asc' or 'desc', got '{value}'."
        ))),
    }
}

fn sort_algorithm(value: &str) -> Result<SortAlgorithm, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "normal" => Ok(SortAlgorithm::Normal),
        "stable" => Ok(SortAlgorithm::Stable),
        _ => Err(StageError(format!(
            "filters.sort algorithm must be 'normal' or 'stable', got '{value}'."
        ))),
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
        "writers.ply" => Ok(Box::new(pdal_io::ply::PlyWriter::new(options))),
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
            Value::Null | Value::Object(_) => {
                return Err(StageError(format!(
                    "Option '{key}' must be a scalar or scalar array."
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
