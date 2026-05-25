use crate::error::string_to_c_ptr;
use crate::registry::{FILTER_DRIVERS, READER_DRIVERS, WRITER_DRIVERS};
use pdal_core::plugin::valid_plugin_name;
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn pdal_plugin_valid_name(
    path: *const c_char,
    types: *const *const c_char,
    type_count: u64,
    dynamic_lib_extension: *const c_char,
) -> *mut c_char {
    let path = if path.is_null() {
        String::new()
    } else {
        CStr::from_ptr(path).to_string_lossy().into_owned()
    };
    let extension = if dynamic_lib_extension.is_null() {
        String::new()
    } else {
        CStr::from_ptr(dynamic_lib_extension)
            .to_string_lossy()
            .into_owned()
    };

    let mut type_values = Vec::new();
    if !types.is_null() {
        for i in 0..type_count {
            let ty = *types.add(i as usize);
            if !ty.is_null() {
                type_values.push(CStr::from_ptr(ty).to_string_lossy().into_owned());
            }
        }
    }
    let type_refs = type_values.iter().map(String::as_str).collect::<Vec<_>>();
    string_to_c_ptr(valid_plugin_name(&path, &type_refs, &extension).unwrap_or_default())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_stage_registry_has(stage_name: *const c_char) -> bool {
    if stage_name.is_null() {
        return false;
    }
    let stage_name = CStr::from_ptr(stage_name).to_string_lossy();
    READER_DRIVERS.contains(&stage_name.as_ref())
        || FILTER_DRIVERS.contains(&stage_name.as_ref())
        || WRITER_DRIVERS.contains(&stage_name.as_ref())
}
