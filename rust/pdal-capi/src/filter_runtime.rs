use crate::error::{clear_last_error, ffi_catch, set_last_error};
use crate::stage_abi::StageWrapper;
use pdal_core::metadata::MetadataNode;
use pdal_core::point::{PointLayout, PointView};

/// Destroy a stage.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`, or null.
/// Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_destroy(stage: *mut StageWrapper) {
    if !stage.is_null() {
        drop(Box::from_raw(stage));
    }
}

/// Reset the streaming state of a stage.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_reset(stage: *mut StageWrapper) {
    if let Some(stage) = stage.as_mut() {
        stage.filter.reset();
    }
}

/// Process one point in streaming mode. Returns `true` to keep the point.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_process_one(stage: *mut StageWrapper) -> bool {
    ffi_catch(false, || {
        clear_last_error();
        if let Some(stage) = stage.as_mut() {
            // Counter-based filters ignore the point; pass an empty view.
            let mut empty = PointView::new(std::rc::Rc::new(PointLayout::new()));
            stage.filter.process_one(&mut empty, 0)
        } else {
            set_last_error("null stage");
            false
        }
    })
}

/// Decide whether to keep point `idx` of `view` in streaming mode.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
/// `view` must be a valid pointer returned by `pdal_point_view_create` or
/// `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_process_one_at(
    stage: *mut StageWrapper,
    view: *mut PointView,
    idx: u64,
) -> bool {
    ffi_catch(false, || {
        clear_last_error();
        if let (Some(stage), Some(view)) = (stage.as_mut(), view.as_mut()) {
            stage.filter.process_one(view, idx)
        } else {
            set_last_error("null stage or view");
            false
        }
    })
}

/// Run the filter over a complete input view. Returns a new output view
/// (caller owns it), or null on error.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
/// `input` must be a valid pointer returned by `pdal_point_view_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_run(
    stage: *mut StageWrapper,
    input: *mut PointView,
) -> *mut PointView {
    ffi_catch(std::ptr::null_mut(), || {
        clear_last_error();
        if let (Some(stage), Some(input)) = (stage.as_mut(), input.as_mut()) {
            match stage.filter.run(std::slice::from_ref(input)) {
                Ok(mut outputs) => {
                    if !outputs.is_empty() {
                        return Box::into_raw(Box::new(outputs.remove(0)));
                    }
                }
                Err(err) => set_last_error(err.to_string()),
            }
        } else {
            set_last_error("null stage or input view");
        }
        std::ptr::null_mut()
    })
}

/// Run the filter over a complete input view, returning multiple output views.
/// Returns the actual number of output views produced.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
/// `input` must be a valid pointer returned by `pdal_point_view_create`.
/// `outputs` must be a valid pointer to a buffer of size `max_outputs` pointer elements.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_run_multi(
    stage: *mut StageWrapper,
    input: *mut PointView,
    outputs: *mut *mut PointView,
    max_outputs: u64,
) -> u64 {
    ffi_catch(0, || {
        clear_last_error();
        if let (Some(stage), Some(input), false) =
            (stage.as_mut(), input.as_mut(), outputs.is_null())
        {
            match stage.filter.run(std::slice::from_ref(input)) {
                Ok(mut results) => {
                    let count = std::cmp::min(results.len() as u64, max_outputs);
                    for i in 0..count {
                        let view = results.remove(0);
                        *outputs.offset(i as isize) = Box::into_raw(Box::new(view));
                    }
                    return count;
                }
                Err(err) => set_last_error(err.to_string()),
            }
        } else {
            set_last_error("null stage, input view, or output buffer");
        }
        0
    })
}

/// Export a stage's accumulated metadata.
///
/// # Safety
///
/// `stage` must be a valid pointer returned by `pdal_stage_create_*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_metadata(stage: *const StageWrapper) -> *mut MetadataNode {
    if let Some(stage) = stage.as_ref() {
        Box::into_raw(Box::new(stage.filter.metadata()))
    } else {
        std::ptr::null_mut()
    }
}
