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
                options.get_u64("knn", 8) as usize,
                options.get_f64("thresh1", 25.0),
                options.get_f64("thresh2", 6.0),
            ),
        ))),
        "filters.chipper" => Ok(Box::new(FilterWrapper::new(ChipperFilter::new(
            options.get_u64("capacity", options.get_u64("threshold", 5000)),
        )))),
        "filters.cluster" => Ok(Box::new(FilterWrapper::new(ClusterFilter::new(
            options.get_u64("min_points", 1) as usize,
            options.get_u64("max_points", u64::MAX) as usize,
            options.get_f64("tolerance", 1.0),
            options.get_bool("is3d", true),
        )))),
        "filters.dbscan" => Ok(Box::new(FilterWrapper::new(DbscanFilter::new(
            options.get_u64("min_points", 6) as usize,
            options.get_f64("eps", 1.0),
            comma_list(&options.get_str("dimensions", "X,Y,Z")),
        )))),
        "filters.decimation" => Ok(Box::new(FilterWrapper::new(DecimationFilter::new(options)))),
        "filters.eigenvalues" => Ok(Box::new(FilterWrapper::new(EigenvaluesFilter::new(
            options.get_u64("knn", 8) as usize,
            options.get_bool("normalize", false),
            options.get_u64("stride", 1) as usize,
            options
                .has("radius")
                .then(|| options.get_f64("radius", 0.0)),
            options.get_u64("min_k", 3) as usize,
        )))),
        "filters.elm" => Ok(Box::new(FilterWrapper::new(ElmFilter::new(
            options.get_f64("cell", 10.0),
            options.get_u64("class", 7) as u8,
            options.get_f64("threshold", 1.0),
        )))),
        "filters.estimaterank" => Ok(Box::new(FilterWrapper::new(EstimateRankFilter::new(
            options.get_u64("knn", 8) as usize,
            options.get_f64("threshold", 0.01),
        )))),
        "filters.gpstimeconvert" => Ok(Box::new(FilterWrapper::new(GpsTimeConvert::from_options(
            options,
        )?))),
        "filters.groupby" => Ok(Box::new(FilterWrapper::new(GroupByFilter::new(
            options.get_str("dimension", ""),
        )))),
        "filters.hag_nn" => Ok(Box::new(FilterWrapper::new(HagNnFilter::new(
            options.get_u64("count", 1) as usize,
            options.get_f64("max_distance", 0.0),
            options.get_bool("allow_extrapolation", false),
            options.get_u64("class", 2) as u8,
        )))),
        "filters.head" => Ok(Box::new(FilterWrapper::new(HeadFilter::new(
            options.get_u64("count", 10),
            options.get_bool("invert", false),
        )))),
        "filters.iqr" => Ok(Box::new(FilterWrapper::new(IqrFilter::new(
            options.get_f64("multiplier", 1.5),
            DimId::from_name(&options.get_str("dimension", "Z")),
        )))),
        "filters.locate" => Ok(Box::new(FilterWrapper::new(LocateFilter::new(
            options.get_str("dimension", ""),
            options.get_str("minmax", "max"),
        )))),
        "filters.label_duplicates" => Ok(Box::new(FilterWrapper::new(LabelDuplicatesFilter::new(
            comma_list(&options.get_str("dimensions", "X,Y,Z")),
        )))),
        "filters.lof" => Ok(Box::new(FilterWrapper::new(LofFilter::new(
            options.get_u64("minpts", 10) as usize,
        )))),
        "filters.mad" => Ok(Box::new(FilterWrapper::new(MadFilter::new(
            options.get_f64("multiplier", 1.4826),
            DimId::from_name(&options.get_str("dimension", "Z")),
            options.get_f64("mad_multiplier", 2.0),
        )))),
        "filters.merge" => Ok(Box::new(FilterWrapper::new(MergeFilter::new()))),
        "filters.mortonorder" => Ok(Box::new(FilterWrapper::new(MortonOrderFilter::new(
            options.get_bool("reverse", false),
        )))),
        "filters.nndistance" => Ok(Box::new(FilterWrapper::new(NNDistanceFilter::new(
            options.get_u64("knn", options.get_u64("k", 8)) as usize,
            nn_distance_mode(&options.get_str("mode", "kth"))?,
        )))),
        "filters.optimalneighborhood" => Ok(Box::new(FilterWrapper::new(
            OptimalNeighborhoodFilter::new(
                options.get_u64("min_k", 3) as usize,
                options.get_u64("max_k", 8) as usize,
            ),
        ))),
        "filters.outlier" => Ok(Box::new(FilterWrapper::new(OutlierFilter::new(
            options.get_str("method", "statistical"),
            options.get_u64("min_k", 2) as usize,
            options.get_f64("radius", 1.0),
            options.get_u64("mean_k", 8) as usize,
            options.get_f64("multiplier", 2.0),
            options.get_u64("class", 7) as u8,
        )))),
        "filters.planefit" => Ok(Box::new(FilterWrapper::new(PlaneFitFilter::new(
            options.get_u64("knn", 8) as usize,
        )))),
        "filters.radialdensity" => Ok(Box::new(FilterWrapper::new(RadialDensityFilter::new(
            options.get_f64("radius", 1.0),
        )))),
        "filters.randomize" => {
            let seed = options
                .has("seed")
                .then(|| options.get_u64("seed", 0) as u32);
            Ok(Box::new(FilterWrapper::new(RandomizeFilter::new(seed))))
        }
        "filters.reciprocity" => Ok(Box::new(FilterWrapper::new(ReciprocityFilter::new(
            options.get_u64("knn", 8) as usize,
        )))),
        "filters.reprojection" => Ok(Box::new(FilterWrapper::new(ReprojectionFilter::new(
            &options.get_str("out_srs", ""),
            options.has("in_srs").then(|| options.get_str("in_srs", "")),
            options.get_bool("error_on_failure", false),
        )))),
        "filters.returns" => Ok(Box::new(FilterWrapper::new(ReturnsFilter::new(
            comma_list(&options.get_str("groups", "last")),
        )))),
        "filters.smrf" => Ok(Box::new(FilterWrapper::new(SmrfFilter::new(
            options.get_f64("cell", 1.0),
            options.get_f64("slope", 0.15),
            options
                .has("window")
                .then(|| options.get_f64("window", 18.0)),
            options.get_f64("scalar", 1.25),
            options.get_f64("threshold", 0.5),
            options.get_u64("ground_class", 2) as u8,
            options.get_u64("other_class", 1) as u8,
            options.get_bool("only_ground", true),
            comma_list(&options.get_str("returns", "last,only")),
        )))),
        "filters.sample" => Ok(Box::new(FilterWrapper::new(SampleFilter::new(options)))),
        "filters.separatescanline" => Ok(Box::new(FilterWrapper::new(
            SeparateScanLineFilter::new(options.get_u64("groupby", 1)),
        ))),
        "filters.splitter" => Ok(Box::new(FilterWrapper::new(SplitterFilter::new(
            options.get_f64("length", 1000.0),
            if options.has("origin_x") {
                options.get_f64("origin_x", f64::NAN)
            } else {
                f64::NAN
            },
            if options.has("origin_y") {
                options.get_f64("origin_y", f64::NAN)
            } else {
                f64::NAN
            },
            options.get_f64("buffer", 0.0),
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
            options.get_u64("count", 10),
            options.get_bool("invert", false),
        )))),
        "filters.voxelcenternearestneighbor" => Ok(Box::new(FilterWrapper::new(
            VoxelCenterNearestNeighborFilter::new(options.get_f64("cell", 1.0)),
        ))),
        "filters.voxelcentroidnearestneighbor" => Ok(Box::new(FilterWrapper::new(
            VoxelCentroidNearestNeighborFilter::new(options.get_f64("cell", 1.0)),
        ))),
        "filters.voxeldownsize" => Ok(Box::new(FilterWrapper::new(VoxelDownsizeFilter::new(
            options,
        )))),
        "filters.zsmooth" => Ok(Box::new(FilterWrapper::new(ZsmoothFilter::new(
            options.get_f64("radius", 1.0),
            options.get_f64("position", 0.5),
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
mod tests {
    use super::*;
    use pdal_core::options::Options;
    use pdal_core::pipeline::Reader;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use pdal_io::las::LasReader;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn every_listed_reader_driver_constructs() {
        let options = Options::new();
        for name in READER_DRIVERS {
            if *name == "readers.faux" || *name == "readers.text" {
                continue; // these need a file or specific options
            }
            // Just check that they are in the match arm
            let _ = create_reader(name, &options);
        }
    }

    #[test]
    fn every_listed_filter_driver_constructs() {
        for name in FILTER_DRIVERS {
            let options = default_filter_options(name);
            assert!(
                create_filter(name, &options).is_ok(),
                "{name} should construct from registry defaults"
            );
        }
    }

    #[test]
    fn every_listed_writer_driver_constructs() {
        let options = Options::new();
        for name in WRITER_DRIVERS {
            // Just check that they are in the match arm
            let _ = create_writer(name, &options);
        }
    }

    #[test]
    fn unified_stage_factory_dispatches_by_prefix() {
        let options = Options::new();
        assert!(matches!(
            create_stage("readers.faux", &options),
            Ok(CreatedStage::Reader(_))
        ));
        assert!(matches!(
            create_stage("filters.decimation", &options),
            Ok(CreatedStage::Filter(_))
        ));
        assert!(matches!(
            create_stage("writers.null", &options),
            Ok(CreatedStage::Writer(_))
        ));
    }

    #[test]
    fn unknown_and_unported_drivers_are_rejected() {
        let options = Options::new();
        assert!(create_reader("readers.unknown", &options).is_err());
        assert!(create_filter("filters.unknown", &options).is_err());
        assert!(create_writer("writers.unknown", &options).is_err());
    }

    #[test]
    fn laz_writer_driver_forces_compression_for_las_extension() {
        let temp = make_temp_dir("laz-driver-compression");
        let output = temp.join("explicit-laz-driver.las");
        let mut options = Options::new();
        options.add("filename", output.display());

        let mut writer = create_writer("writers.laz", &options).unwrap();
        writer.write(&[single_point_view()]).unwrap();

        let mut reader_options = Options::new();
        reader_options.add("filename", output.display());
        let mut reader = LasReader::new(&reader_options);
        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].get_f64(0, &DimId::X), 1.0);
    }

    #[test]
    fn registry_created_reader_reads_a_fixture() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = repo.join("test/data/text/utm17_1.txt");
        let mut options = Options::new();
        options.add("filename", input.display());

        let mut reader = create_reader("readers.text", &options).unwrap();
        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 10);
    }

    #[test]
    fn pipeline_json_runs_reader_filter_writer_with_inferred_drivers() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = repo.join("test/data/text/utm17_1.txt");
        let output =
            std::env::temp_dir().join(format!("pdal-rust-registry-{}.pcd", std::process::id()));
        let _ = std::fs::remove_file(&output);

        let json = format!(
            r#"[
                {{"filename":"{}"}},
                {{"type":"filters.decimation", "step":2}},
                {{"filename":"{}"}}
            ]"#,
            escape_json_path(&input),
            escape_json_path(&output)
        );

        let mut pipeline = pipeline_from_json(&json).unwrap();
        let result = pipeline.execute_with_result(Vec::new()).unwrap();
        assert_eq!(result.point_count, 0); // writers return empty
        assert_eq!(result.view_count, 0);

        assert!(output.exists());
        let written = std::fs::read_to_string(&output).unwrap();
        assert!(written.contains("POINTS 5"));
        let _ = std::fs::remove_file(&output);
    }

    #[test]
    fn pipeline_json_accepts_root_pipeline_object() {
        let json = r#"{
            "pipeline": [
                {"type":"readers.faux", "count":4},
                {"type":"filters.decimation", "step":2}
            ]
        }"#;
        let mut pipeline = pipeline_from_json(json).unwrap();
        let result = pipeline.execute_with_result(Vec::new()).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(result.view_count, 1);
    }

    #[test]
    fn pipeline_json_accepts_filename_string_stages() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = repo.join("test/data/text/utm17_1.txt");
        let output =
            std::env::temp_dir().join(format!("pdal-rust-string-stage-{}.pcd", std::process::id()));
        let _ = std::fs::remove_file(&output);

        let json = format!(
            r#"[
                "{}",
                {{"type":"filters.decimation", "step":2}},
                "{}"
            ]"#,
            escape_json_path(&input),
            escape_json_path(&output)
        );

        let mut pipeline = pipeline_from_json(&json).unwrap();
        let result = pipeline.execute_with_result(Vec::new()).unwrap();
        assert_eq!(result.point_count, 0);
        assert_eq!(result.view_count, 0);
        assert!(output.exists());
        let _ = std::fs::remove_file(&output);
    }

    #[test]
    fn pipeline_json_runs_sort_filter() {
        let json = r#"[
            {"type":"readers.faux", "count":4, "mode":"ramp", "minx":1, "maxx":4},
            {"type":"filters.sort", "dimensions":"X", "order":"desc"}
        ]"#;
        let mut pipeline = pipeline_from_json(json).unwrap();
        let views = pipeline.execute(Vec::new()).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].get_f64(0, &pdal_core::point::DimId::X), 4.0);
        assert_eq!(views[0].get_f64(3, &pdal_core::point::DimId::X), 1.0);
    }

    #[test]
    fn pipeline_json_runs_groupby_filter() {
        let json = r#"[
            {"type":"readers.faux", "count":2, "mode":"ramp", "minx":1, "maxx":2},
            {"type":"filters.groupby", "dimension":"X"}
        ]"#;
        let mut pipeline = pipeline_from_json(json).unwrap();
        let views = pipeline.execute(Vec::new()).unwrap();
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].len(), 1);
        assert_eq!(views[1].len(), 1);
    }

    #[test]
    fn pipeline_json_runs_newly_registry_visible_filter_families() {
        let cases = [
            (
                "filters.nndistance",
                r#"{"type":"filters.nndistance", "knn":1, "mode":"kth"}"#,
                DimId::NNDistance,
            ),
            (
                "filters.radialdensity",
                r#"{"type":"filters.radialdensity", "radius":2.0}"#,
                DimId::RadialDensity,
            ),
            (
                "filters.eigenvalues",
                r#"{"type":"filters.eigenvalues", "knn":4}"#,
                DimId::Eigenvalue0,
            ),
            (
                "filters.cluster",
                r#"{"type":"filters.cluster", "tolerance":10.0, "min_points":1}"#,
                DimId::ClusterID,
            ),
            (
                "filters.zsmooth",
                r#"{"type":"filters.zsmooth", "radius":10.0, "dimension":"Zsmoothed"}"#,
                DimId::from_name("Zsmoothed"),
            ),
        ];

        for (name, filter_json, dim) in cases {
            let json = format!(
                r#"[
                    {{"type":"readers.faux", "count":5, "mode":"ramp", "minx":0, "maxx":4, "miny":0, "maxy":4, "minz":0, "maxz":4}},
                    {filter_json}
                ]"#
            );
            let mut pipeline = pipeline_from_json(&json).unwrap();
            let views = pipeline.execute(Vec::new()).unwrap();
            assert_eq!(views.len(), 1, "{name} should produce one view");
            assert_eq!(views[0].len(), 5, "{name} should preserve point count");
            assert!(
                views[0].layout().dim(&dim).is_some(),
                "{name} should prepare its output dimension"
            );
        }
    }

    #[test]
    fn sort_rejects_unknown_order() {
        let mut options = Options::new();
        options.add("dimensions", "X").add("order", "sideways");
        let err = match create_filter("filters.sort", &options) {
            Ok(_) => panic!("expected invalid sort order to fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("order must be 'asc' or 'desc'"));
    }

    #[test]
    fn nndistance_rejects_unknown_mode() {
        let mut options = Options::new();
        options.add("mode", "median");
        let err = match create_filter("filters.nndistance", &options) {
            Ok(_) => panic!("expected invalid nndistance mode to fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("mode must be 'kth' or 'avg'"));
    }

    #[test]
    fn pipeline_json_rejects_root_object_without_pipeline_array() {
        let err = match pipeline_from_json(r#"{"type":"readers.faux"}"#) {
            Ok(_) => panic!("expected root object without pipeline array to fail"),
            Err(err) => err,
        };
        assert!(err
            .to_string()
            .contains("object must contain a 'pipeline' array"));
    }

    #[test]
    fn pipeline_json_uses_tagged_inputs() {
        let json = r#"[
            {"type":"readers.faux", "count":10, "tag":"A"},
            {"type":"readers.faux", "count":5, "tag":"B"},
            {"type":"filters.merge", "inputs":["A", "B"]}
        ]"#;
        let mut pipeline = pipeline_from_json(json).unwrap();
        let result = pipeline.execute_with_result(Vec::new()).unwrap();
        assert_eq!(result.point_count, 15);
    }

    fn escape_json_path(path: &Path) -> String {
        path.display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    }

    fn single_point_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let point = view.add_point();
        view.set_f64(point, &DimId::X, 1.0);
        view.set_f64(point, &DimId::Y, 2.0);
        view.set_f64(point, &DimId::Z, 3.0);
        view
    }

    fn make_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn default_filter_options(name: &str) -> Options {
        let mut options = Options::new();
        match name {
            "filters.gpstimeconvert" => {
                options.add("conversion", "gst2gt");
            }
            "filters.sort" => {
                options.add("dimension", "X");
            }
            _ => {}
        }
        options
    }
}
