//! `readers.optech` -- Optech CSD reader.
//!
//! Port of `io/OptechReader.cpp` for local CSD fixtures.

use byteorder::{LittleEndian, ReadBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

const MAX_RETURNS: usize = 4;

#[derive(Clone, Copy)]
struct Xyz {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Copy)]
struct RotationMatrix {
    m00: f64,
    m01: f64,
    m02: f64,
    m10: f64,
    m11: f64,
    m12: f64,
    m20: f64,
    m21: f64,
    m22: f64,
}

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
}

impl OptechReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Reader for OptechReader {
    fn name(&self) -> &str {
        "readers.optech"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "OptechReader requires a filename option.".to_string(),
            ));
        }
        let file = File::open(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Unable to open {} for reading.", self.filename)))?;
        let mut reader = BufReader::new(file);
        let header = read_header(&mut reader)?;
        let boresight = create_optech_rotation_matrix(
            header.misalignment_angles[0] + header.imu_offsets[0],
            header.misalignment_angles[1] + header.imu_offsets[1],
            header.misalignment_angles[2] + header.imu_offsets[2],
        );

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

        let mut view = PointView::new(Rc::new(layout));
        view.set_spatial_reference(SpatialReference::new("EPSG:4326"));
        reader
            .seek(SeekFrom::Start(header.header_size as u64))
            .map_err(io_error)?;

        for _ in 0..header.num_records {
            let mut pulse = read_pulse(&mut reader)?;
            if pulse.return_count == 0 {
                continue;
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

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.optech")
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

fn create_optech_rotation_matrix(roll: f64, pitch: f64, heading: f64) -> RotationMatrix {
    RotationMatrix {
        m00: roll.cos() * heading.cos() + pitch.sin() * roll.sin() * heading.sin(),
        m01: pitch.cos() * heading.sin(),
        m02: heading.cos() * roll.sin() - roll.cos() * pitch.sin() * heading.sin(),
        m10: heading.cos() * pitch.sin() * roll.sin() - roll.cos() * heading.sin(),
        m11: pitch.cos() * heading.cos(),
        m12: -roll.sin() * heading.sin() - roll.cos() * heading.cos() * pitch.sin(),
        m20: -pitch.cos() * roll.sin(),
        m21: pitch.sin(),
        m22: pitch.cos() * roll.cos(),
    }
}

fn georeference_wgs84(
    range: f64,
    scan_angle: f64,
    boresight: RotationMatrix,
    imu: RotationMatrix,
    gps_point: Xyz,
) -> Xyz {
    let sensor = Xyz {
        x: range * scan_angle.sin(),
        y: 0.0,
        z: -range * scan_angle.cos(),
    };
    let aligned = rotate(sensor, boresight);
    let local_level = rotate(aligned, imu);
    let curvilinear = cartesian_to_curvilinear(local_level, gps_point.y);
    Xyz {
        x: gps_point.x + curvilinear.x,
        y: gps_point.y + curvilinear.y,
        z: gps_point.z + curvilinear.z,
    }
}

fn rotate(point: Xyz, matrix: RotationMatrix) -> Xyz {
    Xyz {
        x: matrix.m00 * point.x + matrix.m01 * point.y + matrix.m02 * point.z,
        y: matrix.m10 * point.x + matrix.m11 * point.y + matrix.m12 * point.z,
        z: matrix.m20 * point.x + matrix.m21 * point.y + matrix.m22 * point.z,
    }
}

fn cartesian_to_curvilinear(point: Xyz, latitude: f64) -> Xyz {
    let a = 6378137.0;
    let f = 1.0 / 298.257223563;
    let e2 = 2.0 * f - f * f;
    let w = (1.0 - e2 * latitude.sin() * latitude.sin()).sqrt();
    let n = a / w;
    let m = a * (1.0 - e2) / (w * w * w);
    Xyz {
        x: point.x / (n * latitude.cos()),
        y: point.y / m,
        z: point.z,
    }
}

fn io_error(error: std::io::Error) -> StageError {
    StageError(error.to_string())
}
