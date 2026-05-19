//! `readers.las` and `readers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasReader.cpp` using the `las` Rust crate.

use byteorder::{LittleEndian, ReadBytesExt};
use las::point::ScanDirection;
use las_crs::ParseEpsgCRS;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::io::{Cursor, Read};
use std::path::Path;
use std::rc::Rc;

pub struct LasReader {
    filename: String,
    metadata: MetadataNode,
}

struct ExtraDim {
    name: String,
    ty: DimType,
    size: usize,
    offset: usize,
    scale: f64,
    value_offset: f64,
}

impl LasReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            metadata: MetadataNode::new("readers.las"),
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

        // Extract metadata
        self.metadata = MetadataNode::new("readers.las");
        self.metadata.add_value(
            "major_version",
            MetadataValue::U64(header.version().major as u64),
        );
        self.metadata.add_value(
            "minor_version",
            MetadataValue::U64(header.version().minor as u64),
        );
        self.metadata.add_value(
            "dataformat_id",
            MetadataValue::U64(header.point_format().to_u8().unwrap_or(3) as u64),
        );
        self.metadata.add_value(
            "filesource_id",
            MetadataValue::U64(header.file_source_id() as u64),
        );
        self.metadata.add_value(
            "system_id",
            MetadataValue::String(header.system_identifier().to_string()),
        );
        self.metadata.add_value(
            "software_id",
            MetadataValue::String(header.generating_software().to_string()),
        );
        self.metadata
            .add_value("scale_x", MetadataValue::F64(header.transforms().x.scale));
        self.metadata
            .add_value("scale_y", MetadataValue::F64(header.transforms().y.scale));
        self.metadata
            .add_value("scale_z", MetadataValue::F64(header.transforms().z.scale));
        self.metadata
            .add_value("offset_x", MetadataValue::F64(header.transforms().x.offset));
        self.metadata
            .add_value("offset_y", MetadataValue::F64(header.transforms().y.offset));
        self.metadata
            .add_value("offset_z", MetadataValue::F64(header.transforms().z.offset));
        self.metadata
            .add_value("count", MetadataValue::U64(header.number_of_points()));

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

        // Parse Extra Bytes VLR
        let mut extra_dims = Vec::new();
        for vlr in header.vlrs().iter().chain(header.evlrs().iter()) {
            if vlr.user_id == "LASF_Spec" && vlr.record_id == 4 {
                let mut cursor = Cursor::new(&vlr.data);
                let mut current_offset = 0;
                while (cursor.position() as usize) < vlr.data.len() {
                    let _reserved = cursor
                        .read_u16::<LittleEndian>()
                        .map_err(|e| StageError(e.to_string()))?;
                    let data_type = cursor.read_u8().map_err(|e| StageError(e.to_string()))?;
                    let options = cursor.read_u8().map_err(|e| StageError(e.to_string()))?;
                    let mut name_buf = [0u8; 32];
                    cursor
                        .read_exact(&mut name_buf)
                        .map_err(|e| StageError(e.to_string()))?;
                    let name = String::from_utf8_lossy(&name_buf)
                        .trim_matches('\0')
                        .trim()
                        .to_string();
                    let _reserved2 = cursor
                        .read_u32::<LittleEndian>()
                        .map_err(|e| StageError(e.to_string()))?;
                    let mut _unused = [0u8; 24];
                    cursor
                        .read_exact(&mut _unused)
                        .map_err(|e| StageError(e.to_string()))?;
                    cursor
                        .read_exact(&mut _unused)
                        .map_err(|e| StageError(e.to_string()))?;
                    cursor
                        .read_exact(&mut _unused)
                        .map_err(|e| StageError(e.to_string()))?;
                    let mut scales = [0f64; 3];
                    for s in &mut scales {
                        *s = cursor
                            .read_f64::<LittleEndian>()
                            .map_err(|e| StageError(e.to_string()))?;
                    }
                    let mut offsets = [0f64; 3];
                    for o in &mut offsets {
                        *o = cursor
                            .read_f64::<LittleEndian>()
                            .map_err(|e| StageError(e.to_string()))?;
                    }
                    let mut desc_buf = [0u8; 32];
                    cursor
                        .read_exact(&mut desc_buf)
                        .map_err(|e| StageError(e.to_string()))?;

                    let (pdal_ty_opt, field_cnt) = las_to_pdal_type(data_type);
                    if let Some(pdal_ty) = pdal_ty_opt {
                        let scale = if (options & (1 << 3)) != 0 {
                            scales[0]
                        } else {
                            1.0
                        };
                        let offset = if (options & (1 << 4)) != 0 {
                            offsets[0]
                        } else {
                            0.0
                        };

                        let dim_ty = if scale != 1.0 || offset != 0.0 {
                            DimType::F64
                        } else {
                            pdal_ty
                        };

                        if field_cnt == 1 {
                            let dim_id = DimId::from_name(&name);
                            layout.register(dim_id.clone(), dim_ty);
                            extra_dims.push(ExtraDim {
                                name,
                                ty: pdal_ty,
                                size: pdal_ty.size(),
                                offset: current_offset,
                                scale,
                                value_offset: offset,
                            });
                            current_offset += pdal_ty.size();
                        } else {
                            for i in 0..field_cnt {
                                let sub_name = format!("{}{}", name, i);
                                let dim_id = DimId::from_name(&sub_name);
                                let field_scale = if (options & (1 << 3)) != 0 {
                                    scales[i]
                                } else {
                                    1.0
                                };
                                let field_offset = if (options & (1 << 4)) != 0 {
                                    offsets[i]
                                } else {
                                    0.0
                                };
                                let dim_ty = if field_scale != 1.0 || field_offset != 0.0 {
                                    DimType::F64
                                } else {
                                    pdal_ty
                                };
                                layout.register(dim_id, dim_ty);
                                extra_dims.push(ExtraDim {
                                    name: sub_name,
                                    ty: pdal_ty,
                                    size: pdal_ty.size(),
                                    offset: current_offset,
                                    scale: field_scale,
                                    value_offset: field_offset,
                                });
                                current_offset += pdal_ty.size();
                            }
                        }
                    } else {
                        // type 0 means undocumented bytes
                        current_offset += options as usize;
                    }
                }
            }
        }

        let mut view = PointView::new(Rc::new(layout));

        // Extract SRS
        if let Some(wkt_bytes) = header.get_wkt_crs_bytes() {
            if let Ok(wkt) = String::from_utf8(wkt_bytes.to_vec()) {
                view.set_spatial_reference(pdal_core::srs::SpatialReference::new(&wkt));
            }
        } else if let Ok(Some(crs)) = header.get_epsg_crs() {
            view.set_spatial_reference(pdal_core::srs::SpatialReference::new(&format!(
                "EPSG:{}",
                crs.get_horizontal()
            )));
        }

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

            // Decode extra bytes
            for ed in &extra_dims {
                if ed.offset + ed.size <= point.extra_bytes.len() {
                    let mut cursor =
                        Cursor::new(&point.extra_bytes[ed.offset..ed.offset + ed.size]);
                    let val = read_pdal_val(&mut cursor, ed.ty)? * ed.scale + ed.value_offset;
                    view.set_f64(id, &DimId::from_name(&ed.name), val);
                }
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

fn las_to_pdal_type(lastype: u8) -> (Option<DimType>, usize) {
    let mut ty = lastype;
    let mut field_cnt = 1;
    while ty > 10 {
        field_cnt += 1;
        ty -= 10;
    }

    let pdal_ty = match ty {
        1 => Some(DimType::U8),
        2 => Some(DimType::I8),
        3 => Some(DimType::U16),
        4 => Some(DimType::I16),
        5 => Some(DimType::U32),
        6 => Some(DimType::I32),
        7 => Some(DimType::U64),
        8 => Some(DimType::I64),
        9 => Some(DimType::F32),
        10 => Some(DimType::F64),
        _ => None,
    };
    (pdal_ty, field_cnt)
}

fn read_pdal_val(reader: &mut dyn std::io::Read, ty: DimType) -> Result<f64, StageError> {
    match ty {
        DimType::U8 => reader
            .read_u8()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::I8 => reader
            .read_i8()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::U16 => reader
            .read_u16::<LittleEndian>()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::I16 => reader
            .read_i16::<LittleEndian>()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::U32 => reader
            .read_u32::<LittleEndian>()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::I32 => reader
            .read_i32::<LittleEndian>()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::U64 => reader
            .read_u64::<LittleEndian>()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::I64 => reader
            .read_i64::<LittleEndian>()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::F32 => reader
            .read_f32::<LittleEndian>()
            .map(|v| v as f64)
            .map_err(|e| StageError(e.to_string())),
        DimType::F64 => reader
            .read_f64::<LittleEndian>()
            .map_err(|e| StageError(e.to_string())),
    }
}
