//! `writers.fbi` -- TerraScan Fast Binary format.
//!
//! Port of `io/FbiWriter.cpp`, including the stream ordering and header
//! offsets expected by PDAL's FBI reader.

use byteorder::{LittleEndian, WriteBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::fbi::FbiHeader;

const HEADER_SIZE: u32 = 1808;

pub struct FbiWriter {
    filename: String,
}

impl FbiWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Writer for FbiWriter {
    fn name(&self) -> &str {
        "writers.fbi"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "FbiWriter requires a filename option.".to_string(),
            ));
        }
        let view = views
            .first()
            .ok_or_else(|| StageError("FbiWriter requires an input view.".to_string()))?;
        let header = build_header(view);
        let file = File::create(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}' for writing.", self.filename)))?;
        let mut writer = BufWriter::new(file);

        write_header(&mut writer, &header)?;
        write_xyz(&mut writer, view, &header)?;
        write_u64_stream(&mut writer, view, &DimId::OffsetTime, header.bits_time)?;
        write_u32_stream(&mut writer, view, &DimId::NNDistance, header.bits_distance)?;
        write_u32_stream(&mut writer, view, &DimId::ClusterID, header.bits_group)?;
        write_normal_stream(&mut writer, view, header.bits_normal)?;
        write_color_stream(&mut writer, view, &header)?;
        write_u32_stream(&mut writer, view, &DimId::Intensity, header.bits_intensity)?;
        write_u32_stream(&mut writer, view, &DimId::PointSourceId, header.bits_line)?;
        write_u32_stream(&mut writer, view, &DimId::PulseWidth, header.bits_echo_len)?;
        write_u32_stream(&mut writer, view, &DimId::Amplitude, header.bits_amplitude)?;
        write_u32_stream(&mut writer, view, &DimId::UserData, header.bits_scanner)?;
        write_u32_stream(&mut writer, view, &DimId::ReturnNumber, header.bits_echo)?;
        write_i8_stream(&mut writer, view, &DimId::ScanAngleRank, header.bits_angle)?;
        write_u32_stream(&mut writer, view, &DimId::EchoNorm, header.bits_echo_norm)?;
        write_u32_stream(&mut writer, view, &DimId::Classification, header.bits_class)?;
        write_u32_stream(&mut writer, view, &DimId::EchoPos, header.bits_echo_pos)?;
        write_u32_stream(&mut writer, view, &DimId::Image, header.bits_image)?;
        write_u32_stream(&mut writer, view, &DimId::Reflectance, header.bits_reflect)?;
        write_u32_stream(&mut writer, view, &DimId::Deviation, header.bits_deviation)?;
        write_u32_stream(&mut writer, view, &DimId::Reliability, header.bits_reliab)?;

        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.fbi")
    }
}

fn build_header(view: &PointView) -> FbiHeader {
    let mut header = FbiHeader {
        version: 1,
        hdr_size: HEADER_SIZE,
        time_type: 1,
        fast_cnt: view.len(),
        units_xyz: 100,
        ..FbiHeader::default()
    };
    header.software[..4].copy_from_slice(b"PDAL");

    let (min_x, max_x) = min_max(view, &DimId::X);
    let (min_y, max_y) = min_max(view, &DimId::Y);
    let (min_z, max_z) = min_max(view, &DimId::Z);
    header.min_x = min_x;
    header.max_x = max_x;
    header.min_y = min_y;
    header.max_y = max_y;
    header.min_z = min_z;
    header.max_z = max_z;
    header.org_x = min_x.abs() - 1.0;
    header.org_y = min_y.abs() - 1.0;
    header.org_z = min_z.abs() - 1.0;

    header.bits_x = 32;
    header.bits_y = 32;
    header.bits_z = 32;
    header.bits_time = bits_if(view, &DimId::OffsetTime, 64);
    header.bits_group = bits_if(view, &DimId::ClusterID, 32);
    header.bits_intensity = bits_if(view, &DimId::Intensity, 16);
    header.bits_scanner = bits_if(view, &DimId::UserData, 8);
    header.bits_echo = bits_if(view, &DimId::ReturnNumber, 8);
    header.bits_angle = if dim_type(view, &DimId::ScanAngleRank) == Some(DimType::I8) {
        8
    } else {
        0
    };
    header.bits_class = bits_if(view, &DimId::Classification, 8);
    header.bits_line = bits_if(view, &DimId::PointSourceId, 16);
    header.bits_echo_len = bits_if(view, &DimId::ReturnNumber, 16);
    header.bits_color = color_bits(view);
    header.bits_distance = bits_if(view, &DimId::NNDistance, 32);
    header.bits_amplitude = bits_if(view, &DimId::Amplitude, 16);
    header.bits_echo_norm = bits_if(view, &DimId::EchoNorm, 8);
    header.bits_echo_pos = bits_if(view, &DimId::EchoPos, 16);
    header.bits_reflect = bits_if(view, &DimId::Reflectance, 16);
    header.bits_deviation = bits_if(view, &DimId::Deviation, 16);
    header.bits_reliab = bits_if(view, &DimId::Reliability, 8);
    header.bits_normal = bits_if(view, &DimId::NormalX, 32);
    header.bits_image = bits_if(view, &DimId::Image, 16);

    header.pos_xyz = header.hdr_size as u64;
    header.pos_time = header.pos_xyz + 3 * view.len() * 4;
    header.pos_distance = header.pos_time + stream_size(view, header.bits_time);
    header.pos_group = header.pos_distance + stream_size(view, header.bits_distance);
    header.pos_normal = header.pos_group + stream_size(view, header.bits_group);
    header.pos_color = header.pos_normal + stream_size(view, header.bits_normal);
    header.pos_intensity = header.pos_color
        + color_channels(view, header.bits_color) * view.len() * u64::from(header.bits_color) / 8;
    header.pos_line = header.pos_intensity + stream_size(view, header.bits_intensity);
    header.pos_echo_len = header.pos_line + stream_size(view, header.bits_line);
    header.pos_amplitude = header.pos_echo_len + stream_size(view, header.bits_echo_len);
    header.pos_scanner = header.pos_amplitude + stream_size(view, header.bits_amplitude);
    header.pos_echo = header.pos_scanner + stream_size(view, header.bits_scanner);
    header.pos_angle = header.pos_echo + stream_size(view, header.bits_echo);
    header.pos_echo_norm = header.pos_angle + stream_size(view, header.bits_angle);
    header.pos_class = header.pos_echo_norm + stream_size(view, header.bits_echo_norm);
    header.pos_echo_pos = header.pos_class + stream_size(view, header.bits_class);
    header.pos_image = header.pos_echo_pos + stream_size(view, header.bits_echo_pos);
    header.pos_reflect = header.pos_image + stream_size(view, header.bits_image);
    header.pos_deviation = header.pos_reflect + stream_size(view, header.bits_reflect);
    header.pos_reliab = header.pos_deviation + stream_size(view, header.bits_deviation);
    header.pos_img_nbr = header.pos_reliab + stream_size(view, header.bits_reliab);
    header.pos_record = header.pos_img_nbr + view.len() * header.pos_img_nbr / 8;
    header
}

fn write_header<W: Write>(writer: &mut W, header: &FbiHeader) -> Result<(), StageError> {
    writer.write_all(b"FASTBIN\0").map_err(io_error)?;
    write_u32s(
        writer,
        &[
            header.version,
            header.hdr_size,
            header.time_type,
            header.order,
            header.reserved1,
            header.vlr_cnt,
            header.vlr_size,
            header.rec_size,
        ],
    )?;
    writer
        .write_u64::<LittleEndian>(header.fast_cnt)
        .map_err(io_error)?;
    writer
        .write_u64::<LittleEndian>(header.rec_cnt)
        .map_err(io_error)?;
    writer
        .write_u32::<LittleEndian>(header.units_xyz)
        .map_err(io_error)?;
    writer
        .write_u32::<LittleEndian>(header.units_distance)
        .map_err(io_error)?;
    for value in [
        header.org_x,
        header.org_y,
        header.org_z,
        header.min_x,
        header.max_x,
        header.min_y,
        header.max_y,
        header.min_z,
        header.max_z,
    ] {
        writer.write_f64::<LittleEndian>(value).map_err(io_error)?;
    }
    writer.write_all(&header.system).map_err(io_error)?;
    writer.write_all(&header.software).map_err(io_error)?;
    writer.write_all(&header.reserved2).map_err(io_error)?;
    write_u32s(
        writer,
        &[
            header.bits_x,
            header.bits_y,
            header.bits_z,
            header.bits_time,
            header.bits_distance,
            header.bits_group,
            header.bits_normal,
            header.bits_color,
            header.bits_intensity,
            header.bits_line,
            header.bits_echo_len,
            header.bits_amplitude,
            header.bits_scanner,
            header.bits_echo,
            header.bits_angle,
            header.bits_echo_norm,
            header.bits_class,
            header.bits_echo_pos,
            header.bits_image,
            header.bits_reflect,
            header.bits_deviation,
            header.bits_reliab,
        ],
    )?;
    writer
        .write_u64::<LittleEndian>(header.reserved5)
        .map_err(io_error)?;
    write_u64s(
        writer,
        &[
            header.pos_vlr,
            header.pos_xyz,
            header.pos_time,
            header.pos_distance,
            header.pos_group,
            header.pos_normal,
            header.pos_color,
            header.pos_intensity,
            header.pos_line,
            header.pos_echo_len,
            header.pos_amplitude,
            header.pos_scanner,
            header.pos_echo,
            header.pos_angle,
            header.pos_echo_norm,
            header.pos_class,
            header.pos_record,
            header.pos_echo_pos,
            header.pos_image,
            header.pos_reflect,
            header.pos_deviation,
            header.pos_reliab,
            header.pos_img_nbr,
        ],
    )?;
    writer
        .write_u32::<LittleEndian>(header.img_nbr_cnt)
        .map_err(io_error)?;
    writer.write_all(&[0u8; 1260]).map_err(io_error)
}

fn write_xyz<W: Write>(
    writer: &mut W,
    view: &PointView,
    header: &FbiHeader,
) -> Result<(), StageError> {
    let mul = 1.0 / header.units_xyz as f64;
    for i in 0..view.len() {
        writer
            .write_u32::<LittleEndian>(((view.get_f64(i, &DimId::X) - header.org_x) * mul) as u32)
            .map_err(io_error)?;
        writer
            .write_u32::<LittleEndian>(((view.get_f64(i, &DimId::Y) - header.org_y) * mul) as u32)
            .map_err(io_error)?;
        writer
            .write_u32::<LittleEndian>(((view.get_f64(i, &DimId::Z) - header.org_z) * mul) as u32)
            .map_err(io_error)?;
    }
    Ok(())
}

fn write_u64_stream<W: Write>(
    writer: &mut W,
    view: &PointView,
    dim: &DimId,
    bits: u32,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    for i in 0..view.len() {
        writer
            .write_u64::<LittleEndian>(view.get_f64(i, dim) as u64)
            .map_err(io_error)?;
    }
    Ok(())
}

fn write_u32_stream<W: Write>(
    writer: &mut W,
    view: &PointView,
    dim: &DimId,
    bits: u32,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    let bytes = (bits / 8) as usize;
    for i in 0..view.len() {
        write_uint_bytes(writer, view.get_f64(i, dim) as u32, bytes)?;
    }
    Ok(())
}

fn write_i8_stream<W: Write>(
    writer: &mut W,
    view: &PointView,
    dim: &DimId,
    bits: u32,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    for i in 0..view.len() {
        writer
            .write_i8(view.get_f64(i, dim) as i8)
            .map_err(io_error)?;
    }
    Ok(())
}

fn write_normal_stream<W: Write>(
    writer: &mut W,
    view: &PointView,
    bits: u32,
) -> Result<(), StageError> {
    if bits == 0 {
        return Ok(());
    }
    for i in 0..view.len() {
        let dim = view.get_f64(i, &DimId::Dimension) as u32;
        let x = view.get_f64(i, &DimId::NormalX);
        let y = view.get_f64(i, &DimId::NormalY);
        let z = view.get_f64(i, &DimId::NormalZ);
        let encoded = encode_normal(dim, x, y, z);
        writer
            .write_u32::<LittleEndian>(encoded)
            .map_err(io_error)?;
    }
    Ok(())
}

fn write_color_stream<W: Write>(
    writer: &mut W,
    view: &PointView,
    header: &FbiHeader,
) -> Result<(), StageError> {
    if header.bits_color == 0 {
        return Ok(());
    }
    let bytes = (header.bits_color / 8) as usize;
    for i in 0..view.len() {
        write_uint_bytes(writer, view.get_f64(i, &DimId::Blue) as u32, bytes)?;
        write_uint_bytes(writer, view.get_f64(i, &DimId::Green) as u32, bytes)?;
        write_uint_bytes(writer, view.get_f64(i, &DimId::Red) as u32, bytes)?;
        if has_dim(view, &DimId::Infrared) {
            write_uint_bytes(writer, view.get_f64(i, &DimId::Infrared) as u32, bytes)?;
        }
    }
    Ok(())
}

fn write_uint_bytes<W: Write>(writer: &mut W, value: u32, bytes: usize) -> Result<(), StageError> {
    match bytes {
        1 => writer.write_u8(value as u8).map_err(io_error),
        2 => writer
            .write_u16::<LittleEndian>(value as u16)
            .map_err(io_error),
        4 => writer.write_u32::<LittleEndian>(value).map_err(io_error),
        _ => Err(StageError(format!("Unsupported FBI field width {bytes}."))),
    }
}

fn write_u32s<W: Write>(writer: &mut W, values: &[u32]) -> Result<(), StageError> {
    for value in values {
        writer.write_u32::<LittleEndian>(*value).map_err(io_error)?;
    }
    Ok(())
}

fn write_u64s<W: Write>(writer: &mut W, values: &[u64]) -> Result<(), StageError> {
    for value in values {
        writer.write_u64::<LittleEndian>(*value).map_err(io_error)?;
    }
    Ok(())
}

fn min_max(view: &PointView, dim: &DimId) -> (f64, f64) {
    if view.is_empty() {
        return (0.0, 0.0);
    }
    let mut min = view.get_f64(0, dim);
    let mut max = min;
    for i in 1..view.len() {
        let value = view.get_f64(i, dim);
        min = min.min(value);
        max = max.max(value);
    }
    (min, max)
}

fn has_dim(view: &PointView, dim: &DimId) -> bool {
    view.layout().dim(dim).is_some()
}

fn dim_type(view: &PointView, dim: &DimId) -> Option<DimType> {
    view.layout().dim(dim).map(|(_, ty)| ty)
}

fn bits_if(view: &PointView, dim: &DimId, bits: u32) -> u32 {
    if has_dim(view, dim) {
        bits
    } else {
        0
    }
}

fn color_bits(view: &PointView) -> u32 {
    let rgb =
        has_dim(view, &DimId::Red) && has_dim(view, &DimId::Green) && has_dim(view, &DimId::Blue);
    let infrared = has_dim(view, &DimId::Infrared);
    if !rgb && !infrared {
        return 0;
    }
    let dim = if rgb { &DimId::Red } else { &DimId::Infrared };
    dim_type(view, dim).map_or(0, |ty| (ty.size() * 8) as u32)
}

fn color_channels(view: &PointView, bits_color: u32) -> u64 {
    if bits_color == 0 {
        0
    } else if has_dim(view, &DimId::Infrared) {
        4
    } else {
        3
    }
}

fn stream_size(view: &PointView, bits: u32) -> u64 {
    view.len() * u64::from(bits) / 8
}

fn encode_normal(dim: u32, x: f64, y: f64, z: f64) -> u32 {
    let vml = 32767.0 / std::f64::consts::PI;
    let hml = 32767.0 / std::f64::consts::TAU;
    let mut vert = (vml * (z.asin() + std::f64::consts::FRAC_PI_2)).floor() as i32;
    vert = vert.clamp(0, 32767);
    let mut horz = 0;
    if x != 0.0 || y != 0.0 {
        let mut angle = y.atan2(x);
        if angle < 0.0 {
            angle += std::f64::consts::TAU;
        }
        horz = (hml * angle).floor() as i32;
        horz = horz.clamp(0, 32767);
    }
    (dim & 0x3) | ((horz as u32) << 2) | ((vert as u32) << 17)
}

fn io_error(error: std::io::Error) -> StageError {
    StageError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fbi::FbiReader;
    use pdal_core::pipeline::{Reader, Writer};
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn make_view() -> PointView {
        let mut layout = PointLayout::new();
        for (dim, ty) in [
            (DimId::X, DimType::F64),
            (DimId::Y, DimType::F64),
            (DimId::Z, DimType::F64),
            (DimId::Intensity, DimType::U16),
            (DimId::Classification, DimType::U8),
            (DimId::Red, DimType::U16),
            (DimId::Green, DimType::U16),
            (DimId::Blue, DimType::U16),
            (DimId::NormalX, DimType::F64),
            (DimId::NormalY, DimType::F64),
            (DimId::NormalZ, DimType::F64),
            (DimId::Dimension, DimType::U8),
            (DimId::ReturnNumber, DimType::U8),
            (DimId::ScanAngleRank, DimType::I8),
        ] {
            layout.register(dim, ty);
        }
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, intensity, class) in [
            (100.0, 200.0, 300.0, 42.0, 7.0),
            (101.0, 201.0, 301.0, 43.0, 8.0),
        ] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, z);
            view.set_f64(id, &DimId::Intensity, intensity);
            view.set_f64(id, &DimId::Classification, class);
            view.set_f64(id, &DimId::Red, 1000.0 + id as f64);
            view.set_f64(id, &DimId::Green, 2000.0 + id as f64);
            view.set_f64(id, &DimId::Blue, 3000.0 + id as f64);
            view.set_f64(id, &DimId::NormalX, 1.0);
            view.set_f64(id, &DimId::NormalY, 0.0);
            view.set_f64(id, &DimId::NormalZ, 0.0);
            view.set_f64(id, &DimId::Dimension, 2.0);
            view.set_f64(id, &DimId::ReturnNumber, 1.0 + id as f64);
            view.set_f64(id, &DimId::ScanAngleRank, -5.0 + id as f64);
        }
        view
    }

    #[test]
    fn writer_matches_legacy_stream_contract() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().to_string());
        FbiWriter::new(&options).write(&[make_view()]).unwrap();

        let views = FbiReader::new(&options).read().unwrap();
        assert_eq!(views.len(), 1);
        let view = &views[0];
        assert_eq!(view.len(), 2);
        assert!((view.get_f64(0, &DimId::X) - 99.0).abs() < 0.01);
        assert!((view.get_f64(0, &DimId::Y) - 199.0).abs() < 0.01);
        assert!((view.get_f64(0, &DimId::Z) - 299.0).abs() < 0.01);
        assert_eq!(view.get_f64(0, &DimId::Intensity), 42.0);
        assert_eq!(view.get_f64(1, &DimId::Classification), 8.0);
        assert_eq!(view.get_f64(1, &DimId::Red), 3001.0);
        assert_eq!(view.get_f64(1, &DimId::Green), 2001.0);
        assert_eq!(view.get_f64(1, &DimId::Blue), 1001.0);
        assert_eq!(view.get_f64(1, &DimId::ReturnNumber), 2.0);
    }

    #[test]
    fn writer_rejects_missing_filename_and_input() {
        assert!(FbiWriter::new(&Options::new())
            .write(&[make_view()])
            .is_err());

        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().to_string());
        assert!(FbiWriter::new(&options).write(&[]).is_err());
    }

    #[test]
    fn writer_handles_empty_views_with_header_only_streams() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let view = PointView::new(Rc::new(layout));
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().to_string());

        FbiWriter::new(&options).write(&[view]).unwrap();
        assert_eq!(FbiReader::new(&options).read().unwrap()[0].len(), 0);
    }

    #[test]
    fn writer_metadata_and_name() {
        let writer = FbiWriter::new(&Options::new());
        assert_eq!(writer.name(), "writers.fbi");
        assert_eq!(writer.metadata().name(), "writers.fbi");
    }

    #[test]
    fn writer_errors_on_unwritable_path() {
        let mut options = Options::new();
        options.add("filename", "/no/such/directory/out.fbi");
        let mut writer = FbiWriter::new(&options);
        let view = make_view();
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn has_dim_returns_true_for_present_dim() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let view = PointView::new(Rc::new(layout));
        assert!(has_dim(&view, &DimId::X));
        assert!(!has_dim(&view, &DimId::Y));
    }

    #[test]
    fn bits_if_returns_zero_when_dim_missing() {
        let layout = PointLayout::new();
        let view = PointView::new(Rc::new(layout));
        assert_eq!(bits_if(&view, &DimId::X, 32), 0);
    }
}
