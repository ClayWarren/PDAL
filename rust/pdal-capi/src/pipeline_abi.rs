use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use crate::io_abi::{ReaderHandle, WriterHandle};
use crate::point_abi::{pdal_bounds2d_t, pdal_bounds3d_t};
use crate::stage_abi::StageWrapper;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::{generate_stage_tag, Pipeline, StageWrapper as PipelineStageWrapper};
use pdal_core::point::{DimensionSummary, PointView};
use serde_json::json;
use std::ffi::CStr;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Pipeline C ABI
// ---------------------------------------------------------------------------

/// Opaque pipeline handle.
pub struct PipelineHandle {
    pub(crate) pipeline: Pipeline,
}

unsafe fn nullable_cstr_to_string(value: *const c_char) -> String {
    if value.is_null() {
        String::new()
    } else {
        CStr::from_ptr(value).to_string_lossy().into_owned()
    }
}

/// Take ownership of an optional point-view handle for pipeline execution.
///
/// Every non-null handle passed here is consumed. Keeping this conversion in
/// one place makes the ownership transfer visible in each public execution
/// entry point.
unsafe fn take_owned_input_view(owned_input_view: *mut PointView) -> Vec<PointView> {
    if owned_input_view.is_null() {
        Vec::new()
    } else {
        vec![*Box::from_raw(owned_input_view)]
    }
}

#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_pipeline_generate_stage_tag(
    stage_name: *const c_char,
    explicit_tag: *const c_char,
    existing_tags: *const *const c_char,
    existing_count: u64,
) -> *mut c_char {
    let stage_name = nullable_cstr_to_string(stage_name);
    let explicit_tag = nullable_cstr_to_string(explicit_tag);
    if existing_tags.is_null() && existing_count > 0 {
        return std::ptr::null_mut();
    }
    let mut existing = Vec::with_capacity(existing_count as usize);
    for idx in 0..existing_count as usize {
        let ptr = *existing_tags.add(idx);
        if !ptr.is_null() {
            existing.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }
    let existing_refs = existing.iter().map(String::as_str).collect::<Vec<_>>();
    string_to_c_ptr(generate_stage_tag(
        &stage_name,
        &explicit_tag,
        &existing_refs,
    ))
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_pipeline_result_t {
    pub point_count: u64,
    pub view_count: u64,
    pub has_bounds_2d: bool,
    pub bounds_2d: pdal_bounds2d_t,
    pub has_bounds_3d: bool,
    pub bounds_3d: pdal_bounds3d_t,
}

/// Create a new empty pipeline.
///
/// Returns a handle that must be freed with `pdal_pipeline_destroy`.
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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

/// Replace a stage while preserving the pipeline graph edges.
///
/// Returns 0 on success, -1 on error. The replacement stage is consumed.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `stage` must be a valid pointer returned by a `pdal_stage_create_*` function.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_pipeline_replace_stage(
    pipeline: *mut PipelineHandle,
    idx: u64,
    stage: *mut StageWrapper,
) -> i64 {
    if pipeline.is_null() || stage.is_null() {
        return -1;
    }
    let p = &mut (*pipeline).pipeline;
    let stage_wrapper = Box::from_raw(stage);
    let name = stage_wrapper.name().to_string();
    match p.replace_stage(idx as usize, &name, stage_wrapper, Options::new()) {
        Ok(()) => 0,
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Return the number of direct inputs to a stage, or -1 on error.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_pipeline_input_count(
    pipeline: *const PipelineHandle,
    idx: u64,
) -> i64 {
    if pipeline.is_null() {
        return -1;
    }
    match (*pipeline).pipeline.input_count(idx as usize) {
        Ok(count) => count as i64,
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Return one direct input stage index, or -1 on error.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_pipeline_input(
    pipeline: *const PipelineHandle,
    idx: u64,
    input_idx: u64,
) -> i64 {
    if pipeline.is_null() {
        return -1;
    }
    match (*pipeline).pipeline.input(idx as usize, input_idx as usize) {
        Ok(input) => input as i64,
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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
/// A non-null `owned_input_view` is consumed once `pipeline` is validated,
/// whether execution succeeds or fails, and must not be used or freed by the
/// caller afterward.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `owned_input_view` must be null or a valid point view handle.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_pipeline_execute(
    pipeline: *mut PipelineHandle,
    owned_input_view: *mut PointView,
) -> *mut PointView {
    if pipeline.is_null() {
        return std::ptr::null_mut();
    }
    clear_last_error();
    let p = &mut (*pipeline).pipeline;

    let input_views = take_owned_input_view(owned_input_view);

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
/// A non-null `owned_input_view` is consumed once `pipeline` is validated,
/// whether execution succeeds or fails, and must not be used or freed by the
/// caller afterward.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `owned_input_view` must be null or a valid point view handle.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_pipeline_execute_count(
    pipeline: *mut PipelineHandle,
    owned_input_view: *mut PointView,
) -> i64 {
    if pipeline.is_null() {
        return -1;
    }
    clear_last_error();
    let p = &mut (*pipeline).pipeline;

    let input_views = take_owned_input_view(owned_input_view);

    match p.execute_with_result(input_views) {
        Ok(result) => result.point_count as i64,
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Try to execute the pipeline in chunked streaming mode (bounded peak memory).
///
/// Returns the streamed point count (`>= 0`) on success, `-1` on error (message
/// available via the last-error API), or `-2` when the pipeline is not
/// streaming-eligible -- in which case the caller should fall back to
/// `pdal_pipeline_execute`/`_count`/`_result`. Streaming is reader-led, so there
/// is no input-view parameter.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_pipeline_execute_streaming(pipeline: *mut PipelineHandle) -> i64 {
    if pipeline.is_null() {
        return -1;
    }
    clear_last_error();
    let p = &mut (*pipeline).pipeline;
    match p.execute_streaming() {
        Ok(Some(count)) => count as i64,
        Ok(None) => -2,
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Return whether the pipeline is eligible for chunked streaming execution.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_pipeline_streamable(pipeline: *const PipelineHandle) -> bool {
    if pipeline.is_null() {
        return false;
    }
    (*pipeline).pipeline.streamable()
}

/// Execute the pipeline and return a summary result.
///
/// Returns 0 on success, -1 on error.
/// A non-null `owned_input_view` is consumed after `pipeline` and `out_result`
/// are validated, whether execution succeeds or fails, and must not be used or
/// freed by the caller afterward.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `owned_input_view` must be null or a valid point view handle.
/// `out_result` must point to writable memory.
#[pdal_capi_macros::ffi_export(fallback = -1)]
pub unsafe extern "C" fn pdal_pipeline_execute_result(
    pipeline: *mut PipelineHandle,
    owned_input_view: *mut PointView,
    out_result: *mut pdal_pipeline_result_t,
) -> i64 {
    if pipeline.is_null() || out_result.is_null() {
        return -1;
    }
    clear_last_error();
    let p = &mut (*pipeline).pipeline;

    let input_views = take_owned_input_view(owned_input_view);

    match p.execute_with_result(input_views) {
        Ok(result) => {
            *out_result = pipeline_result_to_abi(result);
            0
        }
        Err(e) => {
            set_last_error(&e.0);
            -1
        }
    }
}

/// Execute the pipeline and return a summary result as JSON.
///
/// The returned string must be freed with `pdal_string_free`.
/// A non-null `owned_input_view` is consumed once `pipeline` is validated,
/// whether execution succeeds or fails, and must not be used or freed by the
/// caller afterward.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
/// `owned_input_view` must be null or a valid point view handle.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_pipeline_execute_summary_json(
    pipeline: *mut PipelineHandle,
    owned_input_view: *mut PointView,
) -> *mut c_char {
    if pipeline.is_null() {
        return std::ptr::null_mut();
    }
    clear_last_error();
    let p = &mut (*pipeline).pipeline;

    let input_views = take_owned_input_view(owned_input_view);

    match p.execute_with_result(input_views) {
        Ok(result) => string_to_c_ptr(
            serde_json::to_string(&pipeline_result_to_json(result, Some(p.metadata())))
                .unwrap_or_else(|_| "null".to_string()),
        ),
        Err(e) => {
            set_last_error(&e.0);
            std::ptr::null_mut()
        }
    }
}

/// Return the number of stages in the pipeline.
///
/// # Safety
/// `pipeline` must be a valid pipeline handle.
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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

fn pipeline_result_to_abi(result: pdal_core::pipeline::ExecResult) -> pdal_pipeline_result_t {
    let (has_bounds_2d, bounds_2d) = match result.bounds_2d {
        Some(bounds) => (
            true,
            pdal_bounds2d_t {
                minx: bounds.minx,
                maxx: bounds.maxx,
                miny: bounds.miny,
                maxy: bounds.maxy,
            },
        ),
        None => (false, zero_bounds_2d()),
    };
    let (has_bounds_3d, bounds_3d) = match result.bounds_3d {
        Some(bounds) => (
            true,
            pdal_bounds3d_t {
                minx: bounds.minx,
                maxx: bounds.maxx,
                miny: bounds.miny,
                maxy: bounds.maxy,
                minz: bounds.minz,
                maxz: bounds.maxz,
            },
        ),
        None => (false, zero_bounds_3d()),
    };

    pdal_pipeline_result_t {
        point_count: result.point_count,
        view_count: result.view_count as u64,
        has_bounds_2d,
        bounds_2d,
        has_bounds_3d,
        bounds_3d,
    }
}

fn pipeline_result_to_json(
    result: pdal_core::pipeline::ExecResult,
    metadata: Option<MetadataNode>,
) -> serde_json::Value {
    json!({
        "point_count": result.point_count,
        "view_count": result.view_count,
        "bounds_2d": result.bounds_2d.map(|bounds| {
            json!({
                "minx": bounds.minx,
                "maxx": bounds.maxx,
                "miny": bounds.miny,
                "maxy": bounds.maxy,
            })
        }),
        "bounds_3d": result.bounds_3d.map(|bounds| {
            json!({
                "minx": bounds.minx,
                "maxx": bounds.maxx,
                "miny": bounds.miny,
                "maxy": bounds.maxy,
                "minz": bounds.minz,
                "maxz": bounds.maxz,
            })
        }),
        "dimension_summaries": result
            .dimension_summaries
            .iter()
            .map(dimension_summary_json)
            .collect::<Vec<_>>(),
        "metadata": metadata.as_ref().map(pdal_core::metadata::metadata_node_to_json_flat),
    })
}

pub(crate) fn pipeline_result_to_json_for_kernel(
    result: pdal_core::pipeline::ExecResult,
    pipeline: &PipelineHandle,
) -> String {
    pipeline_result_to_json(result, Some(pipeline.pipeline.metadata())).to_string()
}

fn dimension_summary_json(summary: &DimensionSummary) -> serde_json::Value {
    json!({
        "name": summary.name,
        "count": summary.count,
        "minimum": summary.minimum,
        "maximum": summary.maximum,
        "mean": summary.mean,
    })
}

fn zero_bounds_2d() -> pdal_bounds2d_t {
    pdal_bounds2d_t {
        minx: 0.0,
        maxx: 0.0,
        miny: 0.0,
        maxy: 0.0,
    }
}

fn zero_bounds_3d() -> pdal_bounds3d_t {
    pdal_bounds3d_t {
        minx: 0.0,
        maxx: 0.0,
        miny: 0.0,
        maxy: 0.0,
        minz: 0.0,
        maxz: 0.0,
    }
}
