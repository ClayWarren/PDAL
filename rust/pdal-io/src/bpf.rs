//! `readers.bpf` / `writers.bpf` -- Binary Point Format.
//!
//! This slice covers deterministic local, uncompressed BPF files. Zlib,
//! remote files, bundled files, and polarization/ULEM metadata remain C++
//! territory until a later I/O checkpoint needs them.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::{Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BpfFormat {
    Dim = 0,
    Point = 1,
    Byte = 2,
}

#[derive(Clone, Debug)]
struct BpfDimension {
    offset: f64,
    min: f64,
    max: f64,
    label: String,
    id: DimId,
}

#[derive(Clone, Debug)]
struct BpfHeader {
    len: i32,
    num_dim: usize,
    format: BpfFormat,
    compression: u8,
    num_pts: usize,
    coord_type: i32,
    coord_id: i32,
    spacing: f32,
    xform: [f64; 16],
    start_time: f64,
    end_time: f64,
    static_dims: Vec<BpfDimension>,
}

impl Default for BpfHeader {
    fn default() -> Self {
        Self {
            len: 176,
            num_dim: 0,
            format: BpfFormat::Dim,
            compression: 0,
            num_pts: 0,
            coord_type: 0,
            coord_id: 0,
            spacing: 0.0,
            xform: identity_xform(),
            start_time: 0.0,
            end_time: 0.0,
            static_dims: Vec::new(),
        }
    }
}

pub struct BpfReader {
    filename: String,
    fix_dims: bool,
}

impl BpfReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            fix_dims: options.get_bool("fix_dims", true),
        }
    }
}

impl Reader for BpfReader {
    fn name(&self) -> &str {
        "readers.bpf"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "BpfReader requires a filename option.".to_string(),
            ));
        }

        let file = File::open(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Can't open file '{}'.", self.filename)))?;
        let mut reader = BufReader::new(file);
        let header = read_header(&mut reader)?;
        if header.compression != 0 {
            return Err(StageError(
                "Compressed BPF is not supported by the Rust local I/O slice.".to_string(),
            ));
        }
        let dims = read_dimensions(&mut reader, &header, self.fix_dims)?;

        reader
            .seek(SeekFrom::Start(header.len as u64))
            .map_err(io_error)?;
        let mut layout = PointLayout::new();
        for dim in &dims {
            layout.register(dim.id.clone(), DimType::F32);
        }
        let mut view = PointView::new(Rc::new(layout));
        view.set_spatial_reference(spatial_reference(&header));

        match header.format {
            BpfFormat::Point => read_point_major(&mut reader, &header, &dims, &mut view)?,
            BpfFormat::Dim => read_dim_major(&mut reader, &header, &dims, &mut view)?,
            BpfFormat::Byte => read_byte_major(&mut reader, &header, &dims, &mut view)?,
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.bpf")
    }
}

pub struct BpfWriter {
    filename: String,
    format: BpfFormat,
    coord_id: i32,
}

impl BpfWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            format: parse_format(&options.get_str("format", "dimension")),
            coord_id: parse_coord_id(&options.get_str("coord_id", "0")),
        }
    }
}

impl Writer for BpfWriter {
    fn name(&self) -> &str {
        "writers.bpf"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "BpfWriter requires a filename option.".to_string(),
            ));
        }
        let view = views
            .first()
            .ok_or_else(|| StageError("BpfWriter requires an input view.".to_string()))?;
        let dims = writer_dimensions(view)?;
        let mut header = BpfHeader {
            len: (176 + dims.len() * 56) as i32,
            num_dim: dims.len(),
            format: self.format,
            num_pts: view.len() as usize,
            coord_type: if self.coord_id == 0 { 0 } else { 1 },
            coord_id: self.coord_id,
            ..BpfHeader::default()
        };
        header.xform[0] = 1.0;
        header.xform[5] = 1.0;
        header.xform[10] = 1.0;

        let file = File::create(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}' for writing.", self.filename)))?;
        let mut writer = BufWriter::new(file);
        write_header(&mut writer, &header)?;
        write_dimensions(&mut writer, &dims)?;
        match self.format {
            BpfFormat::Point => write_point_major(&mut writer, view, &dims)?,
            BpfFormat::Dim => write_dim_major(&mut writer, view, &dims)?,
            BpfFormat::Byte => write_byte_major(&mut writer, view, &dims)?,
        }
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.bpf")
    }
}

fn read_header<R: Read + Seek>(reader: &mut R) -> Result<BpfHeader, StageError> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).map_err(io_error)?;
    reader.seek(SeekFrom::Start(0)).map_err(io_error)?;
    if &magic == b"BPF!" {
        read_v3_header(reader)
    } else {
        read_v1_header(reader)
    }
}

fn read_v3_header<R: Read>(reader: &mut R) -> Result<BpfHeader, StageError> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).map_err(io_error)?;
    if &magic != b"BPF!" {
        return Err(StageError("Invalid BPF magic.".to_string()));
    }
    let mut version = [0u8; 4];
    reader.read_exact(&mut version).map_err(io_error)?;
    let version = std::str::from_utf8(&version)
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    if version != 3 {
        return Err(StageError(format!("Unsupported BPF version {version}.")));
    }

    let len = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let num_dim = reader.read_u8().map_err(io_error)? as usize;
    let format = format_from_u8(reader.read_u8().map_err(io_error)?)?;
    let compression = reader.read_u8().map_err(io_error)?;
    let _dummy = reader.read_u8().map_err(io_error)?;
    let num_pts = reader.read_i32::<LittleEndian>().map_err(io_error)? as usize;
    let coord_type = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let coord_id = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let spacing = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    let xform = read_xform(reader)?;
    let start_time = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let end_time = reader.read_f64::<LittleEndian>().map_err(io_error)?;

    Ok(BpfHeader {
        len,
        num_dim,
        format,
        compression,
        num_pts,
        coord_type,
        coord_id,
        spacing,
        xform,
        start_time,
        end_time,
        static_dims: Vec::new(),
    })
}

fn read_v1_header<R: Read>(reader: &mut R) -> Result<BpfHeader, StageError> {
    let len = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let version = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let num_pts = reader.read_i32::<LittleEndian>().map_err(io_error)? as usize;
    let dynamic_dim = reader.read_i32::<LittleEndian>().map_err(io_error)? as usize;
    let coord_type = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let coord_id = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let spacing = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    let format = match version {
        1 => BpfFormat::Dim,
        2 => BpfFormat::Point,
        _ => return Err(StageError(format!("Unsupported BPF version {version}."))),
    };

    let mut static_dims = vec![
        BpfDimension::new("X"),
        BpfDimension::new("Y"),
        BpfDimension::new("Z"),
    ];
    for dim in &mut static_dims {
        dim.offset = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }
    for dim in &mut static_dims {
        dim.min = reader.read_f64::<LittleEndian>().map_err(io_error)?;
        dim.max = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }

    Ok(BpfHeader {
        len,
        num_dim: dynamic_dim + 3,
        format,
        compression: 0,
        num_pts,
        coord_type,
        coord_id,
        spacing,
        xform: identity_xform(),
        start_time: 0.0,
        end_time: 0.0,
        static_dims,
    })
}

fn read_dimensions<R: Read>(
    reader: &mut R,
    header: &BpfHeader,
    fix_dims: bool,
) -> Result<Vec<BpfDimension>, StageError> {
    let mut dims = header.static_dims.clone();
    if header.num_dim < dims.len() {
        return Err(StageError("BPF dimension range looks bad.".to_string()));
    }
    dims.resize_with(header.num_dim, || BpfDimension::new(""));
    let start = header.static_dims.len();
    for dim in dims.iter_mut().skip(start) {
        dim.offset = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }
    for dim in dims.iter_mut().skip(start) {
        dim.min = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }
    for dim in dims.iter_mut().skip(start) {
        dim.max = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }
    for dim in dims.iter_mut().skip(start) {
        let mut label = [0u8; 32];
        reader.read_exact(&mut label).map_err(io_error)?;
        dim.label = fixed_label(&label);
        if fix_dims {
            dim.label = fix_dimension_name(&dim.label);
        }
        dim.id = DimId::from_name(&dim.label);
    }

    if !dims.iter().any(|d| d.label == "X")
        || !dims.iter().any(|d| d.label == "Y")
        || !dims.iter().any(|d| d.label == "Z")
    {
        return Err(StageError(
            "BPF file missing at least one of X, Y or Z dimensions.".to_string(),
        ));
    }
    Ok(dims)
}

fn read_point_major<R: Read + Seek>(
    reader: &mut R,
    header: &BpfHeader,
    dims: &[BpfDimension],
    view: &mut PointView,
) -> Result<(), StageError> {
    reader
        .seek(SeekFrom::Start(header.len as u64))
        .map_err(io_error)?;
    for _ in 0..header.num_pts {
        let idx = view.add_point();
        let mut xyz = [0.0; 3];
        for dim in dims {
            let value = reader.read_f32::<LittleEndian>().map_err(io_error)? as f64 + dim.offset;
            set_or_capture_xyz(view, idx, dim, value, &mut xyz);
        }
        apply_xyz(view, idx, header, xyz);
    }
    Ok(())
}

fn read_dim_major<R: Read + Seek>(
    reader: &mut R,
    header: &BpfHeader,
    dims: &[BpfDimension],
    view: &mut PointView,
) -> Result<(), StageError> {
    for _ in 0..header.num_pts {
        view.add_point();
    }
    for (dim_index, dim) in dims.iter().enumerate() {
        let offset = header.len as u64 + (dim_index * header.num_pts * 4) as u64;
        reader.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        for idx in 0..header.num_pts {
            let value = reader.read_f32::<LittleEndian>().map_err(io_error)? as f64 + dim.offset;
            view.set_f64(idx as u64, &dim.id, value);
        }
    }
    transform_all_xyz(view, header);
    Ok(())
}

fn read_byte_major<R: Read + Seek>(
    reader: &mut R,
    header: &BpfHeader,
    dims: &[BpfDimension],
    view: &mut PointView,
) -> Result<(), StageError> {
    for _ in 0..header.num_pts {
        view.add_point();
    }
    let dim_stride = header.num_pts * 4;
    for (dim_index, dim) in dims.iter().enumerate() {
        let mut bytes = vec![[0u8; 4]; header.num_pts];
        for byte_index in 0..4 {
            let offset =
                header.len as u64 + (dim_index * dim_stride + byte_index * header.num_pts) as u64;
            reader.seek(SeekFrom::Start(offset)).map_err(io_error)?;
            for point_bytes in &mut bytes {
                point_bytes[byte_index] = reader.read_u8().map_err(io_error)?;
            }
        }
        for (idx, point_bytes) in bytes.iter().enumerate() {
            let value = f32::from_le_bytes(*point_bytes) as f64 + dim.offset;
            view.set_f64(idx as u64, &dim.id, value);
        }
    }
    transform_all_xyz(view, header);
    Ok(())
}

fn set_or_capture_xyz(
    view: &mut PointView,
    idx: u64,
    dim: &BpfDimension,
    value: f64,
    xyz: &mut [f64; 3],
) {
    match dim.id {
        DimId::X => xyz[0] = value,
        DimId::Y => xyz[1] = value,
        DimId::Z => xyz[2] = value,
        _ => view.set_f64(idx, &dim.id, value),
    }
}

fn transform_all_xyz(view: &mut PointView, header: &BpfHeader) {
    for idx in 0..view.len() {
        let xyz = [
            view.get_f64(idx, &DimId::X),
            view.get_f64(idx, &DimId::Y),
            view.get_f64(idx, &DimId::Z),
        ];
        apply_xyz(view, idx, header, xyz);
    }
}

fn apply_xyz(view: &mut PointView, idx: u64, header: &BpfHeader, mut xyz: [f64; 3]) {
    apply_xform(&mut xyz, &header.xform);
    view.set_f64(idx, &DimId::X, xyz[0]);
    view.set_f64(idx, &DimId::Y, xyz[1]);
    view.set_f64(idx, &DimId::Z, xyz[2]);
}

fn apply_xform(xyz: &mut [f64; 3], xform: &[f64; 16]) {
    let w = xyz[0] * xform[12] + xyz[1] * xform[13] + xyz[2] * xform[14] + xform[15];
    xyz[0] = (xyz[0] * xform[0] + xyz[1] * xform[1] + xyz[2] * xform[2] + xform[3]) / w;
    xyz[1] = (xyz[0] * xform[4] + xyz[1] * xform[5] + xyz[2] * xform[6] + xform[7]) / w;
    xyz[2] = (xyz[0] * xform[8] + xyz[1] * xform[9] + xyz[2] * xform[10] + xform[11]) / w;
}

fn writer_dimensions(view: &PointView) -> Result<Vec<BpfDimension>, StageError> {
    let mut dims = Vec::new();
    for idx in 0..view.layout().dim_count() {
        let Some((id, _ty)) = view.layout().dim_at(idx) else {
            continue;
        };
        dims.push((
            idx,
            BpfDimension {
                offset: 0.0,
                min: min_max(view, id).0,
                max: min_max(view, id).1,
                label: id.name().to_string(),
                id: id.clone(),
            },
        ));
    }
    if !dims.iter().any(|(_, d)| d.id == DimId::X)
        || !dims.iter().any(|(_, d)| d.id == DimId::Y)
        || !dims.iter().any(|(_, d)| d.id == DimId::Z)
    {
        return Err(StageError(
            "Missing one of dimensions X, Y or Z. Can't write BPF.".to_string(),
        ));
    }
    dims.sort_by_key(|(idx, dim)| (xyz_sort_key(&dim.id), *idx));
    Ok(dims.into_iter().map(|(_, dim)| dim).collect())
}

fn write_header<W: Write>(writer: &mut W, header: &BpfHeader) -> Result<(), StageError> {
    writer.write_all(b"BPF!").map_err(io_error)?;
    writer.write_all(b"0003").map_err(io_error)?;
    writer
        .write_i32::<LittleEndian>(header.len)
        .map_err(io_error)?;
    writer.write_u8(header.num_dim as u8).map_err(io_error)?;
    writer.write_u8(header.format as u8).map_err(io_error)?;
    writer.write_u8(header.compression).map_err(io_error)?;
    writer.write_u8(0).map_err(io_error)?;
    writer
        .write_i32::<LittleEndian>(header.num_pts as i32)
        .map_err(io_error)?;
    writer
        .write_i32::<LittleEndian>(header.coord_type)
        .map_err(io_error)?;
    writer
        .write_i32::<LittleEndian>(header.coord_id)
        .map_err(io_error)?;
    writer
        .write_f32::<LittleEndian>(header.spacing)
        .map_err(io_error)?;
    for value in header.xform {
        writer.write_f64::<LittleEndian>(value).map_err(io_error)?;
    }
    writer
        .write_f64::<LittleEndian>(header.start_time)
        .map_err(io_error)?;
    writer
        .write_f64::<LittleEndian>(header.end_time)
        .map_err(io_error)?;
    Ok(())
}

fn write_dimensions<W: Write>(writer: &mut W, dims: &[BpfDimension]) -> Result<(), StageError> {
    for dim in dims {
        writer
            .write_f64::<LittleEndian>(dim.offset)
            .map_err(io_error)?;
    }
    for dim in dims {
        writer
            .write_f64::<LittleEndian>(dim.min)
            .map_err(io_error)?;
    }
    for dim in dims {
        writer
            .write_f64::<LittleEndian>(dim.max)
            .map_err(io_error)?;
    }
    for dim in dims {
        let mut label = [0u8; 32];
        let bytes = dim.label.as_bytes();
        let len = bytes.len().min(label.len());
        label[..len].copy_from_slice(&bytes[..len]);
        writer.write_all(&label).map_err(io_error)?;
    }
    Ok(())
}

fn write_point_major<W: Write>(
    writer: &mut W,
    view: &PointView,
    dims: &[BpfDimension],
) -> Result<(), StageError> {
    for idx in 0..view.len() {
        for dim in dims {
            writer
                .write_f32::<LittleEndian>(view.get_f64(idx, &dim.id) as f32)
                .map_err(io_error)?;
        }
    }
    Ok(())
}

fn write_dim_major<W: Write>(
    writer: &mut W,
    view: &PointView,
    dims: &[BpfDimension],
) -> Result<(), StageError> {
    for dim in dims {
        for idx in 0..view.len() {
            writer
                .write_f32::<LittleEndian>(view.get_f64(idx, &dim.id) as f32)
                .map_err(io_error)?;
        }
    }
    Ok(())
}

fn write_byte_major<W: Write>(
    writer: &mut W,
    view: &PointView,
    dims: &[BpfDimension],
) -> Result<(), StageError> {
    for dim in dims {
        for byte_index in 0..4 {
            for idx in 0..view.len() {
                let bytes = (view.get_f64(idx, &dim.id) as f32).to_le_bytes();
                writer.write_u8(bytes[byte_index]).map_err(io_error)?;
            }
        }
    }
    Ok(())
}

fn read_xform<R: Read>(reader: &mut R) -> Result<[f64; 16], StageError> {
    let mut xform = [0.0; 16];
    for value in &mut xform {
        *value = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }
    Ok(xform)
}

fn identity_xform() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn format_from_u8(value: u8) -> Result<BpfFormat, StageError> {
    match value {
        0 => Ok(BpfFormat::Dim),
        1 => Ok(BpfFormat::Point),
        2 => Ok(BpfFormat::Byte),
        _ => Err(StageError(
            "Invalid BPF file: unknown interleave type.".to_string(),
        )),
    }
}

fn parse_format(value: &str) -> BpfFormat {
    match value.to_ascii_uppercase().as_str() {
        "POINT" => BpfFormat::Point,
        "BYTE" => BpfFormat::Byte,
        "DIM" | "DIMENSION" => BpfFormat::Dim,
        _ => BpfFormat::Dim,
    }
}

fn parse_coord_id(value: &str) -> i32 {
    if value.eq_ignore_ascii_case("auto") {
        0
    } else {
        value.parse().unwrap_or(0)
    }
}

fn fixed_label(bytes: &[u8; 32]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end])
        .trim_end()
        .to_string()
}

fn fix_dimension_name(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for (idx, ch) in label.chars().enumerate() {
        let valid = ch == '_' || ch.is_ascii_alphanumeric();
        if valid && (idx > 0 || !ch.is_ascii_digit()) {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

fn spatial_reference(header: &BpfHeader) -> SpatialReference {
    match header.coord_type {
        0 => SpatialReference::new("EPSG:4326"),
        1 if header.coord_id > 0 => {
            SpatialReference::new(&format!("EPSG:326{:02}", header.coord_id))
        }
        1 if header.coord_id < 0 => {
            SpatialReference::new(&format!("EPSG:327{:02}", -header.coord_id))
        }
        2 if header.coord_id == 1 => SpatialReference::new("EPSG:4978"),
        _ => SpatialReference::default(),
    }
}

fn min_max(view: &PointView, dim: &DimId) -> (f64, f64) {
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for idx in 0..view.len() {
        let value = view.get_f64(idx, dim);
        min = min.min(value);
        max = max.max(value);
    }
    (min, max)
}

fn xyz_sort_key(id: &DimId) -> usize {
    match id {
        DimId::X => 0,
        DimId::Y => 1,
        DimId::Z => 2,
        _ => 3,
    }
}

impl BpfDimension {
    fn new(label: &str) -> Self {
        Self {
            offset: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            label: label.to_string(),
            id: DimId::from_name(label),
        }
    }
}

fn io_error(error: std::io::Error) -> StageError {
    StageError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::pipeline::Writer;
    use pdal_core::point::DimId;

    fn data_path(name: &str) -> String {
        format!("{}/../../test/data/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("pdal-rust-bpf-{}-{name}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    fn read_bpf(path: &str) -> PointView {
        let mut options = Options::new();
        options.add("filename", path);
        let mut reader = BpfReader::new(&options);
        reader.read().unwrap().remove(0)
    }

    #[test]
    fn reads_uncompressed_dim_major_bpf() {
        let view = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));

        assert_eq!(view.len(), 1065);
        assert!((view.get_f64(0, &DimId::X) - 494057.30).abs() < 0.25);
        assert!((view.get_f64(0, &DimId::Y) - 4877433.35).abs() < 0.25);
        assert!((view.get_f64(0, &DimId::Z) - 130.63).abs() < 0.01);
        assert!(view.layout().dim(&DimId::Intensity).is_some());
    }

    #[test]
    fn reads_uncompressed_point_major_bpf() {
        let view = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3-interleaved.bpf"));

        assert_eq!(view.len(), 1065);
        assert!((view.get_f64(1, &DimId::X) - 494133.82).abs() < 0.25);
        assert!((view.get_f64(1, &DimId::Y) - 4877439.82).abs() < 0.25);
    }

    #[test]
    fn reads_uncompressed_byte_major_bpf() {
        let view = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3-segregated.bpf"));

        assert_eq!(view.len(), 1065);
        assert!((view.get_f64(2, &DimId::Z) - 130.46).abs() < 0.01);
    }

    #[test]
    fn writer_roundtrips_each_interleave() {
        let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));

        for format in ["dimension", "point", "byte"] {
            let output = temp_path(&format!("roundtrip-{format}.bpf"));
            let mut options = Options::new();
            options.add("filename", &output);
            options.add("format", format);
            let mut writer = BpfWriter::new(&options);
            writer.write(std::slice::from_ref(&input)).unwrap();

            let roundtrip = read_bpf(&output);
            assert_eq!(roundtrip.len(), input.len());
            for idx in [0, 17, 1064] {
                for dim in [DimId::X, DimId::Y, DimId::Z, DimId::Intensity] {
                    assert!(
                        (roundtrip.get_f64(idx, &dim) - input.get_f64(idx, &dim)).abs() < 0.01,
                        "format {format}, idx {idx}, dim {}",
                        dim.name()
                    );
                }
            }
            std::fs::remove_file(output).ok();
        }
    }

    #[test]
    fn compressed_bpf_is_deferred() {
        let mut options = Options::new();
        options.add(
            "filename",
            data_path("bpf/autzen-utm-chipped-25-v3-deflate.bpf"),
        );
        let mut reader = BpfReader::new(&options);
        let err = match reader.read() {
            Ok(_) => panic!("compressed BPF unexpectedly read successfully"),
            Err(err) => err,
        };

        assert!(err.0.contains("Compressed BPF"));
    }
}
