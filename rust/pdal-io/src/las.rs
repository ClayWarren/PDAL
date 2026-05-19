//! `readers.las` and `readers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasReader.cpp` using the `las` Rust crate.

use las::point::ScanDirection;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::path::Path;
use std::rc::Rc;

pub struct LasReader {
    filename: String,
}

impl LasReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Reader for LasReader {
    fn name(&self) -> &str {
        "readers.las"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "LasReader requires a filename option.".to_string(),
            ));
        }

        let mut reader = las::Reader::from_path(Path::new(&self.filename))
            .map_err(|e| StageError(format!("Failed to open LAS file: {}", e)))?;

        let header = reader.header();
        let mut layout = PointLayout::new();

        // Register standard LAS dimensions
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::U16);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        layout.register(DimId::ScanDirectionFlag, DimType::U8);
        layout.register(DimId::EdgeOfFlightLine, DimType::U8);
        layout.register(DimId::Classification, DimType::U8);
        layout.register(DimId::ScanAngleRank, DimType::F32);
        layout.register(DimId::UserData, DimType::U8);
        layout.register(DimId::PointSourceId, DimType::U16);

        if header.point_format().has_gps_time {
            layout.register(DimId::GpsTime, DimType::F64);
        }
        if header.point_format().has_color {
            layout.register(DimId::Red, DimType::U16);
            layout.register(DimId::Green, DimType::U16);
            layout.register(DimId::Blue, DimType::U16);
        }
        if header.point_format().has_nir {
            layout.register(DimId::Infrared, DimType::U16);
        }

        let mut view = PointView::new(Rc::new(layout));

        for point in reader.points() {
            let point =
                point.map_err(|e| StageError(format!("Failed to read LAS point: {}", e)))?;
            let id = view.add_point();

            view.set_f64(id, &DimId::X, point.x);
            view.set_f64(id, &DimId::Y, point.y);
            view.set_f64(id, &DimId::Z, point.z);
            view.set_f64(id, &DimId::Intensity, point.intensity as f64);
            view.set_f64(id, &DimId::ReturnNumber, point.return_number as f64);
            view.set_f64(id, &DimId::NumberOfReturns, point.number_of_returns as f64);
            view.set_f64(
                id,
                &DimId::ScanDirectionFlag,
                match point.scan_direction {
                    ScanDirection::LeftToRight => 1.0,
                    ScanDirection::RightToLeft => 0.0,
                },
            );
            view.set_f64(
                id,
                &DimId::EdgeOfFlightLine,
                if point.is_edge_of_flight_line {
                    1.0
                } else {
                    0.0
                },
            );
            view.set_f64(
                id,
                &DimId::Classification,
                u8::from(point.classification) as f64,
            );
            view.set_f64(id, &DimId::ScanAngleRank, point.scan_angle as f64);
            view.set_f64(id, &DimId::UserData, point.user_data as f64);
            view.set_f64(id, &DimId::PointSourceId, point.point_source_id as f64);

            if let Some(gps_time) = point.gps_time {
                view.set_f64(id, &DimId::GpsTime, gps_time);
            }
            if let Some(color) = point.color {
                view.set_f64(id, &DimId::Red, color.red as f64);
                view.set_f64(id, &DimId::Green, color.green as f64);
                view.set_f64(id, &DimId::Blue, color.blue as f64);
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.las")
    }
}
