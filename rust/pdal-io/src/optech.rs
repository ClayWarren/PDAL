//! `readers.optech` -- Optech CSD reader.
//!
//! Port of `io/OptechReader.cpp` for local CSD fixtures.

use crate::source;
use byteorder::{LittleEndian, ReadBytesExt};
use pdal_core::georeference::{
    create_optech_rotation_matrix, georeference_wgs84, RotationMatrix, Xyz,
};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::rc::Rc;

const MAX_RETURNS: usize = 4;

struct Header {
    header_size: u16,
    num_records: u32,
    misalignment_angles: [f64; 3],
    imu_offsets: [f64; 3],
}

struct Pulse {
    gps_time: f64,
    return_count: u8,
    range: [f32; MAX_RETURNS],
    intensity: [u16; MAX_RETURNS],
    scan_angle: f32,
    roll: f32,
    pitch: f32,
    heading: f32,
    latitude: f64,
    longitude: f64,
    elevation: f32,
}

pub struct OptechReader {
    filename: String,
    stream: Option<OptechStreamState>,
}

struct OptechStreamState {
    reader: BufReader<Box<dyn source::ReadSeek>>,
    layout: Rc<PointLayout>,
    boresight: RotationMatrix,
    remaining_pulses: u32,
}

impl OptechReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            stream: None,
        }
    }

    fn layout() -> Rc<PointLayout> {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::GpsTime, DimType::F64);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        layout.register(DimId::EchoRange, DimType::F32);
        layout.register(DimId::Intensity, DimType::U16);
        layout.register(DimId::ScanAngleRank, DimType::F32);
        Rc::new(layout)
    }

    fn open(&self) -> Result<(BufReader<Box<dyn source::ReadSeek>>, Header), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "OptechReader requires a filename option.".to_string(),
            ));
        }
        let file = source::open_seek(&self.filename)
            .map_err(|_| StageError(format!("Unable to open {} for reading.", self.filename)))?;
        let mut reader = BufReader::new(file);
        let header = read_header(&mut reader)?;
        reader
            .seek(SeekFrom::Start(header.header_size as u64))
            .map_err(io_error)?;
        Ok((reader, header))
    }

    fn boresight(header: &Header) -> RotationMatrix {
        create_optech_rotation_matrix(
            header.misalignment_angles[0] + header.imu_offsets[0],
            header.misalignment_angles[1] + header.imu_offsets[1],
            header.misalignment_angles[2] + header.imu_offsets[2],
        )
    }

    fn append_pulse_returns(view: &mut PointView, mut pulse: Pulse, boresight: RotationMatrix) {
        if pulse.return_count == 0 {
            return;
        }
        if pulse.longitude < -std::f64::consts::PI * 2.0 {
            pulse.longitude += std::f64::consts::PI * 2.0;
        } else if pulse.longitude > std::f64::consts::PI * 2.0 {
            pulse.longitude -= std::f64::consts::PI * 2.0;
        }

        for return_index in 0..pulse.return_count.min(MAX_RETURNS as u8) as usize {
            let gps_point = Xyz {
                x: pulse.longitude,
                y: pulse.latitude,
                z: pulse.elevation as f64,
            };
            let rotation = create_optech_rotation_matrix(
                pulse.roll as f64,
                pulse.pitch as f64,
                pulse.heading as f64,
            );
            let point = georeference_wgs84(
                pulse.range[return_index] as f64,
                pulse.scan_angle as f64,
                boresight,
                rotation,
                gps_point,
            );
            let id = view.add_point();
            view.set_f64(id, &DimId::X, point.x.to_degrees());
            view.set_f64(id, &DimId::Y, point.y.to_degrees());
            view.set_f64(id, &DimId::Z, point.z);
            view.set_f64(id, &DimId::GpsTime, pulse.gps_time);
            let return_number = if return_index == MAX_RETURNS - 1 {
                pulse.return_count
            } else {
                (return_index + 1) as u8
            };
            view.set_f64(id, &DimId::ReturnNumber, return_number as f64);
            view.set_f64(id, &DimId::NumberOfReturns, pulse.return_count as f64);
            view.set_f64(id, &DimId::EchoRange, pulse.range[return_index] as f64);
            view.set_f64(id, &DimId::Intensity, pulse.intensity[return_index] as f64);
            view.set_f64(
                id,
                &DimId::ScanAngleRank,
                (pulse.scan_angle as f64).to_degrees(),
            );
        }
    }

    fn stream_init(&mut self) -> Result<(), StageError> {
        let (reader, header) = self.open()?;
        self.stream = Some(OptechStreamState {
            reader,
            layout: Self::layout(),
            boresight: Self::boresight(&header),
            remaining_pulses: header.num_records,
        });
        Ok(())
    }
}

impl Reader for OptechReader {
    fn name(&self) -> &str {
        "readers.optech"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        let (mut reader, header) = self.open()?;
        let boresight = Self::boresight(&header);
        let mut view = PointView::new(Self::layout());
        view.set_spatial_reference(SpatialReference::new("EPSG:4326"));

        for _ in 0..header.num_records {
            let pulse = read_pulse(&mut reader)?;
            Self::append_pulse_returns(&mut view, pulse, boresight);
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.optech")
    }

    fn reset(&mut self) {
        self.stream = None;
    }

    fn streamable(&self) -> bool {
        !self.filename.is_empty()
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.stream.is_none() {
            self.stream_init()?;
        }
        let state = self.stream.as_mut().expect("stream initialized above");
        if state.remaining_pulses == 0 {
            return Ok(None);
        }

        let take = (capacity.max(1) as u32).min(state.remaining_pulses);
        let mut view = PointView::new(Rc::clone(&state.layout));
        view.set_spatial_reference(SpatialReference::new("EPSG:4326"));
        for _ in 0..take {
            let pulse = read_pulse(&mut state.reader)?;
            Self::append_pulse_returns(&mut view, pulse, state.boresight);
            state.remaining_pulses -= 1;
        }
        Ok(Some(view))
    }
}

fn read_header<R: Read>(reader: &mut R) -> Result<Header, StageError> {
    let mut signature = [0u8; 4];
    reader.read_exact(&mut signature).map_err(io_error)?;
    if &signature[..3] != b"CSD" {
        return Err(StageError(format!(
            "Invalid header signature when reading CSD file: '{}'",
            String::from_utf8_lossy(&signature)
        )));
    }
    let mut vendor_id = [0u8; 64];
    let mut software_version = [0u8; 32];
    reader.read_exact(&mut vendor_id).map_err(io_error)?;
    reader.read_exact(&mut software_version).map_err(io_error)?;
    let _format_version = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    let header_size = reader.read_u16::<LittleEndian>().map_err(io_error)?;
    let _gps_week = reader.read_u16::<LittleEndian>().map_err(io_error)?;
    let _min_time = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let _max_time = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let num_records = reader.read_u32::<LittleEndian>().map_err(io_error)?;
    let _num_strips = reader.read_u16::<LittleEndian>().map_err(io_error)?;
    for _ in 0..256 {
        reader.read_u32::<LittleEndian>().map_err(io_error)?;
    }
    let mut misalignment_angles = [0.0; 3];
    let mut imu_offsets = [0.0; 3];
    for value in &mut misalignment_angles {
        *value = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }
    for value in &mut imu_offsets {
        *value = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    }
    let _temperature = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let _pressure = reader.read_f64::<LittleEndian>().map_err(io_error)?;

    Ok(Header {
        header_size,
        num_records,
        misalignment_angles,
        imu_offsets,
    })
}

fn read_pulse<R: Read>(reader: &mut R) -> Result<Pulse, StageError> {
    let gps_time = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let return_count = reader.read_u8().map_err(io_error)?;
    let mut range = [0.0; MAX_RETURNS];
    for value in &mut range {
        *value = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    }
    let mut intensity = [0; MAX_RETURNS];
    for value in &mut intensity {
        *value = reader.read_u16::<LittleEndian>().map_err(io_error)?;
    }
    let scan_angle = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    let roll = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    let pitch = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    let heading = reader.read_f32::<LittleEndian>().map_err(io_error)?;
    let latitude = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let longitude = reader.read_f64::<LittleEndian>().map_err(io_error)?;
    let elevation = reader.read_f32::<LittleEndian>().map_err(io_error)?;

    Ok(Pulse {
        gps_time,
        return_count,
        range,
        intensity,
        scan_angle,
        roll,
        pitch,
        heading,
        latitude,
        longitude,
        elevation,
    })
}

fn io_error(error: std::io::Error) -> StageError {
    StageError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::pipeline::Reader;

    fn data_path(path: &str) -> String {
        format!("{}/../../test/data/{path}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn reads_existing_optech_fixture() {
        let mut options = Options::new();
        options.add("filename", data_path("optech/sample.csd"));
        let views = OptechReader::new(&options).read().unwrap();

        assert_eq!(views.len(), 1);
        let view = &views[0];
        assert_eq!(view.len(), 1000);
        assert_eq!(view.spatial_reference().wkt(), "EPSG:4326");
        assert_eq!(view.get_f64(0, &DimId::GpsTime), 5.756_447_448_456_39e5);
        assert_eq!(view.get_f64(0, &DimId::ReturnNumber), 1.0);
        assert_eq!(view.get_f64(0, &DimId::NumberOfReturns), 1.0);
        assert_eq!(view.get_f64(0, &DimId::Intensity), 384.0);
        assert!((view.get_f64(0, &DimId::X) - -82.554_028_877_408_56).abs() < 1e-12);
        assert!((view.get_f64(0, &DimId::Y) - 36.534_611_447_321_91).abs() < 1e-12);
        assert!((view.get_f64(0, &DimId::Z) - 344.80889224602356).abs() < 1e-12);
        assert!((view.get_f64(0, &DimId::ScanAngleRank) - -14.555161476135254).abs() < 1e-6);
    }

    #[test]
    fn streaming_chunks_match_full_read() {
        let mut options = Options::new();
        options.add("filename", data_path("optech/sample.csd"));

        let mut full_reader = OptechReader::new(&options);
        let full = full_reader.read().unwrap().pop().unwrap();

        let mut stream_reader = OptechReader::new(&options);
        assert!(stream_reader.streamable());
        let first = stream_reader.stream_next(200).unwrap().unwrap();
        let second = stream_reader.stream_next(200).unwrap().unwrap();
        let mut total = first.len() + second.len();
        while let Some(chunk) = stream_reader.stream_next(333).unwrap() {
            total += chunk.len();
        }

        assert_eq!(total, full.len());
        assert_eq!(first.spatial_reference().wkt(), "EPSG:4326");
        assert_eq!(
            first.get_f64(0, &DimId::GpsTime),
            full.get_f64(0, &DimId::GpsTime)
        );
        assert_eq!(first.get_f64(0, &DimId::X), full.get_f64(0, &DimId::X));
        assert_eq!(
            second.get_f64(0, &DimId::Intensity),
            full.get_f64(first.len(), &DimId::Intensity)
        );
    }

    #[test]
    fn rejects_missing_or_non_csd_files() {
        assert!(OptechReader::new(&Options::new()).read().is_err());

        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"NOPE").unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().to_string());

        assert!(OptechReader::new(&options).read().is_err());
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        assert_eq!(
            OptechReader::new(&Options::new()).metadata().name(),
            "readers.optech"
        );
    }

    #[test]
    fn reader_name_returns_expected() {
        assert_eq!(OptechReader::new(&Options::new()).name(), "readers.optech");
    }

    #[test]
    fn read_header_rejects_short_file() {
        let mut cursor = std::io::Cursor::new(b"NOPE");
        assert!(read_header(&mut cursor).is_err());
    }

    #[test]
    fn read_header_rejects_bad_signature() {
        let mut buf = vec![0u8; 256];
        buf[0..4].copy_from_slice(b"NOPE");
        let mut cursor = std::io::Cursor::new(buf);
        assert!(read_header(&mut cursor).is_err());
    }
}
