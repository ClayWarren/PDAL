//! `readers.qfit` -- NASA Airborne Topographic Mapper (ATM) QFIT format.
//!
//! Port of `io/QfitReader.cpp`. QFIT is a big-endian binary format where
//! records consist of 32-bit (4-byte) signed integers.
//!
//! Common formats are 10-word (40 bytes), 12-word (48 bytes), and
//! 14-word (56 bytes).

use byteorder::{BigEndian, ReadBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

pub struct QfitReader {
    filename: String,
    flip_coordinates: bool,
    scale_z: f64,
}

impl QfitReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            flip_coordinates: options.get_bool("flip_coordinates", false),
            scale_z: options.get_f64("scale_z", 0.001), // elevation is mm
        }
    }
}

impl Reader for QfitReader {
    fn name(&self) -> &str {
        "readers.qfit"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "QfitReader requires a filename option.".to_string(),
            ));
        }
        let file = File::open(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}'.", self.filename)))?;
        let mut reader = BufReader::new(file);

        // Header word 1: Record size in bytes
        let record_size = reader
            .read_i32::<BigEndian>()
            .map_err(|_| StageError("Incomplete QFIT header.".to_string()))?;

        if record_size != 40 && record_size != 48 && record_size != 56 {
            return Err(StageError(format!(
                "Invalid QFIT record size: {}. Expected 40, 48, or 56.",
                record_size
            )));
        }
        let word_count = (record_size / 4) as usize;

        // Skip the rest of the first header record
        reader
            .seek(SeekFrom::Start(record_size as u64))
            .map_err(|_| StageError("Failed to seek to data segment.".to_string()))?;

        // Skip processing history records (records starting with a negative value)
        let mut first_word = [0u8; 4];
        loop {
            reader.read_exact(&mut first_word).map_err(|_| {
                StageError("Unexpected end of file before data segment.".to_string())
            })?;
            let val = (&first_word[..]).read_i32::<BigEndian>().unwrap();
            if val >= 0 {
                break;
            }
            reader
                .seek(SeekFrom::Current((record_size - 4) as i64))
                .map_err(|_| StageError("Failed to skip history record.".to_string()))?;
        }

        reader
            .seek(SeekFrom::Current(-4))
            .map_err(|_| StageError("Failed to rewind to start of data record.".to_string()))?;

        let mut layout = PointLayout::new();
        layout.register(DimId::OffsetTime, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::StartPulse, DimType::F64);
        layout.register(DimId::ReflectedPulse, DimType::F64);
        layout.register(DimId::Azimuth, DimType::F64);
        layout.register(DimId::Pitch, DimType::F64);
        layout.register(DimId::Roll, DimType::F64);

        if word_count >= 12 {
            layout.register(DimId::Pdop, DimType::F64);
            layout.register(DimId::PulseWidth, DimType::F64);
        }
        // GPS time is discarded as per PDAL behavior.

        let mut view = PointView::new(Rc::new(layout));
        let mut buf = vec![0i32; word_count];

        while reader.read_i32_into::<BigEndian>(&mut buf).is_ok() {
            let id = view.add_point();

            let (lat_idx, lon_idx) = if self.flip_coordinates {
                (2, 1)
            } else {
                (1, 2)
            };

            view.set_f64(id, &DimId::OffsetTime, buf[0] as f64);
            view.set_f64(id, &DimId::Y, buf[lat_idx] as f64 / 1_000_000.0);
            view.set_f64(id, &DimId::X, buf[lon_idx] as f64 / 1_000_000.0);
            view.set_f64(id, &DimId::Z, buf[3] as f64 * self.scale_z);
            view.set_f64(id, &DimId::StartPulse, buf[4] as f64);
            view.set_f64(id, &DimId::ReflectedPulse, buf[5] as f64);
            view.set_f64(id, &DimId::Azimuth, buf[6] as f64 / 1000.0);
            view.set_f64(id, &DimId::Pitch, buf[7] as f64 / 1000.0);
            view.set_f64(id, &DimId::Roll, buf[8] as f64 / 1000.0);

            if word_count >= 12 {
                view.set_f64(id, &DimId::Pdop, buf[10] as f64 / 10.0);
                view.set_f64(id, &DimId::PulseWidth, buf[11] as f64);
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.qfit")
    }
}
