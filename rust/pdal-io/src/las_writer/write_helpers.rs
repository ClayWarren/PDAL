use super::*;

pub(super) fn generated_srs_vlrs(
    writer: &LasWriter,
    views: &[PointView],
) -> Option<pdal_native::srs::UserInput> {
    let srs_text = writer
        .a_srs
        .as_deref()
        .filter(|srs| !srs.is_empty())
        .map(str::to_string)
        .or_else(|| {
            views.first().and_then(|view| {
                let srs = view.spatial_reference();
                (!srs.is_empty()).then(|| srs.wkt().to_string())
            })
        })?;
    let mut srs = pdal_native::srs::user_input_to_wkt(&srs_text).ok()?;
    if let Ok(wkt1) = pdal_native::srs::wkt_to_wkt1(&srs.wkt2, srs.epoch) {
        if !wkt1.is_empty() {
            srs.wkt = wkt1;
        }
    }
    Some(srs)
}

pub(super) fn min_xyz(views: &[PointView]) -> Option<[f64; 3]> {
    let mut min = [f64::MAX; 3];
    let mut has_points = false;
    for view in views {
        for i in 0..view.len() {
            min[0] = min[0].min(view.get_f64(i, &DimId::X));
            min[1] = min[1].min(view.get_f64(i, &DimId::Y));
            min[2] = min[2].min(view.get_f64(i, &DimId::Z));
            has_points = true;
        }
    }
    has_points.then_some(min)
}

pub(super) fn configured_extra_dims_from_options(options: &Options) -> Vec<ConfiguredExtraDim> {
    let extra_dims = options
        .values("extra_dims")
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value == "all" {
                ConfiguredExtraDim {
                    name: "all".to_string(),
                    type_name: String::new(),
                }
            } else if let Some((name, type_name)) = value.split_once('=') {
                ConfiguredExtraDim {
                    name: name.trim().to_string(),
                    type_name: type_name.trim().to_string(),
                }
            } else {
                ConfiguredExtraDim {
                    name: value.to_string(),
                    type_name: String::new(),
                }
            }
        });
    let names = options.values("extra_dim_name");
    let types = options.values("extra_dim_type");
    let count = names.len().min(types.len());

    extra_dims
        .chain((0..count).map(|idx| ConfiguredExtraDim {
            name: names[idx].clone(),
            type_name: types[idx].clone(),
        }))
        .collect()
}

pub(super) fn numbered_filename(template: &str, number: usize) -> String {
    template.replacen('#', &number.to_string(), 1)
}

pub(super) fn resolve_extra_dims(
    views: &[PointView],
    point_format: u8,
    configured_extra_dims: &[ConfiguredExtraDim],
    reject_standard_extra_dims: bool,
) -> Result<Vec<ExtraDim>, StageError> {
    if configured_extra_dims.is_empty() {
        return Ok(extra_dims_from_views(views, point_format));
    }

    let view = views.first().ok_or_else(|| {
        StageError("LasWriter requires at least one point view for extra_dims.".to_string())
    })?;

    if configured_extra_dims
        .iter()
        .any(|spec| spec.name.eq_ignore_ascii_case("all"))
    {
        return Ok(extra_dims_from_views(views, point_format));
    }
    let standard_dims = pdrf_dims(point_format);
    configured_extra_dims
        .iter()
        .map(|spec| {
            if spec.type_name.is_empty() {
                return Err(StageError(format!(
                    "No type was specified for extra_dim '{}'.",
                    spec.name
                )));
            }
            let id = DimId::from_name(&spec.name);
            // writers.copc rejects standard dimensions as extra_dims; writers.las
            // does not (C++ CopcWriter validates this, C++ LasWriter does not).
            if reject_standard_extra_dims && standard_dims.contains(&id) {
                return Err(StageError(format!(
                    "Dimension '{}' specified in 'extra_dim' option is a standard dimension.",
                    spec.name
                )));
            }
            let ty = dim_type_from_interpretation(&spec.type_name).ok_or_else(|| {
                StageError(format!(
                    "Invalid extra_dim type '{}' for dimension '{}'.",
                    spec.type_name, spec.name
                ))
            })?;
            if view.layout().dim(&id).is_none() {
                return Err(StageError(format!(
                    "Dimension '{}' specified in extra_dim option not found.",
                    spec.name
                )));
            }
            Ok(ExtraDim {
                id,
                ty,
                size: ty.size(),
            })
        })
        .collect()
}

/// For `writers.copc`: resolve the configured `extra_dims` (rejecting standard
/// point-format dimensions, as the COPC writer does) and build the `LASF_Spec`
/// Extra Bytes VLR payload. Returns the dims and the VLR data (if any).
pub(crate) fn copc_extra_dims(
    options: &Options,
    views: &[PointView],
    point_format: u8,
) -> Result<(Vec<ExtraDim>, Option<Vec<u8>>), StageError> {
    let configured = configured_extra_dims_from_options(options);
    let dims = resolve_extra_dims(views, point_format, &configured, true)?;
    if dims.is_empty() {
        return Ok((dims, None));
    }
    let mut data = Vec::new();
    for ed in &dims {
        write_extra_dim_vlr_record(&mut data, ed)?;
    }
    Ok((dims, Some(data)))
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

pub(super) fn extra_dims_from_views(views: &[PointView], point_format: u8) -> Vec<ExtraDim> {
    let Some(view) = views.first() else {
        return Vec::new();
    };

    let layout = view.layout();
    let standard_dims = pdrf_dims(point_format);
    let mut extra_dims = Vec::new();
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
    extra_dims
}

pub(super) fn user_vlrs_from_options(options: &Options) -> Vec<UserVlr> {
    let user_ids = options.values("user_vlr_user_id");
    let record_ids = options.values("user_vlr_record_id");
    let descriptions = options.values("user_vlr_description");
    let data_values = options.values("user_vlr_data");
    let evlr_flags = options.values("user_vlr_evlr");
    let count = user_ids
        .len()
        .min(record_ids.len())
        .min(descriptions.len())
        .min(data_values.len())
        .min(evlr_flags.len());

    (0..count)
        .filter_map(|idx| {
            let record_id = record_ids[idx].trim().parse::<u16>().ok()?;
            Some(UserVlr {
                user_id: user_ids[idx].clone(),
                record_id,
                description: descriptions[idx].clone(),
                data: base64_decode(data_values[idx].trim()),
                write_as_evlr: matches!(
                    evlr_flags[idx].trim().to_lowercase().as_str(),
                    "true" | "1" | "yes" | "on"
                ),
            })
        })
        .collect()
}

pub(super) fn forward_vlrs_from_options(options: &Options) -> Vec<ForwardedVlr> {
    let user_ids = options.values("forward_vlr_user_id");
    let record_ids = options.values("forward_vlr_record_id");
    let descriptions = options.values("forward_vlr_description");
    let data_values = options.values("forward_vlr_data");
    let count = user_ids
        .len()
        .min(record_ids.len())
        .min(descriptions.len())
        .min(data_values.len());

    (0..count)
        .filter_map(|idx| {
            let record_id = record_ids[idx].trim().parse::<u16>().ok()?;
            Some(ForwardedVlr {
                user_id: user_ids[idx].clone(),
                record_id,
                description: descriptions[idx].clone(),
                data: base64_decode(data_values[idx].trim()),
            })
        })
        .collect()
}

pub(super) fn add_pdal_vlrs(
    builder: &mut Builder,
    metadata_json: Option<&str>,
    pipeline_json: Option<&str>,
) {
    let supports_evlr = builder.version.minor >= 4;

    if let Some(json) = metadata_json {
        if json.len() <= MAX_VLR_DATA_SIZE || supports_evlr {
            builder.vlrs.push(Vlr {
                user_id: PDAL_USER_ID.to_string(),
                record_id: PDAL_METADATA_RECORD_ID,
                description: "PDAL metadata".to_string(),
                data: json.as_bytes().to_vec(),
            });
        }
    }
    if let Some(json) = pipeline_json {
        if json.len() <= MAX_VLR_DATA_SIZE || supports_evlr {
            builder.vlrs.push(Vlr {
                user_id: PDAL_USER_ID.to_string(),
                record_id: PDAL_PIPELINE_RECORD_ID,
                description: "PDAL pipeline".to_string(),
                data: json.as_bytes().to_vec(),
            });
        }
    }
}

pub(super) fn add_enhanced_srs_vlrs(
    builder: &mut Builder,
    wkt2: Option<&[u8]>,
    projjson: Option<&[u8]>,
    wkt1: Option<&[u8]>,
) {
    if let Some(data) = wkt2 {
        builder.vlrs.push(Vlr {
            user_id: TRANSFORM_USER_ID.to_string(),
            record_id: WKT2_RECORD_ID,
            description: "PDAL WKT2 Record".to_string(),
            data: null_terminated(data),
        });
        builder.has_wkt_crs = true;
    }
    if let Some(data) = projjson {
        builder.vlrs.push(Vlr {
            user_id: PDAL_USER_ID.to_string(),
            record_id: PROJJSON_RECORD_ID,
            description: "PDAL PROJJSON Record".to_string(),
            data: null_terminated(data),
        });
    }
    if let Some(data) = wkt1 {
        builder.vlrs.push(Vlr {
            user_id: TRANSFORM_USER_ID.to_string(),
            record_id: WKT_RECORD_ID,
            description: "OGC Transformation Record".to_string(),
            data: null_terminated(data),
        });
        builder.vlrs.push(Vlr {
            user_id: LIBLAS_USER_ID.to_string(),
            record_id: WKT_RECORD_ID,
            description: "OGR variant of OpenGIS WKT SRS".to_string(),
            data: null_terminated(data),
        });
        builder.has_wkt_crs = true;
    }
}

pub(super) fn null_terminated(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    if !out.ends_with(&[0]) {
        out.push(0);
    }
    out
}

pub(super) fn add_user_vlrs(
    builder: &mut Builder,
    user_vlrs: &[UserVlr],
) -> Result<(), StageError> {
    let minor = builder.version.minor;
    for user_vlr in user_vlrs {
        let vlr = Vlr {
            user_id: user_vlr.user_id.clone(),
            record_id: user_vlr.record_id,
            description: user_vlr.description.clone(),
            data: user_vlr.data.clone(),
        };
        if user_vlr.data.len() > MAX_VLR_DATA_SIZE {
            if minor >= 4 {
                builder.evlrs.push(vlr);
            } else {
                return Err(StageError(format!(
                    "Can't write VLR with user ID/record ID = {}/{}.  The data size exceeds the maximum supported.",
                    user_vlr.user_id, user_vlr.record_id
                )));
            }
        } else if user_vlr.write_as_evlr {
            if minor >= 4 {
                builder.evlrs.push(vlr);
            } else {
                return Err(StageError(
                    "User specified writing as EVLR but the file is not a 1.4+ file!".to_string(),
                ));
            }
        } else {
            builder.vlrs.push(vlr);
        }
    }
    Ok(())
}

pub(super) fn add_forward_vlrs(builder: &mut Builder, forward_vlrs: &[ForwardedVlr]) {
    for vlr in forward_vlrs {
        builder.vlrs.push(Vlr {
            user_id: vlr.user_id.clone(),
            record_id: vlr.record_id,
            description: vlr.description.clone(),
            data: vlr.data.clone(),
        });
    }
}

pub(super) fn add_extra_bytes_vlr(
    builder: &mut Builder,
    extra_dims: &[ExtraDim],
) -> Result<(), StageError> {
    if extra_dims.is_empty() {
        return Ok(());
    }

    let mut vlr_data = Vec::new();
    for ed in extra_dims {
        write_extra_dim_vlr_record(&mut vlr_data, ed)?;
    }
    builder.vlrs.push(Vlr {
        user_id: "LASF_Spec".to_string(),
        record_id: 4,
        description: "Extra Bytes Record".to_string(),
        data: vlr_data,
    });
    builder.point_format.extra_bytes = extra_dims.iter().map(|ed| ed.size).sum::<usize>() as u16;
    Ok(())
}

pub(super) fn write_extra_dim_vlr_record(
    output: &mut Vec<u8>,
    ed: &ExtraDim,
) -> Result<(), StageError> {
    output
        .write_u16::<LittleEndian>(0)
        .map_err(|e| StageError(e.to_string()))?;
    output
        .write_u8(pdal_to_las_type(ed.ty))
        .map_err(|e| StageError(e.to_string()))?;
    output.write_u8(0).map_err(|e| StageError(e.to_string()))?;
    let mut name_buf = [0u8; 32];
    let bytes = ed.id.name().as_bytes();
    let len = bytes.len().min(32);
    name_buf[..len].copy_from_slice(&bytes[..len]);
    output.extend_from_slice(&name_buf);
    output
        .write_u32::<LittleEndian>(0)
        .map_err(|e| StageError(e.to_string()))?;
    output.extend_from_slice(&[0u8; 72]);
    for _ in 0..6 {
        output
            .write_f64::<LittleEndian>(0.0)
            .map_err(|e| StageError(e.to_string()))?;
    }
    output.extend_from_slice(&[0u8; 32]);
    Ok(())
}

pub(super) fn add_srs_vlr(builder: &mut Builder, views: &[PointView], a_srs: Option<&str>) {
    let srs_text = a_srs
        .filter(|srs| !srs.is_empty())
        .map(str::to_string)
        .or_else(|| {
            views.first().and_then(|view| {
                let srs = view.spatial_reference();
                (!srs.is_empty()).then(|| srs.wkt().to_string())
            })
        });

    let Some(wkt) = srs_text else {
        return;
    };

    if builder.version < las::Version::new(1, 4) {
        return;
    }
    let wkt = pdal_native::srs::user_input_to_wkt(&wkt)
        .map(|srs| {
            pdal_native::srs::wkt_to_wkt1(&srs.wkt2, srs.epoch)
                .ok()
                .filter(|wkt1| !wkt1.is_empty())
                .unwrap_or(srs.wkt)
        })
        .unwrap_or(wkt);
    builder.vlrs.retain(|vlr| !vlr.is_crs());
    builder.evlrs.retain(|vlr| !vlr.is_crs());
    let mut wkt_bytes = wkt.into_bytes();
    wkt_bytes.push(0);
    builder.vlrs.push(Vlr {
        user_id: TRANSFORM_USER_ID.to_string(),
        record_id: WKT_RECORD_ID,
        description: "OGC Transformation Record".to_string(),
        data: wkt_bytes.clone(),
    });
    builder.vlrs.push(Vlr {
        user_id: LIBLAS_USER_ID.to_string(),
        record_id: WKT_RECORD_ID,
        description: "OGR variant of OpenGIS WKT SRS".to_string(),
        data: wkt_bytes,
    });
    builder.has_wkt_crs = true;
}

pub(super) fn quantize_coord(value: f64, transform: &Transform) -> f64 {
    let scaled = pdal_scaled_i32(value, transform);
    let mut world = transform.direct(scaled);
    // The las crate adapts header bounds with Ceil/Floor inverses. At the i32
    // extrema, floating-point error in `direct(scaled)` can exceed INT_MAX on
    // the Ceil path even when PDAL's sround encoding is valid.
    if scaled == i32::MAX {
        while las_inverse_ceil(world, transform) > f64::from(i32::MAX) {
            world = world.next_down();
        }
    } else if scaled == i32::MIN {
        while las_inverse_floor(world, transform) < f64::from(i32::MIN) {
            world = world.next_up();
        }
    }
    world
}

pub(super) fn las_inverse_ceil(value: f64, transform: &Transform) -> f64 {
    ((value - transform.offset) / transform.scale).ceil()
}

pub(super) fn las_inverse_floor(value: f64, transform: &Transform) -> f64 {
    ((value - transform.offset) / transform.scale).floor()
}

pub(super) fn max_return_count(minor_version: u8) -> u8 {
    if minor_version >= 4 {
        LAS14_MAX_RETURN_COUNT
    } else {
        LEGACY_MAX_RETURN_COUNT
    }
}

pub(super) fn write_las_points(
    writer: &mut las::Writer<BufWriter<File>>,
    views: &[PointView],
    extra_dims: &[ExtraDim],
    point_format: u8,
    discard_high_return_numbers: bool,
    minor_version: u8,
) -> Result<(), StageError> {
    let has_gps_time = writer.header().point_format().has_gps_time;
    let has_color = writer.header().point_format().has_color;
    let transforms = *writer.header().transforms();
    let max_returns = max_return_count(minor_version);
    for view in views {
        for i in 0..view.len() {
            if discard_high_return_numbers {
                let return_number = dim_u8(view, i, &DimId::ReturnNumber, 1);
                let number_of_returns = dim_u8(view, i, &DimId::NumberOfReturns, 1);
                if number_of_returns > max_returns && return_number > max_returns {
                    continue;
                }
            }
            let point = point_from_view(
                view,
                i,
                extra_dims,
                has_gps_time,
                has_color,
                point_format,
                &transforms,
                discard_high_return_numbers,
                max_returns,
            )?;
            writer
                .write_point(point)
                .map_err(|e| StageError(format!("Failed to write LAS point: {}", e)))?;
        }
    }
    Ok(())
}

pub(super) fn quantize_scan_angle(point_format: u8, degrees: f64) -> f32 {
    if point_format >= 6 {
        let target = (degrees / SCAN_ANGLE_SCALE_FACTOR).round() as i16;
        scan_angle_f32_for_i16(target)
    } else {
        degrees.round() as f32
    }
}

pub(super) fn scan_angle_f32_for_i16(target: i16) -> f32 {
    let grid = f64::from(target) * SCAN_ANGLE_SCALE_FACTOR;
    if target < 0 {
        (grid - 1e-4) as f32
    } else if target > 0 {
        (grid + 1e-4) as f32
    } else {
        0.0
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn point_from_view(
    view: &PointView,
    i: u64,
    extra_dims: &[ExtraDim],
    has_gps_time: bool,
    has_color: bool,
    point_format: u8,
    transforms: &las::Vector<Transform>,
    discard_high_return_numbers: bool,
    max_returns: u8,
) -> Result<Point, StageError> {
    let return_number = dim_u8(view, i, &DimId::ReturnNumber, 1);
    let mut number_of_returns = dim_u8(view, i, &DimId::NumberOfReturns, 1);
    if discard_high_return_numbers && number_of_returns > max_returns {
        number_of_returns = max_returns;
    }

    let mut point = Point {
        x: quantize_coord(view.get_f64(i, &DimId::X), &transforms.x),
        y: quantize_coord(view.get_f64(i, &DimId::Y), &transforms.y),
        z: quantize_coord(view.get_f64(i, &DimId::Z), &transforms.z),
        intensity: dim_u16(view, i, &DimId::Intensity),
        return_number,
        number_of_returns,
        scan_direction: scan_direction(view, i),
        is_edge_of_flight_line: dim_flag(view, i, &DimId::EdgeOfFlightLine),
        classification: classification(dim_u8(view, i, &DimId::Classification, 0)),
        scan_angle: quantize_scan_angle(point_format, view.get_f64(i, &DimId::ScanAngleRank)),
        user_data: dim_u8(view, i, &DimId::UserData, 0),
        point_source_id: dim_u16(view, i, &DimId::PointSourceId),
        is_synthetic: dim_flag(view, i, &DimId::Synthetic),
        is_key_point: dim_flag(view, i, &DimId::KeyPoint),
        is_withheld: dim_flag(view, i, &DimId::Withheld),
        is_overlap: dim_flag(view, i, &DimId::Overlap),
        ..Default::default()
    };
    let scan_channel = DimId::from_name("ScanChannel");
    if view.layout().dim(&scan_channel).is_some() {
        point.scanner_channel = dim_u8(view, i, &scan_channel, 0);
    }
    add_optional_point_dims(&mut point, view, i, has_gps_time, has_color, view.layout());
    point.extra_bytes = extra_bytes_from_view(view, i, extra_dims)?;
    Ok(point)
}

pub(super) fn dim_flag(view: &PointView, i: u64, dim: &DimId) -> bool {
    view.layout().dim(dim).is_some() && view.get_f64(i, dim) > 0.0
}

pub(super) fn dim_u8(view: &PointView, i: u64, dim: &DimId, default: u8) -> u8 {
    if view.layout().dim(dim).is_some() {
        view.get_f64(i, dim) as u8
    } else {
        default
    }
}

pub(super) fn dim_u16(view: &PointView, i: u64, dim: &DimId) -> u16 {
    if view.layout().dim(dim).is_some() {
        view.get_f64(i, dim) as u16
    } else {
        0
    }
}

pub(super) fn scan_direction(view: &PointView, i: u64) -> ScanDirection {
    if view.get_f64(i, &DimId::ScanDirectionFlag) > 0.0 {
        ScanDirection::LeftToRight
    } else {
        ScanDirection::RightToLeft
    }
}

pub(super) fn classification(value: u8) -> Classification {
    match value {
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
    }
}

pub(super) fn add_optional_point_dims(
    point: &mut Point,
    view: &PointView,
    i: u64,
    has_gps_time: bool,
    has_color: bool,
    layout: &pdal_core::point::PointLayout,
) {
    if layout.dim(&DimId::GpsTime).is_some() {
        point.gps_time = Some(view.get_f64(i, &DimId::GpsTime));
    } else if has_gps_time {
        point.gps_time = Some(0.0);
    }
    if layout.dim(&DimId::Red).is_some() {
        point.color = Some(las::Color {
            red: view.get_f64(i, &DimId::Red) as u16,
            green: view.get_f64(i, &DimId::Green) as u16,
            blue: view.get_f64(i, &DimId::Blue) as u16,
        });
    } else if has_color {
        point.color = Some(las::Color {
            red: 0,
            green: 0,
            blue: 0,
        });
    }
    if layout.dim(&DimId::Infrared).is_some() {
        point.nir = Some(view.get_f64(i, &DimId::Infrared) as u16);
    }
}

pub(super) fn extra_bytes_from_view(
    view: &PointView,
    i: u64,
    extra_dims: &[ExtraDim],
) -> Result<Vec<u8>, StageError> {
    let mut bytes = Vec::new();
    for ed in extra_dims {
        write_pdal_val(&mut bytes, view.get_f64(i, &ed.id), ed.ty)
            .map_err(|e| StageError(e.to_string()))?;
    }
    Ok(bytes)
}

pub(super) fn pdrf_dims(pdrf: u8) -> Vec<DimId> {
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
        DimId::Synthetic,
        DimId::KeyPoint,
        DimId::Withheld,
        DimId::Overlap,
        DimId::ScanAngleRank,
        DimId::UserData,
        DimId::PointSourceId,
    ];
    if pdrf >= 6 {
        dims.push(DimId::from_name("ScanChannel"));
    }
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

pub(super) fn numeric_option_f64(options: &Options, key: &str) -> Option<f64> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<f64>().ok())
}

pub(super) fn numeric_option_u8(options: &Options, key: &str) -> Option<u8> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<u8>().ok())
}

pub(super) fn numeric_option_u16(options: &Options, key: &str) -> Option<u16> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<u16>().ok())
}

pub(super) fn numeric_option_u32(options: &Options, key: &str) -> Option<u32> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<u32>().ok())
}

pub(super) fn numeric_option_i32(options: &Options, key: &str) -> Option<i32> {
    options
        .value(key)
        .and_then(|value| value.trim().parse::<i32>().ok())
}

pub(super) fn string_option(options: &Options, key: &str) -> Option<String> {
    options.value(key).map(ToString::to_string)
}

pub(super) fn binary_option(options: &Options, key: &str) -> Option<Vec<u8>> {
    options
        .value(key)
        .map(|value| base64_decode(value.trim()))
        .filter(|value| !value.is_empty())
}

pub(super) fn pdal_to_las_type(ty: DimType) -> u8 {
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

pub(super) fn write_pdal_val(
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

pub(super) fn pdal_sround(value: f64) -> f64 {
    if value > 0.0 {
        (value + 0.5).floor()
    } else {
        (value - 0.5).ceil()
    }
}

pub(super) fn pdal_scaled_i32(value: f64, transform: &Transform) -> i32 {
    let scaled = (value - transform.offset) / transform.scale;
    pdal_sround(scaled) as i32
}

pub(super) fn pdal_from_scaled(value: i32, transform: &Transform) -> f64 {
    (f64::from(value) * transform.scale) + transform.offset
}
