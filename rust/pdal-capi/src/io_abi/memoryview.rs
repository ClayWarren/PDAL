use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use std::ffi::{c_char, CStr};
use std::rc::Rc;

#[repr(C)]
pub struct pdal_memoryview_field_t {
    pub name: *const c_char,
    pub type_id: i32,
    pub offset: u64,
}

fn parse_memoryview_shape(input: &str) -> Result<(u64, u64, u64), String> {
    let values: Vec<&str> = input.split(',').collect();
    if values.len() != 3 {
        return Err(
            "Shape must be specified as three integers: 'depth, rows, columns'.".to_string(),
        );
    }

    fn parse_field(label: &str, value: &str) -> Result<u64, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() || !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("Invalid {label} value in shape: '{trimmed}'."));
        }
        trimmed
            .parse()
            .map_err(|_| format!("Invalid {label} value in shape: '{trimmed}'."))
    }

    let depth = parse_field("depth", values[0])?;
    let rows = parse_field("rows", values[1])?;
    let columns = parse_field("rows", values[2])?;

    Ok((depth, rows, columns))
}

/// Parse a memory-view shape option such as `1, 2, 3`.
///
/// # Safety
///
/// Output pointers must be valid when non-null.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_memoryview_shape_parse(
    input: *const c_char,
    out_depth: *mut u64,
    out_rows: *mut u64,
    out_columns: *mut u64,
) -> *mut c_char {
    if input.is_null() {
        return string_to_c_ptr(
            "Shape must be specified as three integers: 'depth, rows, columns'.".to_string(),
        );
    }

    let input = CStr::from_ptr(input).to_string_lossy();
    match parse_memoryview_shape(&input) {
        Ok((depth, rows, columns)) => {
            if let Some(out_depth) = out_depth.as_mut() {
                *out_depth = depth;
            }
            if let Some(out_rows) = out_rows.as_mut() {
                *out_rows = rows;
            }
            if let Some(out_columns) = out_columns.as_mut() {
                *out_columns = columns;
            }
            std::ptr::null_mut()
        }
        Err(err) => string_to_c_ptr(err),
    }
}

pub type MemoryViewIncrementer =
    Option<unsafe extern "C" fn(point_id: u64, user_data: *mut std::ffi::c_void) -> *const u8>;

/// Read a C++ memory-view callback into a Rust-owned point view.
///
/// # Safety
/// `fields` must point to `field_count` valid field descriptors. `incrementer`
/// must return either a valid point base pointer for the requested point or
/// null to end the stream.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_memoryview_read(
    fields: *const pdal_memoryview_field_t,
    field_count: u64,
    incrementer: MemoryViewIncrementer,
    user_data: *mut std::ffi::c_void,
    depth: u64,
    rows: u64,
    columns: u64,
    column_major: bool,
) -> *mut PointView {
    clear_last_error();
    if fields.is_null() && field_count != 0 {
        set_last_error("null memoryview fields");
        return std::ptr::null_mut();
    }
    let Some(incrementer) = incrementer else {
        set_last_error("null memoryview incrementer");
        return std::ptr::null_mut();
    };

    let raw_fields = std::slice::from_raw_parts(fields, field_count as usize);
    let mut parsed_fields = Vec::with_capacity(raw_fields.len());
    let mut has_x = false;
    let mut has_y = false;
    let mut has_z = false;
    for field in raw_fields {
        if field.name.is_null() {
            set_last_error("null memoryview field name");
            return std::ptr::null_mut();
        }
        let name = CStr::from_ptr(field.name).to_string_lossy().into_owned();
        has_x |= name == "X";
        has_y |= name == "Y";
        has_z |= name == "Z";
        let Some(ty) = dim_type_from_pdal_type(field.type_id) else {
            set_last_error("unsupported memoryview field type");
            return std::ptr::null_mut();
        };
        parsed_fields.push((name, ty, field.offset as usize));
    }

    let has_shape = depth != 0 && rows != 0 && columns != 0;
    let mut layout = PointLayout::new();
    if has_shape && !(has_x && has_y && has_z) {
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
    }
    for (name, ty, _) in &parsed_fields {
        layout.register(DimId::from_name(name), *ty);
    }

    let layout = Rc::new(layout);
    let mut view = PointView::new(layout);
    let mut idx = 0;
    loop {
        let base = incrementer(idx, user_data);
        if base.is_null() {
            break;
        }
        view.add_point();
        for (name, ty, offset) in &parsed_fields {
            let value = read_memoryview_value(base.add(*offset), *ty);
            view.set_f64(idx, &DimId::from_name(name), value);
        }
        if has_shape {
            let (x, y, z) = memoryview_coordinates(idx, depth, rows, columns, column_major);
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }
        idx += 1;
    }

    Box::into_raw(Box::new(view))
}

fn dim_type_from_pdal_type(type_id: i32) -> Option<DimType> {
    match type_id as u32 {
        0x201 => Some(DimType::U8),
        0x202 => Some(DimType::U16),
        0x204 => Some(DimType::U32),
        0x208 => Some(DimType::U64),
        0x101 => Some(DimType::I8),
        0x102 => Some(DimType::I16),
        0x104 => Some(DimType::I32),
        0x108 => Some(DimType::I64),
        0x404 => Some(DimType::F32),
        0x408 => Some(DimType::F64),
        _ => None,
    }
}

unsafe fn read_memoryview_value(ptr: *const u8, ty: DimType) -> f64 {
    match ty {
        DimType::U8 => ptr.read_unaligned() as f64,
        DimType::U16 => (ptr as *const u16).read_unaligned() as f64,
        DimType::U32 => (ptr as *const u32).read_unaligned() as f64,
        DimType::U64 => (ptr as *const u64).read_unaligned() as f64,
        DimType::I8 => (ptr as *const i8).read_unaligned() as f64,
        DimType::I16 => (ptr as *const i16).read_unaligned() as f64,
        DimType::I32 => (ptr as *const i32).read_unaligned() as f64,
        DimType::I64 => (ptr as *const i64).read_unaligned() as f64,
        DimType::F32 => (ptr as *const f32).read_unaligned() as f64,
        DimType::F64 => (ptr as *const f64).read_unaligned(),
    }
}

fn memoryview_coordinates(
    idx: u64,
    depth: u64,
    rows: u64,
    columns: u64,
    column_major: bool,
) -> (f64, f64, f64) {
    let coords = if column_major {
        let x_div = depth * rows;
        let y_div = depth;
        let x_iter = depth * rows * columns;
        let y_iter = depth * rows;
        ((idx % x_iter) / x_div, (idx % y_iter) / y_div, idx % depth)
    } else {
        let y_iter = columns * rows;
        (
            idx % columns,
            (idx % y_iter) / columns,
            idx / (columns * rows) % depth,
        )
    };
    (coords.0 as f64, coords.1 as f64, coords.2 as f64)
}
