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

/// ASCII PCD reader.
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

        let text = fs::read_to_string(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Can't open file '{}'.", self.filename)))?;
        let lines: Vec<&str> = text.lines().collect();
        let header = parse_header(&lines)?;
        if header.storage != "ascii" {
            return Err(StageError(format!(
                "PCD data storage '{}' is not supported by the Rust ASCII slice.",
                header.storage
            )));
        }

        let mut layout = PointLayout::new();
        for field in &header.fields {
            layout.register(field.id.clone(), dim_type(field));
        }
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for line in lines.iter().skip(header.data_start) {
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

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.pcd")
    }
}

/// ASCII PCD writer.
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
        if self.compression != "ascii" {
            return Err(StageError(format!(
                "PCD compression '{}' is not supported by the Rust ASCII slice.",
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

        let mut output = String::new();
        output.push_str("VERSION 0.7\n");
        output.push_str("FIELDS");
        for field in &specs {
            output.push(' ');
            output.push_str(&field.label.to_lowercase());
        }
        output.push_str("\nSIZE");
        for field in &specs {
            output.push_str(&format!(" {}", field.size));
        }
        output.push_str("\nTYPE");
        for field in &specs {
            output.push_str(match field.ty {
                FieldType::Signed => " I",
                FieldType::Unsigned => " U",
                FieldType::Float => " F",
            });
        }
        output.push_str("\nCOUNT");
        for field in &specs {
            output.push_str(&format!(" {}", field.count));
        }
        output.push_str(&format!("\nWIDTH {count}\nHEIGHT 1\n"));
        output
            .push_str("VIEWPOINT 0.000000 0.000000 0.000000 1.000000 0.000000 0.000000 0.000000\n");
        output.push_str(&format!("POINTS {count}\nDATA ascii\n"));

        for view in views {
            for point in 0..view.len() {
                for field in &specs {
                    output.push_str(&format_number(
                        view.get_f64(point, &field.id),
                        field.precision,
                        field.ty,
                        field.size,
                    ));
                    output.push(' ');
                }
                output.push('\n');
            }
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

fn parse_header(lines: &[&str]) -> Result<Header, StageError> {
    let mut labels: Vec<String> = Vec::new();
    let mut sizes: Vec<u32> = Vec::new();
    let mut types: Vec<FieldType> = Vec::new();
    let mut counts: Vec<u32> = Vec::new();
    let mut width = 1;
    let mut height = 0;
    let mut points = 0;

    for (idx, raw) in lines.iter().enumerate() {
        let line = raw.trim();
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
                    data_start: idx + 1,
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
    match label.to_uppercase().as_str() {
        "X" => "X".to_string(),
        "Y" => "Y".to_string(),
        "Z" => "Z".to_string(),
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
mod tests {
    use super::*;
    use pdal_core::pipeline::{FilterWrapper, Pipeline};
    use pdal_filters::decimation::DecimationFilter;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn temp_path(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-pcd-{}-{name}", std::process::id()));
        let _ = fs::remove_file(&path);
        path.display().to_string()
    }

    #[test]
    fn reads_ascii_space_separated_pcd() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_space.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(0, &DimId::X), 289814.15625);
        assert_eq!(view.get_f64(0, &DimId::Y), 4320978.5);
        assert_eq!(view.get_f64(0, &DimId::Z), 170.75999450683594);
        assert_eq!(view.get_f64(9, &DimId::X), 289818.5);
    }

    #[test]
    fn reads_ascii_tab_separated_pcd() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_tab.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(9, &DimId::Y), 4320980.5);
    }

    #[test]
    fn comma_separated_ascii_rows_are_skipped_like_cpp_reader() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_comma.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        assert_eq!(view.len(), 0);
    }

    #[test]
    fn missing_header_is_rejected() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/missingheader.pcd"));
        let mut reader = PcdReader::new(&options);

        assert!(reader.read().is_err());
    }

    #[test]
    fn writes_ascii_pcd_that_reader_roundtrips() {
        let mut options = Options::new();
        options.add("filename", data_path("pcd/utm17_space.pcd"));
        let mut reader = PcdReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let output = temp_path("roundtrip.pcd");
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X,Y,Z")
            .add("precision", 2);
        let mut writer = PcdWriter::new(&writer_options);
        writer.write(std::slice::from_ref(&view)).unwrap();

        let mut read_options = Options::new();
        read_options.add("filename", &output);
        let mut roundtrip = PcdReader::new(&read_options);
        let roundtrip = roundtrip.read().unwrap().pop().unwrap();

        assert_eq!(roundtrip.len(), view.len());
        assert_eq!(roundtrip.get_f64(0, &DimId::X), view.get_f64(0, &DimId::X));
        assert_eq!(roundtrip.get_f64(9, &DimId::Z), view.get_f64(9, &DimId::Z));
    }

    #[test]
    fn per_dimension_precision_matches_existing_writer_shape() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for values in [
            [1.0, 1.0, 1.0, 1.0],
            [
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_22,
            ],
            [3.33, 3.33, 3.33, 3.33],
        ] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, values[0]);
            view.set_f64(point, &DimId::Y, values[1]);
            view.set_f64(point, &DimId::Z, values[2]);
            view.set_f64(point, &DimId::Intensity, values[3]);
        }

        let output = temp_path("precision.pcd");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("precision", 5)
            .add("order", "X=Float:0,Y=Float:0,Z=Float:0,Intensity=Float:0");
        let mut writer = PcdWriter::new(&options);
        writer.write(&[view]).unwrap();

        let written = fs::read_to_string(output).unwrap();
        assert!(written.contains("1 1 1 1"));
        assert!(written.contains("2 2 2 2"));
        assert!(written.contains("3 3 3 3"));
    }

    #[test]
    fn reader_filter_writer_pipeline_writes_ascii_pcd() {
        let input = data_path("pcd/utm17_space.pcd");
        let output = temp_path("pipeline.pcd");

        let mut reader_options = Options::new();
        reader_options.add("filename", input);
        let mut filter_options = Options::new();
        filter_options.add("step", 2);
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X,Y,Z")
            .add("precision", 2);

        let mut pipeline = Pipeline::new();
        let reader = pipeline.add_reader(
            "readers.pcd",
            Box::new(PcdReader::new(&reader_options)),
            reader_options,
        );
        let filter = pipeline.add_stage(
            "filters.decimation",
            Box::new(FilterWrapper::new(DecimationFilter::new(&filter_options))),
            filter_options,
        );
        let writer = pipeline.add_writer(
            "writers.pcd",
            Box::new(PcdWriter::new(&writer_options)),
            writer_options,
        );
        pipeline.add_dependency(filter, reader).unwrap();
        pipeline.add_dependency(writer, filter).unwrap();

        assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
        let written = fs::read_to_string(output).unwrap();
        assert!(written.contains("POINTS 5\nDATA ascii\n"));
    }
}
