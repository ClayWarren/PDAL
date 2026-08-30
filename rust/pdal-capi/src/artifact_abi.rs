use crate::error::{set_last_error, string_to_c_ptr};
use std::collections::BTreeMap;
use std::ffi::{c_char, CStr};

pub struct ArtifactManagerHandle {
    storage: BTreeMap<String, ArtifactValue>,
}

#[derive(Clone)]
struct ArtifactValue {
    type_name: String,
    value: String,
}

#[pdal_capi_macros::ffi_export]
pub extern "C" fn pdal_artifact_manager_create() -> *mut ArtifactManagerHandle {
    Box::into_raw(Box::new(ArtifactManagerHandle {
        storage: BTreeMap::new(),
    }))
}

/// # Safety
/// `manager` must be null or a pointer returned by
/// `pdal_artifact_manager_create`, and must not be destroyed twice.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_destroy(manager: *mut ArtifactManagerHandle) {
    if !manager.is_null() {
        drop(Box::from_raw(manager));
    }
}

/// # Safety
/// `manager` must be valid. `name`, `type_name`, and `value` must be valid
/// NUL-terminated strings.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_put(
    manager: *mut ArtifactManagerHandle,
    name: *const c_char,
    type_name: *const c_char,
    value: *const c_char,
) -> bool {
    let Some(manager) = manager.as_mut() else {
        set_last_error("null artifact manager");
        return false;
    };
    let Some((name, artifact)) = artifact_parts(name, type_name, value) else {
        return false;
    };
    manager.storage.insert(name, artifact).is_none()
}

/// # Safety
/// `manager` must be valid. `name` and `type_name` must be valid
/// NUL-terminated strings. Returned string must be freed with
/// `pdal_string_free`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_get(
    manager: *const ArtifactManagerHandle,
    name: *const c_char,
    type_name: *const c_char,
) -> *mut c_char {
    let Some(manager) = manager.as_ref() else {
        set_last_error("null artifact manager");
        return std::ptr::null_mut();
    };
    let Some(name) = cstr(name) else {
        return std::ptr::null_mut();
    };
    let Some(type_name) = cstr(type_name) else {
        return std::ptr::null_mut();
    };
    match manager.storage.get(&name) {
        Some(artifact) if artifact.type_name == type_name => {
            string_to_c_ptr(artifact.value.clone())
        }
        _ => std::ptr::null_mut(),
    }
}

/// # Safety
/// `manager` must be valid. `name`, `type_name`, and `value` must be valid
/// NUL-terminated strings.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_replace(
    manager: *mut ArtifactManagerHandle,
    name: *const c_char,
    type_name: *const c_char,
    value: *const c_char,
) -> bool {
    let Some(manager) = manager.as_mut() else {
        set_last_error("null artifact manager");
        return false;
    };
    let Some((name, artifact)) = artifact_parts(name, type_name, value) else {
        return false;
    };
    let Some(existing) = manager.storage.get_mut(&name) else {
        return false;
    };
    if existing.type_name != artifact.type_name {
        return false;
    }
    *existing = artifact;
    true
}

/// # Safety
/// `manager` must be valid. `name`, `type_name`, and `value` must be valid
/// NUL-terminated strings.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_replace_or_put(
    manager: *mut ArtifactManagerHandle,
    name: *const c_char,
    type_name: *const c_char,
    value: *const c_char,
) -> bool {
    if pdal_artifact_manager_replace(manager, name, type_name, value) {
        return true;
    }
    pdal_artifact_manager_put(manager, name, type_name, value)
}

/// # Safety
/// `manager` must be valid and `name` must be a valid NUL-terminated string.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_erase(
    manager: *mut ArtifactManagerHandle,
    name: *const c_char,
) -> bool {
    let Some(manager) = manager.as_mut() else {
        set_last_error("null artifact manager");
        return false;
    };
    let Some(name) = cstr(name) else {
        return false;
    };
    manager.storage.remove(&name).is_some()
}

/// # Safety
/// `manager` must be valid and `name` must be a valid NUL-terminated string.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_exists(
    manager: *const ArtifactManagerHandle,
    name: *const c_char,
) -> bool {
    let Some(manager) = manager.as_ref() else {
        set_last_error("null artifact manager");
        return false;
    };
    let Some(name) = cstr(name) else {
        return false;
    };
    manager.storage.contains_key(&name)
}

/// # Safety
/// `manager` must be valid. Returned string must be freed with
/// `pdal_string_free`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_artifact_manager_keys_json(
    manager: *const ArtifactManagerHandle,
) -> *mut c_char {
    let Some(manager) = manager.as_ref() else {
        set_last_error("null artifact manager");
        return std::ptr::null_mut();
    };
    let keys: Vec<&String> = manager.storage.keys().collect();
    match serde_json::to_string(&keys) {
        Ok(text) => string_to_c_ptr(text),
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

unsafe fn artifact_parts(
    name: *const c_char,
    type_name: *const c_char,
    value: *const c_char,
) -> Option<(String, ArtifactValue)> {
    Some((
        cstr(name)?,
        ArtifactValue {
            type_name: cstr(type_name)?,
            value: cstr(value)?,
        },
    ))
}

unsafe fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        set_last_error("null artifact string");
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn enforces_name_and_type_for_get_replace_and_keys() {
        unsafe {
            let manager = pdal_artifact_manager_create();
            let name = CString::new("MyTest").unwrap();
            let other = CString::new("MyTest2").unwrap();
            let ty = CString::new("TestArtifact").unwrap();
            let ty2 = CString::new("TestArtifact2").unwrap();
            let value = CString::new("MyTest").unwrap();
            let value2 = CString::new("MyTestA").unwrap();

            assert!(pdal_artifact_manager_put(
                manager,
                name.as_ptr(),
                ty.as_ptr(),
                value.as_ptr()
            ));
            assert!(!pdal_artifact_manager_put(
                manager,
                name.as_ptr(),
                ty.as_ptr(),
                value.as_ptr()
            ));
            assert!(!pdal_artifact_manager_replace(
                manager,
                name.as_ptr(),
                ty2.as_ptr(),
                value2.as_ptr()
            ));
            assert!(pdal_artifact_manager_replace(
                manager,
                name.as_ptr(),
                ty.as_ptr(),
                value2.as_ptr()
            ));

            let got = pdal_artifact_manager_get(manager, name.as_ptr(), ty.as_ptr());
            assert!(!got.is_null());
            crate::pdal_string_free(got);
            assert!(pdal_artifact_manager_get(manager, name.as_ptr(), ty2.as_ptr()).is_null());

            assert!(pdal_artifact_manager_put(
                manager,
                other.as_ptr(),
                ty.as_ptr(),
                value.as_ptr()
            ));
            let keys_ptr = pdal_artifact_manager_keys_json(manager);
            let keys = CStr::from_ptr(keys_ptr).to_string_lossy().into_owned();
            crate::pdal_string_free(keys_ptr);
            assert_eq!(keys, r#"["MyTest","MyTest2"]"#);

            assert!(pdal_artifact_manager_erase(manager, name.as_ptr()));
            assert!(!pdal_artifact_manager_exists(manager, name.as_ptr()));
            pdal_artifact_manager_destroy(manager);
        }
    }
}
