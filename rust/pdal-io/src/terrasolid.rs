//! `readers.terrasolid` -- TerraSolid binary point format.
//!
//! Port of `io/TerrasolidReader.cpp` for the deterministic local format 2 path
//! covered by PDAL's existing fixture.

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

const FORMAT_1: i32 = 20010712;
const FORMAT_2: i32 = 20020715;

struct Header {
    hdr_version: i32,
    pnt_cnt: i32,
    units: i32,
    org_x: f64,
    org_y: f64,
    org_z: f64,
    time: i32,
    color: i32,
}

pub struct TerrasolidReader {
    filename: String,
}

impl TerrasolidReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Reader for TerrasolidReader {
    fn name(&self) -> &str {
        "readers.terrasolid"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "TerrasolidReader requires a filename option.".to_string(),
            ));
        }
        let file = File::open(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}'.", self.filename)))?;
        let mut reader = BufReader::new(file);
        let header = read_header(&mut reader)?;
        let mut layout = PointLayout::new();
        register_dimensions(&mut layout, &header);
        let mut view = PointView::new(Rc::new(layout));

        reader.seek(SeekFrom::Start(56)).map_err(io_error)?;
        let mut base_time = 0u32;
        for point_idx in 0..header.pnt_cnt {
            let point = view.add_point();
            match header.hdr_version {
                FORMAT_1 => read_format_1(&mut reader, &mut view, point, &header)?,
                FORMAT_2 => read_format_2(&mut reader, &mut view, point, &header)?,
                _ => unreachable!(),
            }
            if header.time != 0 {
                let mut t = reader.read_u32::<LittleEndian>().map_err(io_error)?;
                if point_idx == 0 {
                    base_time = t;
                }
                t -= base_time;
                t /= 5;
                view.set_f64(point, &DimId::OffsetTime, t as f64);
            }
            if header.color != 0 {
                view.set_f64(
                    point,
                    &DimId::Red,
                    reader.read_u8().map_err(io_error)? as f64,
                );
                view.set_f64(
                    point,
                    &DimId::Green,
                    reader.read_u8().map_err(io_error)? as f64,
                );
                view.set_f64(
                    point,
                    &DimId::Blue,
                    reader.read_u8().map_err(io_error)? as f64,
                );
                view.set_f64(
                    point,
                    &DimId::Alpha,
                    reader.read_u8().map_err(io_error)? as f64,
                );
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.terrasolid")
    }
}

fn read_header<R: Read>(reader: &mut R) -> Result<Header, StageError> {
    let hdr_size = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let hdr_version = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let recog_val = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let mut recog_str = [0u8; 4];
    reader.read_exact(&mut recog_str).map_err(io_error)?;
    let pnt_cnt = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let units = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let org_x = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let org_y = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let org_z = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let time = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let color = reader.read_i32::<LittleEndian>().map_err(io_error)?;

    if recog_val != 970401 {
        return Err(StageError(
            "Header identifier was not '970401', is this a TerraSolid .bin file?".to_string(),
        ));
    }
    if hdr_version != FORMAT_1 && hdr_version != FORMAT_2 {
        return Err(StageError(format!(
            "Version was '{hdr_version}', not '{FORMAT_1}' or '{FORMAT_2}'"
        )));
    }
    if hdr_size < 56 || pnt_cnt < 0 || units == 0 {
        return Err(StageError("Invalid TerraSolid header.".to_string()));
    }

    Ok(Header {
        hdr_version,
        pnt_cnt,
        units,
        org_x,
        org_y,
        org_z,
        time,
        color,
    })
}

fn register_dimensions(layout: &mut PointLayout, header: &Header) {
    layout.register(DimId::Classification, DimType::U8);
    layout.register(DimId::PointSourceId, DimType::U16);
    layout.register(DimId::Intensity, DimType::U16);
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::ReturnNumber, DimType::U8);
    layout.register(DimId::NumberOfReturns, DimType::U8);
    if header.hdr_version == FORMAT_2 {
        layout.register(DimId::Flag, DimType::U8);
        layout.register(DimId::Mark, DimType::U8);
    }
    if header.time != 0 {
        layout.register(DimId::OffsetTime, DimType::U32);
    }
    if header.color != 0 {
        layout.register(DimId::Red, DimType::U8);
        layout.register(DimId::Green, DimType::U8);
        layout.register(DimId::Blue, DimType::U8);
        layout.register(DimId::Alpha, DimType::U8);
    }
}

fn read_format_1<R: Read>(
    reader: &mut R,
    view: &mut PointView,
    point: u64,
    header: &Header,
) -> Result<(), StageError> {
    let classification = reader.read_u8().map_err(io_error)?;
    let flight_line = reader.read_u8().map_err(io_error)?;
    let echo_int = reader.read_u8().map_err(io_error)?;
    let x = reader.read_u8().map_err(io_error)?;
    let y = reader.read_u8().map_err(io_error)?;
    let z = reader.read_u8().map_err(io_error)?;

    view.set_f64(point, &DimId::Classification, classification as f64);
    view.set_f64(point, &DimId::PointSourceId, flight_line as f64);
    set_echo_dims(view, point, echo_int);
    view.set_f64(
        point,
        &DimId::X,
        scaled(x as f64, header.org_x, header.units),
    );
    view.set_f64(
        point,
        &DimId::Y,
        scaled(y as f64, header.org_y, header.units),
    );
    view.set_f64(
        point,
        &DimId::Z,
        scaled(z as f64, header.org_z, header.units),
    );
    Ok(())
}

fn read_format_2<R: Read>(
    reader: &mut R,
    view: &mut PointView,
    point: u64,
    header: &Header,
) -> Result<(), StageError> {
    let x = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let y = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let z = reader.read_i32::<LittleEndian>().map_err(io_error)?;
    let classification = reader.read_u8().map_err(io_error)?;
    let echo_int = reader.read_u8().map_err(io_error)?;
    let flag = reader.read_u8().map_err(io_error)?;
    let mark = reader.read_u8().map_err(io_error)?;
    let flight_line = reader.read_u16::<LittleEndian>().map_err(io_error)?;
    let intensity = reader.read_u16::<LittleEndian>().map_err(io_error)?;

    view.set_f64(
        point,
        &DimId::X,
        scaled(x as f64, header.org_x, header.units),
    );
    view.set_f64(
        point,
        &DimId::Y,
        scaled(y as f64, header.org_y, header.units),
    );
    view.set_f64(
        point,
        &DimId::Z,
        scaled(z as f64, header.org_z, header.units),
    );
    view.set_f64(point, &DimId::Classification, classification as f64);
    set_echo_dims(view, point, echo_int);
    view.set_f64(point, &DimId::Flag, flag as f64);
    view.set_f64(point, &DimId::Mark, mark as f64);
    view.set_f64(point, &DimId::PointSourceId, flight_line as f64);
    view.set_f64(point, &DimId::Intensity, intensity as f64);
    Ok(())
}

fn set_echo_dims(view: &mut PointView, point: u64, echo_int: u8) {
    match echo_int {
        0 => {
            view.set_f64(point, &DimId::ReturnNumber, 1.0);
            view.set_f64(point, &DimId::NumberOfReturns, 1.0);
        }
        1 => view.set_f64(point, &DimId::ReturnNumber, 1.0),
        _ => {}
    }
}

fn scaled(value: f64, origin: f64, units: i32) -> f64 {
    (value - origin) / units as f64
}

fn io_error(error: std::io::Error) -> StageError {
    StageError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn scaled_positive_units() {
        assert_eq!(scaled(1000.0, 500.0, 100), 5.0);
    }

    #[test]
    fn scaled_negative_units() {
        assert_eq!(scaled(500.0, 1000.0, 250), -2.0);
    }

    #[test]
    fn set_echo_dims_zero_is_single_return() {
        let mut layout = PointLayout::new();
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        set_echo_dims(&mut view, idx, 0);
        assert_eq!(view.get_f64(idx, &DimId::ReturnNumber), 1.0);
        assert_eq!(view.get_f64(idx, &DimId::NumberOfReturns), 1.0);
    }

    #[test]
    fn set_echo_dims_one_sets_return_number_only() {
        let mut layout = PointLayout::new();
        layout.register(DimId::ReturnNumber, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        set_echo_dims(&mut view, idx, 1);
        assert_eq!(view.get_f64(idx, &DimId::ReturnNumber), 1.0);
    }

    #[test]
    fn set_echo_dims_other_does_nothing() {
        let mut layout = PointLayout::new();
        layout.register(DimId::ReturnNumber, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let idx = view.add_point();
        set_echo_dims(&mut view, idx, 2);
        assert_eq!(view.get_f64(idx, &DimId::ReturnNumber), 0.0);
    }

    #[test]
    fn read_header_rejects_missing_magic() {
        let buf = [0u8; 56];
        let result = read_header(&mut Cursor::new(&buf));
        match result.err() {
            Some(e) => assert!(e.0.contains("970401")),
            None => panic!("expected error"),
        }
    }

    #[test]
    fn read_header_rejects_unknown_version() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&56i32.to_le_bytes());
        buf.extend_from_slice(&9999i32.to_le_bytes());
        buf.extend_from_slice(&970401i32.to_le_bytes());
        buf.extend_from_slice(b"abcd");
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&0f64.to_le_bytes());
        buf.extend_from_slice(&0f64.to_le_bytes());
        buf.extend_from_slice(&0f64.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        let result = read_header(&mut Cursor::new(&buf));
        match result.err() {
            Some(e) => assert!(e.0.contains("Version")),
            None => panic!("expected error"),
        }
    }

    #[test]
    fn read_header_rejects_invalid_header_size() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&10i32.to_le_bytes()); // hdr_size < 56
        buf.extend_from_slice(&20020715i32.to_le_bytes());
        buf.extend_from_slice(&970401i32.to_le_bytes());
        buf.extend_from_slice(b"abcd");
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&0f64.to_le_bytes());
        buf.extend_from_slice(&0f64.to_le_bytes());
        buf.extend_from_slice(&0f64.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        let result = read_header(&mut Cursor::new(&buf));
        assert!(result.is_err());
    }

    #[test]
    fn empty_filename_is_error() {
        let mut r = TerrasolidReader {
            filename: String::new(),
        };
        match r.read().err() {
            Some(e) => assert!(e.0.contains("requires a filename")),
            None => panic!("expected error"),
        }
    }

    #[test]
    fn name_returns_readers_terrasolid() {
        let r = TerrasolidReader {
            filename: "dummy".to_string(),
        };
        assert_eq!(r.name(), "readers.terrasolid");
    }

    #[test]
    fn register_dimensions_format_2() {
        let mut layout = PointLayout::new();
        let header = Header {
            hdr_version: FORMAT_2,
            pnt_cnt: 0,
            units: 1,
            org_x: 0.0,
            org_y: 0.0,
            org_z: 0.0,
            time: 0,
            color: 0,
        };
        register_dimensions(&mut layout, &header);
        // Flag and Mark should be registered
        // We can't easily inspect the layout, but we can verify no panic
    }

    #[test]
    fn register_dimensions_with_time_and_color() {
        let mut layout = PointLayout::new();
        let header = Header {
            hdr_version: FORMAT_2,
            pnt_cnt: 0,
            units: 1,
            org_x: 0.0,
            org_y: 0.0,
            org_z: 0.0,
            time: 1,
            color: 1,
        };
        register_dimensions(&mut layout, &header);
        // OffsetTime, Red, Green, Blue, Alpha should be registered
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = TerrasolidReader::new(&Options::new());
        let err = reader.read().err().expect("missing filename");
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/file.bin");
        let mut reader = TerrasolidReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        let reader = TerrasolidReader::new(&Options::new());
        assert_eq!(reader.metadata().name(), "readers.terrasolid");
    }

    #[test]
    fn reads_terrasolid_fixture() {
        let path = format!(
            "{}/../../test/data/terrasolid/20020715-time-color.bin",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut options = Options::new();
        options.add("filename", path);
        let mut reader = TerrasolidReader::new(&options);
        let views = reader.read().expect("read terrasolid fixture");
        assert!(!views.is_empty());
        assert!(views[0].len() > 0);
    }
}
