//! `readers.ply` -- Stanford PLY format.
//!
//! Port of the ASCII path of `io/PlyReader.cpp`. A PLY file has a text header
//! (`ply`, `format`, `element`/`property` declarations, `end_header`) followed
//! by a data section. The reader emits the `vertex` element's instances as
//! points; elements declared before `vertex` are consumed to reach it, and
//! face lists can be stored in the view mesh.
//!
//! ASCII and binary little/big endian storage modes are supported.

use crate::source;
#[path = "ply_stream.rs"]
mod ply_stream;
use ply_stream::PlyReaderStreamState;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::{Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor, Write};
use std::path::Path;
use std::rc::Rc;
use std::str::SplitWhitespace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

impl PlyFormat {
    fn from_writer_option(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ascii" => Some(Self::Ascii),
            "little endian" | "binary_little_endian" => Some(Self::BinaryLittleEndian),
            "big endian" | "binary_big_endian" => Some(Self::BinaryBigEndian),
            _ => None,
        }
    }

    fn header_name(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::BinaryLittleEndian => "binary_little_endian",
            Self::BinaryBigEndian => "binary_big_endian",
        }
    }
}

/// One declared property of a PLY element.
#[derive(Clone)]
enum PlyProp {
    /// A scalar value: one ASCII token.
    Simple { dim: DimId, ty: DimType },
    /// A list value: a count token followed by that many value tokens.
    List {
        name: String,
        count_ty: DimType,
        list_ty: DimType,
    },
}

/// One declared PLY element (`vertex`, `face`, ...).
#[derive(Clone)]
struct Element {
    name: String,
    count: usize,
    props: Vec<PlyProp>,
}

enum PlyData<'a> {
    Ascii(SplitWhitespace<'a>),
    Binary {
        cursor: Cursor<&'a [u8]>,
        format: PlyFormat,
    },
}

impl PlyData<'_> {
    fn read_simple(&mut self, prop: &PlyProp, element_name: &str) -> Result<f64, StageError> {
        let PlyProp::Simple { ty, .. } = prop else {
            return Err(StageError(format!(
                "Expected scalar data for the '{element_name}' element."
            )));
        };
        self.read_value(*ty, element_name)
    }

    fn read_count(&mut self, ty: DimType, element_name: &str) -> Result<usize, StageError> {
        Ok(self.read_value(ty, element_name)? as usize)
    }

    fn read_index(&mut self, ty: DimType, element_name: &str) -> Result<u64, StageError> {
        Ok(self.read_value(ty, element_name)? as u64)
    }

    fn read_value(&mut self, ty: DimType, element_name: &str) -> Result<f64, StageError> {
        match self {
            PlyData::Ascii(tokens) => {
                let token = tokens.next().ok_or_else(|| {
                    StageError(format!(
                        "Error reading data for the '{element_name}' element."
                    ))
                })?;
                token.parse::<f64>().map_err(|_| {
                    StageError(format!(
                        "Invalid numeric value '{token}' in '{element_name}' data."
                    ))
                })
            }
            PlyData::Binary { cursor, format } => read_binary_value(cursor, *format, ty),
        }
    }
}

fn read_binary_value(
    cursor: &mut Cursor<&[u8]>,
    format: PlyFormat,
    ty: DimType,
) -> Result<f64, StageError> {
    let value = match (format, ty) {
        (_, DimType::I8) => cursor.read_i8().map(f64::from),
        (_, DimType::U8) => cursor.read_u8().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::I16) => {
            cursor.read_i16::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::I16) => cursor.read_i16::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::U16) => {
            cursor.read_u16::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::U16) => cursor.read_u16::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::I32) => {
            cursor.read_i32::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::I32) => cursor.read_i32::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::U32) => {
            cursor.read_u32::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::U32) => cursor.read_u32::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::F32) => {
            cursor.read_f32::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::F32) => cursor.read_f32::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::F64) => cursor.read_f64::<LittleEndian>(),
        (PlyFormat::BinaryBigEndian, DimType::F64) => cursor.read_f64::<BigEndian>(),
        (PlyFormat::Ascii, _) | (_, DimType::I64 | DimType::U64) => {
            return Err(StageError(
                "Unsupported binary PLY property type.".to_string(),
            ));
        }
    };
    value.map_err(|err| StageError(err.to_string()))
}

/// Reader for the Stanford PLY format (ASCII encoding).
pub struct PlyReader {
    filename: String,
    stream: Option<PlyReaderStreamState>,
}

impl PlyReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            stream: None,
        }
    }

    fn stream_init(&mut self) -> Result<(), StageError> {
        self.stream = Some(ply_stream::stream_init(&self.filename)?);
        Ok(())
    }
}

impl Reader for PlyReader {
    fn name(&self) -> &str {
        "readers.ply"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PlyReader requires a filename option.".to_string(),
            ));
        }
        let bytes = source::read_bytes(&self.filename)
            .map_err(|_| StageError(format!("Couldn't open '{}'.", self.filename)))?;
        let header_end = find_header_end(&bytes)
            .ok_or_else(|| StageError("'end_header' not found in PLY file.".to_string()))?;
        let header = std::str::from_utf8(&bytes[..header_end])
            .map_err(|_| StageError("PLY header is not valid UTF-8.".to_string()))?;

        let (elements, format) = parse_header(header)?;

        // The vertex element fixes the point layout.
        let vertex = elements
            .iter()
            .find(|element| element.name == "vertex")
            .ok_or_else(|| {
                StageError("Can't read PLY file without a 'vertex' element.".to_string())
            })?;
        let mut layout = PointLayout::new();
        // PDAL overrides X/Y/Z to doubles regardless of the declared type.
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        for prop in &vertex.props {
            if let PlyProp::Simple { dim, ty } = prop {
                layout.register(dim.clone(), *ty);
            }
        }
        let mut view = PointView::new(Rc::new(layout));

        let data = &bytes[header_end..];
        let data_text;
        let mut data_reader = if format == PlyFormat::Ascii {
            data_text = std::str::from_utf8(data)
                .map_err(|_| StageError("PLY ASCII data is not valid UTF-8.".to_string()))?;
            PlyData::Ascii(data_text.split_whitespace())
        } else {
            PlyData::Binary {
                cursor: Cursor::new(data),
                format,
            }
        };

        for element in &elements {
            let is_vertex = element.name == "vertex";
            for _ in 0..element.count {
                let point = is_vertex.then(|| view.add_point());
                for prop in &element.props {
                    match prop {
                        PlyProp::Simple { dim, .. } => {
                            let value = data_reader.read_simple(prop, &element.name)?;
                            if let Some(point) = point {
                                view.set_f64(point, dim, value);
                            }
                        }
                        PlyProp::List {
                            name,
                            count_ty,
                            list_ty,
                        } => {
                            let count = data_reader.read_count(*count_ty, &element.name)?;
                            let mut values = Vec::with_capacity(count);
                            for _ in 0..count {
                                values.push(data_reader.read_index(*list_ty, &element.name)?);
                            }
                            if element.name == "face" && name == "vertex_indices" && count >= 3 {
                                let mesh = view.create_mesh();
                                for idx in 1..(values.len() - 1) {
                                    mesh.add(values[0], values[idx], values[idx + 1]);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.ply")
    }

    fn reset(&mut self) {
        self.stream = None;
    }

    fn streamable(&self) -> bool {
        ply_stream::streamable(&self.filename)
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.stream.is_none() {
            self.stream_init()?;
        }
        ply_stream::stream_next(
            self.stream.as_mut().expect("stream initialized above"),
            capacity,
        )
    }
}

/// Parse the PLY header, returning the elements and storage format.
fn parse_header(text: &str) -> Result<(Vec<Element>, PlyFormat), StageError> {
    let mut elements: Vec<Element> = Vec::new();
    let mut seen = 0;
    let mut format = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        seen += 1;

        if seen == 1 {
            if line != "ply" {
                return Err(StageError(
                    "File isn't a PLY file.  'ply' not found.".to_string(),
                ));
            }
            continue;
        }
        if seen == 2 {
            if words[0] != "format" {
                return Err(StageError(
                    "Expected format line not found in PLY file.".to_string(),
                ));
            }
            format = Some(match words.get(1).copied() {
                Some("ascii") => PlyFormat::Ascii,
                Some("binary_little_endian") => PlyFormat::BinaryLittleEndian,
                Some("binary_big_endian") => PlyFormat::BinaryBigEndian,
                other => {
                    return Err(StageError(format!(
                        "Unrecognized PLY format: '{}'.",
                        other.unwrap_or("")
                    )));
                }
            });
            if words.get(2).copied() != Some("1.0") {
                return Err(StageError("Unsupported PLY version.".to_string()));
            }
            continue;
        }

        match words[0] {
            "comment" | "obj_info" => {}
            "element" => {
                let name = words
                    .get(1)
                    .ok_or_else(|| StageError("Missing element name.".to_string()))?
                    .to_string();
                let count: usize = words
                    .get(2)
                    .and_then(|word| word.parse().ok())
                    .ok_or_else(|| StageError(format!("Invalid count for element '{name}'.")))?;
                elements.push(Element {
                    name,
                    count,
                    props: Vec::new(),
                });
            }
            "property" => {
                let element = elements.last_mut().ok_or_else(|| {
                    StageError("PLY 'property' found outside of an element.".to_string())
                })?;
                let prop = parse_property(&words, &element.name)?;
                element.props.push(prop);
            }
            "end_header" => {
                return Ok((
                    elements,
                    format.ok_or_else(|| {
                        StageError("Expected format line not found in PLY file.".to_string())
                    })?,
                ));
            }
            other => {
                return Err(StageError(format!(
                    "Invalid keyword '{other}' when expecting an element."
                )));
            }
        }
    }
    Err(StageError(
        "'end_header' not found in PLY file.".to_string(),
    ))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    let marker = b"end_header";
    bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .map(|pos| {
            let after_marker = pos + marker.len();
            if bytes.get(after_marker) == Some(&b'\r')
                && bytes.get(after_marker + 1) == Some(&b'\n')
            {
                after_marker + 2
            } else if bytes.get(after_marker) == Some(&b'\n') {
                after_marker + 1
            } else {
                after_marker
            }
        })
}

/// Parse a `property` declaration line.
fn parse_property(words: &[&str], element_name: &str) -> Result<PlyProp, StageError> {
    match words.get(1).copied() {
        Some("list") => {
            let count_type = words.get(2).and_then(|w| ply_type(w));
            let list_type = words.get(3).and_then(|w| ply_type(w));
            if count_type.is_none() || list_type.is_none() {
                return Err(StageError(format!(
                    "Invalid list property for element '{element_name}'."
                )));
            }
            let name = words.get(4).ok_or_else(|| {
                StageError(format!("No name for property of element '{element_name}'."))
            })?;
            if words.get(5).is_some() {
                return Err(StageError(format!(
                    "Invalid list property for element '{element_name}'."
                )));
            }
            Ok(PlyProp::List {
                name: name.to_string(),
                count_ty: count_type.unwrap(),
                list_ty: list_type.unwrap(),
            })
        }
        Some(type_name) => {
            let ty = ply_type(type_name)
                .ok_or_else(|| StageError(format!("Invalid property type '{type_name}'.")))?;
            let name = words.get(2).ok_or_else(|| {
                StageError(format!("No name for property of element '{element_name}'."))
            })?;
            Ok(PlyProp::Simple {
                dim: dim_for(name),
                ty,
            })
        }
        None => Err(StageError(format!(
            "Invalid property declaration for element '{element_name}'."
        ))),
    }
}

/// Map a PLY type name to a storage type.
fn ply_type(name: &str) -> Option<DimType> {
    Some(match name {
        "int8" | "char" => DimType::I8,
        "uint8" | "uchar" => DimType::U8,
        "int16" | "short" => DimType::I16,
        "uint16" | "ushort" => DimType::U16,
        "int32" | "int" => DimType::I32,
        "uint32" | "uint" => DimType::U32,
        "float32" | "float" => DimType::F32,
        "float64" | "double" => DimType::F64,
        _ => return None,
    })
}

/// Resolve a PLY property name to a dimension, canonicalizing `x`/`y`/`z`.
fn dim_for(name: &str) -> DimId {
    match name.to_ascii_uppercase().as_str() {
        "X" => DimId::X,
        "Y" => DimId::Y,
        "Z" => DimId::Z,
        _ => DimId::from_name(name),
    }
}

/// Writer for the Stanford PLY format.
///
/// When `precision` is unset, floating values use Rust's shortest round-trip
/// formatting rather than PDAL's 6-significant-digit default.
pub struct PlyWriter {
    filename: String,
    format: Option<PlyFormat>,
    dim_specs: Vec<String>,
    sized_types: bool,
    precision: Option<usize>,
    faces: bool,
    point_count: u64,
    stream: Option<PlyStreamState>,
}

struct PlyStreamState {
    rows: BufWriter<File>,
    rows_path: String,
    dims: Vec<(DimId, DimType, String)>,
    format: PlyFormat,
}

impl PlyWriter {
    pub fn new(options: &Options) -> Result<Self, StageError> {
        let dim_specs = options
            .get_str("dims", "")
            .split(',')
            .map(|spec| spec.trim().to_string())
            .filter(|spec| !spec.is_empty())
            .collect();
        Self {
            filename: options.get_str("filename", ""),
            format: PlyFormat::from_writer_option(&options.get_str("storage_mode", "ascii")),
            dim_specs,
            sized_types: options.get_bool("sized_types", true),
            precision: options
                .has("precision")
                .then(|| options.get_u64("precision", 3) as usize),
            faces: options.get_bool("faces", false),
            point_count: 0,
            stream: None,
        }
        .validate_options()
    }

    fn validate_options(self) -> Result<Self, StageError> {
        let format = self
            .format
            .ok_or_else(|| StageError("Invalid PLY storage mode.".to_string()))?;
        if format != PlyFormat::Ascii && self.precision.is_some() {
            return Err(StageError(
                "Option 'precision' can only be set of the 'storage_mode' is ascii.".to_string(),
            ));
        }
        Ok(self)
    }

    /// Resolve the `(dimension, write type, property name)` triples to write.
    fn resolve_dims(
        &self,
        layout: &PointLayout,
    ) -> Result<Vec<(DimId, DimType, String)>, StageError> {
        if self.dim_specs.is_empty() {
            // Default: every layout dimension in order, with its stored type.
            let mut dims = Vec::new();
            for idx in 0..layout.dim_count() {
                if let Some((id, ty)) = layout.dim_at(idx) {
                    dims.push((id.clone(), ty, id.name().to_lowercase()));
                }
            }
            return Ok(dims);
        }

        let mut dims = Vec::new();
        for spec in &self.dim_specs {
            let mut parts = spec.splitn(2, '=');
            let name = parts.next().unwrap_or("").trim();
            let dim = dim_for(name);
            let stored = layout.dim(&dim).ok_or_else(|| {
                StageError(format!(
                    "Unknown dimension '{name}' in provided dimension list."
                ))
            })?;
            let ty = match parts.next() {
                Some(type_name) => writer_type(type_name.trim()).ok_or_else(|| {
                    StageError(format!(
                        "Invalid type '{type_name}' for dimension '{name}'."
                    ))
                })?,
                None => stored.1,
            };
            dims.push((dim, ty, name.to_lowercase()));
        }
        Ok(dims)
    }

    fn expanded_filename(&self, index: usize) -> String {
        self.filename.replace('#', &(index + 1).to_string())
    }

    fn header_bytes(
        &self,
        format: PlyFormat,
        dims: &[(DimId, DimType, String)],
        count: u64,
        face_count: usize,
    ) -> Result<Vec<u8>, StageError> {
        let mut output = Vec::new();
        append_line(&mut output, "ply");
        append_line(&mut output, &format!("format {} 1.0", format.header_name()));
        append_line(&mut output, "comment Generated by PDAL");
        append_line(&mut output, &format!("element vertex {count}"));
        for (_, ty, name) in dims {
            let type_string = ply_type_string(*ty, self.sized_types).ok_or_else(|| {
                StageError(format!("Can't write PLY dimension '{name}' of its type."))
            })?;
            append_line(&mut output, &format!("property {type_string} {name}"));
        }
        if self.faces {
            append_line(&mut output, &format!("element face {face_count}"));
            append_line(&mut output, "property list uint8 uint32 vertex_indices");
        }
        append_line(&mut output, "end_header");
        Ok(output)
    }

    fn write_vertex_rows<W: Write>(
        &mut self,
        writer: &mut W,
        views: &[PointView],
        format: PlyFormat,
        dims: &[(DimId, DimType, String)],
    ) -> Result<(), StageError> {
        let mut row = Vec::new();
        for view in views {
            for point in 0..view.len() {
                row.clear();
                for (idx, (dim, ty, _)) in dims.iter().enumerate() {
                    let precision = (format == PlyFormat::Ascii)
                        .then_some(self.precision)
                        .flatten();
                    write_ply_value(&mut row, format, view.get_f64(point, dim), *ty, precision)?;
                    if format == PlyFormat::Ascii && idx + 1 < dims.len() {
                        row.push(b' ');
                    }
                }
                if format == PlyFormat::Ascii {
                    row.push(b'\n');
                }
                writer.write_all(&row).map_err(|err| {
                    StageError(format!("Failed writing '{}': {err}", self.filename))
                })?;
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
                "pdal-rust-ply-stream-{}-{suffix}.rows",
                std::process::id()
            ))
            .display()
            .to_string()
    }

    fn write_file(
        &mut self,
        filename: &str,
        views: &[PointView],
        format: PlyFormat,
    ) -> Result<u64, StageError> {
        let count: u64 = views.iter().map(PointView::len).sum();
        let dims = match views.first() {
            Some(view) => self.resolve_dims(view.layout())?,
            None => Vec::new(),
        };

        let face_count: usize = if self.faces {
            views
                .iter()
                .filter_map(PointView::mesh)
                .map(|mesh| mesh.len())
                .sum()
        } else {
            0
        };
        let mut output = self.header_bytes(format, &dims, count, face_count)?;

        if format == PlyFormat::Ascii {
            self.point_count = 0;
            self.write_vertex_rows(&mut output, views, format, &dims)?;
        } else {
            self.write_vertex_rows(&mut output, views, format, &dims)?;
        }
        if self.faces {
            let mut point_offset = 0;
            for view in views {
                if let Some(mesh) = view.mesh() {
                    for triangle in mesh.triangles() {
                        write_triangle(&mut output, format, triangle, point_offset)?;
                    }
                }
                point_offset += view.len();
            }
        }

        fs::write(Path::new(filename), output)
            .map_err(|_| StageError(format!("Couldn't open file '{filename}' for output.")))?;
        Ok(count)
    }
}

impl Writer for PlyWriter {
    fn name(&self) -> &str {
        "writers.ply"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PlyWriter requires a filename option.".to_string(),
            ));
        }
        let format = self
            .format
            .ok_or_else(|| StageError("Invalid PLY storage mode.".to_string()))?;

        if self.filename.contains('#') {
            let mut count = 0;
            for (idx, view) in views.iter().enumerate() {
                count += self.write_file(
                    &self.expanded_filename(idx),
                    std::slice::from_ref(view),
                    format,
                )?;
            }
            self.point_count = count;
            return Ok(());
        }

        self.point_count = self.write_file(&self.filename.clone(), views, format)?;
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.ply");
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
        !self.filename.is_empty()
            && !self.filename.contains('#')
            && matches!(
                self.format,
                Some(PlyFormat::Ascii | PlyFormat::BinaryLittleEndian | PlyFormat::BinaryBigEndian)
            )
            && !self.faces
    }

    fn stream_write(&mut self, chunk: &PointView) -> Result<(), StageError> {
        if !self.streamable() {
            return Err(StageError(
                "PLY streaming is only supported for single-file vertex output.".to_string(),
            ));
        }
        let format = self
            .format
            .ok_or_else(|| StageError("Invalid PLY storage mode.".to_string()))?;
        if self.stream.is_none() {
            let dims = self.resolve_dims(chunk.layout())?;
            let rows_path = self.stream_rows_path();
            let rows = File::create(&rows_path)
                .map(BufWriter::new)
                .map_err(|err| StageError(format!("Failed creating PLY row stream: {err}")))?;
            self.stream = Some(PlyStreamState {
                rows,
                rows_path,
                dims,
                format,
            });
        }

        let mut state = self.stream.take().expect("stream initialized above");
        self.write_vertex_rows(
            &mut state.rows,
            std::slice::from_ref(chunk),
            state.format,
            &state.dims,
        )?;
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
            .map_err(|err| StageError(format!("Failed writing PLY row stream: {err}")))?;
        drop(state.rows);

        let mut output = File::create(Path::new(&self.filename))
            .map(BufWriter::new)
            .map_err(|_| {
                StageError(format!(
                    "Couldn't open file '{}' for output.",
                    self.filename
                ))
            })?;
        output
            .write_all(&self.header_bytes(state.format, &state.dims, self.point_count, 0)?)
            .map_err(|err| StageError(format!("Failed writing '{}': {err}", self.filename)))?;
        let mut rows = File::open(&state.rows_path)
            .map(BufReader::new)
            .map_err(|err| StageError(format!("Failed reopening PLY row stream: {err}")))?;
        std::io::copy(&mut rows, &mut output)
            .map_err(|err| StageError(format!("Failed writing '{}': {err}", self.filename)))?;
        output
            .flush()
            .map_err(|err| StageError(format!("Failed writing '{}': {err}", self.filename)))?;
        let _ = fs::remove_file(state.rows_path);
        Ok(())
    }
}

/// Map a PDAL type name (as used in the `dims` option) to a storage type.
fn writer_type(name: &str) -> Option<DimType> {
    Some(match name {
        "int8" | "int8_t" | "char" => DimType::I8,
        "uint8" | "uint8_t" | "uchar" => DimType::U8,
        "int16" | "int16_t" | "short" => DimType::I16,
        "uint16" | "uint16_t" | "ushort" => DimType::U16,
        "int32" | "int32_t" | "int" => DimType::I32,
        "uint32" | "uint32_t" | "uint" => DimType::U32,
        "int64" | "int64_t" => DimType::I64,
        "uint64" | "uint64_t" => DimType::U64,
        "float" | "float32" => DimType::F32,
        "double" | "float64" => DimType::F64,
        _ => return None,
    })
}

/// The PLY header type string for a storage type. PLY has no 64-bit integer
/// types, so those return `None` (matching PDAL's `getType` failure).
fn ply_type_string(ty: DimType, sized: bool) -> Option<&'static str> {
    Some(match (ty, sized) {
        (DimType::I8, true) => "int8",
        (DimType::I8, false) => "char",
        (DimType::U8, true) => "uint8",
        (DimType::U8, false) => "uchar",
        (DimType::I16, true) => "int16",
        (DimType::I16, false) => "short",
        (DimType::U16, true) => "uint16",
        (DimType::U16, false) => "ushort",
        (DimType::I32, true) => "int32",
        (DimType::I32, false) => "int",
        (DimType::U32, true) => "uint32",
        (DimType::U32, false) => "uint",
        (DimType::F32, true) => "float32",
        (DimType::F32, false) => "float",
        (DimType::F64, true) => "float64",
        (DimType::F64, false) => "double",
        (DimType::I64 | DimType::U64, _) => return None,
    })
}

fn append_line(output: &mut Vec<u8>, line: &str) {
    output.extend_from_slice(line.as_bytes());
    output.push(b'\n');
}

fn write_ply_value(
    output: &mut Vec<u8>,
    format: PlyFormat,
    value: f64,
    ty: DimType,
    precision: Option<usize>,
) -> Result<(), StageError> {
    if format == PlyFormat::Ascii {
        output.extend_from_slice(format_value(value, ty, precision).as_bytes());
        return Ok(());
    }

    let result = match (format, ty) {
        (_, DimType::I8) => output.write_i8(value.round() as i8),
        (_, DimType::U8) => output.write_u8(value.round().max(0.0) as u8),
        (PlyFormat::BinaryLittleEndian, DimType::I16) => {
            output.write_i16::<LittleEndian>(value.round() as i16)
        }
        (PlyFormat::BinaryBigEndian, DimType::I16) => {
            output.write_i16::<BigEndian>(value.round() as i16)
        }
        (PlyFormat::BinaryLittleEndian, DimType::U16) => {
            output.write_u16::<LittleEndian>(value.round().max(0.0) as u16)
        }
        (PlyFormat::BinaryBigEndian, DimType::U16) => {
            output.write_u16::<BigEndian>(value.round().max(0.0) as u16)
        }
        (PlyFormat::BinaryLittleEndian, DimType::I32) => {
            output.write_i32::<LittleEndian>(value.round() as i32)
        }
        (PlyFormat::BinaryBigEndian, DimType::I32) => {
            output.write_i32::<BigEndian>(value.round() as i32)
        }
        (PlyFormat::BinaryLittleEndian, DimType::U32) => {
            output.write_u32::<LittleEndian>(value.round().max(0.0) as u32)
        }
        (PlyFormat::BinaryBigEndian, DimType::U32) => {
            output.write_u32::<BigEndian>(value.round().max(0.0) as u32)
        }
        (PlyFormat::BinaryLittleEndian, DimType::F32) => {
            output.write_f32::<LittleEndian>(value as f32)
        }
        (PlyFormat::BinaryBigEndian, DimType::F32) => output.write_f32::<BigEndian>(value as f32),
        (PlyFormat::BinaryLittleEndian, DimType::F64) => output.write_f64::<LittleEndian>(value),
        (PlyFormat::BinaryBigEndian, DimType::F64) => output.write_f64::<BigEndian>(value),
        (PlyFormat::Ascii, _) | (_, DimType::I64 | DimType::U64) => {
            return Err(StageError(
                "Can't write PLY dimension of its type.".to_string(),
            ));
        }
    };
    result.map_err(|err| StageError(err.to_string()))
}

fn write_triangle(
    output: &mut Vec<u8>,
    format: PlyFormat,
    triangle: &pdal_core::point::Triangle,
    offset: u64,
) -> Result<(), StageError> {
    if format == PlyFormat::Ascii {
        append_line(
            output,
            &format!(
                "3 {} {} {}",
                triangle.a + offset,
                triangle.b + offset,
                triangle.c + offset
            ),
        );
        return Ok(());
    }

    output
        .write_u8(3)
        .map_err(|err| StageError(err.to_string()))?;
    match format {
        PlyFormat::BinaryLittleEndian => {
            output
                .write_u32::<LittleEndian>((triangle.a + offset) as u32)
                .map_err(|err| StageError(err.to_string()))?;
            output
                .write_u32::<LittleEndian>((triangle.b + offset) as u32)
                .map_err(|err| StageError(err.to_string()))?;
            output
                .write_u32::<LittleEndian>((triangle.c + offset) as u32)
                .map_err(|err| StageError(err.to_string()))
        }
        PlyFormat::BinaryBigEndian => {
            output
                .write_u32::<BigEndian>((triangle.a + offset) as u32)
                .map_err(|err| StageError(err.to_string()))?;
            output
                .write_u32::<BigEndian>((triangle.b + offset) as u32)
                .map_err(|err| StageError(err.to_string()))?;
            output
                .write_u32::<BigEndian>((triangle.c + offset) as u32)
                .map_err(|err| StageError(err.to_string()))
        }
        PlyFormat::Ascii => Ok(()),
    }
}

/// Format one value for ASCII output, honoring the dimension's write type.
fn format_value(value: f64, ty: DimType, precision: Option<usize>) -> String {
    match ty {
        DimType::F32 => {
            let value = value as f32 as f64;
            match precision {
                Some(p) => format!("{value:.p$}"),
                None => format!("{value}"),
            }
        }
        DimType::F64 => match precision {
            Some(p) => format!("{value:.p$}"),
            None => format!("{value}"),
        },
        // PDAL rounds when converting a value to an integer storage type.
        DimType::U8 | DimType::U16 | DimType::U32 | DimType::U64 => {
            format!("{}", value.round().max(0.0) as u64)
        }
        DimType::I8 | DimType::I16 | DimType::I32 | DimType::I64 => {
            format!("{}", value.round() as i64)
        }
    }
}

#[cfg(test)]
include!("ply_tests.rs");
#[cfg(test)]
include!("ply_stream_tests.rs");
