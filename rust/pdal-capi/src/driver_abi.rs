use crate::error::string_to_c_ptr;
use crate::registry::{FILTER_DRIVERS, READER_DRIVERS, WRITER_DRIVERS};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::raw::c_char;

pub struct StageExtensionsHandle {
    readers: BTreeMap<String, String>,
    writers: BTreeMap<String, String>,
}

#[no_mangle]
pub unsafe extern "C" fn pdal_infer_reader_driver(filename: *const c_char) -> *mut c_char {
    if filename.is_null() {
        return string_to_c_ptr(String::new());
    }
    let filename = CStr::from_ptr(filename).to_string_lossy();
    string_to_c_ptr(infer_reader_driver(&filename).unwrap_or("").to_string())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_infer_writer_driver(filename: *const c_char) -> *mut c_char {
    if filename.is_null() {
        return string_to_c_ptr(String::new());
    }
    let filename = CStr::from_ptr(filename).to_string_lossy();
    string_to_c_ptr(infer_writer_driver(&filename).unwrap_or("").to_string())
}

#[no_mangle]
pub extern "C" fn pdal_rust_stage_list_json() -> *mut c_char {
    let stages = READER_DRIVERS
        .iter()
        .chain(FILTER_DRIVERS.iter())
        .chain(WRITER_DRIVERS.iter())
        .copied()
        .collect::<Vec<_>>();
    string_to_c_ptr(serde_json::to_string(&stages).unwrap_or_else(|_| "[]".to_string()))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_stage_extensions_create() -> *mut StageExtensionsHandle {
    Box::into_raw(Box::new(StageExtensionsHandle {
        readers: BTreeMap::new(),
        writers: BTreeMap::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_stage_extensions_set(
    extensions: *mut StageExtensionsHandle,
    stage: *const c_char,
    values: *const *const c_char,
    value_count: u64,
) {
    let Some(extensions) = extensions.as_mut() else {
        return;
    };
    if stage.is_null() {
        return;
    }
    let stage = CStr::from_ptr(stage).to_string_lossy().to_string();
    if !stage.starts_with("readers.") && !stage.starts_with("writers.") {
        return;
    }
    if values.is_null() && value_count > 0 {
        return;
    }
    for idx in 0..value_count {
        let value = *values.add(idx as usize);
        if value.is_null() {
            continue;
        }
        let value = CStr::from_ptr(value).to_string_lossy().to_string();
        if stage.starts_with("readers.") {
            extensions.readers.insert(value, stage.clone());
        } else {
            extensions.writers.insert(value, stage.clone());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_stage_extensions_default_reader(
    extensions: *const StageExtensionsHandle,
    extension: *const c_char,
) -> *mut c_char {
    stage_extension_default(extensions, extension, true)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_stage_extensions_default_writer(
    extensions: *const StageExtensionsHandle,
    extension: *const c_char,
) -> *mut c_char {
    stage_extension_default(extensions, extension, false)
}

unsafe fn stage_extension_default(
    extensions: *const StageExtensionsHandle,
    extension: *const c_char,
    reader: bool,
) -> *mut c_char {
    if extension.is_null() {
        return string_to_c_ptr(String::new());
    }
    let extension = CStr::from_ptr(extension).to_string_lossy();
    if let Some(extensions) = extensions.as_ref() {
        let custom = if reader {
            extensions.readers.get(extension.as_ref())
        } else {
            extensions.writers.get(extension.as_ref())
        };
        if let Some(stage) = custom {
            return string_to_c_ptr(stage.clone());
        }
    }

    let inferred = if reader {
        infer_reader_driver(&format!("stage.{extension}"))
    } else {
        infer_writer_driver(&format!("stage.{extension}"))
    };
    string_to_c_ptr(inferred.unwrap_or("").to_string())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_stage_extensions_destroy(extensions: *mut StageExtensionsHandle) {
    if !extensions.is_null() {
        drop(Box::from_raw(extensions));
    }
}
