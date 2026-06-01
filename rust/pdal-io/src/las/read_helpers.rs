use super::*;

pub(super) fn srs_vlr_order_from_options(options: &Options) -> Vec<SrsVlrKind> {
    let spec = options.get_str("srs_vlr_order", "");
    if spec.trim().is_empty() {
        return Vec::new();
    }

    spec.split(',')
        .filter_map(|part| parse_srs_vlr_kind(part.trim()))
        .collect()
}

pub(super) fn parse_srs_vlr_kind(name: &str) -> Option<SrsVlrKind> {
    match name.to_ascii_lowercase().as_str() {
        "wkt1" => Some(SrsVlrKind::Wkt1),
        "geotiff" => Some(SrsVlrKind::Geotiff),
        "projjson" => Some(SrsVlrKind::Proj),
        "wkt2" | "wkt" => Some(SrsVlrKind::Wkt2),
        _ => None,
    }
}

pub(super) fn header_must_use_wkt(header: &Header) -> bool {
    header.version().minor >= 4 || header.point_format().is_extended
}

pub(super) fn default_srs_vlr_order(header: &Header) -> Vec<SrsVlrKind> {
    if header_must_use_wkt(header) {
        vec![SrsVlrKind::Wkt2, SrsVlrKind::Proj, SrsVlrKind::Wkt1]
    } else {
        vec![SrsVlrKind::Wkt2, SrsVlrKind::Proj, SrsVlrKind::Geotiff]
    }
}

pub(super) fn find_vlr<'a>(
    header: &'a Header,
    user_id: &str,
    record_id: u16,
) -> Option<&'a las::Vlr> {
    header
        .vlrs()
        .iter()
        .chain(header.evlrs().iter())
        .find(|vlr| vlr.user_id == user_id && vlr.record_id == record_id)
}

pub(super) fn vlr_as_string(data: &[u8]) -> String {
    let len = data
        .iter()
        .rposition(|byte| *byte != 0)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    String::from_utf8_lossy(&data[..len]).trim().to_string()
}

pub(super) fn resolve_spatial_reference_from_vlrs(
    header: &Header,
    order: &[SrsVlrKind],
) -> Result<Option<pdal_core::srs::SpatialReference>, StageError> {
    let order = if order.is_empty() {
        default_srs_vlr_order(header)
    } else {
        order.to_vec()
    };

    for kind in order {
        match kind {
            SrsVlrKind::Wkt2 => {
                if let Some(vlr) = find_vlr(header, TRANSFORM_USER_ID, WKT2_RECORD_ID) {
                    let wkt = vlr_as_string(&vlr.data);
                    if !wkt.is_empty() {
                        return Ok(Some(pdal_core::srs::SpatialReference::new(&wkt)));
                    }
                }
            }
            SrsVlrKind::Proj => {
                if let Some(vlr) = find_vlr(header, PDAL_USER_ID, PROJJSON_RECORD_ID) {
                    let text = vlr_as_string(&vlr.data);
                    if !text.is_empty() {
                        return Ok(Some(pdal_core::srs::SpatialReference::new(&text)));
                    }
                }
            }
            SrsVlrKind::Wkt1 => {
                let vlr = find_vlr(header, TRANSFORM_USER_ID, WKT_RECORD_ID)
                    .or_else(|| find_vlr(header, LIBLAS_USER_ID, WKT_RECORD_ID));
                if let Some(vlr) = vlr {
                    let wkt = vlr_as_string(&vlr.data);
                    if !wkt.is_empty() {
                        return Ok(Some(pdal_core::srs::SpatialReference::new(&wkt)));
                    }
                }
            }
            SrsVlrKind::Geotiff => {
                if find_vlr(header, TRANSFORM_USER_ID, GEOTIFF_DIRECTORY_RECORD_ID).is_some() {
                    if let Some(srs) = spatial_reference_from_geotiff_vlrs(header)? {
                        return Ok(Some(srs));
                    }
                }
            }
        }
    }

    Ok(None)
}

pub(super) fn spatial_reference_from_geotiff_vlrs(
    header: &Header,
) -> Result<Option<pdal_core::srs::SpatialReference>, StageError> {
    // Match C++ las::Srs::extractGeotiff: a GeoTIFF directory that cannot be
    // turned into a CRS -- empty/missing ascii or double VLRs, or keys that
    // libgeotiff rejects -- is tolerated as "no SRS", not a hard read failure.
    // The points still load; downstream stages that need an SRS (e.g.
    // filters.reprojection) error on the resulting empty SRS instead.
    let geotiff = match header.get_geotiff_crs() {
        Ok(Some(geotiff)) => geotiff,
        Ok(None) | Err(_) => return Ok(None),
    };
    let crs = match get_epsg_from_geotiff_crs(&geotiff) {
        Ok(crs) => crs,
        Err(_) => return Ok(None),
    };
    Ok(Some(pdal_core::srs::SpatialReference::new(&format!(
        "EPSG:{}",
        crs.get_horizontal()
    ))))
}

pub(super) fn configured_extra_dims_from_options(options: &Options) -> Vec<ConfiguredExtraDim> {
    let names = options.values("extra_dim_name");
    let types = options.values("extra_dim_type");
    let count = names.len().min(types.len());

    (0..count)
        .map(|idx| ConfiguredExtraDim {
            name: names[idx].clone(),
            type_name: types[idx].clone(),
        })
        .collect()
}

pub(super) fn dim_type_from_interpretation(name: &str) -> Option<DimType> {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.contains("int8") {
        Some(DimType::I8)
    } else if normalized.contains("int16") {
        Some(DimType::I16)
    } else if normalized.contains("int32") {
        Some(DimType::I32)
    } else if normalized.contains("int64") {
        Some(DimType::I64)
    } else if normalized.contains("uint8") || normalized.contains("unsigned8") {
        Some(DimType::U8)
    } else if normalized.contains("uint16") || normalized.contains("unsigned16") {
        Some(DimType::U16)
    } else if normalized.contains("uint32") || normalized.contains("unsigned32") {
        Some(DimType::U32)
    } else if normalized.contains("uint64") || normalized.contains("unsigned64") {
        Some(DimType::U64)
    } else if normalized.contains("float") {
        Some(DimType::F32)
    } else if normalized.contains("double") {
        Some(DimType::F64)
    } else {
        None
    }
}

pub(super) fn las_layout(
    header: &Header,
    configured_extra_dims: &[ConfiguredExtraDim],
) -> Result<(PointLayout, Vec<ExtraDim>), StageError> {
    let mut layout = PointLayout::new();
    register_standard_dims(&mut layout, header);
    let extra_dims = if configured_extra_dims.is_empty() {
        extra_dims_from_header(&mut layout, header)?
    } else {
        extra_dims_from_configured(&mut layout, header, configured_extra_dims)?
    };
    Ok((layout, extra_dims))
}

pub(super) fn extra_dims_from_configured(
    layout: &mut PointLayout,
    header: &Header,
    configured_extra_dims: &[ConfiguredExtraDim],
) -> Result<Vec<ExtraDim>, StageError> {
    let mut extra_dims = Vec::new();
    let mut offset = 0usize;
    let mut remaining = header.point_format().extra_bytes as usize;

    for spec in configured_extra_dims {
        let ty = dim_type_from_interpretation(&spec.type_name).ok_or_else(|| {
            StageError(format!(
                "Invalid extra_dim type '{}' for dimension '{}'.",
                spec.type_name, spec.name
            ))
        })?;
        let size = ty.size();
        if size > remaining {
            return Err(StageError(
                "Extra byte specification exceeds point length beyond base format length."
                    .to_string(),
            ));
        }
        remaining -= size;
        layout.register(DimId::from_name(&spec.name), ty);
        extra_dims.push(ExtraDim {
            name: spec.name.clone(),
            ty,
            size,
            offset,
            scale: 1.0,
            value_offset: 0.0,
        });
        offset += size;
    }

    Ok(extra_dims)
}

pub(super) fn register_standard_dims(layout: &mut PointLayout, header: &Header) {
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
    if header.point_format().is_extended {
        layout.register(DimId::from_name("ScanChannel"), DimType::U8);
    }
}

pub(super) fn extra_dims_from_header(
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

pub(super) fn parse_extra_bytes_vlr(
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

pub(super) struct ExtraDimRecord {
    pub(super) data_type: u8,
    pub(super) options: u8,
    pub(super) name: String,
    pub(super) scales: [f64; 3],
    pub(super) offsets: [f64; 3],
}

pub(super) fn read_extra_dim_record(
    cursor: &mut Cursor<&[u8]>,
) -> Result<ExtraDimRecord, StageError> {
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

pub(super) fn skip_extra_dim_triplet(cursor: &mut Cursor<&[u8]>) -> Result<(), StageError> {
    let mut unused = [0u8; 24];
    for _ in 0..3 {
        cursor
            .read_exact(&mut unused)
            .map_err(|e| StageError(e.to_string()))?;
    }
    Ok(())
}

pub(super) fn read_extra_dim_f64s(cursor: &mut Cursor<&[u8]>) -> Result<[f64; 3], StageError> {
    let mut values = [0.0; 3];
    for value in &mut values {
        *value = cursor
            .read_f64::<LittleEndian>()
            .map_err(|e| StageError(e.to_string()))?;
    }
    Ok(values)
}

pub(super) fn add_extra_dim_fields(
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

pub(super) fn extra_dim_scale(record: &ExtraDimRecord, field_idx: usize) -> f64 {
    if (record.options & (1 << 3)) != 0 {
        record.scales[field_idx]
    } else {
        1.0
    }
}

pub(super) fn extra_dim_offset(record: &ExtraDimRecord, field_idx: usize) -> f64 {
    if (record.options & (1 << 4)) != 0 {
        record.offsets[field_idx]
    } else {
        0.0
    }
}

pub(super) fn scan_angle_degrees(point: &las::Point, point_format: u8) -> f64 {
    if point_format >= 6 {
        let scaled = (f64::from(point.scan_angle) / SCAN_ANGLE_SCALE_FACTOR).round() as i16;
        f64::from(scaled) * SCAN_ANGLE_SCALE_FACTOR
    } else {
        f64::from(point.scan_angle)
    }
}

pub(super) fn set_standard_dims(
    view: &mut PointView,
    id: u64,
    point: &las::Point,
    point_format: u8,
) {
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
    view.set_f64(
        id,
        &DimId::ScanAngleRank,
        scan_angle_degrees(point, point_format),
    );
    view.set_f64(id, &DimId::UserData, point.user_data as f64);
    view.set_f64(id, &DimId::PointSourceId, point.point_source_id as f64);
    let scan_channel = DimId::from_name("ScanChannel");
    if view.layout().dim(&scan_channel).is_some() {
        view.set_f64(id, &scan_channel, point.scanner_channel as f64);
    }
}

pub(super) fn set_optional_dims(view: &mut PointView, id: u64, point: &las::Point) {
    if let Some(gps_time) = point.gps_time {
        view.set_f64(id, &DimId::GpsTime, gps_time);
    }
    if let Some(color) = point.color {
        view.set_f64(id, &DimId::Red, color.red as f64);
        view.set_f64(id, &DimId::Green, color.green as f64);
        view.set_f64(id, &DimId::Blue, color.blue as f64);
    }
}

pub(super) fn set_extra_dims(
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

pub(super) fn las_to_pdal_type(lastype: u8) -> (Option<DimType>, usize) {
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

pub(super) fn read_pdal_val(
    reader: &mut dyn std::io::Read,
    ty: DimType,
) -> Result<f64, StageError> {
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
