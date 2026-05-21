//! `readers.las` and `readers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasReader.cpp` using the `las` Rust crate.

use byteorder::{LittleEndian, ReadBytesExt};
use las::point::ScanDirection;
use las::Header;
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
    start: u64,
    count: Option<u64>,
    nosrs: bool,
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
            start: options.get_u64("start", 0),
            count: options.has("count").then(|| options.get_u64("count", 0)),
            nosrs: options.get_bool("nosrs", false),
            metadata: MetadataNode::new("readers.las"),
        }
    }

    fn add_metadata(&mut self, header: &Header) {
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
    }

    fn set_spatial_reference(&self, view: &mut PointView, header: &Header) {
        if self.nosrs {
            return;
        }

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
    }

    fn read_points(
        &self,
        reader: &mut las::Reader,
        point_count: u64,
        view: &mut PointView,
        extra_dims: &[ExtraDim],
    ) -> Result<(), StageError> {
        let take_count = self.count.unwrap_or(point_count.saturating_sub(self.start));
        for point in reader
            .points()
            .skip(self.start as usize)
            .take(take_count as usize)
        {
            let point =
                point.map_err(|e| StageError(format!("Failed to read LAS point: {}", e)))?;
            let id = view.add_point();
            set_standard_dims(view, id, &point);
            set_optional_dims(view, id, &point);
            set_extra_dims(view, id, &point, extra_dims)?;
        }
        Ok(())
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
        let point_count = header.number_of_points();
        if self.start >= point_count && point_count > 0 {
            return Err(StageError(format!(
                "LAS start point {} is outside the file's {} points.",
                self.start, point_count
            )));
        }

        self.add_metadata(header);
        let (layout, extra_dims) = las_layout(header)?;

        let mut view = PointView::new(Rc::new(layout));
        self.set_spatial_reference(&mut view, header);
        self.read_points(&mut reader, point_count, &mut view, &extra_dims)?;

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

fn las_layout(header: &Header) -> Result<(PointLayout, Vec<ExtraDim>), StageError> {
    let mut layout = PointLayout::new();
    register_standard_dims(&mut layout, header);
    let extra_dims = extra_dims_from_header(&mut layout, header)?;
    Ok((layout, extra_dims))
}

fn register_standard_dims(layout: &mut PointLayout, header: &Header) {
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Intensity, DimType::U16);
    layout.register(DimId::ReturnNumber, DimType::U8);
    layout.register(DimId::NumberOfReturns, DimType::U8);
    layout.register(DimId::ScanDirectionFlag, DimType::U8);
    layout.register(DimId::EdgeOfFlightLine, DimType::U8);
    layout.register(DimId::Synthetic, DimType::U8);
    layout.register(DimId::KeyPoint, DimType::U8);
    layout.register(DimId::Withheld, DimType::U8);
    layout.register(DimId::Overlap, DimType::U8);
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
}

fn extra_dims_from_header(
    layout: &mut PointLayout,
    header: &Header,
) -> Result<Vec<ExtraDim>, StageError> {
    let mut extra_dims = Vec::new();
    for vlr in header.vlrs().iter().chain(header.evlrs().iter()) {
        if vlr.user_id == "LASF_Spec" && vlr.record_id == 4 {
            parse_extra_bytes_vlr(layout, &mut extra_dims, &vlr.data)?;
        }
    }
    Ok(extra_dims)
}

fn parse_extra_bytes_vlr(
    layout: &mut PointLayout,
    extra_dims: &mut Vec<ExtraDim>,
    data: &[u8],
) -> Result<(), StageError> {
    let mut cursor = Cursor::new(data);
    let mut current_offset = 0;
    while (cursor.position() as usize) < data.len() {
        let record = read_extra_dim_record(&mut cursor)?;
        let (pdal_ty_opt, field_cnt) = las_to_pdal_type(record.data_type);
        if let Some(pdal_ty) = pdal_ty_opt {
            add_extra_dim_fields(
                layout,
                extra_dims,
                &record,
                pdal_ty,
                field_cnt,
                current_offset,
            );
            current_offset += pdal_ty.size() * field_cnt;
        } else {
            current_offset += record.options as usize;
        }
    }
    Ok(())
}

struct ExtraDimRecord {
    data_type: u8,
    options: u8,
    name: String,
    scales: [f64; 3],
    offsets: [f64; 3],
}

fn read_extra_dim_record(cursor: &mut Cursor<&[u8]>) -> Result<ExtraDimRecord, StageError> {
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
    skip_extra_dim_triplet(cursor)?;
    let scales = read_extra_dim_f64s(cursor)?;
    let offsets = read_extra_dim_f64s(cursor)?;
    let mut desc_buf = [0u8; 32];
    cursor
        .read_exact(&mut desc_buf)
        .map_err(|e| StageError(e.to_string()))?;
    Ok(ExtraDimRecord {
        data_type,
        options,
        name,
        scales,
        offsets,
    })
}

fn skip_extra_dim_triplet(cursor: &mut Cursor<&[u8]>) -> Result<(), StageError> {
    let mut unused = [0u8; 24];
    for _ in 0..3 {
        cursor
            .read_exact(&mut unused)
            .map_err(|e| StageError(e.to_string()))?;
    }
    Ok(())
}

fn read_extra_dim_f64s(cursor: &mut Cursor<&[u8]>) -> Result<[f64; 3], StageError> {
    let mut values = [0.0; 3];
    for value in &mut values {
        *value = cursor
            .read_f64::<LittleEndian>()
            .map_err(|e| StageError(e.to_string()))?;
    }
    Ok(values)
}

fn add_extra_dim_fields(
    layout: &mut PointLayout,
    extra_dims: &mut Vec<ExtraDim>,
    record: &ExtraDimRecord,
    pdal_ty: DimType,
    field_cnt: usize,
    current_offset: usize,
) {
    for field_idx in 0..field_cnt {
        let name = if field_cnt == 1 {
            record.name.clone()
        } else {
            format!("{}{}", record.name, field_idx)
        };
        let scale = extra_dim_scale(record, field_idx);
        let value_offset = extra_dim_offset(record, field_idx);
        let dim_ty = if scale != 1.0 || value_offset != 0.0 {
            DimType::F64
        } else {
            pdal_ty
        };
        layout.register(DimId::from_name(&name), dim_ty);
        extra_dims.push(ExtraDim {
            name,
            ty: pdal_ty,
            size: pdal_ty.size(),
            offset: current_offset + pdal_ty.size() * field_idx,
            scale,
            value_offset,
        });
    }
}

fn extra_dim_scale(record: &ExtraDimRecord, field_idx: usize) -> f64 {
    if (record.options & (1 << 3)) != 0 {
        record.scales[field_idx]
    } else {
        1.0
    }
}

fn extra_dim_offset(record: &ExtraDimRecord, field_idx: usize) -> f64 {
    if (record.options & (1 << 4)) != 0 {
        record.offsets[field_idx]
    } else {
        0.0
    }
}

fn set_standard_dims(view: &mut PointView, id: u64, point: &las::Point) {
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
    view.set_f64(id, &DimId::Synthetic, point.is_synthetic as u8 as f64);
    view.set_f64(id, &DimId::KeyPoint, point.is_key_point as u8 as f64);
    view.set_f64(id, &DimId::Withheld, point.is_withheld as u8 as f64);
    view.set_f64(id, &DimId::Overlap, point.is_overlap as u8 as f64);
    view.set_f64(
        id,
        &DimId::Classification,
        u8::from(point.classification) as f64,
    );
    view.set_f64(id, &DimId::ScanAngleRank, point.scan_angle as f64);
    view.set_f64(id, &DimId::UserData, point.user_data as f64);
    view.set_f64(id, &DimId::PointSourceId, point.point_source_id as f64);
}

fn set_optional_dims(view: &mut PointView, id: u64, point: &las::Point) {
    if let Some(gps_time) = point.gps_time {
        view.set_f64(id, &DimId::GpsTime, gps_time);
    }
    if let Some(color) = point.color {
        view.set_f64(id, &DimId::Red, color.red as f64);
        view.set_f64(id, &DimId::Green, color.green as f64);
        view.set_f64(id, &DimId::Blue, color.blue as f64);
    }
}

fn set_extra_dims(
    view: &mut PointView,
    id: u64,
    point: &las::Point,
    extra_dims: &[ExtraDim],
) -> Result<(), StageError> {
    for ed in extra_dims {
        if ed.offset + ed.size <= point.extra_bytes.len() {
            let mut cursor = Cursor::new(&point.extra_bytes[ed.offset..ed.offset + ed.size]);
            let val = read_pdal_val(&mut cursor, ed.ty)? * ed.scale + ed.value_offset;
            view.set_f64(id, &DimId::from_name(&ed.name), val);
        }
    }
    Ok(())
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
