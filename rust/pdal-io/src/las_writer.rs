//! `writers.las` and `writers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasWriter.cpp` using the `las` Rust crate.

use byteorder::{LittleEndian, WriteBytesExt};
use chrono::NaiveDate;
use las::point::{Classification, Format, ScanDirection};
use las::{Builder, Header, Point, Vlr};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub struct LasWriter {
    filename: String,
    compression: bool,
    minor_version: Option<u8>,
    point_format: u8,
    scale_x: Option<f64>,
    scale_y: Option<f64>,
    scale_z: Option<f64>,
    offset_x: Option<f64>,
    offset_y: Option<f64>,
    offset_z: Option<f64>,
    file_source_id: Option<u16>,
    system_id: Option<String>,
    software_id: Option<String>,
    creation_doy: Option<u32>,
    creation_year: Option<i32>,
    project_id: Option<uuid::Uuid>,
}

struct ExtraDim {
    id: DimId,
    ty: DimType,
    size: usize,
}

impl LasWriter {
    pub fn new(options: &Options) -> Self {
        Self::new_with_compression(options, false)
    }

    pub fn new_laz(options: &Options) -> Self {
        Self::new_with_compression(options, true)
    }

    fn new_with_compression(options: &Options, driver_requests_compression: bool) -> Self {
        let point_format = ["dataformat_id", "format", "point_format"]
            .into_iter()
            .find_map(|key| numeric_option_u8(options, key))
            .unwrap_or(3);

        Self {
            filename: options.get_str("filename", ""),
            compression: driver_requests_compression || options.get_bool("compression", false),
            minor_version: numeric_option_u8(options, "minor_version"),
            point_format,
            scale_x: numeric_option_f64(options, "scale_x"),
            scale_y: numeric_option_f64(options, "scale_y"),
            scale_z: numeric_option_f64(options, "scale_z"),
            offset_x: numeric_option_f64(options, "offset_x"),
            offset_y: numeric_option_f64(options, "offset_y"),
            offset_z: numeric_option_f64(options, "offset_z"),
            file_source_id: numeric_option_u16(options, "filesource_id"),
            system_id: string_option(options, "system_id"),
            software_id: string_option(options, "software_id"),
            creation_doy: numeric_option_u32(options, "creation_doy"),
            creation_year: numeric_option_i32(options, "creation_year"),
            project_id: options
                .value("project_id")
                .and_then(|value| uuid::Uuid::parse_str(value.trim()).ok()),
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
        if let Some(minor) = self.minor_version {
            builder.version = las::Version { major: 1, minor };
        }
        if let Some(file_source_id) = self.file_source_id {
            builder.file_source_id = file_source_id;
        }
        if let Some(system_id) = &self.system_id {
            builder.system_identifier = system_id.clone();
        }
        if let Some(software_id) = &self.software_id {
            builder.generating_software = software_id.clone();
        }
        if let Some(project_id) = self.project_id {
            builder.guid = project_id;
        }
        if let (Some(year), Some(doy)) = (self.creation_year, self.creation_doy) {
            builder.date = NaiveDate::from_yo_opt(year, doy);
        }

        // If an SRS is present, we must use LAS 1.4 for WKT support
        if !views.is_empty() && !views[0].spatial_reference().is_empty() {
            builder.version = las::Version { major: 1, minor: 4 };
        }

        let path = Path::new(&self.filename);
        let extension_requests_laz = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("laz"));
        let should_compress = self.compression || extension_requests_laz;

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
                    scale: self.scale_x.unwrap_or(0.01),
                    offset: self.offset_x.unwrap_or(min_x),
                },
                y: las::Transform {
                    scale: self.scale_y.unwrap_or(0.01),
                    offset: self.offset_y.unwrap_or(min_y),
                },
                z: las::Transform {
                    scale: self.scale_z.unwrap_or(0.01),
                    offset: self.offset_z.unwrap_or(min_z),
                },
            };
        }

        // Identify extra dimensions
        let mut extra_dims = Vec::new();
        if !views.is_empty() {
            let layout = views[0].layout();
            let standard_dims = pdrf_dims(self.point_format);
            for i in 0..layout.dim_count() {
                let (dim_id, dim_ty) = layout.dim_at(i).unwrap();
                if !standard_dims.contains(dim_id) {
                    extra_dims.push(ExtraDim {
                        id: dim_id.clone(),
                        ty: dim_ty,
                        size: dim_ty.size(),
                    });
                }
            }
        }

        // Create Extra Bytes VLR
        if !extra_dims.is_empty() {
            let mut vlr_data = Vec::new();
            for ed in &extra_dims {
                vlr_data.write_u16::<LittleEndian>(0).unwrap(); // reserved
                vlr_data.write_u8(pdal_to_las_type(ed.ty)).unwrap();
                vlr_data.write_u8(0).unwrap(); // options
                let mut name_buf = [0u8; 32];
                let name = ed.id.name();
                let bytes = name.as_bytes();
                let len = bytes.len().min(32);
                name_buf[..len].copy_from_slice(&bytes[..len]);
                vlr_data.extend_from_slice(&name_buf);
                vlr_data.write_u32::<LittleEndian>(0).unwrap(); // reserved2
                for _ in 0..24 {
                    vlr_data.write_u8(0).unwrap();
                } // no_data
                for _ in 0..24 {
                    vlr_data.write_u8(0).unwrap();
                } // min
                for _ in 0..24 {
                    vlr_data.write_u8(0).unwrap();
                } // max
                for _ in 0..3 {
                    vlr_data.write_f64::<LittleEndian>(0.0).unwrap();
                } // scales
                for _ in 0..3 {
                    vlr_data.write_f64::<LittleEndian>(0.0).unwrap();
                } // offsets
                let desc_buf = [0u8; 32];
                vlr_data.extend_from_slice(&desc_buf);
            }
            builder.vlrs.push(Vlr {
                user_id: "LASF_Spec".to_string(),
                record_id: 4,
                description: "Extra Bytes Record".to_string(),
                data: vlr_data,
            });
            builder.point_format.extra_bytes =
                extra_dims.iter().map(|ed| ed.size).sum::<usize>() as u16;
        }

        builder.point_format.is_compressed = should_compress;

        let mut header = builder
            .into_header()
            .map_err(|e| StageError(format!("Failed to create LAS header: {}", e)))?;

        // Set SRS on header
        if !views.is_empty() {
            let srs = views[0].spatial_reference();
            if !srs.is_empty() && header.version().major == 1 && header.version().minor == 4 {
                header
                    .set_wkt_crs(srs.wkt().as_bytes().to_vec())
                    .unwrap_or(());
            }
        }

        let file = File::create(path)
            .map(BufWriter::new)
            .map_err(|e| StageError(format!("Failed to create LAS/LAZ file: {}", e)))?;
        let mut writer = las::Writer::new(file, header)
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
                } else if writer.header().point_format().has_gps_time {
                    point.gps_time = Some(0.0);
                }
                if view.layout().dim(&DimId::Red).is_some() {
                    point.color = Some(las::Color {
                        red: view.get_f64(i, &DimId::Red) as u16,
                        green: view.get_f64(i, &DimId::Green) as u16,
                        blue: view.get_f64(i, &DimId::Blue) as u16,
                    });
                } else if writer.header().point_format().has_color {
                    point.color = Some(las::Color {
                        red: 0,
                        green: 0,
                        blue: 0,
                    });
                }

                // Pack extra bytes
                if !extra_dims.is_empty() {
                    let mut eb = Vec::new();
                    for ed in &extra_dims {
                        write_pdal_val(&mut eb, view.get_f64(i, &ed.id), ed.ty).unwrap();
                    }
                    point.extra_bytes = eb;
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

fn pdrf_dims(pdrf: u8) -> Vec<DimId> {
    let mut dims = vec![
        DimId::X,
        DimId::Y,
        DimId::Z,
        DimId::Intensity,
        DimId::ReturnNumber,
        DimId::NumberOfReturns,
        DimId::ScanDirectionFlag,
        DimId::EdgeOfFlightLine,
        DimId::Classification,
        DimId::ScanAngleRank,
        DimId::UserData,
        DimId::PointSourceId,
    ];
    if pdrf == 1 || pdrf == 3 || pdrf >= 6 {
        dims.push(DimId::GpsTime);
    }
    if pdrf == 2 || pdrf == 3 || pdrf == 7 || pdrf == 8 {
        dims.push(DimId::Red);
        dims.push(DimId::Green);
        dims.push(DimId::Blue);
    }
    if pdrf == 8 {
        dims.push(DimId::Infrared);
    }
    dims
}

fn numeric_option_f64(options: &Options, key: &str) -> Option<f64> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<f64>().ok())
}

fn numeric_option_u8(options: &Options, key: &str) -> Option<u8> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<u8>().ok())
}

fn numeric_option_u16(options: &Options, key: &str) -> Option<u16> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<u16>().ok())
}

fn numeric_option_u32(options: &Options, key: &str) -> Option<u32> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<u32>().ok())
}

fn numeric_option_i32(options: &Options, key: &str) -> Option<i32> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<i32>().ok())
}

fn string_option(options: &Options, key: &str) -> Option<String> {
    options.value(key).map(ToString::to_string)
}

fn pdal_to_las_type(ty: DimType) -> u8 {
    match ty {
        DimType::U8 => 1,
        DimType::I8 => 2,
        DimType::U16 => 3,
        DimType::I16 => 4,
        DimType::U32 => 5,
        DimType::I32 => 6,
        DimType::U64 => 7,
        DimType::I64 => 8,
        DimType::F32 => 9,
        DimType::F64 => 10,
    }
}

fn write_pdal_val(
    writer: &mut dyn std::io::Write,
    val: f64,
    ty: DimType,
) -> Result<(), std::io::Error> {
    match ty {
        DimType::U8 => writer.write_u8(val as u8),
        DimType::I8 => writer.write_i8(val as i8),
        DimType::U16 => writer.write_u16::<LittleEndian>(val as u16),
        DimType::I16 => writer.write_i16::<LittleEndian>(val as i16),
        DimType::U32 => writer.write_u32::<LittleEndian>(val as u32),
        DimType::I32 => writer.write_i32::<LittleEndian>(val as i32),
        DimType::U64 => writer.write_u64::<LittleEndian>(val as u64),
        DimType::I64 => writer.write_i64::<LittleEndian>(val as i64),
        DimType::F32 => writer.write_f32::<LittleEndian>(val as f32),
        DimType::F64 => writer.write_f64::<LittleEndian>(val),
    }
}
