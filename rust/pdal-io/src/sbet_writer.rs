//! `writers.sbet` -- Applanix (Trimble) SBET trajectory format.
//!
//! Port of `io/SbetWriter.cpp`. SBET is a binary format consisting of a
//! sequence of 17 double-precision (8-byte) floating point values per point.
//! All values are Little Endian.

use byteorder::{LittleEndian, WriteBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub struct SbetWriter {
    filename: String,
    angles_are_degrees: bool,
}

impl SbetWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            angles_are_degrees: options.get_bool("angles_are_degrees", true),
        }
    }

    fn file_dimensions() -> &'static [DimId] {
        &[
            DimId::GpsTime,
            DimId::Y,
            DimId::X,
            DimId::Z,
            DimId::XVelocity,
            DimId::YVelocity,
            DimId::ZVelocity,
            DimId::Roll,
            DimId::Pitch,
            DimId::Azimuth,
            DimId::WanderAngle,
            DimId::XBodyAccel,
            DimId::YBodyAccel,
            DimId::ZBodyAccel,
            DimId::XBodyAngRate,
            DimId::YBodyAngRate,
            DimId::ZBodyAngRate,
        ]
    }

    fn is_angular(dim: &DimId) -> bool {
        matches!(
            dim,
            DimId::X
                | DimId::Y
                | DimId::Roll
                | DimId::Pitch
                | DimId::Azimuth
                | DimId::WanderAngle
                | DimId::XBodyAngRate
                | DimId::YBodyAngRate
                | DimId::ZBodyAngRate
        )
    }
}

impl Writer for SbetWriter {
    fn name(&self) -> &str {
        "writers.sbet"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "SbetWriter requires a filename option.".to_string(),
            ));
        }
        let file = File::create(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}' for writing.", self.filename)))?;
        let mut writer = BufWriter::new(file);

        let dims = Self::file_dimensions();
        let deg_to_rad = std::f64::consts::PI / 180.0;

        for view in views {
            for i in 0..view.len() {
                for dim in dims {
                    let mut val = view.get_f64(i, dim);
                    if self.angles_are_degrees && Self::is_angular(dim) {
                        val *= deg_to_rad;
                    }
                    writer
                        .write_f64::<LittleEndian>(val)
                        .map_err(|e| StageError(e.to_string()))?;
                }
            }
        }

        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.sbet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "pdal-sbet-writer-{name}-{}-{}.sbet",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }

    fn one_point_view() -> PointView {
        let mut layout = PointLayout::new();
        for dim in SbetWriter::file_dimensions() {
            layout.register(dim.clone(), DimType::F64);
        }

        let mut view = PointView::new(Rc::new(layout));
        let point = view.add_point();
        for (idx, dim) in SbetWriter::file_dimensions().iter().enumerate() {
            view.set_f64(point, dim, (idx + 1) as f64);
        }
        view
    }

    fn read_doubles(path: &std::path::Path) -> Vec<f64> {
        std::fs::read(path)
            .unwrap()
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn writer_reports_metadata_and_requires_filename() {
        let mut writer = SbetWriter::new(&Options::default());

        assert_eq!(writer.name(), "writers.sbet");
        assert_eq!(writer.metadata().name(), "writers.sbet");
        assert_eq!(
            writer.write(&[one_point_view()]).unwrap_err().0,
            "SbetWriter requires a filename option."
        );
    }

    #[test]
    fn writer_outputs_all_sbet_dimensions_and_converts_angles() {
        let path = temp_path("degrees");
        let mut options = Options::default();
        options.add("filename", path.to_string_lossy());
        let mut writer = SbetWriter::new(&options);

        writer.write(&[one_point_view()]).unwrap();
        let values = read_doubles(&path);

        assert_eq!(values.len(), SbetWriter::file_dimensions().len());
        assert_eq!(values[0], 1.0);
        assert_eq!(values[1], 2.0_f64.to_radians());
        assert_eq!(values[2], 3.0_f64.to_radians());
        assert_eq!(values[3], 4.0);
        assert_eq!(values[7], 8.0_f64.to_radians());
        assert_eq!(values[16], 17.0_f64.to_radians());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_can_preserve_angle_values_as_radians() {
        let path = temp_path("radians");
        let mut options = Options::default();
        options.add("filename", path.to_string_lossy());
        options.add("angles_are_degrees", false);
        let mut writer = SbetWriter::new(&options);

        writer.write(&[one_point_view()]).unwrap();
        let values = read_doubles(&path);

        assert_eq!(values[1], 2.0);
        assert_eq!(values[16], 17.0);
        let _ = std::fs::remove_file(path);
    }
}
