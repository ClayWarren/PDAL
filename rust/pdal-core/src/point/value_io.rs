use super::DimType;

pub(super) fn value_fits_type(value: f64, ty: DimType) -> bool {
    if value.is_nan() {
        return matches!(ty, DimType::F32 | DimType::F64);
    }
    if !value.is_finite() {
        return matches!(ty, DimType::F32 | DimType::F64);
    }

    match ty {
        DimType::U8 => (u8::MIN as f64..=u8::MAX as f64).contains(&value),
        DimType::U16 => (u16::MIN as f64..=u16::MAX as f64).contains(&value),
        DimType::U32 => (u32::MIN as f64..=u32::MAX as f64).contains(&value),
        DimType::U64 => (u64::MIN as f64..=u64::MAX as f64).contains(&value),
        DimType::I8 => (i8::MIN as f64..=i8::MAX as f64).contains(&value),
        DimType::I16 => (i16::MIN as f64..=i16::MAX as f64).contains(&value),
        DimType::I32 => (i32::MIN as f64..=i32::MAX as f64).contains(&value),
        DimType::I64 => (i64::MIN as f64..=i64::MAX as f64).contains(&value),
        DimType::F32 => value.abs() <= f32::MAX as f64,
        DimType::F64 => true,
    }
}

/// Decode a little-endian dimension value to `f64`.
pub(super) fn read_value(buf: &[u8], ty: DimType) -> f64 {
    match ty {
        DimType::U8 => buf[0] as f64,
        DimType::I8 => (buf[0] as i8) as f64,
        DimType::U16 => u16::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::I16 => i16::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::U32 => u32::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::I32 => i32::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::U64 => u64::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::I64 => i64::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::F32 => f32::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::F64 => f64::from_le_bytes(buf.try_into().unwrap()),
    }
}

/// Encode an `f64` into a little-endian dimension value of type `ty`.
pub(super) fn write_value(buf: &mut [u8], ty: DimType, v: f64) {
    match ty {
        DimType::U8 => buf[0] = v as u8,
        DimType::I8 => buf[0] = (v as i8) as u8,
        DimType::U16 => buf.copy_from_slice(&(v as u16).to_le_bytes()),
        DimType::I16 => buf.copy_from_slice(&(v as i16).to_le_bytes()),
        DimType::U32 => buf.copy_from_slice(&(v as u32).to_le_bytes()),
        DimType::I32 => buf.copy_from_slice(&(v as i32).to_le_bytes()),
        DimType::U64 => buf.copy_from_slice(&(v as u64).to_le_bytes()),
        DimType::I64 => buf.copy_from_slice(&(v as i64).to_le_bytes()),
        DimType::F32 => buf.copy_from_slice(&(v as f32).to_le_bytes()),
        DimType::F64 => buf.copy_from_slice(&v.to_le_bytes()),
    }
}
