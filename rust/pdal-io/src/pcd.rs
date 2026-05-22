use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::{Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs;
use std::path::Path;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FieldType {
    Signed,
    Unsigned,
    Float,
}

#[derive(Clone, Debug)]
struct Field {
    id: DimId,
    label: String,
    size: u32,
    ty: FieldType,
    count: u32,
    precision: usize,
}

#[derive(Clone, Debug)]
struct Header {
    fields: Vec<Field>,
    points: u64,
    data_start: usize,
    storage: String,
}

/// PCD reader.
pub struct PcdReader {
    filename: String,
}

impl PcdReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Reader for PcdReader {
    fn name(&self) -> &str {
        "readers.pcd"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PcdReader requires a filename option.".to_string(),
            ));
        }

        let bytes = fs::read(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Can't open file '{}'.", self.filename)))?;
        let header = parse_header(&bytes)?;

        let mut layout = PointLayout::new();
        for field in &header.fields {
            layout.register(field.id.clone(), dim_type(field));
        }
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        if header.storage == "ascii" {
            let body = std::str::from_utf8(&bytes[header.data_start..])
                .map_err(|_| StageError("PCD ASCII body is not valid UTF-8.".to_string()))?;
            for line in body.lines() {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() != header.fields.len() {
                    continue;
                }

                let point = view.add_point();
                for (field, value) in header.fields.iter().zip(fields) {
                    let parsed = value.parse::<f64>().unwrap_or(0.0);
                    view.set_f64(point, &field.id, storage_value(parsed, field));
                }
                if view.len() >= header.points {
                    break;
                }
            }
        } else if header.storage == "binary" {
            read_interleaved_binary_points(&mut view, &header, &bytes[header.data_start..])?;
        } else if header.storage == "binary_compressed" {
            let payload = read_compressed_payload(&header, &bytes[header.data_start..])?;
            read_transposed_binary_points(&mut view, &header, &payload)?;
        } else {
            return Err(StageError(format!(
                "Unrecognized PCD data storage '{}'.",
                header.storage
            )));
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.pcd")
    }
}

/// PCD writer.
pub struct PcdWriter {
    filename: String,
    compression: String,
    write_all_dims: bool,
    dim_order: String,
    precision: usize,
    point_count: u64,
}

impl PcdWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            compression: options.get_str("compression", "ascii").to_lowercase(),
            write_all_dims: options.get_bool("keep_unspecified", true),
            dim_order: options.get_str("order", ""),
            precision: options.get_u64("precision", 2) as usize,
            point_count: 0,
        }
    }

    fn dimension_specs(&self, layout: &PointLayout) -> Result<Vec<Field>, StageError> {
        let mut specs = Vec::new();
        for item in self
            .dim_order
            .split(',')
            .filter(|item| !item.trim().is_empty())
        {
            specs.push(self.extract_dim(item, layout)?);
        }

        if self.dim_order.trim().is_empty() || self.write_all_dims {
            for idx in 0..layout.dim_count() {
                let Some((id, _ty)) = layout.dim_at(idx) else {
                    continue;
                };
                if specs.iter().any(|spec| spec.id == *id) {
                    continue;
                }
                specs.push(default_field(id.clone(), self.precision));
            }
        }

        Ok(specs)
    }

    fn extract_dim(&self, text: &str, layout: &PointLayout) -> Result<Field, StageError> {
        let mut parts = text.trim().split('=');
        let name = parts.next().unwrap_or("").trim();
        let id = DimId::from_name(name);
        if layout.dim(&id).is_none() {
            return Err(StageError(format!(
                "Dimension not found with name '{text}'."
            )));
        }

        let mut field = default_field(id, self.precision);
        field.label = name.to_string();

        if let Some(type_spec) = parts.next() {
            let mut type_parts = type_spec.split(':');
            apply_writer_type(&mut field, type_parts.next().unwrap_or(""))?;
            if let Some(precision) = type_parts.next() {
                field.precision = precision.parse::<usize>().map_err(|_| {
                    StageError(format!("Can't convert dimension precision for '{text}'."))
                })?;
            }
            if type_parts.next().is_some() {
                return Err(StageError(format!(
                    "Invalid dimension specification '{text}'."
                )));
            }
        }
        if parts.next().is_some() {
            return Err(StageError(format!(
                "Invalid dimension specification '{text}'."
            )));
        }

        Ok(field)
    }
}

impl Writer for PcdWriter {
    fn name(&self) -> &str {
        "writers.pcd"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PcdWriter requires a filename option.".to_string(),
            ));
        }
        if !matches!(
            self.compression.as_str(),
            "ascii" | "binary" | "compressed" | "binary_compressed"
        ) {
            return Err(StageError(format!(
                "PCD compression '{}' is not supported by the Rust port.",
                self.compression
            )));
        }

        let Some(first) = views.first() else {
            fs::write(Path::new(&self.filename), "").map_err(|_| {
                StageError(format!("Couldn't open '{}' for output.", self.filename))
            })?;
            return Ok(());
        };
        let specs = self.dimension_specs(first.layout())?;
        let count: u64 = views.iter().map(PointView::len).sum();
        self.point_count = count;

        let mut output = Vec::new();
        let mut header = String::new();
        header.push_str("VERSION 0.7\n");
        header.push_str("FIELDS");
        for field in &specs {
            header.push(' ');
            header.push_str(&field.label.to_lowercase());
        }
        header.push_str("\nSIZE");
        for field in &specs {
            header.push_str(&format!(" {}", field.size));
        }
        header.push_str("\nTYPE");
        for field in &specs {
            header.push_str(match field.ty {
                FieldType::Signed => " I",
                FieldType::Unsigned => " U",
                FieldType::Float => " F",
            });
        }
        header.push_str("\nCOUNT");
        for field in &specs {
            header.push_str(&format!(" {}", field.count));
        }
        header.push_str(&format!("\nWIDTH {count}\nHEIGHT 1\n"));
        header
            .push_str("VIEWPOINT 0.000000 0.000000 0.000000 1.000000 0.000000 0.000000 0.000000\n");
        header.push_str(&format!(
            "POINTS {count}\nDATA {}\n",
            data_storage_label(&self.compression)
        ));
        output.extend_from_slice(header.as_bytes());

        if self.compression == "ascii" {
            for view in views {
                for point in 0..view.len() {
                    for field in &specs {
                        output.extend_from_slice(
                            format_number(
                                view.get_f64(point, &field.id),
                                field.precision,
                                field.ty,
                                field.size,
                            )
                            .as_bytes(),
                        );
                        output.push(b' ');
                    }
                    output.push(b'\n');
                }
            }
        } else if self.compression == "binary" {
            write_interleaved_binary_points(&mut output, views, &specs)?;
        } else {
            let payload = compressed_payload(views, &specs)?;
            output.extend_from_slice(&payload);
        }

        fs::write(Path::new(&self.filename), output)
            .map_err(|_| StageError(format!("Couldn't open '{}' for output.", self.filename)))
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.pcd");
        node.add_value("filename", MetadataValue::String(self.filename.clone()));
        node.add_value("point_count", MetadataValue::U64(self.point_count));
        node
    }
}

fn parse_header(bytes: &[u8]) -> Result<Header, StageError> {
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

fn read_binary_value(bytes: &[u8], offset: &mut usize, field: &Field) -> Result<f64, StageError> {
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

fn read_interleaved_binary_points(
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

fn read_transposed_binary_points(
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

fn read_compressed_payload(header: &Header, bytes: &[u8]) -> Result<Vec<u8>, StageError> {
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

fn write_binary_value(
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

fn write_interleaved_binary_points(
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

fn compressed_payload(views: &[PointView], specs: &[Field]) -> Result<Vec<u8>, StageError> {
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

fn binary_payload_size(header: &Header) -> Result<usize, StageError> {
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

fn data_storage_label(compression: &str) -> &str {
    match compression {
        "compressed" | "binary_compressed" => "binary_compressed",
        other => other,
    }
}

fn parse_numbers(values: &[&str], label: &str) -> Result<Vec<u32>, StageError> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| StageError(format!("failed parsing PCD {label} value")))
        })
        .collect()
}

fn parse_one(values: &[&str], label: &str) -> Result<u64, StageError> {
    values
        .first()
        .ok_or_else(|| StageError(format!("PCD {label} missing value")))?
        .parse::<u64>()
        .map_err(|_| StageError(format!("failed parsing PCD {label} value")))
}

fn parse_field_type(value: &str) -> Result<FieldType, StageError> {
    match value.to_uppercase().as_str() {
        "I" => Ok(FieldType::Signed),
        "U" => Ok(FieldType::Unsigned),
        "F" => Ok(FieldType::Float),
        other => Err(StageError(format!(
            "failed parsing PCD field type (\"{other}\")"
        ))),
    }
}

fn canonical_dim_name(label: &str) -> String {
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

fn dim_type(field: &Field) -> DimType {
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

fn default_field(id: DimId, precision: usize) -> Field {
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

fn apply_writer_type(field: &mut Field, spec: &str) -> Result<(), StageError> {
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

fn storage_value(value: f64, field: &Field) -> f64 {
    match (field.ty, field.size) {
        (FieldType::Float, 4) => value as f32 as f64,
        _ => value,
    }
}

fn format_number(value: f64, precision: usize, ty: FieldType, size: u32) -> String {
    match ty {
        FieldType::Float if size == 4 => format!("{:.precision$}", value as f32),
        FieldType::Float => format!("{value:.precision$}"),
        FieldType::Signed => format!("{}", value as i64),
        FieldType::Unsigned => format!("{}", value as u64),
    }
}

#[cfg(test)]
include!("pcd_tests.rs");
