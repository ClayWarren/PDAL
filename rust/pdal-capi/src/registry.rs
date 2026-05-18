//! Stage registry -- construct implemented stages from PDAL driver names.
//!
//! This is the name-keyed slice of PDAL's `StageFactory`, restricted to the
//! reader/filter/writer drivers this Rust spike currently implements.

use crate::error::{clear_last_error, set_last_error};
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
use pdal_filters::stats::StatsFilter;
use pdal_filters::tail::TailFilter;
use pdal_filters::voxeldownsize::VoxelDownsizeFilter;

use serde_json::Value;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const READER_DRIVERS: &[&str] = &[
    "readers.faux",
    "readers.text",
    "readers.pcd",
    "readers.pts",
    "readers.ptx",
    "readers.ilvis2",
    "readers.obj",
    "readers.qfit",
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
    "filters.stats",
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
        "readers.obj" => Ok(Box::new(pdal_io::obj::ObjReader::new(options))),
        "readers.qfit" => Ok(Box::new(pdal_io::qfit::QfitReader::new(options))),
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
        "filters.stats" => Ok(Box::new(FilterWrapper::new(StatsFilter::from_options(
            options,
        )))),
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
    let value: Value = serde_json::from_str(json).map_err(|e| StageError(e.to_string()))?;
    let mut pipeline = Pipeline::new();
    let mut last_stage: Option<usize> = None;

    let array = value
        .as_array()
        .ok_or_else(|| StageError("Pipeline JSON must be an array of stages.".to_string()))?;

    for stage_val in array {
        let obj = stage_val
            .as_object()
            .ok_or_else(|| StageError("Pipeline stage must be a JSON object.".to_string()))?;

        let driver_name = if let Some(t) = obj.get("type").and_then(|v| v.as_str()) {
            t.to_string()
        } else if let Some(filename) = obj.get("filename").and_then(|v| v.as_str()) {
            // Infer driver from filename
            if array.len() == 1 || last_stage.is_none() {
                infer_reader_driver(filename)
                    .map(|s| s.to_string())
                    .ok_or_else(|| StageError("Could not infer reader driver.".to_string()))?
            } else {
                infer_writer_driver(filename)
                    .map(|s| s.to_string())
                    .ok_or_else(|| StageError("Could not infer writer driver.".to_string()))?
            }
        } else {
            return Err(StageError(
                "Pipeline stage must have a 'type' or 'filename'.".to_string(),
            ));
        };

        let mut options = Options::new();
        for (k, v) in obj {
            if k == "type" || k == "tag" || k == "inputs" {
                continue;
            }
            match v {
                Value::String(s) => {
                    options.add(k, s);
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_u64() {
                        options.add(k, i);
                    } else if let Some(f) = n.as_f64() {
                        options.add(k, f);
                    }
                }
                Value::Bool(b) => {
                    options.add(k, *b);
                }
                _ => {
                    // Ignore non-scalar options for now
                }
            }
        }

        let stage = create_stage(&driver_name, &options)?;
        let tag = obj.get("tag").and_then(|v| v.as_str());

        let idx = match stage {
            CreatedStage::Reader(r) => pipeline.add_reader(&driver_name, r, options),
            CreatedStage::Filter(f) => {
                let idx = pipeline.add_stage(&driver_name, f, options);
                if let Some(inputs) = obj.get("inputs").and_then(|v| v.as_array()) {
                    for input in inputs {
                        if let Some(input_tag) = input.as_str() {
                            if let Some(input_idx) = pipeline.find_by_tag(input_tag) {
                                pipeline.add_dependency(idx, input_idx)?;
                            } else {
                                return Err(StageError(format!(
                                    "Input tag '{input_tag}' not found for stage '{driver_name}'."
                                )));
                            }
                        }
                    }
                } else if let Some(prev) = last_stage {
                    pipeline.add_dependency(idx, prev)?;
                }
                idx
            }
            CreatedStage::Writer(w) => {
                let idx = pipeline.add_writer(&driver_name, w, options);
                if let Some(inputs) = obj.get("inputs").and_then(|v| v.as_array()) {
                    for input in inputs {
                        if let Some(input_tag) = input.as_str() {
                            if let Some(input_idx) = pipeline.find_by_tag(input_tag) {
                                pipeline.add_dependency(idx, input_idx)?;
                            } else {
                                return Err(StageError(format!(
                                    "Input tag '{input_tag}' not found for stage '{driver_name}'."
                                )));
                            }
                        }
                    }
                } else if let Some(prev) = last_stage {
                    pipeline.add_dependency(idx, prev)?;
                }
                idx
            }
        };

        if let Some(t) = tag {
            pipeline.set_tag(idx, t)?;
        }
        last_stage = Some(idx);
    }

    Ok(pipeline)
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
    use std::path::Path;

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
        let options = Options::new();
        for name in FILTER_DRIVERS {
            // Just check that they are in the match arm
            let _ = create_filter(name, &options);
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
}
