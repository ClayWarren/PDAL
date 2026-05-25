use crate::error::string_to_c_ptr;
use crate::registry::{FILTER_DRIVERS, READER_DRIVERS, WRITER_DRIVERS};
use pdal_core::plugin::valid_plugin_name;
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Mutex;

/// Runtime entry for one C++-side plugin (`PluginManager<T>` registration).
///
/// The `creator` value is a C function pointer (e.g. `T* (*)()`) reinterpreted
/// as `usize` so the Rust `HashMap` is `Send + Sync`. The C++ wrapper passes
/// in a stateless static thunk that calls `new C` and downcasts to `T*`; Rust
/// stores and returns it back unchanged.
#[derive(Clone)]
struct RuntimePlugin {
    creator: usize,
    description: String,
    link: String,
}

/// Keyed by `(type_namespace, plugin_name)`. The C++ wrapper passes
/// `typeid(T).name()` as the namespace so each `PluginManager<T>` template
/// instantiation has its own slice of the map.
fn registry() -> &'static Mutex<BTreeMap<(String, String), RuntimePlugin>> {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<Mutex<BTreeMap<(String, String), RuntimePlugin>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

unsafe fn cstr_or_empty(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Register a runtime stage plugin. `creator` is a C function pointer that
/// returns a `T*` (the wrapper passes it as `void*`). Re-registering with the
/// same key replaces the entry (mirrors the C++ map `insert` semantics, which
/// silently keeps the original — we replace to keep last-writer-wins
/// consistent with `loadByPath`).
///
/// # Safety
/// All pointer arguments must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pdal_runtime_plugin_register(
    type_ns: *const c_char,
    name: *const c_char,
    creator: *const std::ffi::c_void,
    description: *const c_char,
    link: *const c_char,
) {
    let ns = cstr_or_empty(type_ns);
    let name = cstr_or_empty(name);
    if ns.is_empty() || name.is_empty() {
        return;
    }
    let entry = RuntimePlugin {
        creator: creator as usize,
        description: cstr_or_empty(description),
        link: cstr_or_empty(link),
    };
    if let Ok(mut map) = registry().lock() {
        map.entry((ns, name)).or_insert(entry);
    }
}

/// Look up a plugin's creator function pointer. Returns null if not found.
///
/// # Safety
/// All pointer arguments must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pdal_runtime_plugin_lookup_creator(
    type_ns: *const c_char,
    name: *const c_char,
) -> *const std::ffi::c_void {
    let ns = cstr_or_empty(type_ns);
    let name = cstr_or_empty(name);
    if ns.is_empty() || name.is_empty() {
        return std::ptr::null();
    }
    let map = match registry().lock() {
        Ok(m) => m,
        Err(_) => return std::ptr::null(),
    };
    map.get(&(ns, name))
        .map(|p| p.creator as *const std::ffi::c_void)
        .unwrap_or(std::ptr::null())
}

/// Returns true if a plugin with the given (type_ns, name) is registered.
///
/// # Safety
/// All pointer arguments must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pdal_runtime_plugin_has(
    type_ns: *const c_char,
    name: *const c_char,
) -> bool {
    let ns = cstr_or_empty(type_ns);
    let name = cstr_or_empty(name);
    if ns.is_empty() || name.is_empty() {
        return false;
    }
    let Ok(map) = registry().lock() else {
        return false;
    };
    map.contains_key(&(ns, name))
}

/// Return the registered plugin names for a type namespace as a JSON array.
/// The caller must free with `pdal_string_free`.
///
/// # Safety
/// `type_ns` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn pdal_runtime_plugin_names_json(type_ns: *const c_char) -> *mut c_char {
    let ns = cstr_or_empty(type_ns);
    let Ok(map) = registry().lock() else {
        return string_to_c_ptr("[]".to_string());
    };
    let names: Vec<&str> = map
        .keys()
        .filter(|(n, _)| n == &ns)
        .map(|(_, name)| name.as_str())
        .collect();
    string_to_c_ptr(serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string()))
}

/// Return the registered description string for a plugin, or an empty string
/// if not found. Caller must free with `pdal_string_free`.
///
/// # Safety
/// All pointer arguments must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pdal_runtime_plugin_description(
    type_ns: *const c_char,
    name: *const c_char,
) -> *mut c_char {
    let ns = cstr_or_empty(type_ns);
    let name = cstr_or_empty(name);
    let Ok(map) = registry().lock() else {
        return string_to_c_ptr(String::new());
    };
    string_to_c_ptr(
        map.get(&(ns, name))
            .map(|p| p.description.clone())
            .unwrap_or_default(),
    )
}

/// Return the registered link string for a plugin, or an empty string if not
/// found. Caller must free with `pdal_string_free`.
///
/// # Safety
/// All pointer arguments must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pdal_runtime_plugin_link(
    type_ns: *const c_char,
    name: *const c_char,
) -> *mut c_char {
    let ns = cstr_or_empty(type_ns);
    let name = cstr_or_empty(name);
    let Ok(map) = registry().lock() else {
        return string_to_c_ptr(String::new());
    };
    string_to_c_ptr(
        map.get(&(ns, name))
            .map(|p| p.link.clone())
            .unwrap_or_default(),
    )
}

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
