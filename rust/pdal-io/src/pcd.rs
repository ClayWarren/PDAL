use crate::source;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::{Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::rc::Rc;

#[path = "pcd_codec.rs"]
mod pcd_codec;
use pcd_codec::*;

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
    stream: Option<PcdReaderStreamState>,
}

struct PcdReaderStreamState {
    reader: BufReader<Box<dyn source::ReadSeek>>,
    header: Header,
    layout: Rc<PointLayout>,
    remaining: u64,
    eof: bool,
}

impl PcdReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            stream: None,
        }
    }

    fn layout_for(header: &Header) -> Rc<PointLayout> {
        let mut layout = PointLayout::new();
        for field in &header.fields {
            layout.register(field.id.clone(), dim_type(field));
        }
        Rc::new(layout)
    }

    fn append_ascii_line(view: &mut PointView, header: &Header, line: &str) -> bool {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != header.fields.len() {
            return false;
        }

        let point = view.add_point();
        for (field, value) in header.fields.iter().zip(fields) {
            let parsed = value.parse::<f64>().unwrap_or(0.0);
            view.set_f64(point, &field.id, storage_value(parsed, field));
        }
        true
    }

    fn stream_init(&mut self) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PcdReader requires a filename option.".to_string(),
            ));
        }

        let file = source::open_seek(&self.filename)
            .map_err(|_| StageError(format!("Can't open file '{}'.", self.filename)))?;
        let mut reader = BufReader::new(file);
        let mut header_bytes = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            let read = reader
                .read_line(&mut line)
                .map_err(|err| StageError(err.to_string()))?;
            if read == 0 {
                break;
            }
            header_bytes.extend_from_slice(line.as_bytes());
            if line.trim_start().to_ascii_lowercase().starts_with("data ") {
                break;
            }
        }

        let header = parse_header(&header_bytes)?;
        if header.storage != "ascii" && header.storage != "binary" {
            return Err(StageError(
                "PCD streaming is only supported for ASCII and binary input.".to_string(),
            ));
        }
        let layout = Self::layout_for(&header);
        self.stream = Some(PcdReaderStreamState {
            reader,
            remaining: header.points,
            header,
            layout,
            eof: false,
        });
        Ok(())
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

        let bytes = source::read_bytes(&self.filename)
            .map_err(|_| StageError(format!("Can't open file '{}'.", self.filename)))?;
        let header = parse_header(&bytes)?;

        let mut view = PointView::new(Self::layout_for(&header));

        if header.storage == "ascii" {
            let body = std::str::from_utf8(&bytes[header.data_start..])
                .map_err(|_| StageError("PCD ASCII body is not valid UTF-8.".to_string()))?;
            for line in body.lines() {
                Self::append_ascii_line(&mut view, &header, line);
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

    fn reset(&mut self) {
        self.stream = None;
    }

    fn streamable(&self) -> bool {
        pcd_input_is_streamable(&self.filename)
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.stream.is_none() {
            self.stream_init()?;
        }
        let state = self.stream.as_mut().expect("stream initialized above");
        if state.eof || state.remaining == 0 {
            return Ok(None);
        }

        let mut view = PointView::new(Rc::clone(&state.layout));
        if state.header.storage == "ascii" {
            let mut line = String::new();
            while view.len() < capacity.max(1) as u64 && state.remaining > 0 {
                line.clear();
                if state
                    .reader
                    .read_line(&mut line)
                    .map_err(|err| StageError(err.to_string()))?
                    == 0
                {
                    state.eof = true;
                    break;
                }
                if Self::append_ascii_line(
                    &mut view,
                    &state.header,
                    line.trim_end_matches(['\r', '\n']),
                ) {
                    state.remaining -= 1;
                }
            }
        } else if state.header.storage == "binary" {
            let target = capacity.max(1) as u64;
            while view.len() < target && state.remaining > 0 {
                append_binary_point(&mut view, &state.header, &mut state.reader)?;
                state.remaining -= 1;
            }
        } else {
            return Err(StageError(format!(
                "Unrecognized PCD data storage '{}'.",
                state.header.storage
            )));
        }

        if view.is_empty() && (state.eof || state.remaining == 0) {
            Ok(None)
        } else {
            Ok(Some(view))
        }
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
    stream: Option<PcdStreamState>,
}

struct PcdStreamState {
    rows: BufWriter<File>,
    rows_path: String,
    specs: Vec<Field>,
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
            stream: None,
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
    fn validate(&self) -> Result<(), StageError> {
        if !matches!(
            self.compression.as_str(),
            "ascii" | "binary" | "compressed" | "binary_compressed"
        ) {
            return Err(StageError(format!(
                "PCD compression '{}' is not supported by the Rust port.",
                self.compression
            )));
        }
        Ok(())
    }

    fn header_bytes(&self, specs: &[Field], count: u64) -> Vec<u8> {
        let mut header = String::new();
        header.push_str("VERSION 0.7\n");
        header.push_str("FIELDS");
        for field in specs {
            header.push(' ');
            header.push_str(&field.label.to_lowercase());
        }
        header.push_str("\nSIZE");
        for field in specs {
            header.push_str(&format!(" {}", field.size));
        }
        header.push_str("\nTYPE");
        for field in specs {
            header.push_str(match field.ty {
                FieldType::Signed => " I",
                FieldType::Unsigned => " U",
                FieldType::Float => " F",
            });
        }
        header.push_str("\nCOUNT");
        for field in specs {
            header.push_str(&format!(" {}", field.count));
        }
        header.push_str(&format!("\nWIDTH {count}\nHEIGHT 1\n"));
        header
            .push_str("VIEWPOINT 0.000000 0.000000 0.000000 1.000000 0.000000 0.000000 0.000000\n");
        header.push_str(&format!(
            "POINTS {count}\nDATA {}\n",
            data_storage_label(&self.compression)
        ));
        header.into_bytes()
    }

    fn write_ascii_rows<W: Write>(
        &mut self,
        writer: &mut W,
        views: &[PointView],
        specs: &[Field],
    ) -> Result<(), StageError> {
        let mut row = Vec::new();
        for view in views {
            for point in 0..view.len() {
                row.clear();
                for field in specs {
                    row.extend_from_slice(
                        format_number(
                            view.get_f64(point, &field.id),
                            field.precision,
                            field.ty,
                            field.size,
                        )
                        .as_bytes(),
                    );
                    row.push(b' ');
                }
                row.push(b'\n');
                writer
                    .write_all(&row)
                    .map_err(|e| StageError(format!("Failed writing '{}': {e}", self.filename)))?;
                self.point_count += 1;
            }
        }
        Ok(())
    }

    fn stream_rows_path(&self) -> String {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir()
            .join(format!(
                "pdal-rust-pcd-stream-{}-{suffix}.rows",
                std::process::id()
            ))
            .display()
            .to_string()
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
        self.validate()?;

        let Some(first) = views.first() else {
            fs::write(Path::new(&self.filename), "").map_err(|_| {
                StageError(format!("Couldn't open '{}' for output.", self.filename))
            })?;
            return Ok(());
        };
        let specs = self.dimension_specs(first.layout())?;
        let count: u64 = views.iter().map(PointView::len).sum();
        self.point_count = count;

        let mut output = self.header_bytes(&specs, count);
        if self.compression == "ascii" {
            self.point_count = 0;
            self.write_ascii_rows(&mut output, views, &specs)?;
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

    fn reset(&mut self) {
        if let Some(state) = self.stream.take() {
            let _ = fs::remove_file(state.rows_path);
        }
        self.point_count = 0;
    }

    fn streamable(&self) -> bool {
        !self.filename.is_empty() && matches!(self.compression.as_str(), "ascii" | "binary")
    }

    fn stream_write(&mut self, chunk: &PointView) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PcdWriter requires a filename option.".to_string(),
            ));
        }
        self.validate()?;
        if self.compression != "ascii" && self.compression != "binary" {
            return Err(StageError(
                "PCD streaming is only supported for ASCII and binary output.".to_string(),
            ));
        }
        if self.stream.is_none() {
            let specs = self.dimension_specs(chunk.layout())?;
            let rows_path = self.stream_rows_path();
            let rows = File::create(&rows_path)
                .map(BufWriter::new)
                .map_err(|e| StageError(format!("Failed creating PCD row stream: {e}")))?;
            self.stream = Some(PcdStreamState {
                rows,
                rows_path,
                specs,
            });
        }

        let mut state = self.stream.take().expect("stream initialized above");
        if self.compression == "ascii" {
            self.write_ascii_rows(&mut state.rows, std::slice::from_ref(chunk), &state.specs)?;
        } else {
            let mut bytes = Vec::new();
            write_interleaved_binary_points(&mut bytes, std::slice::from_ref(chunk), &state.specs)?;
            state
                .rows
                .write_all(&bytes)
                .map_err(|e| StageError(format!("Failed writing '{}': {e}", self.filename)))?;
            self.point_count += chunk.len();
        }
        self.stream = Some(state);
        Ok(())
    }

    fn stream_finish(&mut self) -> Result<(), StageError> {
        let Some(mut state) = self.stream.take() else {
            return self.write(&[]);
        };
        state
            .rows
            .flush()
            .map_err(|e| StageError(format!("Failed writing PCD row stream: {e}")))?;
        drop(state.rows);

        let mut output = File::create(Path::new(&self.filename))
            .map(BufWriter::new)
            .map_err(|_| StageError(format!("Couldn't open '{}' for output.", self.filename)))?;
        output
            .write_all(&self.header_bytes(&state.specs, self.point_count))
            .map_err(|e| StageError(format!("Failed writing '{}': {e}", self.filename)))?;
        let mut rows = File::open(&state.rows_path)
            .map(BufReader::new)
            .map_err(|e| StageError(format!("Failed reopening PCD row stream: {e}")))?;
        std::io::copy(&mut rows, &mut output)
            .map_err(|e| StageError(format!("Failed writing '{}': {e}", self.filename)))?;
        output
            .flush()
            .map_err(|e| StageError(format!("Failed writing '{}': {e}", self.filename)))?;
        let _ = fs::remove_file(state.rows_path);
        Ok(())
    }
}

#[cfg(test)]
include!("pcd_tests.rs");
