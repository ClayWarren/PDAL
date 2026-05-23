//! `readers.fbi` -- TerraScan Fast Binary format.
//!
//! Port of `io/FbiReader.cpp`. FBI stores each dimension as a separate
//! little-endian stream. Header offsets point at the start of each stream.

use byteorder::{LittleEndian, ReadBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

const HEADER_SIZE: u32 = 1808;
const SIGNATURE: &[u8; 8] = b"FASTBIN\0";

#[derive(Clone)]
pub(crate) struct FbiHeader {
    pub(crate) version: u32,
    pub(crate) hdr_size: u32,
    pub(crate) time_type: u32,
    pub(crate) order: u32,
    pub(crate) reserved1: u32,
    pub(crate) vlr_cnt: u32,
    pub(crate) vlr_size: u32,
    pub(crate) rec_size: u32,
    pub(crate) fast_cnt: u64,
    pub(crate) rec_cnt: u64,
    pub(crate) units_xyz: u32,
    pub(crate) units_distance: u32,
    pub(crate) org_x: f64,
    pub(crate) org_y: f64,
    pub(crate) org_z: f64,
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
    pub(crate) min_z: f64,
    pub(crate) max_z: f64,
    pub(crate) system: [u8; 32],
    pub(crate) software: [u8; 32],
    pub(crate) reserved2: [u8; 64],
    pub(crate) bits_x: u32,
    pub(crate) bits_y: u32,
    pub(crate) bits_z: u32,
    pub(crate) bits_time: u32,
    pub(crate) bits_distance: u32,
    pub(crate) bits_group: u32,
    pub(crate) bits_normal: u32,
    pub(crate) bits_color: u32,
    pub(crate) bits_intensity: u32,
    pub(crate) bits_line: u32,
    pub(crate) bits_echo_len: u32,
    pub(crate) bits_amplitude: u32,
    pub(crate) bits_scanner: u32,
    pub(crate) bits_echo: u32,
    pub(crate) bits_angle: u32,
    pub(crate) bits_echo_norm: u32,
    pub(crate) bits_class: u32,
    pub(crate) bits_echo_pos: u32,
    pub(crate) bits_image: u32,
    pub(crate) bits_reflect: u32,
    pub(crate) bits_deviation: u32,
    pub(crate) bits_reliab: u32,
    pub(crate) reserved5: u64,
    pub(crate) pos_vlr: u64,
    pub(crate) pos_xyz: u64,
    pub(crate) pos_time: u64,
    pub(crate) pos_distance: u64,
    pub(crate) pos_group: u64,
    pub(crate) pos_normal: u64,
    pub(crate) pos_color: u64,
    pub(crate) pos_intensity: u64,
    pub(crate) pos_line: u64,
    pub(crate) pos_echo_len: u64,
    pub(crate) pos_amplitude: u64,
    pub(crate) pos_scanner: u64,
    pub(crate) pos_echo: u64,
    pub(crate) pos_angle: u64,
    pub(crate) pos_echo_norm: u64,
    pub(crate) pos_class: u64,
    pub(crate) pos_record: u64,
    pub(crate) pos_echo_pos: u64,
    pub(crate) pos_image: u64,
    pub(crate) pos_reflect: u64,
    pub(crate) pos_deviation: u64,
    pub(crate) pos_reliab: u64,
    pub(crate) pos_img_nbr: u64,
    pub(crate) img_nbr_cnt: u32,
}

impl Default for FbiHeader {
    fn default() -> Self {
        Self {
            version: 0,
            hdr_size: 0,
            time_type: 0,
            order: 0,
            reserved1: 0,
            vlr_cnt: 0,
            vlr_size: 0,
            rec_size: 0,
            fast_cnt: 0,
            rec_cnt: 0,
            units_xyz: 0,
            units_distance: 0,
            org_x: 0.0,
            org_y: 0.0,
            org_z: 0.0,
            min_x: 0.0,
            max_x: 0.0,
            min_y: 0.0,
            max_y: 0.0,
            min_z: 0.0,
            max_z: 0.0,
            system: [0; 32],
            software: [0; 32],
            reserved2: [0; 64],
            bits_x: 0,
            bits_y: 0,
            bits_z: 0,
            bits_time: 0,
            bits_distance: 0,
            bits_group: 0,
            bits_normal: 0,
            bits_color: 0,
            bits_intensity: 0,
            bits_line: 0,
            bits_echo_len: 0,
            bits_amplitude: 0,
            bits_scanner: 0,
            bits_echo: 0,
            bits_angle: 0,
            bits_echo_norm: 0,
            bits_class: 0,
            bits_echo_pos: 0,
            bits_image: 0,
            bits_reflect: 0,
            bits_deviation: 0,
            bits_reliab: 0,
            reserved5: 0,
            pos_vlr: 0,
            pos_xyz: 0,
            pos_time: 0,
            pos_distance: 0,
            pos_group: 0,
            pos_normal: 0,
            pos_color: 0,
            pos_intensity: 0,
            pos_line: 0,
            pos_echo_len: 0,
            pos_amplitude: 0,
            pos_scanner: 0,
            pos_echo: 0,
            pos_angle: 0,
            pos_echo_norm: 0,
            pos_class: 0,
            pos_record: 0,
            pos_echo_pos: 0,
            pos_image: 0,
            pos_reflect: 0,
            pos_deviation: 0,
            pos_reliab: 0,
            pos_img_nbr: 0,
            img_nbr_cnt: 0,
        }
    }
}

pub struct FbiReader {
    filename: String,
}

impl FbiReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Reader for FbiReader {
    fn name(&self) -> &str {
        "readers.fbi"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "FbiReader requires a filename option.".to_string(),
            ));
        }
        let file = File::open(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}'.", self.filename)))?;
        let mut reader = BufReader::new(file);
        let header = read_header(&mut reader)?;
        let mut layout = PointLayout::new();
        register_dimensions(&mut layout, &header);
        let mut view = PointView::new(Rc::new(layout));
        for _ in 0..header.fast_cnt {
            view.add_point();
        }

        let mul = 1.0 / header.units_xyz as f64;
        reader
            .seek(SeekFrom::Start(header.pos_xyz))
            .map_err(|e| StageError(e.to_string()))?;
        for i in 0..header.fast_cnt {
            let x = read_uint(&mut reader, header.bits_x)? as f64 * mul + header.org_x;
            let y = read_uint(&mut reader, header.bits_y)? as f64 * mul + header.org_y;
            let z = read_uint(&mut reader, header.bits_z)? as f64 * mul + header.org_z;
            view.set_f64(i, &DimId::X, x);
            view.set_f64(i, &DimId::Y, y);
            view.set_f64(i, &DimId::Z, z);
        }

        read_u64_stream(
            &mut reader,
            &mut view,
            header.pos_time,
            header.bits_time,
            &DimId::OffsetTime,
        )?;
        read_u32_stream(
            &mut reader,
            &mut view,
            header.pos_distance,
            header.bits_distance,
            &DimId::NNDistance,
        )?;
        read_u32_stream(
            &mut reader,
            &mut view,
            header.pos_group,
            header.bits_group,
            &DimId::ClusterID,
        )?;
        read_normal_stream(&mut reader, &mut view, &header)?;
        read_color_stream(&mut reader, &mut view, &header)?;
        read_u32_stream(
            &mut reader,
            &mut view,
            header.pos_intensity,
            header.bits_intensity,
            &DimId::Intensity,
        )?;
        read_u32_stream_truncated_to_u8(
            &mut reader,
            &mut view,
            header.pos_line,
            header.bits_line,
            &DimId::PointSourceId,
        )?;
        read_u32_stream_truncated_to_u8(
            &mut reader,
            &mut view,
            header.pos_echo_len,
            header.bits_echo_len,
            &DimId::PulseWidth,
        )?;
        read_u32_stream(
            &mut reader,
            &mut view,
            header.pos_amplitude,
            header.bits_amplitude,
            &DimId::Amplitude,
        )?;
        read_u8_stream(
            &mut reader,
            &mut view,
            header.pos_scanner,
            header.bits_scanner,
            &DimId::UserData,
        )?;
        read_u8_stream(
            &mut reader,
            &mut view,
            header.pos_echo,
            header.bits_echo,
            &DimId::ReturnNumber,
        )?;
        read_i8_stream(
            &mut reader,
            &mut view,
            header.pos_angle,
            header.bits_angle,
            &DimId::ScanAngleRank,
        )?;
        read_u8_stream(
            &mut reader,
            &mut view,
            header.pos_echo_norm,
            header.bits_echo_norm,
            &DimId::EchoNorm,
        )?;
        read_u32_stream(
            &mut reader,
            &mut view,
            header.pos_echo_pos,
            header.bits_echo_pos,
            &DimId::EchoPos,
        )?;
        let image_indexes = read_image_indexes(&mut reader, &mut view, &header)?;
        read_u8_stream(
            &mut reader,
            &mut view,
            header.pos_reliab,
            header.bits_reliab,
            &DimId::Reliability,
        )?;
        read_u8_stream(
            &mut reader,
            &mut view,
            header.pos_class,
            header.bits_class,
            &DimId::Classification,
        )?;
        read_image_numbers(&mut reader, &mut view, &header, &image_indexes)?;
        read_u32_stream(
            &mut reader,
            &mut view,
            header.pos_reflect,
            header.bits_reflect,
            &DimId::Reflectance,
        )?;
        read_u32_stream(
            &mut reader,
            &mut view,
            header.pos_deviation,
            header.bits_deviation,
            &DimId::Deviation,
        )?;

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.fbi")
    }
}

pub(crate) fn read_header<R: Read>(reader: &mut R) -> Result<FbiHeader, StageError> {
    let mut signature = [0u8; 8];
    reader
        .read_exact(&mut signature)
        .map_err(|_| StageError("Incomplete FBI header.".to_string()))?;
    if &signature != SIGNATURE {
        return Err(StageError("Invalid FBI signature.".to_string()));
    }

    let version = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    if version != 1 {
        return Err(StageError(format!("Unsupported FBI version {version}.")));
    }

    let mut header = FbiHeader {
        version,
        hdr_size: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        time_type: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        order: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        reserved1: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        vlr_cnt: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        vlr_size: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        rec_size: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        fast_cnt: reader.read_u64::<LittleEndian>().map_err(io_error)?,
        rec_cnt: reader.read_u64::<LittleEndian>().map_err(io_error)?,
        units_xyz: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        units_distance: reader.read_u32::<LittleEndian>().map_err(io_error)?,
        org_x: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        org_y: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        org_z: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        min_x: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        max_x: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        min_y: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        max_y: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        min_z: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        max_z: reader.read_f64::<LittleEndian>().map_err(io_error)?,
        ..FbiHeader::default()
    };
    reader.read_exact(&mut header.system).map_err(io_error)?;
    reader.read_exact(&mut header.software).map_err(io_error)?;
    reader.read_exact(&mut header.reserved2).map_err(io_error)?;

    header.bits_x = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_y = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_z = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_time = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_distance = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_group = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_normal = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_color = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_intensity = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_line = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_echo_len = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_amplitude = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_scanner = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_echo = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_angle = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_echo_norm = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_class = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_echo_pos = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_image = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_reflect = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_deviation = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.bits_reliab = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    header.reserved5 = reader.read_u64::<LittleEndian>().map_err(io_error)?;

    header.pos_vlr = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_xyz = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_time = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_distance = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_group = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_normal = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_color = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_intensity = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_line = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_echo_len = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_amplitude = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_scanner = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_echo = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_angle = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_echo_norm = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_class = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_record = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_echo_pos = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_image = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_reflect = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_deviation = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_reliab = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.pos_img_nbr = reader.read_u64::<LittleEndian>().map_err(io_error)?;
    header.img_nbr_cnt = reader.read_u32::<LittleEndian>().map_err(io_error)?;

    if header.hdr_size < HEADER_SIZE || header.units_xyz == 0 {
        return Err(StageError("Invalid FBI header.".to_string()));
    }

    Ok(header)
}

pub(crate) fn register_dimensions(layout: &mut PointLayout, header: &FbiHeader) {
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    register_if(layout, header.bits_echo, DimId::ReturnNumber, DimType::U8);
    register_if(layout, header.bits_time, DimId::EchoRange, DimType::U32);
    register_if(layout, header.bits_angle, DimId::ScanAngleRank, DimType::I8);
    register_if(
        layout,
        header.bits_class,
        DimId::Classification,
        DimType::U8,
    );
    register_if(layout, header.bits_line, DimId::PointSourceId, DimType::U16);
    register_if(
        layout,
        header.bits_intensity,
        DimId::Intensity,
        DimType::U16,
    );
    register_if(layout, header.bits_group, DimId::ClusterID, DimType::U32);
    register_if(layout, header.bits_scanner, DimId::UserData, DimType::U8);
    register_if(layout, header.bits_time, DimId::OffsetTime, DimType::U64);
    register_if(
        layout,
        header.bits_distance,
        DimId::NNDistance,
        DimType::U32,
    );
    register_if(layout, header.bits_reliab, DimId::Reliability, DimType::U8);
    register_if(
        layout,
        header.bits_reflect,
        DimId::Reflectance,
        DimType::U16,
    );
    register_if(
        layout,
        header.bits_deviation,
        DimId::Deviation,
        DimType::U16,
    );
    register_if(
        layout,
        header.bits_amplitude,
        DimId::Amplitude,
        DimType::U16,
    );
    register_if(layout, header.bits_echo_pos, DimId::EchoPos, DimType::U16);
    register_if(layout, header.bits_echo_norm, DimId::EchoNorm, DimType::U8);
    register_if(
        layout,
        header.bits_echo_len,
        DimId::PulseWidth,
        DimType::U16,
    );
    register_if(layout, header.bits_image, DimId::Image, DimType::U16);

    if header.bits_normal > 0 {
        layout.register(DimId::NormalX, DimType::F64);
        layout.register(DimId::NormalY, DimType::F64);
        layout.register(DimId::NormalZ, DimType::F64);
        layout.register(DimId::Dimension, DimType::U8);
    }
    if header.bits_color > 0 {
        layout.register(DimId::Red, DimType::U16);
        layout.register(DimId::Green, DimType::U16);
        layout.register(DimId::Blue, DimType::U16);
        if header.bits_color == 32 || header.bits_color == 64 {
            layout.register(DimId::Infrared, DimType::U16);
        }
    }
}

fn register_if(layout: &mut PointLayout, bits: u32, dim: DimId, ty: DimType) {
    if bits > 0 {
        layout.register(dim, ty);
    }
}

fn read_u64_stream<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    pos: u64,
    bits: u32,
    dim: &DimId,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    reader.seek(SeekFrom::Start(pos)).map_err(io_error)?;
    for i in 0..view.len() {
        view.set_f64(i, dim, read_uint64(reader, bits)? as f64);
    }
    Ok(())
}

fn read_u32_stream<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    pos: u64,
    bits: u32,
    dim: &DimId,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    reader.seek(SeekFrom::Start(pos)).map_err(io_error)?;
    for i in 0..view.len() {
        view.set_f64(i, dim, read_uint(reader, bits)? as f64);
    }
    Ok(())
}

fn read_u32_stream_truncated_to_u8<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    pos: u64,
    bits: u32,
    dim: &DimId,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    reader.seek(SeekFrom::Start(pos)).map_err(io_error)?;
    for i in 0..view.len() {
        view.set_f64(i, dim, (read_uint(reader, bits)? as u8) as f64);
    }
    Ok(())
}

fn read_u8_stream<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    pos: u64,
    bits: u32,
    dim: &DimId,
) -> Result<(), StageError> {
    read_u32_stream(reader, view, pos, bits, dim)
}

fn read_i8_stream<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    pos: u64,
    bits: u32,
    dim: &DimId,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    reader.seek(SeekFrom::Start(pos)).map_err(io_error)?;
    for i in 0..view.len() {
        view.set_f64(i, dim, reader.read_i8().map_err(io_error)? as f64);
    }
    Ok(())
}

fn read_normal_stream<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    header: &FbiHeader,
) -> Result<(), StageError> {
    if header.bits_normal == 0 {
        return Ok(());
    }
    reader
        .seek(SeekFrom::Start(header.pos_normal))
        .map_err(io_error)?;
    for i in 0..view.len() {
        let encoded = reader.read_u32::<LittleEndian>().map_err(io_error)?;
        let dim = encoded & 0x3;
        let horz = (encoded >> 2) & 0x7fff;
        let vert = (encoded >> 17) & 0x7fff;
        let (x, y, z) = normal_vector(horz, vert);
        view.set_f64(i, &DimId::Dimension, dim as f64);
        view.set_f64(i, &DimId::NormalX, x);
        view.set_f64(i, &DimId::NormalY, y);
        view.set_f64(i, &DimId::NormalZ, z);
    }
    Ok(())
}

fn read_color_stream<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    header: &FbiHeader,
) -> Result<(), StageError> {
    if header.bits_color == 0 {
        return Ok(());
    }
    let bytes = color_channel_bytes(header.bits_color)?;
    let with_ir = header.bits_color == 32 || header.bits_color == 64;
    reader
        .seek(SeekFrom::Start(header.pos_color))
        .map_err(io_error)?;
    for i in 0..view.len() {
        let red = read_uint_bytes(reader, bytes)?;
        let green = read_uint_bytes(reader, bytes)?;
        let blue = read_uint_bytes(reader, bytes)?;
        view.set_f64(i, &DimId::Red, red as f64);
        view.set_f64(i, &DimId::Green, green as f64);
        view.set_f64(i, &DimId::Blue, blue as f64);
        if with_ir {
            let infrared = read_uint_bytes(reader, bytes)?;
            view.set_f64(i, &DimId::Infrared, infrared as f64);
        }
    }
    Ok(())
}

fn read_image_indexes<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    header: &FbiHeader,
) -> Result<Vec<u32>, StageError> {
    if header.bits_image == 0 {
        return Ok(Vec::new());
    }
    reader
        .seek(SeekFrom::Start(header.pos_image))
        .map_err(io_error)?;
    let mut indexes = Vec::with_capacity(view.len() as usize);
    for i in 0..view.len() {
        let image = read_uint(reader, header.bits_image)?;
        view.set_f64(i, &DimId::Image, image as f64);
        indexes.push(image);
    }
    Ok(indexes)
}

fn read_image_numbers<R: Read + Seek>(
    reader: &mut R,
    view: &mut PointView,
    header: &FbiHeader,
    indexes: &[u32],
) -> Result<(), StageError> {
    if header.img_nbr_cnt == 0 {
        return Ok(());
    }
    reader
        .seek(SeekFrom::Start(header.pos_img_nbr))
        .map_err(io_error)?;
    let mut names = Vec::with_capacity(header.img_nbr_cnt as usize);
    for _ in 0..header.img_nbr_cnt {
        names.push(reader.read_u64::<LittleEndian>().map_err(io_error)?);
    }
    for (i, index) in indexes.iter().enumerate() {
        if let Some(value) = names.get(*index as usize) {
            view.set_f64(i as u64, &DimId::Image, *value as f64);
        }
    }
    Ok(())
}

pub(crate) fn read_uint<R: Read>(reader: &mut R, bits: u32) -> Result<u32, StageError> {
    read_uint_bytes(reader, bytes_for_bits(bits)?)
}

fn read_uint64<R: Read>(reader: &mut R, bits: u32) -> Result<u64, StageError> {
    match bits {
        64 => reader.read_u64::<LittleEndian>().map_err(io_error),
        _ => read_uint(reader, bits).map(u64::from),
    }
}

fn read_uint_bytes<R: Read>(reader: &mut R, bytes: usize) -> Result<u32, StageError> {
    match bytes {
        1 => reader.read_u8().map(u32::from).map_err(io_error),
        2 => reader
            .read_u16::<LittleEndian>()
            .map(u32::from)
            .map_err(io_error),
        4 => reader.read_u32::<LittleEndian>().map_err(io_error),
        _ => Err(StageError(format!("Unsupported FBI field width {bytes}."))),
    }
}

fn bytes_for_bits(bits: u32) -> Result<usize, StageError> {
    match bits {
        8 | 16 | 24 | 32 | 64 => Ok((bits / 8) as usize),
        _ => Err(StageError(format!("Unsupported FBI field width {bits}."))),
    }
}

fn color_channel_bytes(bits_color: u32) -> Result<usize, StageError> {
    match bits_color {
        24 | 32 => Ok(1),
        48 | 64 => Ok(2),
        _ => Err(StageError(format!(
            "Unsupported FBI color width {bits_color}."
        ))),
    }
}

fn normal_vector(horz: u32, vert: u32) -> (f64, f64, f64) {
    let h_ang = std::f64::consts::TAU * horz as f64 / 32767.0;
    let v_ang = std::f64::consts::PI * vert as f64 / 32767.0 - std::f64::consts::FRAC_PI_2;
    let z = v_ang.sin();
    let xy = (1.0 - z * z).sqrt();
    (xy * h_ang.cos(), xy * h_ang.sin(), z)
}

fn io_error(error: std::io::Error) -> StageError {
    StageError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::pipeline::Reader;

    fn data_path(name: &str) -> String {
        format!("{}/../../test/data/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = FbiReader::new(&Options::new());
        let err = reader.read().err().expect("missing filename");
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/file.fbi");
        let mut reader = FbiReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        let reader = FbiReader::new(&Options::new());
        assert_eq!(reader.metadata().name(), "readers.fbi");
    }

    #[test]
    fn reads_fbi_fixture() {
        let mut options = Options::new();
        options.add("filename", data_path("fbi/1.2-with-color.fbi"));
        let mut reader = FbiReader::new(&options);
        let views = reader.read().expect("read fbi fixture");
        assert!(!views.is_empty());
        assert!(views[0].len() > 0);
    }
}
