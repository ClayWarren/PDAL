//! `writers.las` and `writers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasWriter.cpp` using the `las` Rust crate.

use byteorder::{LittleEndian, WriteBytesExt};
use chrono::NaiveDate;
use las::point::{Classification, Format, ScanDirection};
use las::{Builder, GpsTimeType, Header, Point, Transform, Vlr};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::StageError;
use pdal_core::utils::base64_decode;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom};
use std::path::Path;

const HEADER_MAX_X_OFFSET: u64 = 179;
const LEGACY_POINT_COUNT_OFFSET: u64 = 107;
const LEGACY_POINTS_BY_RETURN_OFFSET: u64 = 111;
const SCAN_ANGLE_SCALE_FACTOR: f64 = 0.006;
const PDAL_USER_ID: &str = "PDAL";
const PDAL_METADATA_RECORD_ID: u16 = 12;
const PDAL_PIPELINE_RECORD_ID: u16 = 13;
const MAX_VLR_DATA_SIZE: usize = u16::MAX as usize;
const TRANSFORM_USER_ID: &str = "LASF_Projection";
const LIBLAS_USER_ID: &str = "liblas";
const WKT_RECORD_ID: u16 = 2112;
const WKT2_RECORD_ID: u16 = 4224;
const PROJJSON_RECORD_ID: u16 = 4225;

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
    global_encoding: Option<u16>,
    a_srs: Option<String>,
    pdal_metadata_json: Option<String>,
    pdal_pipeline_json: Option<String>,
    enhanced_srs_vlrs: bool,
    srs_wkt2_vlr: Option<Vec<u8>>,
    srs_projjson_vlr: Option<Vec<u8>>,
    srs_wkt1_vlr: Option<Vec<u8>>,
    user_vlrs: Vec<UserVlr>,
    configured_extra_dims: Vec<ConfiguredExtraDim>,
    forward_vlrs: Vec<ForwardedVlr>,
}

struct ConfiguredExtraDim {
    name: String,
    type_name: String,
}

struct UserVlr {
    user_id: String,
    record_id: u16,
    description: String,
    data: Vec<u8>,
    write_as_evlr: bool,
}

struct ForwardedVlr {
    user_id: String,
    record_id: u16,
    description: String,
    data: Vec<u8>,
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
            global_encoding: numeric_option_u16(options, "global_encoding"),
            a_srs: string_option(options, "a_srs"),
            pdal_metadata_json: string_option(options, "pdal_metadata_json"),
            pdal_pipeline_json: string_option(options, "pdal_pipeline_json"),
            enhanced_srs_vlrs: options.get_bool("enhanced_srs_vlrs", false),
            srs_wkt2_vlr: binary_option(options, "srs_wkt2_vlr"),
            srs_projjson_vlr: binary_option(options, "srs_projjson_vlr"),
            srs_wkt1_vlr: binary_option(options, "srs_wkt1_vlr"),
            user_vlrs: user_vlrs_from_options(options),
            configured_extra_dims: configured_extra_dims_from_options(options),
            forward_vlrs: forward_vlrs_from_options(options),
        }
    }

    fn initial_builder(&self, views: &[PointView]) -> Result<Builder, StageError> {
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
        if let Some(global_encoding) = self.global_encoding {
            builder.gps_time_type = GpsTimeType::from(global_encoding & 1);
            builder.has_synthetic_return_numbers = global_encoding & 8 != 0;
            builder.has_wkt_crs = global_encoding & 16 != 0;
        }
        let bounds = min_xyz(views);
        if self.scale_x.is_some()
            || self.scale_y.is_some()
            || self.scale_z.is_some()
            || self.offset_x.is_some()
            || self.offset_y.is_some()
            || self.offset_z.is_some()
            || bounds.is_some()
        {
            builder.transforms = las::Vector {
                x: las::Transform {
                    scale: self.scale_x.unwrap_or(0.01),
                    offset: self.offset_x.unwrap_or(0.0),
                },
                y: las::Transform {
                    scale: self.scale_y.unwrap_or(0.01),
                    offset: self.offset_y.unwrap_or(0.0),
                },
                z: las::Transform {
                    scale: self.scale_z.unwrap_or(0.01),
                    offset: self.offset_z.unwrap_or(0.0),
                },
            };
        }
        if self.minor_version.is_none()
            && (self.a_srs.as_ref().is_some_and(|srs| !srs.is_empty())
                || bounds.is_some_and(|_| {
                    views
                        .first()
                        .is_some_and(|view| !view.spatial_reference().is_empty())
                }))
        {
            builder.version = las::Version { major: 1, minor: 4 };
        }
        Ok(builder)
    }

    fn should_compress(&self, path: &Path) -> bool {
        let extension_requests_laz = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("laz"));
        self.compression || extension_requests_laz
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

        let path = Path::new(&self.filename);
        let should_compress = self.should_compress(path);
        let mut builder = self.initial_builder(views)?;
        add_pdal_vlrs(
            &mut builder,
            self.pdal_metadata_json.as_deref(),
            self.pdal_pipeline_json.as_deref(),
        );
        let extra_dims = resolve_extra_dims(views, self.point_format, &self.configured_extra_dims)?;
        add_extra_bytes_vlr(&mut builder, &extra_dims)?;
        add_user_vlrs(&mut builder, &self.user_vlrs)?;
        add_forward_vlrs(&mut builder, &self.forward_vlrs);
        if self.enhanced_srs_vlrs {
            add_enhanced_srs_vlrs(
                &mut builder,
                self.srs_wkt2_vlr.as_deref(),
                self.srs_projjson_vlr.as_deref(),
                self.srs_wkt1_vlr.as_deref(),
            );
        }
        builder.point_format.is_compressed = should_compress;

        let mut header = builder
            .into_header()
            .map_err(|e| StageError(format!("Failed to create LAS header: {}", e)))?;
        if !self.enhanced_srs_vlrs {
            set_header_srs(&mut header, views, self.a_srs.as_deref());
        }

        let file = File::create(path)
            .map(BufWriter::new)
            .map_err(|e| StageError(format!("Failed to create LAS/LAZ file: {}", e)))?;
        let mut writer = las::Writer::new(file, header)
            .map_err(|e| StageError(format!("Failed to create LAS/LAZ writer: {}", e)))?;

        write_las_points(&mut writer, views, &extra_dims, self.point_format)?;

        let transforms = *writer.header().transforms();
        writer
            .close()
            .map_err(|e| StageError(format!("Failed to close LAS writer: {}", e)))?;
        if !views.is_empty() {
            patch_pdal_header_bounds(path, &transforms, views)?;
            patch_pdal_legacy_header_counts(
                path,
                self.point_format,
                self.minor_version.unwrap_or(2),
            )?;
        }

        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.las")
    }
}

fn min_xyz(views: &[PointView]) -> Option<[f64; 3]> {
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

fn configured_extra_dims_from_options(options: &Options) -> Vec<ConfiguredExtraDim> {
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

fn resolve_extra_dims(
    views: &[PointView],
    point_format: u8,
    configured_extra_dims: &[ConfiguredExtraDim],
) -> Result<Vec<ExtraDim>, StageError> {
    if configured_extra_dims.is_empty() {
        return Ok(extra_dims_from_views(views, point_format));
    }

    let view = views.first().ok_or_else(|| {
        StageError("LasWriter requires at least one point view for extra_dims.".to_string())
    })?;

    configured_extra_dims
        .iter()
        .map(|spec| {
            let id = DimId::from_name(&spec.name);
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

fn dim_type_from_interpretation(name: &str) -> Option<DimType> {
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

fn extra_dims_from_views(views: &[PointView], point_format: u8) -> Vec<ExtraDim> {
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

fn user_vlrs_from_options(options: &Options) -> Vec<UserVlr> {
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

fn forward_vlrs_from_options(options: &Options) -> Vec<ForwardedVlr> {
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

fn add_pdal_vlrs(builder: &mut Builder, metadata_json: Option<&str>, pipeline_json: Option<&str>) {
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

fn add_enhanced_srs_vlrs(
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
            data: data.to_vec(),
        });
        builder.has_wkt_crs = true;
    }
    if let Some(data) = projjson {
        builder.vlrs.push(Vlr {
            user_id: PDAL_USER_ID.to_string(),
            record_id: PROJJSON_RECORD_ID,
            description: "PDAL PROJJSON Record".to_string(),
            data: data.to_vec(),
        });
    }
    if let Some(data) = wkt1 {
        builder.vlrs.push(Vlr {
            user_id: TRANSFORM_USER_ID.to_string(),
            record_id: WKT_RECORD_ID,
            description: "OGC Transformation Record".to_string(),
            data: data.to_vec(),
        });
        builder.vlrs.push(Vlr {
            user_id: LIBLAS_USER_ID.to_string(),
            record_id: WKT_RECORD_ID,
            description: "OGR variant of OpenGIS WKT SRS".to_string(),
            data: data.to_vec(),
        });
        builder.has_wkt_crs = true;
    }
}

fn add_user_vlrs(builder: &mut Builder, user_vlrs: &[UserVlr]) -> Result<(), StageError> {
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

fn add_forward_vlrs(builder: &mut Builder, forward_vlrs: &[ForwardedVlr]) {
    for vlr in forward_vlrs {
        builder.vlrs.push(Vlr {
            user_id: vlr.user_id.clone(),
            record_id: vlr.record_id,
            description: vlr.description.clone(),
            data: vlr.data.clone(),
        });
    }
}

fn add_extra_bytes_vlr(builder: &mut Builder, extra_dims: &[ExtraDim]) -> Result<(), StageError> {
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

fn write_extra_dim_vlr_record(output: &mut Vec<u8>, ed: &ExtraDim) -> Result<(), StageError> {
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

fn set_header_srs(header: &mut Header, views: &[PointView], a_srs: Option<&str>) {
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

    if header.version() < las::Version::new(1, 4) {
        return;
    }
    header.set_wkt_crs(wkt.into_bytes()).unwrap_or(());
}

fn quantize_coord(value: f64, transform: &Transform) -> f64 {
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

fn las_inverse_ceil(value: f64, transform: &Transform) -> f64 {
    ((value - transform.offset) / transform.scale).ceil()
}

fn las_inverse_floor(value: f64, transform: &Transform) -> f64 {
    ((value - transform.offset) / transform.scale).floor()
}

fn write_las_points(
    writer: &mut las::Writer<BufWriter<File>>,
    views: &[PointView],
    extra_dims: &[ExtraDim],
    point_format: u8,
) -> Result<(), StageError> {
    let has_gps_time = writer.header().point_format().has_gps_time;
    let has_color = writer.header().point_format().has_color;
    let transforms = *writer.header().transforms();
    for view in views {
        for i in 0..view.len() {
            let point = point_from_view(
                view,
                i,
                extra_dims,
                has_gps_time,
                has_color,
                point_format,
                &transforms,
            )?;
            writer
                .write_point(point)
                .map_err(|e| StageError(format!("Failed to write LAS point: {}", e)))?;
        }
    }
    Ok(())
}

fn quantize_scan_angle(point_format: u8, degrees: f64) -> f32 {
    if point_format >= 6 {
        let target = (degrees / SCAN_ANGLE_SCALE_FACTOR).round() as i16;
        scan_angle_f32_for_i16(target)
    } else {
        degrees.round() as f32
    }
}

fn scan_angle_f32_for_i16(target: i16) -> f32 {
    let grid = f64::from(target) * SCAN_ANGLE_SCALE_FACTOR;
    if target < 0 {
        (grid - 1e-4) as f32
    } else if target > 0 {
        (grid + 1e-4) as f32
    } else {
        0.0
    }
}

fn point_from_view(
    view: &PointView,
    i: u64,
    extra_dims: &[ExtraDim],
    has_gps_time: bool,
    has_color: bool,
    point_format: u8,
    transforms: &las::Vector<Transform>,
) -> Result<Point, StageError> {
    let mut point = Point {
        x: quantize_coord(view.get_f64(i, &DimId::X), &transforms.x),
        y: quantize_coord(view.get_f64(i, &DimId::Y), &transforms.y),
        z: quantize_coord(view.get_f64(i, &DimId::Z), &transforms.z),
        intensity: dim_u16(view, i, &DimId::Intensity),
        return_number: dim_u8(view, i, &DimId::ReturnNumber, 1),
        number_of_returns: dim_u8(view, i, &DimId::NumberOfReturns, 1),
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

fn dim_flag(view: &PointView, i: u64, dim: &DimId) -> bool {
    view.layout().dim(dim).is_some() && view.get_f64(i, dim) > 0.0
}

fn dim_u8(view: &PointView, i: u64, dim: &DimId, default: u8) -> u8 {
    if view.layout().dim(dim).is_some() {
        view.get_f64(i, dim) as u8
    } else {
        default
    }
}

fn dim_u16(view: &PointView, i: u64, dim: &DimId) -> u16 {
    if view.layout().dim(dim).is_some() {
        view.get_f64(i, dim) as u16
    } else {
        0
    }
}

fn scan_direction(view: &PointView, i: u64) -> ScanDirection {
    if view.get_f64(i, &DimId::ScanDirectionFlag) > 0.0 {
        ScanDirection::LeftToRight
    } else {
        ScanDirection::RightToLeft
    }
}

fn classification(value: u8) -> Classification {
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

fn add_optional_point_dims(
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

fn extra_bytes_from_view(
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

fn binary_option(options: &Options, key: &str) -> Option<Vec<u8>> {
    options
        .value(key)
        .map(|value| base64_decode(value.trim()))
        .filter(|value| !value.is_empty())
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

fn pdal_sround(value: f64) -> f64 {
    if value > 0.0 {
        (value + 0.5).floor()
    } else {
        (value - 0.5).ceil()
    }
}

fn pdal_scaled_i32(value: f64, transform: &Transform) -> i32 {
    let scaled = (value - transform.offset) / transform.scale;
    pdal_sround(scaled) as i32
}

fn pdal_from_scaled(value: i32, transform: &Transform) -> f64 {
    (f64::from(value) * transform.scale) + transform.offset
}

fn pdal_header_bounds(views: &[PointView], transforms: &las::Vector<Transform>) -> las::Bounds {
    let mut min = [i32::MAX; 3];
    let mut max = [i32::MIN; 3];

    for view in views {
        for i in 0..view.len() {
            let coords = [
                (view.get_f64(i, &DimId::X), &transforms.x),
                (view.get_f64(i, &DimId::Y), &transforms.y),
                (view.get_f64(i, &DimId::Z), &transforms.z),
            ];
            for (axis, (coord, transform)) in coords.into_iter().enumerate() {
                let scaled = pdal_scaled_i32(coord, transform);
                min[axis] = min[axis].min(scaled);
                max[axis] = max[axis].max(scaled);
            }
        }
    }

    las::Bounds {
        min: las::Vector {
            x: pdal_from_scaled(min[0], &transforms.x),
            y: pdal_from_scaled(min[1], &transforms.y),
            z: pdal_from_scaled(min[2], &transforms.z),
        },
        max: las::Vector {
            x: pdal_from_scaled(max[0], &transforms.x),
            y: pdal_from_scaled(max[1], &transforms.y),
            z: pdal_from_scaled(max[2], &transforms.z),
        },
    }
}

fn patch_pdal_legacy_header_counts(
    path: &Path,
    point_format: u8,
    minor_version: u8,
) -> Result<(), StageError> {
    if point_format < 6 || minor_version < 4 {
        return Ok(());
    }

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| StageError(format!("Failed to reopen LAS/LAZ file: {}", e)))?;
    file.seek(SeekFrom::Start(LEGACY_POINT_COUNT_OFFSET))
        .map_err(|e| StageError(e.to_string()))?;
    file.write_u32::<LittleEndian>(0)
        .map_err(|e| StageError(e.to_string()))?;
    file.seek(SeekFrom::Start(LEGACY_POINTS_BY_RETURN_OFFSET))
        .map_err(|e| StageError(e.to_string()))?;
    for _ in 0..5 {
        file.write_u32::<LittleEndian>(0)
            .map_err(|e| StageError(e.to_string()))?;
    }
    Ok(())
}

fn patch_pdal_header_bounds(
    path: &Path,
    transforms: &las::Vector<Transform>,
    views: &[PointView],
) -> Result<(), StageError> {
    let bounds = pdal_header_bounds(views, transforms);
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| StageError(format!("Failed to reopen LAS/LAZ file: {}", e)))?;
    file.seek(SeekFrom::Start(HEADER_MAX_X_OFFSET))
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.max.x)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.min.x)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.max.y)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.min.y)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.max.z)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.min.z)
        .map_err(|e| StageError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::las::LasReader;
    use pdal_core::pipeline::Reader;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn pdal_header_bounds_match_scaled_roundtrip() {
        let transforms = las::Vector {
            x: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
            y: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
            z: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
        };
        let view = header_bbox_view();
        let bounds = pdal_header_bounds(&[view], &transforms);
        assert!((bounds.min.x - -136.8310).abs() < 1e-4);
        assert!((bounds.max.x - 194.1731).abs() < 1e-4);
        assert!((bounds.min.y - -165.4601).abs() < 1e-4);
        assert!((bounds.max.y - 165.5438).abs() < 1e-4);
        assert!((bounds.min.z - -20.4150).abs() < 1e-4);
        assert!((bounds.max.z - 310.5888).abs() < 1e-4);
    }

    #[test]
    fn writer_honors_global_encoding_option() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-global-encoding-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        options.add("minor_version", "3");
        options.add("dataformat_id", "3");
        options.add("global_encoding", "0");
        LasWriter::new(&options)
            .write(&[synthetic_point_view()])
            .unwrap();

        let reader = las::Reader::from_path(&temp).unwrap();
        let header = reader.header();
        assert_eq!(header.version(), las::Version::new(1, 3));
        assert!(!header.has_wkt_crs());

        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn writer_preserves_synthetic_flag_for_las10() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-synthetic-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        let mut writer = LasWriter::new(&options);
        writer.write(&[synthetic_point_view()]).unwrap();

        let mut reader = las::Reader::from_path(&temp).unwrap();
        let point = reader.points().next().unwrap().unwrap();
        assert!(point.is_synthetic);
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn quantize_scan_angle_matches_pdal_roundtrip() {
        let degrees = -16.998001098632812_f64;
        let target = (degrees / SCAN_ANGLE_SCALE_FACTOR).round() as i16;
        let quantized = quantize_scan_angle(7, degrees);
        let encoded = (f64::from(quantized) / SCAN_ANGLE_SCALE_FACTOR) as i16;
        assert_eq!(encoded, target);
        assert_eq!(target, -2833);
    }

    #[test]
    fn format7_laz_roundtrip_preserves_first_point_fields() {
        let source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/autzen_trim_7.las"
        );
        let output = std::env::temp_dir().join(format!(
            "pdal-format7-laz-{}-{}.laz",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let mut read_options = Options::new();
        read_options.add("filename", source);
        let input = LasReader::new(&read_options)
            .read()
            .expect("read source las");

        let mut write_options = Options::new();
        write_options.add("filename", output.display().to_string());
        write_options.add("dataformat_id", "7");
        write_options.add("minor_version", "4");
        write_options.add("compression", "true");
        LasWriter::new_laz(&write_options)
            .write(&input)
            .expect("write format 7 laz");

        let mut roundtrip_options = Options::new();
        roundtrip_options.add("filename", output.display().to_string());
        let output_views = LasReader::new(&roundtrip_options)
            .read()
            .expect("read written laz");

        let source_view = &input[0];
        let written_view = &output_views[0];
        assert_eq!(source_view.len(), written_view.len());

        let scan_channel = DimId::from_name("ScanChannel");
        for idx in 0..source_view.len().min(10) {
            for dim in [
                DimId::X,
                DimId::Y,
                DimId::Z,
                DimId::Intensity,
                DimId::ReturnNumber,
                DimId::NumberOfReturns,
                DimId::Classification,
                DimId::ScanAngleRank,
                DimId::GpsTime,
                DimId::Red,
                DimId::Green,
                DimId::Blue,
                scan_channel.clone(),
            ] {
                let left = source_view.get_f64(idx, &dim);
                let right = written_view.get_f64(idx, &dim);
                assert!(
                    (left - right).abs() <= 1e-9,
                    "point {idx} dim {:?}: {left} vs {right}",
                    dim
                );
            }
        }

        let _ = std::fs::remove_file(output);
    }

    fn header_bbox_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let coords = [
            (-136.8309503964847, -165.4601240504369, -20.415032985882097),
            (194.17314124182556, 165.54376758787334, 310.58878865242816),
        ];
        for (x, y, z) in coords {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, z);
        }
        view
    }

    fn synthetic_point_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        layout.register(DimId::Synthetic, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let id = view.add_point();
        view.set_f64(id, &DimId::X, 1.0);
        view.set_f64(id, &DimId::Y, 2.0);
        view.set_f64(id, &DimId::Z, 3.0);
        view.set_f64(id, &DimId::Classification, 2.0);
        view.set_f64(id, &DimId::Synthetic, 1.0);
        view
    }
}
