use super::*;

pub(super) fn parse_header(bytes: &[u8]) -> Result<Header, StageError> {
    let mut labels: Vec<String> = Vec::new();
    let mut sizes: Vec<u32> = Vec::new();
    let mut types: Vec<FieldType> = Vec::new();
    let mut counts: Vec<u32> = Vec::new();
    let mut width = 1;
    let mut height = 0;
    let mut points = 0;
    let mut start = 0;

    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|b| *b == b'\n')
            .map(|pos| start + pos)
            .unwrap_or(bytes.len());
        let next = if end < bytes.len() { end + 1 } else { end };
        let line_bytes = if end > start && bytes[end - 1] == b'\r' {
            &bytes[start..end - 1]
        } else {
            &bytes[start..end]
        };
        let line = std::str::from_utf8(line_bytes)
            .map_err(|_| StageError("PCD header is not valid UTF-8.".to_string()))?
            .trim();
        start = next;

        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some((kind, values)) = parts.split_first() else {
            continue;
        };

        match *kind {
            "VERSION" => {}
            "FIELDS" | "COLUMNS" => labels = values.iter().map(|value| value.to_string()).collect(),
            "SIZE" => sizes = parse_numbers(values, "SIZE")?,
            "TYPE" => {
                types = values
                    .iter()
                    .map(|value| parse_field_type(value))
                    .collect::<Result<Vec<_>, _>>()?;
            }
            "COUNT" => counts = parse_numbers(values, "COUNT")?,
            "WIDTH" => width = parse_one(values, "WIDTH")?,
            "HEIGHT" => height = parse_one(values, "HEIGHT")?,
            "VIEWPOINT" => {}
            "POINTS" => points = parse_one(values, "POINTS")?,
            "DATA" => {
                let storage = values
                    .first()
                    .ok_or_else(|| StageError("PCD DATA marker missing storage.".to_string()))?
                    .to_lowercase();
                if labels.is_empty() {
                    return Err(StageError(
                        "unrecognized PCD header, or missing DATA marker".to_string(),
                    ));
                }
                if sizes.is_empty() {
                    sizes = vec![4; labels.len()];
                }
                if types.is_empty() {
                    types = vec![FieldType::Float; labels.len()];
                }
                if counts.is_empty() {
                    counts = vec![1; labels.len()];
                }
                if sizes.len() != labels.len()
                    || types.len() != labels.len()
                    || counts.len() != labels.len()
                {
                    return Err(StageError(
                        "PCD field metadata counts do not match FIELDS.".to_string(),
                    ));
                }
                if points == 0 {
                    points = width * height;
                }
                let fields = labels
                    .iter()
                    .zip(sizes)
                    .zip(types)
                    .zip(counts)
                    .map(|(((label, size), ty), count)| Field {
                        id: DimId::from_name(&canonical_dim_name(label)),
                        label: canonical_dim_name(label),
                        size,
                        ty,
                        count,
                        precision: 2,
                    })
                    .collect();
                return Ok(Header {
                    fields,
                    points,
                    data_start: start,
                    storage,
                });
            }
            _ => {
                return Err(StageError(
                    "unrecognized PCD header, or missing DATA marker".to_string(),
                ));
            }
        }
    }

    Err(StageError(
        "unrecognized PCD header, or missing DATA marker".to_string(),
    ))
}

pub(super) fn pcd_input_is_streamable(filename: &str) -> bool {
    if filename.is_empty() {
        return false;
    }

    let Ok(file) = source::open_seek(filename) else {
        return false;
    };
    let mut reader = BufReader::new(file);
    let mut header_bytes = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let Ok(read) = reader.read_line(&mut line) else {
            return false;
        };
        if read == 0 {
            return false;
        }
        header_bytes.extend_from_slice(line.as_bytes());
        if line.trim_start().to_ascii_lowercase().starts_with("data ") {
            break;
        }
    }

    parse_header(&header_bytes)
        .map(|header| matches!(header.storage.as_str(), "ascii" | "binary"))
        .unwrap_or(false)
}

pub(super) fn read_binary_value(
    bytes: &[u8],
    offset: &mut usize,
    field: &Field,
) -> Result<f64, StageError> {
    let size = field.size as usize;
    if *offset + size > bytes.len() {
        return Err(StageError("Unexpected end of binary PCD data.".to_string()));
    }
    let value = &bytes[*offset..*offset + size];
    *offset += size;

    match (field.ty, field.size) {
        (FieldType::Signed, 1) => Ok(i8::from_le_bytes([value[0]]) as f64),
        (FieldType::Signed, 2) => Ok(i16::from_le_bytes(value.try_into().unwrap()) as f64),
        (FieldType::Signed, 4) => Ok(i32::from_le_bytes(value.try_into().unwrap()) as f64),
        (FieldType::Signed, 8) => Ok(i64::from_le_bytes(value.try_into().unwrap()) as f64),
        (FieldType::Unsigned, 1) => Ok(value[0] as f64),
        (FieldType::Unsigned, 2) => Ok(u16::from_le_bytes(value.try_into().unwrap()) as f64),
        (FieldType::Unsigned, 4) => Ok(u32::from_le_bytes(value.try_into().unwrap()) as f64),
        (FieldType::Unsigned, 8) => Ok(u64::from_le_bytes(value.try_into().unwrap()) as f64),
        (FieldType::Float, 4) => Ok(f32::from_le_bytes(value.try_into().unwrap()) as f64),
        (FieldType::Float, 8) => Ok(f64::from_le_bytes(value.try_into().unwrap())),
        _ => Err(StageError(format!(
            "Unsupported PCD binary field size {}.",
            field.size
        ))),
    }
}

pub(super) fn read_binary_value_from_reader<R: Read>(
    reader: &mut R,
    field: &Field,
) -> Result<f64, StageError> {
    let size = usize::try_from(field.size)
        .map_err(|_| StageError("Unsupported PCD binary field size.".to_string()))?;
    if size > 8 {
        return Err(StageError(format!(
            "Unsupported PCD binary field size {}.",
            field.size
        )));
    }
    let mut bytes = [0; 8];
    reader
        .read_exact(&mut bytes[..size])
        .map_err(|_| StageError("Unexpected end of binary PCD data.".to_string()))?;
    let mut offset = 0;
    read_binary_value(&bytes[..size], &mut offset, field)
}

pub(super) fn append_binary_point<R: Read>(
    view: &mut PointView,
    header: &Header,
    reader: &mut R,
) -> Result<(), StageError> {
    let point = view.add_point();
    for field in &header.fields {
        for count in 0..field.count {
            let value = read_binary_value_from_reader(reader, field)?;
            if count == 0 {
                view.set_f64(point, &field.id, value);
            }
        }
    }
    Ok(())
}

pub(super) fn read_interleaved_binary_points(
    view: &mut PointView,
    header: &Header,
    bytes: &[u8],
) -> Result<(), StageError> {
    let mut offset = 0;
    for _ in 0..header.points {
        let point = view.add_point();
        for field in &header.fields {
            for count in 0..field.count {
                let value = read_binary_value(bytes, &mut offset, field)?;
                if count == 0 {
                    view.set_f64(point, &field.id, value);
                }
            }
        }
    }
    Ok(())
}

pub(super) fn read_transposed_binary_points(
    view: &mut PointView,
    header: &Header,
    bytes: &[u8],
) -> Result<(), StageError> {
    let expected = binary_payload_size(header)?;
    if bytes.len() != expected {
        return Err(StageError(
            "Unexpected binary-compressed PCD size.".to_string(),
        ));
    }

    let mut offset = 0;
    for (field_idx, field) in header.fields.iter().enumerate() {
        for point_id in 0..header.points {
            let point = if field_idx == 0 {
                view.add_point()
            } else {
                point_id
            };
            for count in 0..field.count {
                let value = read_binary_value(bytes, &mut offset, field)?;
                if count == 0 {
                    view.set_f64(point, &field.id, value);
                }
            }
        }
    }
    Ok(())
}

pub(super) fn read_compressed_payload(
    header: &Header,
    bytes: &[u8],
) -> Result<Vec<u8>, StageError> {
    if bytes.len() < 8 {
        return Err(StageError(
            "Unexpected end of binary-compressed PCD data.".to_string(),
        ));
    }

    let compressed_size = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let uncompressed_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let expected = binary_payload_size(header)?;
    if uncompressed_size != expected {
        return Err(StageError(format!(
            "Binary-compressed PCD payload size {uncompressed_size} did not match header size {expected}."
        )));
    }
    let end = 8usize
        .checked_add(compressed_size)
        .ok_or_else(|| StageError("Binary-compressed PCD payload is too large.".to_string()))?;
    if end > bytes.len() {
        return Err(StageError(
            "Unexpected end of binary-compressed PCD data.".to_string(),
        ));
    }

    lzf_rust::decompress_into_vec(&bytes[8..end], uncompressed_size).map_err(|err| {
        StageError(format!(
            "Failed to decompress binary-compressed PCD data: {err}"
        ))
    })
}

pub(super) fn write_binary_value(
    output: &mut Vec<u8>,
    value: f64,
    ty: FieldType,
    size: u32,
) -> Result<(), StageError> {
    match (ty, size) {
        (FieldType::Signed, 1) => output.extend_from_slice(&(value as i8).to_le_bytes()),
        (FieldType::Signed, 2) => output.extend_from_slice(&(value as i16).to_le_bytes()),
        (FieldType::Signed, 4) => output.extend_from_slice(&(value as i32).to_le_bytes()),
        (FieldType::Signed, 8) => output.extend_from_slice(&(value as i64).to_le_bytes()),
        (FieldType::Unsigned, 1) => output.extend_from_slice(&(value as u8).to_le_bytes()),
        (FieldType::Unsigned, 2) => output.extend_from_slice(&(value as u16).to_le_bytes()),
        (FieldType::Unsigned, 4) => output.extend_from_slice(&(value as u32).to_le_bytes()),
        (FieldType::Unsigned, 8) => output.extend_from_slice(&(value as u64).to_le_bytes()),
        (FieldType::Float, 4) => output.extend_from_slice(&(value as f32).to_le_bytes()),
        (FieldType::Float, 8) => output.extend_from_slice(&value.to_le_bytes()),
        _ => {
            return Err(StageError(format!(
                "Unsupported PCD binary field size {size}."
            )))
        }
    }
    Ok(())
}

pub(super) fn write_interleaved_binary_points(
    output: &mut Vec<u8>,
    views: &[PointView],
    specs: &[Field],
) -> Result<(), StageError> {
    for view in views {
        for point in 0..view.len() {
            for field in specs {
                write_binary_value(output, view.get_f64(point, &field.id), field.ty, field.size)?;
            }
        }
    }
    Ok(())
}

pub(super) fn compressed_payload(
    views: &[PointView],
    specs: &[Field],
) -> Result<Vec<u8>, StageError> {
    let mut uncompressed = Vec::new();
    for field in specs {
        for view in views {
            for point in 0..view.len() {
                write_binary_value(
                    &mut uncompressed,
                    view.get_f64(point, &field.id),
                    field.ty,
                    field.size,
                )?;
            }
        }
    }

    let uncompressed_size = u32::try_from(uncompressed.len())
        .map_err(|_| StageError("PCD payload is too large to compress.".to_string()))?;
    let mut compressed = vec![0; lzf_rust::max_compressed_size(uncompressed.len())];
    let compressed_len = lzf_rust::compress(&uncompressed, &mut compressed)
        .map_err(|err| StageError(format!("Failed to compress PCD payload: {err}")))?;
    compressed.truncate(compressed_len);
    let compressed_size = u32::try_from(compressed.len())
        .map_err(|_| StageError("Compressed PCD payload is too large.".to_string()))?;

    let mut output = Vec::with_capacity(8 + compressed.len());
    output.extend_from_slice(&compressed_size.to_le_bytes());
    output.extend_from_slice(&uncompressed_size.to_le_bytes());
    output.extend_from_slice(&compressed);
    Ok(output)
}

pub(super) fn binary_payload_size(header: &Header) -> Result<usize, StageError> {
    let point_size = header.fields.iter().try_fold(0u64, |total, field| {
        total
            .checked_add(u64::from(field.size) * u64::from(field.count))
            .ok_or(())
    });
    let size = point_size
        .and_then(|point_size| point_size.checked_mul(header.points).ok_or(()))
        .map_err(|()| StageError("PCD binary payload is too large.".to_string()))?;
    usize::try_from(size).map_err(|_| StageError("PCD binary payload is too large.".to_string()))
}

pub(super) fn data_storage_label(compression: &str) -> &str {
    match compression {
        "compressed" | "binary_compressed" => "binary_compressed",
        other => other,
    }
}

pub(super) fn parse_numbers(values: &[&str], label: &str) -> Result<Vec<u32>, StageError> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| StageError(format!("failed parsing PCD {label} value")))
        })
        .collect()
}

pub(super) fn parse_one(values: &[&str], label: &str) -> Result<u64, StageError> {
    values
        .first()
        .ok_or_else(|| StageError(format!("PCD {label} missing value")))?
        .parse::<u64>()
        .map_err(|_| StageError(format!("failed parsing PCD {label} value")))
}

pub(super) fn parse_field_type(value: &str) -> Result<FieldType, StageError> {
    match value.to_uppercase().as_str() {
        "I" => Ok(FieldType::Signed),
        "U" => Ok(FieldType::Unsigned),
        "F" => Ok(FieldType::Float),
        other => Err(StageError(format!(
            "failed parsing PCD field type (\"{other}\")"
        ))),
    }
}

pub(super) fn canonical_dim_name(label: &str) -> String {
    match label.to_ascii_lowercase().as_str() {
        "x" => "X".to_string(),
        "y" => "Y".to_string(),
        "z" => "Z".to_string(),
        "intensity" => "Intensity".to_string(),
        "returnnumber" => "ReturnNumber".to_string(),
        "numberofreturns" => "NumberOfReturns".to_string(),
        "scandirectionflag" => "ScanDirectionFlag".to_string(),
        "edgeofflightline" => "EdgeOfFlightLine".to_string(),
        "classification" => "Classification".to_string(),
        "scananglerank" => "ScanAngleRank".to_string(),
        "userdata" => "UserData".to_string(),
        "pointsourceid" => "PointSourceId".to_string(),
        "gpstime" => "GpsTime".to_string(),
        "red" => "Red".to_string(),
        "green" => "Green".to_string(),
        "blue" => "Blue".to_string(),
        _ => label.to_string(),
    }
}

pub(super) fn dim_type(field: &Field) -> DimType {
    match (field.ty, field.size) {
        (FieldType::Signed, 1) => DimType::I8,
        (FieldType::Signed, 2) => DimType::I16,
        (FieldType::Signed, 4) => DimType::I32,
        (FieldType::Signed, 8) => DimType::I64,
        (FieldType::Unsigned, 1) => DimType::U8,
        (FieldType::Unsigned, 2) => DimType::U16,
        (FieldType::Unsigned, 4) => DimType::U32,
        (FieldType::Unsigned, 8) => DimType::U64,
        (FieldType::Float, 4) => {
            if matches!(field.id, DimId::X | DimId::Y | DimId::Z) {
                DimType::F64
            } else {
                DimType::F32
            }
        }
        (FieldType::Float, 8) => DimType::F64,
        _ => DimType::F64,
    }
}

pub(super) fn default_field(id: DimId, precision: usize) -> Field {
    let is_xyz = matches!(id, DimId::X | DimId::Y | DimId::Z);
    Field {
        label: id.name().to_string(),
        id,
        size: if is_xyz { 4 } else { 8 },
        ty: FieldType::Float,
        count: 1,
        precision,
    }
}

pub(super) fn apply_writer_type(field: &mut Field, spec: &str) -> Result<(), StageError> {
    match spec {
        "Unsigned8" => {
            field.ty = FieldType::Unsigned;
            field.size = 1;
        }
        "Unsigned16" => {
            field.ty = FieldType::Unsigned;
            field.size = 2;
        }
        "Unsigned32" => {
            field.ty = FieldType::Unsigned;
            field.size = 4;
        }
        "Unsigned64" => {
            field.ty = FieldType::Unsigned;
            field.size = 8;
        }
        "Signed8" => {
            field.ty = FieldType::Signed;
            field.size = 1;
        }
        "Signed16" => {
            field.ty = FieldType::Signed;
            field.size = 2;
        }
        "Signed32" => {
            field.ty = FieldType::Signed;
            field.size = 4;
        }
        "Signed64" => {
            field.ty = FieldType::Signed;
            field.size = 8;
        }
        "Float" => {
            field.ty = FieldType::Float;
            field.size = 4;
        }
        "Double" => {
            field.ty = FieldType::Float;
            field.size = 8;
        }
        _ => return Err(StageError(format!("Unknown PCD field type '{spec}'."))),
    }
    Ok(())
}

pub(super) fn storage_value(value: f64, field: &Field) -> f64 {
    match (field.ty, field.size) {
        (FieldType::Float, 4) => value as f32 as f64,
        _ => value,
    }
}

pub(super) fn format_number(value: f64, precision: usize, ty: FieldType, size: u32) -> String {
    match ty {
        FieldType::Float if size == 4 => format!("{:.precision$}", value as f32),
        FieldType::Float => format!("{value:.precision$}"),
        FieldType::Signed => format!("{}", value as i64),
        FieldType::Unsigned => format!("{}", value as u64),
    }
}
