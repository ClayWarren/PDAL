use super::{parse_header, Element, PlyFormat, PlyProp};
use crate::source;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::rc::Rc;

pub(super) struct PlyReaderStreamState {
    reader: BufReader<Box<dyn source::ReadSeek>>,
    ascii_tokens: VecDeque<String>,
    layout: Rc<PointLayout>,
    vertex: Element,
    format: PlyFormat,
    remaining: usize,
}

pub(super) fn streamable(filename: &str) -> bool {
    open_vertex_stream(filename)
        .and_then(|(_reader, elements, _format)| streamable_vertex_element(&elements).map(|_| ()))
        .is_ok()
}

pub(super) fn stream_init(filename: &str) -> Result<PlyReaderStreamState, StageError> {
    let (reader, elements, format) = open_vertex_stream(filename)?;
    let vertex = streamable_vertex_element(&elements)?.clone();
    let layout = layout_for_vertex(&vertex);
    Ok(PlyReaderStreamState {
        reader,
        ascii_tokens: VecDeque::new(),
        layout,
        remaining: vertex.count,
        vertex,
        format,
    })
}

pub(super) fn stream_next(
    state: &mut PlyReaderStreamState,
    capacity: usize,
) -> Result<Option<PointView>, StageError> {
    if state.remaining == 0 {
        return Ok(None);
    }

    let mut view = PointView::new(Rc::clone(&state.layout));
    let target = capacity.max(1).min(state.remaining);
    for _ in 0..target {
        let point = view.add_point();
        for prop in state.vertex.props.clone() {
            let PlyProp::Simple { dim, ty } = prop else {
                return Err(StageError(
                    "PLY streaming does not support list properties.".to_string(),
                ));
            };
            let value = read_value(state, ty)?;
            view.set_f64(point, &dim, value);
        }
        state.remaining -= 1;
    }
    Ok(Some(view))
}

fn open_vertex_stream(
    filename: &str,
) -> Result<
    (
        BufReader<Box<dyn source::ReadSeek>>,
        Vec<Element>,
        PlyFormat,
    ),
    StageError,
> {
    if filename.is_empty() {
        return Err(StageError(
            "PlyReader requires a filename option.".to_string(),
        ));
    }
    let file = source::open_seek(filename)
        .map_err(|_| StageError(format!("Couldn't open '{filename}'.")))?;
    let mut reader = BufReader::new(file);
    let mut header = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| StageError(err.to_string()))?;
        if read == 0 {
            return Err(StageError(
                "'end_header' not found in PLY file.".to_string(),
            ));
        }
        header.push_str(&line);
        if line.trim() == "end_header" {
            break;
        }
    }
    let (elements, format) = parse_header(&header)?;
    Ok((reader, elements, format))
}

fn streamable_vertex_element(elements: &[Element]) -> Result<&Element, StageError> {
    let vertex = elements
        .iter()
        .find(|element| element.name == "vertex")
        .ok_or_else(|| StageError("Can't read PLY file without a 'vertex' element.".to_string()))?;
    if vertex
        .props
        .iter()
        .any(|prop| matches!(prop, PlyProp::List { .. }))
    {
        return Err(StageError(
            "PLY streaming does not support list properties.".to_string(),
        ));
    }
    if elements
        .iter()
        .any(|element| element.name != "vertex" && element.count > 0)
    {
        return Err(StageError(
            "PLY streaming does not support non-vertex elements.".to_string(),
        ));
    }
    Ok(vertex)
}

fn read_value(state: &mut PlyReaderStreamState, ty: DimType) -> Result<f64, StageError> {
    match state.format {
        PlyFormat::Ascii => read_ascii_value(state, ty),
        PlyFormat::BinaryLittleEndian | PlyFormat::BinaryBigEndian => {
            read_binary_value(&mut state.reader, state.format, ty)
        }
    }
}

fn read_ascii_value(state: &mut PlyReaderStreamState, ty: DimType) -> Result<f64, StageError> {
    while state.ascii_tokens.is_empty() {
        let mut line = String::new();
        let read = state
            .reader
            .read_line(&mut line)
            .map_err(|err| StageError(err.to_string()))?;
        if read == 0 {
            return Err(StageError(
                "Error reading data for the 'vertex' element.".to_string(),
            ));
        }
        state.ascii_tokens.extend(
            line.split_whitespace()
                .map(std::string::ToString::to_string),
        );
    }

    let token = state.ascii_tokens.pop_front().expect("checked above");
    parse_ascii_token(&token, ty)
}

fn parse_ascii_token(token: &str, ty: DimType) -> Result<f64, StageError> {
    match ty {
        DimType::I8
        | DimType::U8
        | DimType::I16
        | DimType::U16
        | DimType::I32
        | DimType::U32
        | DimType::F32
        | DimType::F64 => token
            .parse::<f64>()
            .map_err(|_| StageError("Error reading PLY ASCII value.".to_string())),
        DimType::I64 | DimType::U64 => Err(StageError(
            "Unsupported ASCII PLY property type.".to_string(),
        )),
    }
}

fn layout_for_vertex(vertex: &Element) -> Rc<PointLayout> {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    for prop in &vertex.props {
        if let PlyProp::Simple { dim, ty } = prop {
            layout.register(dim.clone(), *ty);
        }
    }
    Rc::new(layout)
}

fn read_binary_value<R: Read>(
    reader: &mut R,
    format: PlyFormat,
    ty: DimType,
) -> Result<f64, StageError> {
    let value = match (format, ty) {
        (_, DimType::I8) => reader.read_i8().map(f64::from),
        (_, DimType::U8) => reader.read_u8().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::I16) => {
            reader.read_i16::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::I16) => reader.read_i16::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::U16) => {
            reader.read_u16::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::U16) => reader.read_u16::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::I32) => {
            reader.read_i32::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::I32) => reader.read_i32::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::U32) => {
            reader.read_u32::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::U32) => reader.read_u32::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::F32) => {
            reader.read_f32::<LittleEndian>().map(f64::from)
        }
        (PlyFormat::BinaryBigEndian, DimType::F32) => reader.read_f32::<BigEndian>().map(f64::from),
        (PlyFormat::BinaryLittleEndian, DimType::F64) => reader.read_f64::<LittleEndian>(),
        (PlyFormat::BinaryBigEndian, DimType::F64) => reader.read_f64::<BigEndian>(),
        (PlyFormat::Ascii, _) | (_, DimType::I64 | DimType::U64) => {
            return Err(StageError(
                "Unsupported binary PLY property type.".to_string(),
            ));
        }
    };
    value.map_err(|err| StageError(err.to_string()))
}
