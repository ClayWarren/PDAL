//! `readers.ply` -- Stanford PLY format (ASCII).
//!
//! Port of the ASCII path of `io/PlyReader.cpp`. A PLY file has a text header
//! (`ply`, `format`, `element`/`property` declarations, `end_header`) followed
//! by a data section. The reader emits the `vertex` element's instances as
//! points; elements declared before `vertex` are consumed to reach it, and
//! face lists can be stored in the view mesh.
//!
//! Binary PLY (`binary_little_endian` / `binary_big_endian`) is intentionally
//! deferred -- this slice covers the deterministic ASCII path only.

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::{Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;
use std::str::SplitWhitespace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

/// One declared property of a PLY element.
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
}

impl PlyReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
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
        let bytes = fs::read(Path::new(&self.filename))
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
            if element_name == "vertex" {
                return Err(StageError(
                    "List properties are not supported for the 'vertex' element.".to_string(),
                ));
            }
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

/// Writer for the Stanford PLY format (ASCII encoding).
///
/// Port of the ASCII path of `io/PlyWriter.cpp`. Binary storage modes are
/// intentionally deferred. When `precision` is unset, floating values use
/// Rust's shortest round-trip formatting rather than PDAL's
/// 6-significant-digit default.
pub struct PlyWriter {
    filename: String,
    storage_mode: String,
    dim_specs: Vec<String>,
    sized_types: bool,
    precision: Option<usize>,
    faces: bool,
    point_count: u64,
}

impl PlyWriter {
    pub fn new(options: &Options) -> Self {
        let dim_specs = options
            .get_str("dims", "")
            .split(',')
            .map(|spec| spec.trim().to_string())
            .filter(|spec| !spec.is_empty())
            .collect();
        Self {
            filename: options.get_str("filename", ""),
            storage_mode: options.get_str("storage_mode", "ascii").to_lowercase(),
            dim_specs,
            sized_types: options.get_bool("sized_types", true),
            precision: options
                .has("precision")
                .then(|| options.get_u64("precision", 3) as usize),
            faces: options.get_bool("faces", false),
            point_count: 0,
        }
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
        if self.storage_mode != "ascii" {
            return Err(StageError(format!(
                "PLY storage mode '{}' is not supported by the Rust ASCII slice.",
                self.storage_mode
            )));
        }

        let count: u64 = views.iter().map(PointView::len).sum();
        self.point_count = count;
        let dims = match views.first() {
            Some(view) => self.resolve_dims(view.layout())?,
            None => Vec::new(),
        };

        let mut output = String::new();
        output.push_str("ply\nformat ascii 1.0\ncomment Generated by PDAL\n");
        output.push_str(&format!("element vertex {count}\n"));
        for (_, ty, name) in &dims {
            let type_string = ply_type_string(*ty, self.sized_types).ok_or_else(|| {
                StageError(format!("Can't write PLY dimension '{name}' of its type."))
            })?;
            output.push_str(&format!("property {type_string} {name}\n"));
        }
        let face_count: usize = if self.faces {
            views
                .iter()
                .filter_map(PointView::mesh)
                .map(|mesh| mesh.len())
                .sum()
        } else {
            0
        };
        if self.faces {
            output.push_str(&format!(
                "element face {face_count}\nproperty list uint8 uint32 vertex_indices\n"
            ));
        }
        output.push_str("end_header\n");

        for view in views {
            for point in 0..view.len() {
                for (idx, (dim, ty, _)) in dims.iter().enumerate() {
                    if idx > 0 {
                        output.push(' ');
                    }
                    output.push_str(&format_value(view.get_f64(point, dim), *ty, self.precision));
                }
                output.push('\n');
            }
        }
        if self.faces {
            let mut point_offset = 0;
            for view in views {
                if let Some(mesh) = view.mesh() {
                    for triangle in mesh.triangles() {
                        output.push_str(&format!(
                            "3 {} {} {}\n",
                            triangle.a + point_offset,
                            triangle.b + point_offset,
                            triangle.c + point_offset
                        ));
                    }
                }
                point_offset += view.len();
            }
        }

        fs::write(Path::new(&self.filename), output).map_err(|_| {
            StageError(format!(
                "Couldn't open file '{}' for output.",
                self.filename
            ))
        })
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.ply");
        node.add_value("filename", MetadataValue::String(self.filename.clone()));
        node.add_value("point_count", MetadataValue::U64(self.point_count));
        node
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
mod tests {
    use super::*;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn read_ply(path: &str) -> PointView {
        let mut options = Options::new();
        options.add("filename", data_path(path));
        let mut reader = PlyReader::new(&options);
        let mut views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        views.pop().unwrap()
    }

    #[test]
    fn reads_ascii_text_vertices() {
        let view = read_ply("ply/simple_text.ply");
        assert_eq!(view.len(), 3);

        for (idx, (x, y, z)) in [(-1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)]
            .into_iter()
            .enumerate()
        {
            let idx = idx as u64;
            assert_eq!(view.get_f64(idx, &DimId::X), x);
            assert_eq!(view.get_f64(idx, &DimId::Y), y);
            assert_eq!(view.get_f64(idx, &DimId::Z), z);
        }
    }

    #[test]
    fn reads_extra_dimensions_and_empty_face_element() {
        let view = read_ply("ply/text_extradim.ply");
        assert_eq!(view.len(), 1);

        assert_eq!(view.get_f64(0, &DimId::X), -2.64944);
        assert_eq!(view.get_f64(0, &DimId::Y), -13.0955);
        assert_eq!(view.get_f64(0, &DimId::Z), 0.00640115);
        assert_eq!(view.get_f64(0, &DimId::from_name("red")), 63.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("green")), 200.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("blue")), 64.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("alpha")), 255.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("omg")), 1234.0);
    }

    #[test]
    fn reads_ascii_mesh_faces() {
        let view = read_ply("ply/mesh.ply");
        assert_eq!(view.len(), 4);

        let mesh = view.mesh().unwrap();
        assert_eq!(mesh.len(), 2);
        assert_eq!(mesh.triangles()[0].a, 0);
        assert_eq!(mesh.triangles()[0].b, 1);
        assert_eq!(mesh.triangles()[0].c, 2);
        assert_eq!(mesh.triangles()[1].a, 1);
        assert_eq!(mesh.triangles()[1].b, 2);
        assert_eq!(mesh.triangles()[1].c, 3);
    }

    #[test]
    fn reads_sized_dimensions_with_xyz_forced_to_double() {
        let view = read_ply("ply/sized_dims.ply");
        assert_eq!(view.len(), 1);

        // `x` is declared int8 but X is always stored as a double.
        assert_eq!(view.get_f64(0, &DimId::X), 1.0);
        assert_eq!(view.get_f64(0, &DimId::Y), 12346.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("j")), 12345.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("i")), 1234567890.0);
    }

    #[test]
    fn rejects_a_file_without_a_vertex_element() {
        let mut options = Options::new();
        options.add("filename", data_path("ply/no_vertex.ply"));
        assert!(PlyReader::new(&options).read().is_err());
    }

    #[test]
    fn reads_binary_little_endian_vertices() {
        let view = read_ply("ply/simple_binary.ply");
        assert_eq!(view.len(), 3);

        for (idx, (x, y, z)) in [(-1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)]
            .into_iter()
            .enumerate()
        {
            let idx = idx as u64;
            assert_eq!(view.get_f64(idx, &DimId::X), x);
            assert_eq!(view.get_f64(idx, &DimId::Y), y);
            assert_eq!(view.get_f64(idx, &DimId::Z), z);
        }
    }

    fn temp_path(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-ply-{}-{name}", std::process::id()));
        let _ = fs::remove_file(&path);
        path.display().to_string()
    }

    fn xyz_view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for &(x, y, z) in points {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, x);
            view.set_f64(point, &DimId::Y, y);
            view.set_f64(point, &DimId::Z, z);
        }
        view
    }

    fn read_back(path: &str) -> PointView {
        let mut options = Options::new();
        options.add("filename", path);
        PlyReader::new(&options).read().unwrap().pop().unwrap()
    }

    #[test]
    fn writes_ascii_ply_that_reader_round_trips() {
        let view = xyz_view(&[(-1.5, 0.0, 0.25), (0.0, 1.0, 2.0), (3.5, -4.25, 5.0)]);
        let output = temp_path("roundtrip.ply");

        let mut options = Options::new();
        options.add("filename", &output).add("precision", 6);
        PlyWriter::new(&options).write(&[view]).unwrap();

        let back = read_back(&output);
        assert_eq!(back.len(), 3);
        assert_eq!(back.get_f64(0, &DimId::X), -1.5);
        assert_eq!(back.get_f64(0, &DimId::Z), 0.25);
        assert_eq!(back.get_f64(2, &DimId::X), 3.5);
        assert_eq!(back.get_f64(2, &DimId::Y), -4.25);
    }

    #[test]
    fn writes_ascii_mesh_faces_matching_existing_fixture() {
        let mut view = xyz_view(&[
            (1.0, 1.0, 0.0),
            (2.0, 1.0, 0.0),
            (1.0, 2.0, 0.0),
            (2.0, 2.0, 2.0),
        ]);
        let mesh = view.create_mesh();
        mesh.add(0, 1, 2);
        mesh.add(1, 2, 3);

        let output = temp_path("mesh.ply");
        let mut options = Options::new();
        options.add("filename", &output).add("faces", true);
        PlyWriter::new(&options).write(&[view]).unwrap();

        let written = fs::read_to_string(&output).unwrap();
        let expected = fs::read_to_string(data_path("ply/mesh.ply")).unwrap();
        assert_eq!(written, expected);
    }

    #[test]
    fn writes_ascii_mesh_faces_with_precision() {
        let mut view = xyz_view(&[
            (1.0, 1.0, 0.0),
            (2.0, 1.0, 0.0),
            (1.0, 2.0, 0.0),
            (2.0, 2.0, 2.0),
        ]);
        let mesh = view.create_mesh();
        mesh.add(0, 1, 2);
        mesh.add(1, 2, 3);

        let output = temp_path("mesh-fixed.ply");
        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("faces", true)
            .add("precision", 3);
        PlyWriter::new(&options).write(&[view]).unwrap();

        let written = fs::read_to_string(&output).unwrap();
        let expected = fs::read_to_string(data_path("ply/mesh_fixed.ply")).unwrap();
        assert_eq!(written, expected);
    }

    #[test]
    fn dims_option_selects_and_orders_properties() {
        let view = xyz_view(&[(1.0, 2.0, 3.0)]);
        let output = temp_path("dimorder.ply");

        let mut options = Options::new();
        options
            .add("filename", &output)
            .add("precision", 3)
            .add("dims", "Z,X");
        PlyWriter::new(&options).write(&[view]).unwrap();

        let written = fs::read_to_string(&output).unwrap();
        let header: Vec<&str> = written.lines().collect();
        assert!(header.contains(&"property float64 z"));
        assert!(header.contains(&"property float64 x"));
        assert!(!header.contains(&"property float64 y"));

        let back = read_back(&output);
        assert_eq!(back.get_f64(0, &DimId::X), 1.0);
        assert_eq!(back.get_f64(0, &DimId::Z), 3.0);
    }

    #[test]
    fn rejects_a_binary_storage_mode() {
        let view = xyz_view(&[(1.0, 1.0, 1.0)]);
        let mut options = Options::new();
        options
            .add("filename", temp_path("binary.ply"))
            .add("storage_mode", "binary_little_endian");
        assert!(PlyWriter::new(&options).write(&[view]).is_err());
    }
}
