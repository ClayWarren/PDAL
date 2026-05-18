use crate::error::{clear_last_error, set_last_error};
use crate::io_abi::{ReaderHandle, WriterHandle};
use crate::stage_abi::StageWrapper;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, StageWrapper as PipelineStageWrapper};
use pdal_core::point::PointView;
use std::ffi::CStr;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Pipeline C ABI
// ---------------------------------------------------------------------------

/// Opaque pipeline handle.
pub struct PipelineHandle {
    pipeline: Pipeline,
}

/// Create a new empty pipeline.
///
/// Returns a handle that must be freed with `pdal_pipeline_destroy`.
#[no_mangle]
pub extern "C" fn pdal_pipeline_create() -> *mut PipelineHandle {
    Box::into_raw(Box::new(PipelineHandle {
        pipeline: Pipeline::new(),
    }))
}

/// Destroy a pipeline handle.
///
/// # Safety
/// `pipeline` must be a valid pointer returned by `pdal_pipeline_create`,
/// and must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_destroy(pipeline: *mut PipelineHandle) {
    if !pipeline.is_null() {
        drop(Box::from_raw(pipeline));
    }
}

/// Add a stage to the pipeline.
///
/// Returns the stage index, or -1 on error. The stage is consumed by the
/// pipeline and must not be used or freed separately after this call.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `stage` must be a valid pointer returned by a `pdal_stage_create_*` function.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_add_stage(
    pipeline: *mut PipelineHandle,
    stage: *mut StageWrapper,
) -> i64 {
    if pipeline.is_null() || stage.is_null() {
        return -1;
    }
    let p = &mut (*pipeline).pipeline;
    let stage_wrapper = Box::from_raw(stage);
    let name = stage_wrapper.name().to_string();
    let idx = p.add_stage(&name, stage_wrapper, Options::new());
    idx as i64
}

/// Add a stage to the pipeline with a tag.
///
/// Returns the stage index, or -1 on error. The stage is consumed by the
/// pipeline and must not be used or freed separately after this call.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `stage` must be a valid pointer returned by a `pdal_stage_create_*` function.
/// `tag` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_add_stage_tagged(
    pipeline: *mut PipelineHandle,
    stage: *mut StageWrapper,
    tag: *const c_char,
) -> i64 {
    if pipeline.is_null() || stage.is_null() || tag.is_null() {
        return -1;
    }
    let p = &mut (*pipeline).pipeline;
    let stage_wrapper = Box::from_raw(stage);
    let name = stage_wrapper.name().to_string();
    let tag_str = CStr::from_ptr(tag).to_string_lossy();
    match p.add_stage_tagged(&name, stage_wrapper, Options::new(), &tag_str) {
        Ok(idx) => idx as i64,
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Add a dependency: `target` depends on `input`.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_add_dependency(
    pipeline: *mut PipelineHandle,
    target: u64,
    input: u64,
) -> i64 {
    if pipeline.is_null() {
        return -1;
    }
    let p = &mut (*pipeline).pipeline;
    match p.add_dependency(target as usize, input as usize) {
        Ok(()) => 0,
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Add a reader to the pipeline.
///
/// Returns the stage index, or -1 on error. The reader is consumed by the
/// pipeline and must not be used or freed separately after this call.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `reader` must be a valid pointer returned by `pdal_reader_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_add_reader(
    pipeline: *mut PipelineHandle,
    reader: *mut ReaderHandle,
) -> i64 {
    if pipeline.is_null() || reader.is_null() {
        return -1;
    }
    let p = &mut (*pipeline).pipeline;
    let reader_handle = Box::from_raw(reader);
    let name = reader_handle.reader.name().to_string();
    let idx = p.add_reader(&name, reader_handle.reader, Options::new());
    idx as i64
}

/// Add a writer to the pipeline.
///
/// Returns the stage index, or -1 on error. The writer is consumed by the
/// pipeline and must not be used or freed separately after this call.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `writer` must be a valid pointer returned by `pdal_writer_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_add_writer(
    pipeline: *mut PipelineHandle,
    writer: *mut WriterHandle,
) -> i64 {
    if pipeline.is_null() || writer.is_null() {
        return -1;
    }
    let p = &mut (*pipeline).pipeline;
    let writer_handle = Box::from_raw(writer);
    let name = writer_handle.writer.name().to_string();
    let idx = p.add_writer(&name, writer_handle.writer, Options::new());
    idx as i64
}

/// Execute the pipeline with an input view.
///
/// Returns a new `PointView` containing the pipeline output, or null on error.
/// The returned view must be freed with `pdal_point_view_destroy`.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `input_view` must be a valid point view handle.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_execute(
    pipeline: *mut PipelineHandle,
    input_view: *mut PointView,
) -> *mut PointView {
    if pipeline.is_null() {
        return std::ptr::null_mut();
    }
    clear_last_error();
    let p = &mut (*pipeline).pipeline;

    let input_views = if input_view.is_null() {
        Vec::new()
    } else {
        let input = Box::from_raw(input_view);
        vec![*input]
    };

    match p.execute(input_views) {
        Ok(views) => {
            if views.is_empty() {
                std::ptr::null_mut()
            } else {
                Box::into_raw(Box::new(views.into_iter().next().unwrap()))
            }
        }
        Err(e) => {
            set_last_error(&e.0);
            std::ptr::null_mut()
        }
    }
}

/// Execute the pipeline and return the point count.
///
/// Returns the total number of points in the output, or -1 on error.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `input_view` must be a valid point view handle.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_execute_count(
    pipeline: *mut PipelineHandle,
    input_view: *mut PointView,
) -> i64 {
    if pipeline.is_null() {
        return -1;
    }
    clear_last_error();
    let p = &mut (*pipeline).pipeline;

    let input_views = if input_view.is_null() {
        Vec::new()
    } else {
        let input = Box::from_raw(input_view);
        vec![*input]
    };

    match p.execute_with_result(input_views) {
        Ok(result) => result.point_count as i64,
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Return the number of stages in the pipeline.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_stage_count(pipeline: *const PipelineHandle) -> u64 {
    if pipeline.is_null() {
        return 0;
    }
    (*pipeline).pipeline.len() as u64
}

/// Return pipeline metadata as a metadata node.
///
/// The returned node must be freed with `pdal_metadata_node_destroy`.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_metadata(
    pipeline: *const PipelineHandle,
) -> *mut MetadataNode {
    if pipeline.is_null() {
        return std::ptr::null_mut();
    }
    let meta = (*pipeline).pipeline.metadata();
    Box::into_raw(Box::new(meta))
}

/// Find a stage index by tag.
///
/// Returns the stage index, or -1 if not found.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `tag` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_find_by_tag(
    pipeline: *const PipelineHandle,
    tag: *const c_char,
) -> i64 {
    if pipeline.is_null() || tag.is_null() {
        return -1;
    }
    let tag_str = CStr::from_ptr(tag).to_string_lossy();
    match (*pipeline).pipeline.find_by_tag(&tag_str) {
        Some(idx) => idx as i64,
        None => -1,
    }
}
