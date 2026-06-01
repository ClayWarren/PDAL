use crate::error::string_to_c_ptr;
use pdal_core::metadata::{
    json_scalar_value, scalar_as_bool, scalar_as_f64, scalar_as_i64, scalar_as_u64, MetadataKind,
    MetadataNode, MetadataValue,
};
use serde_json::json;
use std::ffi::c_void;
use std::ffi::CStr;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Metadata ABI
// ---------------------------------------------------------------------------

/// Create a metadata node. Caller owns the returned pointer.
///
/// # Safety
///
/// `name` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_create(name: *const c_char) -> *mut MetadataNode {
    let name = if name.is_null() {
        String::new()
    } else {
        CStr::from_ptr(name).to_string_lossy().into_owned()
    };
    Box::into_raw(Box::new(MetadataNode::new(name)))
}

#[no_mangle]
/// Return a deep copy of a metadata node. Caller owns the returned pointer.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
pub unsafe extern "C" fn pdal_metadata_node_clone(node: *const MetadataNode) -> *mut MetadataNode {
    node.as_ref()
        .map(|node| Box::into_raw(Box::new(node.clone())))
        .unwrap_or(std::ptr::null_mut())
}

/// Return a node's name. Caller must free with `pdal_string_free`.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_name(node: *const MetadataNode) -> *mut c_char {
    string_to_c_ptr(
        node.as_ref()
            .map(|node| node.name().to_string())
            .unwrap_or_default(),
    )
}

#[no_mangle]
/// Return a node's type name. Caller must free with `pdal_string_free`.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
pub unsafe extern "C" fn pdal_metadata_node_type(node: *const MetadataNode) -> *mut c_char {
    string_to_c_ptr(
        node.as_ref()
            .and_then(MetadataNode::type_name)
            .unwrap_or_default()
            .to_string(),
    )
}

#[no_mangle]
/// Return a node's description. Caller must free with `pdal_string_free`.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
pub unsafe extern "C" fn pdal_metadata_node_description(node: *const MetadataNode) -> *mut c_char {
    string_to_c_ptr(
        node.as_ref()
            .and_then(MetadataNode::description)
            .unwrap_or_default()
            .to_string(),
    )
}

/// Set a metadata node's string value.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
/// `value` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_set_string(
    node: *mut MetadataNode,
    value: *const c_char,
) {
    if let Some(node) = node.as_mut() {
        let value = if value.is_null() {
            String::new()
        } else {
            CStr::from_ptr(value).to_string_lossy().into_owned()
        };
        node.set_value(MetadataValue::String(value));
    }
}

#[no_mangle]
/// Set a metadata node's type name.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `type_name` must be null or a valid
/// NUL-terminated C string.
pub unsafe extern "C" fn pdal_metadata_node_set_type(
    node: *mut MetadataNode,
    type_name: *const c_char,
) {
    if let Some(node) = node.as_mut() {
        node.set_type_name(c_string_lossy(type_name));
    }
}

#[no_mangle]
/// Set a metadata node's description.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `description` must be null or a valid
/// NUL-terminated C string.
pub unsafe extern "C" fn pdal_metadata_node_set_description(
    node: *mut MetadataNode,
    description: *const c_char,
) {
    if let Some(node) = node.as_mut() {
        node.set_description(c_string_lossy(description));
    }
}

/// Set a metadata node's signed integer value.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_set_i64(node: *mut MetadataNode, value: i64) {
    if let Some(node) = node.as_mut() {
        node.set_value(MetadataValue::I64(value));
    }
}

/// Set a metadata node's unsigned integer value.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_set_u64(node: *mut MetadataNode, value: u64) {
    if let Some(node) = node.as_mut() {
        node.set_value(MetadataValue::U64(value));
    }
}

/// Set a metadata node's floating-point value.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_set_f64(node: *mut MetadataNode, value: f64) {
    if let Some(node) = node.as_mut() {
        node.set_value(MetadataValue::F64(value));
    }
}

/// Set a metadata node's boolean value.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_set_bool(node: *mut MetadataNode, value: bool) {
    if let Some(node) = node.as_mut() {
        node.set_value(MetadataValue::Bool(value));
    }
}

/// Set a metadata node's opaque pointer value.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
/// The pointed-to object remains owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_set_pointer(
    node: *mut MetadataNode,
    value: *mut c_void,
) {
    if let Some(node) = node.as_mut() {
        node.set_value(MetadataValue::Pointer(value as usize));
    }
}

/// Return the metadata scalar value kind: 0 string, 1 i64, 2 u64, 3 f64,
/// 4 bool, 255 no value.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_value_kind(node: *const MetadataNode) -> u8 {
    node.as_ref()
        .and_then(MetadataNode::value)
        .map(MetadataValue::kind_id)
        .unwrap_or(255)
}

/// Return the metadata node kind: 0 instance, 1 array, 255 null.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_kind(node: *const MetadataNode) -> u8 {
    node.as_ref().map(|node| node.kind().as_u8()).unwrap_or(255)
}

/// Return a node's scalar value as a string. Caller must free with
/// `pdal_string_free`.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_value(node: *const MetadataNode) -> *mut c_char {
    string_to_c_ptr(
        node.as_ref()
            .and_then(MetadataNode::value)
            .map(MetadataValue::as_string)
            .unwrap_or_default(),
    )
}

/// Return a node's scalar value as a signed integer.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_value_i64(node: *const MetadataNode) -> i64 {
    node.as_ref()
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_i64)
        .unwrap_or_default()
}

/// Return a node's scalar value as an unsigned integer.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_value_u64(node: *const MetadataNode) -> u64 {
    node.as_ref()
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_u64)
        .unwrap_or_default()
}

/// Return a node's scalar value as a floating-point value.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_value_f64(node: *const MetadataNode) -> f64 {
    node.as_ref()
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_f64)
        .unwrap_or_default()
}

/// Return a node's scalar value as a boolean.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_value_bool(node: *const MetadataNode) -> bool {
    node.as_ref()
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_bool)
        .unwrap_or_default()
}

/// Return a node's scalar value as an opaque pointer.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_value_pointer(
    node: *const MetadataNode,
) -> *mut c_void {
    node.as_ref()
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_pointer)
        .unwrap_or_default() as *mut c_void
}

/// Format a PDAL metadata scalar value as JSON text. Caller must free with
/// `pdal_string_free`.
///
/// # Safety
///
/// `type_name` and `value` must be null or valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_json_value(
    type_name: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let type_name = c_string_lossy(type_name);
    let value = c_string_lossy(value);
    string_to_c_ptr(json_scalar_value(&type_name, &value))
}

#[no_mangle]
/// Convert a scalar metadata value to a signed integer.
///
/// # Safety
///
/// `type_name` and `value` must be null or valid NUL-terminated C strings.
/// `out_value` must be null or valid for writes.
pub unsafe extern "C" fn pdal_metadata_value_as_i64(
    type_name: *const c_char,
    value: *const c_char,
    out_value: *mut i64,
) -> bool {
    if let Some(converted) = scalar_as_i64(&c_string_lossy(type_name), &c_string_lossy(value)) {
        if let Some(out_value) = out_value.as_mut() {
            *out_value = converted;
        }
        true
    } else {
        false
    }
}

#[no_mangle]
/// Convert a scalar metadata value to an unsigned integer.
///
/// # Safety
///
/// `type_name` and `value` must be null or valid NUL-terminated C strings.
/// `out_value` must be null or valid for writes.
pub unsafe extern "C" fn pdal_metadata_value_as_u64(
    type_name: *const c_char,
    value: *const c_char,
    out_value: *mut u64,
) -> bool {
    if let Some(converted) = scalar_as_u64(&c_string_lossy(type_name), &c_string_lossy(value)) {
        if let Some(out_value) = out_value.as_mut() {
            *out_value = converted;
        }
        true
    } else {
        false
    }
}

#[no_mangle]
/// Convert a scalar metadata value to a double.
///
/// # Safety
///
/// `type_name` and `value` must be null or valid NUL-terminated C strings.
/// `out_value` must be null or valid for writes.
pub unsafe extern "C" fn pdal_metadata_value_as_f64(
    type_name: *const c_char,
    value: *const c_char,
    out_value: *mut f64,
) -> bool {
    if let Some(converted) = scalar_as_f64(&c_string_lossy(type_name), &c_string_lossy(value)) {
        if let Some(out_value) = out_value.as_mut() {
            *out_value = converted;
        }
        true
    } else {
        false
    }
}

#[no_mangle]
/// Convert a scalar metadata value to a bool.
///
/// # Safety
///
/// `type_name` and `value` must be null or valid NUL-terminated C strings.
/// `out_value` must be null or valid for writes.
pub unsafe extern "C" fn pdal_metadata_value_as_bool(
    type_name: *const c_char,
    value: *const c_char,
    out_value: *mut bool,
) -> bool {
    if let Some(converted) = scalar_as_bool(&c_string_lossy(type_name), &c_string_lossy(value)) {
        if let Some(out_value) = out_value.as_mut() {
            *out_value = converted;
        }
        true
    } else {
        false
    }
}

/// Add `child` to `node`, transferring ownership of `child`.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
/// `child` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. If non-null, it must not be used after this
/// call.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_add_child(
    node: *mut MetadataNode,
    child: *mut MetadataNode,
) {
    if let (Some(node), false) = (node.as_mut(), child.is_null()) {
        node.add_child(*Box::from_raw(child));
    }
}

#[no_mangle]
/// Add a child as a metadata list/array entry, transferring ownership.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `child` must be null or a valid pointer
/// returned by `pdal_metadata_node_create`; if non-null, it must not be used
/// after this call.
pub unsafe extern "C" fn pdal_metadata_node_add_list_child(
    node: *mut MetadataNode,
    child: *mut MetadataNode,
) {
    if let (Some(node), false) = (node.as_mut(), child.is_null()) {
        node.add_list_child(*Box::from_raw(child));
    }
}

#[no_mangle]
/// Add a cloned child to a node without transferring ownership.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `child` must be null or a valid metadata node
/// pointer.
pub unsafe extern "C" fn pdal_metadata_node_add_child_clone(
    node: *mut MetadataNode,
    child: *const MetadataNode,
) {
    if let (Some(node), Some(child)) = (node.as_mut(), child.as_ref()) {
        node.add_child(child.clone());
    }
}

#[no_mangle]
/// Add a cloned child as a metadata list/array entry without transferring
/// ownership.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `child` must be null or a valid metadata node
/// pointer.
pub unsafe extern "C" fn pdal_metadata_node_add_list_child_clone(
    node: *mut MetadataNode,
    child: *const MetadataNode,
) {
    if let (Some(node), Some(child)) = (node.as_mut(), child.as_ref()) {
        node.add_list_child(child.clone());
    }
}

#[no_mangle]
/// Add or replace a child, transferring ownership of `child`.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `child` must be null or a valid pointer
/// returned by `pdal_metadata_node_create`; if non-null, it must not be used
/// after this call.
pub unsafe extern "C" fn pdal_metadata_node_add_or_update_child(
    node: *mut MetadataNode,
    child: *mut MetadataNode,
) {
    if let (Some(node), false) = (node.as_mut(), child.is_null()) {
        node.add_or_update(*Box::from_raw(child));
    }
}

#[no_mangle]
/// Add or replace a cloned child without transferring ownership.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `child` must be null or a valid metadata node
/// pointer.
pub unsafe extern "C" fn pdal_metadata_node_add_or_update_child_clone(
    node: *mut MetadataNode,
    child: *const MetadataNode,
) {
    if let (Some(node), Some(child)) = (node.as_mut(), child.as_ref()) {
        node.add_or_update(child.clone());
    }
}

/// Return the number of child nodes.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_child_count(node: *const MetadataNode) -> u64 {
    node.as_ref()
        .map(|node| node.children().len() as u64)
        .unwrap_or(0)
}

/// Return a copy of a child node. Caller owns the returned pointer.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_child(
    node: *const MetadataNode,
    idx: u64,
) -> *mut MetadataNode {
    node.as_ref()
        .and_then(|node| node.children().get(idx as usize))
        .map(|child| Box::into_raw(Box::new(child.clone())))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
/// Return the count of child nodes with `name`.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `name` must be null or a valid NUL-terminated
/// C string.
pub unsafe extern "C" fn pdal_metadata_node_child_named_count(
    node: *const MetadataNode,
    name: *const c_char,
) -> u64 {
    let name = c_string_lossy(name);
    node.as_ref()
        .map(|node| node.children_named(&name).len() as u64)
        .unwrap_or(0)
}

#[no_mangle]
/// Return a copy of the named child at `idx`. Caller owns the returned pointer.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `name` must be null or a valid NUL-terminated
/// C string.
pub unsafe extern "C" fn pdal_metadata_node_child_named(
    node: *const MetadataNode,
    name: *const c_char,
    idx: u64,
) -> *mut MetadataNode {
    let name = c_string_lossy(name);
    node.as_ref()
        .and_then(|node| node.children_named(&name).get(idx as usize).copied())
        .map(|child| Box::into_raw(Box::new(child.clone())))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
/// Return a copy of the child selected by a colon-delimited path.
/// Caller owns the returned pointer.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`. `path` must be null or a valid
/// NUL-terminated C string.
pub unsafe extern "C" fn pdal_metadata_node_find_child_path(
    node: *const MetadataNode,
    path: *const c_char,
) -> *mut MetadataNode {
    let path = c_string_lossy(path);
    if path.is_empty() {
        return std::ptr::null_mut();
    }
    let Some(mut current) = node.as_ref() else {
        return std::ptr::null_mut();
    };
    for part in path.split(':') {
        let Some(child) = current.find_child(part) else {
            return std::ptr::null_mut();
        };
        current = child;
    }
    Box::into_raw(Box::new(current.clone()))
}

/// Serialize a metadata node tree as JSON. Caller must free with
/// `pdal_string_free`.
///
/// # Safety
///
/// `node` must be null or a valid pointer returned by
/// `pdal_metadata_node_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_to_json(node: *const MetadataNode) -> *mut c_char {
    let value = node
        .as_ref()
        .map(metadata_node_to_json)
        .unwrap_or_else(|| json!(null));
    string_to_c_ptr(serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()))
}

/// Destroy a metadata node.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `pdal_metadata_node_create`, or
/// null. Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_metadata_node_destroy(node: *mut MetadataNode) {
    if !node.is_null() {
        drop(Box::from_raw(node));
    }
}

pub(crate) fn metadata_node_to_json(node: &MetadataNode) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert("name".to_string(), json!(node.name()));
    if node.kind() == MetadataKind::Array {
        object.insert("kind".to_string(), json!("array"));
    }

    if let Some(value) = node.value() {
        object.insert("value".to_string(), metadata_value_to_json(value));
        object.insert("value_type".to_string(), json!(metadata_value_type(value)));
    }
    if let Some(type_name) = node.type_name() {
        object.insert("type".to_string(), json!(type_name));
    }
    if let Some(description) = node.description() {
        object.insert("description".to_string(), json!(description));
    }
    if !node.children().is_empty() {
        object.insert(
            "children".to_string(),
            serde_json::Value::Array(node.children().iter().map(metadata_node_to_json).collect()),
        );
    }

    serde_json::Value::Object(object)
}

pub(crate) fn metadata_node_to_json_flat(node: &MetadataNode) -> serde_json::Value {
    if node.children().is_empty() {
        if let Some(value) = node.value() {
            return metadata_value_to_json(value);
        }
    }

    let mut object = serde_json::Map::new();
    for child in node.children() {
        object.insert(child.name().to_string(), metadata_node_to_json_flat(child));
    }
    serde_json::Value::Object(object)
}

fn metadata_value_to_json(value: &MetadataValue) -> serde_json::Value {
    match value {
        MetadataValue::String(value) => json!(value),
        MetadataValue::I64(value) => json!(value),
        MetadataValue::U64(value) => json!(value),
        MetadataValue::F64(value) => json!(value),
        MetadataValue::Bool(value) => json!(value),
        MetadataValue::Pointer(value) => json!(value),
    }
}

fn metadata_value_type(value: &MetadataValue) -> &'static str {
    match value {
        MetadataValue::String(_) => "string",
        MetadataValue::I64(_) => "i64",
        MetadataValue::U64(_) => "u64",
        MetadataValue::F64(_) => "f64",
        MetadataValue::Bool(_) => "bool",
        MetadataValue::Pointer(_) => "pointer",
    }
}

unsafe fn c_string_lossy(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}
