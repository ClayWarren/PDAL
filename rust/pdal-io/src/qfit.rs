//! `readers.qfit` -- NASA Airborne Topographic Mapper (ATM) QFIT format.
//!
//! Port of `io/QfitReader.cpp`. QFIT is a binary format where records consist
//! of 32-bit (4-byte) signed integers; files may be big- or little-endian.
//!
//! Common formats are 10-word (40 bytes), 12-word (48 bytes), and
//! 14-word (56 bytes).

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum QfitEndian {
    Big,
    Little,
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

        let first_word = reader
            .read_i32::<LittleEndian>()
            .map_err(|_| StageError("Incomplete QFIT header.".to_string()))?;
        let endian = if first_word < 100 {
            QfitEndian::Little
        } else {
            QfitEndian::Big
        };
        let record_size = if endian == QfitEndian::Little {
            first_word
        } else {
            BigEndian::read_i32(&first_word.to_le_bytes())
        };

        if record_size != 40 && record_size != 48 && record_size != 56 {
            return Err(StageError(format!(
                "Invalid QFIT record size: {}. Expected 40, 48, or 56.",
                record_size
            )));
        }
        let word_count = (record_size / 4) as usize;

        reader
            .seek(SeekFrom::Start(record_size as u64 + 4))
            .map_err(|_| StageError("Failed to seek to QFIT offset record.".to_string()))?;
        let offset = read_i32(&mut reader, endian)
            .map_err(|_| StageError("Incomplete QFIT offset record.".to_string()))?;
        if offset < 0 {
            return Err(StageError("Invalid negative QFIT data offset.".to_string()));
        }

        reader
            .seek(SeekFrom::Start(offset as u64))
            .map_err(|_| StageError("Failed to seek to QFIT data segment.".to_string()))?;

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

        if word_count == 12 {
            layout.register(DimId::Pdop, DimType::F64);
            layout.register(DimId::PulseWidth, DimType::F64);
        } else if word_count == 14 {
            layout.register(DimId::PassiveSignal, DimType::F64);
            layout.register(DimId::PassiveY, DimType::F64);
            layout.register(DimId::PassiveX, DimType::F64);
            layout.register(DimId::PassiveZ, DimType::F64);
        }
        // GPS time is discarded as per PDAL behavior.

        let mut view = PointView::new(Rc::new(layout));
        let mut buf = vec![0i32; word_count];

        loop {
            for value in &mut buf {
                match read_i32(&mut reader, endian) {
                    Ok(v) => *value = v,
                    Err(_) => return Ok(vec![view]),
                }
            }
            let id = view.add_point();

            let mut x = buf[2] as f64 / 1_000_000.0;
            if self.flip_coordinates && x > 180.0 {
                x -= 360.0;
            }

            view.set_f64(id, &DimId::OffsetTime, buf[0] as f64);
            view.set_f64(id, &DimId::Y, buf[1] as f64 / 1_000_000.0);
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Z, buf[3] as f64 * self.scale_z);
            view.set_f64(id, &DimId::StartPulse, buf[4] as f64);
            view.set_f64(id, &DimId::ReflectedPulse, buf[5] as f64);
            view.set_f64(id, &DimId::Azimuth, buf[6] as f64 / 1000.0);
            view.set_f64(id, &DimId::Pitch, buf[7] as f64 / 1000.0);
            view.set_f64(id, &DimId::Roll, buf[8] as f64 / 1000.0);

            if word_count == 12 {
                view.set_f64(id, &DimId::Pdop, buf[10] as f64 / 10.0);
                view.set_f64(id, &DimId::PulseWidth, buf[11] as f64);
            } else if word_count == 14 {
                let mut passive_x = buf[11] as f64 / 1_000_000.0;
                if self.flip_coordinates && passive_x > 180.0 {
                    passive_x -= 360.0;
                }
                view.set_f64(id, &DimId::PassiveSignal, buf[9] as f64);
                view.set_f64(id, &DimId::PassiveY, buf[10] as f64 / 1_000_000.0);
                view.set_f64(id, &DimId::PassiveX, passive_x);
                view.set_f64(id, &DimId::PassiveZ, buf[12] as f64 * self.scale_z);
            }
        }
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.qfit")
    }
}

fn read_i32<R: Read>(reader: &mut R, endian: QfitEndian) -> Result<i32, std::io::Error> {
    match endian {
        QfitEndian::Big => reader.read_i32::<BigEndian>(),
        QfitEndian::Little => reader.read_i32::<LittleEndian>(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_i32_little_endian() {
        let buf = [0x78, 0x56, 0x34, 0x12];
        let val = read_i32(&mut &buf[..], QfitEndian::Little).unwrap();
        assert_eq!(val, 0x12345678);
    }

    #[test]
    fn read_i32_big_endian() {
        let buf = [0x12, 0x34, 0x56, 0x78];
        let val = read_i32(&mut &buf[..], QfitEndian::Big).unwrap();
        assert_eq!(val, 0x12345678);
    }

    #[test]
    fn empty_filename_is_error() {
        let mut r = QfitReader {
            filename: String::new(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        match r.read().err() {
            Some(e) => assert!(e.0.contains("requires a filename")),
            None => panic!("expected error"),
        }
    }

    #[test]
    fn nonexistent_file_is_error() {
        let mut r = QfitReader {
            filename: "/nonexistent/qfit-file.qi".to_string(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        match r.read().err() {
            Some(e) => assert!(e.0.contains("Couldn't open")),
            None => panic!("expected error"),
        }
    }

    #[test]
    fn flip_coordinates_reads_file() {
        // Use the existing 10-word fixture to test the reader
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = repo.join("test/data/qfit/10-word.qi");
        let mut r = QfitReader {
            filename: input.display().to_string(),
            flip_coordinates: true,
            scale_z: 0.001,
        };
        let views = r.read().unwrap();
        assert!(!views.is_empty());
        assert!(views[0].len() > 0);
    }

    #[test]
    fn name_returns_readers_qfit() {
        let r = QfitReader {
            filename: "dummy".to_string(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        assert_eq!(r.name(), "readers.qfit");
    }

    #[test]
    fn metadata_returns_readers_qfit() {
        let r = QfitReader {
            filename: "dummy".to_string(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        assert_eq!(r.metadata().name(), "readers.qfit");
    }

    #[test]
    fn reads_14_word_file() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = repo.join("test/data/qfit/14-word.qi");
        let mut r = QfitReader {
            filename: input.display().to_string(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        let views = r.read().unwrap();
        assert!(!views.is_empty());
        assert!(views[0].len() > 0);
    }

    #[test]
    fn reads_10_word_without_flip() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = repo.join("test/data/qfit/10-word.qi");
        let mut r = QfitReader {
            filename: input.display().to_string(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        let views = r.read().unwrap();
        assert!(!views.is_empty());
    }

    #[test]
    fn errors_on_invalid_record_size() {
        use std::io::Write;
        // Build a tiny file with first i32 LE = 32 (not 40, 48, or 56)
        let dir = std::env::temp_dir();
        let path = dir.join(format!("pdal-qfit-bad-{}.qi", std::process::id()));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&32i32.to_le_bytes()).unwrap();
        // pad
        f.write_all(&[0u8; 64]).unwrap();
        let mut r = QfitReader {
            filename: path.display().to_string(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        let err = r.read().err().unwrap();
        assert!(err.0.contains("record size"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn errors_on_incomplete_header() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join(format!("pdal-qfit-empty-{}.qi", std::process::id()));
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"a")
            .unwrap();
        let mut r = QfitReader {
            filename: path.display().to_string(),
            flip_coordinates: false,
            scale_z: 0.001,
        };
        let err = r.read().err().unwrap();
        assert!(err.0.contains("Incomplete"));
        std::fs::remove_file(&path).ok();
    }
}
