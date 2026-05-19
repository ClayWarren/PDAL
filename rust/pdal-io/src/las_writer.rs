//! `writers.las` and `writers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasWriter.cpp` using the `las` Rust crate.

use las::point::{Classification, Format, ScanDirection};
use las::{Builder, Header, Point};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;
use std::path::Path;

pub struct LasWriter {
    filename: String,
    _compression: bool,
    point_format: u8,
}

impl LasWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            _compression: options.get_bool("compression", false),
            point_format: options.get_u64("point_format", 3) as u8,
        }
    }
}

impl Writer for LasWriter {
    fn name(&self) -> &str {
        "writers.las"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "LasWriter requires a filename option.".to_string(),
            ));
        }

        let mut builder = Builder::from(Header::default());
        builder.point_format = Format::new(self.point_format)
            .map_err(|e| StageError(format!("Invalid point format: {}", e)))?;

        let path = Path::new(&self.filename);

        // Calculate bounds and offsets based on input views
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut min_z = f64::MAX;

        let mut has_points = false;
        for view in views {
            for i in 0..view.len() {
                let x = view.get_f64(i, &DimId::X);
                let y = view.get_f64(i, &DimId::Y);
                let z = view.get_f64(i, &DimId::Z);
                if x < min_x {
                    min_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if z < min_z {
                    min_z = z;
                }
                has_points = true;
            }
        }

        if has_points {
            builder.transforms = las::Vector {
                x: las::Transform {
                    scale: 0.01,
                    offset: min_x,
                },
                y: las::Transform {
                    scale: 0.01,
                    offset: min_y,
                },
                z: las::Transform {
                    scale: 0.01,
                    offset: min_z,
                },
            };
        }

        let header = builder
            .into_header()
            .map_err(|e| StageError(format!("Failed to create LAS header: {}", e)))?;

        // las::Writer::from_path automatically handles .laz extension if 'laz' feature is enabled
        let mut writer = las::Writer::from_path(path, header)
            .map_err(|e| StageError(format!("Failed to create LAS/LAZ writer: {}", e)))?;

        for view in views {
            for i in 0..view.len() {
                let scan_direction = if view.get_f64(i, &DimId::ScanDirectionFlag) > 0.0 {
                    ScanDirection::LeftToRight
                } else {
                    ScanDirection::RightToLeft
                };

                let class_val = view.get_f64(i, &DimId::Classification) as u8;
                let classification = match class_val {
                    0 => Classification::CreatedNeverClassified,
                    1 => Classification::Unclassified,
                    2 => Classification::Ground,
                    3 => Classification::LowVegetation,
                    4 => Classification::MediumVegetation,
                    5 => Classification::HighVegetation,
                    6 => Classification::Building,
                    7 => Classification::LowPoint,
                    8 => Classification::ModelKeyPoint,
                    9 => Classification::Water,
                    v => Classification::Reserved(v),
                };

                let mut point = Point {
                    x: view.get_f64(i, &DimId::X),
                    y: view.get_f64(i, &DimId::Y),
                    z: view.get_f64(i, &DimId::Z),
                    intensity: view.get_f64(i, &DimId::Intensity) as u16,
                    return_number: view.get_f64(i, &DimId::ReturnNumber) as u8,
                    number_of_returns: view.get_f64(i, &DimId::NumberOfReturns) as u8,
                    scan_direction,
                    is_edge_of_flight_line: view.get_f64(i, &DimId::EdgeOfFlightLine) > 0.0,
                    classification,
                    scan_angle: view.get_f64(i, &DimId::ScanAngleRank) as f32,
                    user_data: view.get_f64(i, &DimId::UserData) as u8,
                    point_source_id: view.get_f64(i, &DimId::PointSourceId) as u16,
                    ..Default::default()
                };

                if view.layout().dim(&DimId::GpsTime).is_some() {
                    point.gps_time = Some(view.get_f64(i, &DimId::GpsTime));
                }
                if view.layout().dim(&DimId::Red).is_some() {
                    point.color = Some(las::Color {
                        red: view.get_f64(i, &DimId::Red) as u16,
                        green: view.get_f64(i, &DimId::Green) as u16,
                        blue: view.get_f64(i, &DimId::Blue) as u16,
                    });
                }

                writer
                    .write_point(point)
                    .map_err(|e| StageError(format!("Failed to write LAS point: {}", e)))?;
            }
        }

        writer
            .close()
            .map_err(|e| StageError(format!("Failed to close LAS writer: {}", e)))?;

        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.las")
    }
}
