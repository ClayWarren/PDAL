//! Stage registry -- construct implemented stages from PDAL driver names.
//!
//! This is the name-keyed slice of PDAL's `StageFactory`, restricted to the
//! reader/filter/writer drivers this Rust spike currently implements.

use crate::error::{clear_last_error, set_last_error};
use crate::io_abi::{ReaderHandle, WriterHandle};
use crate::pipeline_abi::PipelineHandle;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline, Reader, Writer};
use pdal_core::stage::StageError;
use pdal_filters::decimation::DecimationFilter;
use pdal_filters::head::HeadFilter;
use pdal_filters::locate::LocateFilter;
use pdal_filters::merge::MergeFilter;
use pdal_filters::mortonorder::MortonOrderFilter;
use pdal_filters::randomize::RandomizeFilter;
use pdal_filters::sample::SampleFilter;
use pdal_filters::tail::TailFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const READER_DRIVERS: &[&str] = &[
    "readers.faux",
    "readers.text",
    "readers.pcd",
    "readers.pts",
    "readers.ptx",
    "readers.ilvis2",
    "readers.ply",
];

pub const FILTER_DRIVERS: &[&str] = &[
    "filters.decimation",
    "filters.head",
    "filters.locate",
    "filters.merge",
    "filters.mortonorder",
    "filters.randomize",
    "filters.sample",
    "filters.tail",
    "filters.voxeldownsize",
];

pub const WRITER_DRIVERS: &[&str] = &["writers.null", "writers.text", "writers.pcd", "writers.ply"];

pub enum CreatedStage {
    Reader(Box<dyn Reader>),
    Filter(Box<dyn pdal_core::pipeline::StageWrapper>),
    Writer(Box<dyn Writer>),
}

impl CreatedStage {
    #[cfg(test)]
    fn kind_name(&self) -> &'static str {
        match self {
            CreatedStage::Reader(_) => "reader",
            CreatedStage::Filter(_) => "filter",
            CreatedStage::Writer(_) => "writer",
        }
    }
}

pub fn create_reader(name: &str, options: &Options) -> Result<Box<dyn Reader>, StageError> {
    match name {
        "readers.faux" => Ok(Box::new(pdal_io::faux::FauxReader::new(options))),
        "readers.text" => Ok(Box::new(pdal_io::text::TextReader::new(options))),
        "readers.pcd" => Ok(Box::new(pdal_io::pcd::PcdReader::new(options))),
        "readers.pts" => Ok(Box::new(pdal_io::pts::PtsReader::new(options))),
        "readers.ptx" => Ok(Box::new(pdal_io::ptx::PtxReader::new(options))),
        "readers.ilvis2" => Ok(Box::new(pdal_io::ilvis2::Ilvis2Reader::new(options))),
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
        "filters.decimation" => Ok(Box::new(FilterWrapper::new(DecimationFilter::new(options)))),
        "filters.head" => Ok(Box::new(FilterWrapper::new(HeadFilter::new(
            options.get_u64("count", 10),
            options.get_bool("invert", false),
        )))),
        "filters.locate" => Ok(Box::new(FilterWrapper::new(LocateFilter::new(
            options.get_str("dimension", ""),
            options.get_str("minmax", "max"),
        )))),
        "filters.merge" => Ok(Box::new(FilterWrapper::new(MergeFilter::new()))),
        "filters.mortonorder" => Ok(Box::new(FilterWrapper::new(MortonOrderFilter::new(
            options.get_bool("reverse", false),
        )))),
        "filters.randomize" => {
            let seed = options
                .has("seed")
                .then(|| options.get_u64("seed", 0) as u32);
            Ok(Box::new(FilterWrapper::new(RandomizeFilter::new(seed))))
        }
        "filters.sample" => Ok(Box::new(FilterWrapper::new(SampleFilter::new(options)))),
        "filters.tail" => Ok(Box::new(FilterWrapper::new(TailFilter::new(
            options.get_u64("count", 10),
            options.get_bool("invert", false),
        )))),
        "filters.voxeldownsize" => Ok(Box::new(FilterWrapper::new(VoxelDownsizeFilter::new(
            options,
        )))),
        _ => Err(StageError(format!(
            "Filter driver '{name}' is not available in the Rust port registry."
        ))),
    }
}

pub fn create_writer(name: &str, options: &Options) -> Result<Box<dyn Writer>, StageError> {
    match name {
        "writers.null" => Ok(Box::new(pdal_io::nullwriter::NullWriter::new(options))),
        "writers.text" => Ok(Box::new(pdal_io::text_writer::TextWriter::new(options))),
        "writers.pcd" => Ok(Box::new(pdal_io::pcd::PcdWriter::new(options))),
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
    let stages = value.as_array().ok_or_else(|| {
        StageError("Pipeline JSON must be an array of stage objects.".to_string())
    })?;

    let mut pipeline = Pipeline::new();
    let mut tags = HashMap::new();
    let mut previous: Option<usize> = None;

    for (position, stage_value) in stages.iter().enumerate() {
        let object = stage_value.as_object().ok_or_else(|| {
            StageError(format!("Pipeline stage {position} must be a JSON object."))
        })?;
        let options = options_from_object(object)?;
        let name = stage_name(object, position, stages.len(), &options)?;
        let stage = create_stage(&name, &options)?;
        let idx = match stage {
            CreatedStage::Reader(reader) => pipeline.add_reader(&name, reader, options),
            CreatedStage::Filter(filter) => pipeline.add_stage(&name, filter, options),
            CreatedStage::Writer(writer) => pipeline.add_writer(&name, writer, options),
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

/// Create a reader stage by its PDAL driver name.
///
/// Returns null and sets the last error if `name` is null or the reader is not
/// implemented by the Rust port.
///
/// # Safety
///
/// `name` must be a valid NUL-terminated C string. `ops` may be null (treated
/// as empty options) or a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_create_reader(
    name: *const c_char,
    ops: *const Options,
) -> *mut ReaderHandle {
    clear_last_error();
    if name.is_null() {
        set_last_error("null reader driver name");
        return std::ptr::null_mut();
    }
    let name = CStr::from_ptr(name).to_string_lossy().into_owned();
    let options = ops.as_ref().cloned().unwrap_or_default();
    match create_reader(&name, &options) {
        Ok(reader) => Box::into_raw(Box::new(ReaderHandle { reader })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a writer stage by its PDAL driver name.
///
/// Returns null and sets the last error if `name` is null or the writer is not
/// implemented by the Rust port.
///
/// # Safety
///
/// `name` must be a valid NUL-terminated C string. `ops` may be null (treated
/// as empty options) or a valid pointer returned by `pdal_options_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_create_writer(
    name: *const c_char,
    ops: *const Options,
) -> *mut WriterHandle {
    clear_last_error();
    if name.is_null() {
        set_last_error("null writer driver name");
        return std::ptr::null_mut();
    }
    let name = CStr::from_ptr(name).to_string_lossy().into_owned();
    let options = ops.as_ref().cloned().unwrap_or_default();
    match create_writer(&name, &options) {
        Ok(writer) => Box::into_raw(Box::new(WriterHandle { writer })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Build a Rust pipeline from a PDAL-style pipeline JSON array.
///
/// The supported subset is intentionally narrow: stage objects with a `type`
/// field or first/last-stage filename inference, scalar options, and optional
/// `tag` / `inputs` dependencies.
///
/// # Safety
///
/// `json` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_create_json(json: *const c_char) -> *mut PipelineHandle {
    clear_last_error();
    if json.is_null() {
        set_last_error("null pipeline JSON");
        return std::ptr::null_mut();
    }
    let json = CStr::from_ptr(json).to_string_lossy();
    match pipeline_from_json(&json) {
        Ok(pipeline) => Box::into_raw(Box::new(PipelineHandle { pipeline })),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::pipeline::Reader;
    use pdal_core::point::DimId;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn data_path(rel: &str) -> String {
        format!("{}/../../test/data/{rel}", env!("CARGO_MANIFEST_DIR"))
    }

    fn temp_path(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-registry-{}-{name}", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn every_listed_reader_driver_constructs() {
        let options = Options::new();
        for name in READER_DRIVERS {
            assert!(create_reader(name, &options).is_ok(), "{name}");
        }
    }

    #[test]
    fn every_listed_filter_driver_constructs() {
        let options = Options::new();
        for name in FILTER_DRIVERS {
            assert!(create_filter(name, &options).is_ok(), "{name}");
        }
    }

    #[test]
    fn every_listed_writer_driver_constructs() {
        let options = Options::new();
        for name in WRITER_DRIVERS {
            assert!(create_writer(name, &options).is_ok(), "{name}");
        }
    }

    #[test]
    fn unknown_and_unported_drivers_are_rejected() {
        let options = Options::new();
        assert!(create_reader("readers.bogus", &options).is_err());
        assert!(create_reader("readers.las", &options).is_err());
        assert!(create_filter("filters.bogus", &options).is_err());
        assert!(create_writer("writers.bogus", &options).is_err());
    }

    #[test]
    fn unified_stage_factory_dispatches_by_prefix() {
        let options = Options::new();
        assert_eq!(
            create_stage("readers.ply", &options).unwrap().kind_name(),
            "reader"
        );
        assert_eq!(
            create_stage("filters.decimation", &options)
                .unwrap()
                .kind_name(),
            "filter"
        );
        assert_eq!(
            create_stage("writers.ply", &options).unwrap().kind_name(),
            "writer"
        );
    }

    #[test]
    fn registry_created_reader_reads_a_fixture() {
        let mut options = Options::new();
        options.add("filename", data_path("ply/simple_text.ply"));
        let mut reader = create_reader("readers.ply", &options).unwrap();
        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 3);
    }

    #[test]
    fn pipeline_json_runs_reader_filter_writer_with_inferred_drivers() {
        let input = data_path("ply/simple_text.ply");
        let output = temp_path("out.pcd");
        let json = format!(
            r#"[
  {{"filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"filename":"{}","order":"X,Y,Z","precision":6}}
]"#,
            escape_json_path(Path::new(&input)),
            escape_json_path(&output)
        );
        let mut pipeline = pipeline_from_json(&json).unwrap();
        assert!(pipeline.execute(Vec::new()).unwrap().is_empty());

        let mut options = Options::new();
        options.add("filename", output.display());
        let mut reader = pdal_io::pcd::PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();
        assert_eq!(view.len(), 2);
        assert_eq!(view.get_f64(0, &DimId::X), -1.0);
        assert_eq!(view.get_f64(1, &DimId::X), 1.0);
    }

    #[test]
    fn pipeline_json_uses_tagged_inputs() {
        let input = data_path("ply/simple_text.ply");
        let output = temp_path("tagged.pcd");
        let json = format!(
            r#"[
  {{"type":"readers.ply","filename":"{}","tag":"source"}},
  {{"type":"filters.head","count":1,"inputs":"source","tag":"first"}},
  {{"type":"writers.pcd","filename":"{}","order":"X,Y,Z","inputs":["first"]}}
]"#,
            escape_json_path(Path::new(&input)),
            escape_json_path(&output)
        );
        let mut pipeline = pipeline_from_json(&json).unwrap();
        assert!(pipeline.execute(Vec::new()).unwrap().is_empty());

        let mut options = Options::new();
        options.add("filename", output.display());
        let mut reader = pdal_io::pcd::PcdReader::new(&options);
        assert_eq!(reader.read().unwrap().pop().unwrap().len(), 1);
    }

    fn escape_json_path(path: &Path) -> String {
        path.display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    }
}
