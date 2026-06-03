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
const GEOTIFF_DIRECTORY_RECORD_ID: u16 = 34735;
const GEOTIFF_DOUBLES_RECORD_ID: u16 = 34736;
const GEOTIFF_ASCII_RECORD_ID: u16 = 34737;
const LEGACY_MAX_RETURN_COUNT: u8 = 5;
const LAS14_MAX_RETURN_COUNT: u8 = 15;

#[derive(Clone)]
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
    forward: String,
    a_srs: Option<String>,
    pdal_metadata_json: Option<String>,
    pdal_pipeline_json: Option<String>,
    enhanced_srs_vlrs: bool,
    srs_wkt2_vlr: Option<Vec<u8>>,
    srs_projjson_vlr: Option<Vec<u8>>,
    srs_wkt1_vlr: Option<Vec<u8>>,
    user_vlrs: Vec<UserVlr>,
    configured_extra_dims: Vec<ConfiguredExtraDim>,
    discard_high_return_numbers: bool,
    forward_vlrs: Vec<ForwardedVlr>,
    // writers.copc rejects extra_dims that name a standard point-format
    // dimension ("is a standard dimension"); writers.las allows them (it may
    // write a standard dimension as an additional extra-bytes field). Mirrors
    // the C++ CopcWriter vs LasWriter difference.
    reject_standard_extra_dims: bool,
    /// Streaming write state (open writer + accumulated bounds). Clones to
    /// `None` so the `#[derive(Clone)]` for the `#`-templated multi-file path
    /// stays trivial.
    stream: StreamSlot,
}

/// Holds the in-progress streaming writer. Clones to empty so `LasWriter` can
/// keep deriving `Clone` (a clone is a fresh writer, not a shared file handle).
#[derive(Default)]
struct StreamSlot(Option<LasWriterStreamState>);

impl Clone for StreamSlot {
    fn clone(&self) -> Self {
        StreamSlot(None)
    }
}

/// Open LAS writer plus the running header bounds accumulated across chunks.
struct LasWriterStreamState {
    writer: las::Writer<BufWriter<File>>,
    extra_dims: Vec<ExtraDim>,
    transforms: las::Vector<Transform>,
    bounds: Option<las::Bounds>,
}

#[derive(Clone)]
struct ConfiguredExtraDim {
    name: String,
    type_name: String,
}

#[derive(Clone)]
struct UserVlr {
    user_id: String,
    record_id: u16,
    description: String,
    data: Vec<u8>,
    write_as_evlr: bool,
}

#[derive(Clone)]
struct ForwardedVlr {
    user_id: String,
    record_id: u16,
    description: String,
    data: Vec<u8>,
}

pub(crate) struct ExtraDim {
    pub(crate) id: DimId,
    pub(crate) ty: DimType,
    pub(crate) size: usize,
}

impl LasWriter {
    pub fn new(options: &Options) -> Self {
        Self::new_with_compression(options, false)
    }

    pub fn new_laz(options: &Options) -> Self {
        Self::new_with_compression(options, true)
    }

    /// LAS/LAZ writer configured for `writers.copc`: like `new_laz` but rejects
    /// extra_dims that name a standard point-format dimension.
    pub fn new_copc(options: &Options) -> Self {
        let mut writer = Self::new_laz(options);
        writer.reject_standard_extra_dims = true;
        writer
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
            forward: options.get_str("forward", ""),
            a_srs: string_option(options, "a_srs"),
            pdal_metadata_json: string_option(options, "pdal_metadata_json"),
            pdal_pipeline_json: string_option(options, "pdal_pipeline_json"),
            enhanced_srs_vlrs: options.get_bool("enhanced_srs_vlrs", false),
            srs_wkt2_vlr: binary_option(options, "srs_wkt2_vlr"),
            srs_projjson_vlr: binary_option(options, "srs_projjson_vlr"),
            srs_wkt1_vlr: binary_option(options, "srs_wkt1_vlr"),
            user_vlrs: user_vlrs_from_options(options),
            configured_extra_dims: configured_extra_dims_from_options(options),
            discard_high_return_numbers: options.get_bool("discard_high_return_numbers", false),
            forward_vlrs: forward_vlrs_from_options(options),
            reject_standard_extra_dims: false,
            stream: StreamSlot(None),
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
        } else if self.forward.split(',').any(|part| part.trim() == "all")
            && views
                .first()
                .is_some_and(|view| view.layout().dim(&DimId::GpsTime).is_some())
        {
            builder.gps_time_type = GpsTimeType::Standard;
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

    /// Build the LAS header and resolved extra-dim layout from a representative
    /// set of views. The header is data-independent (offset is options/default,
    /// not auto-min), so the streaming path can build it from the first chunk
    /// and patch the min/max bounds at the end. Shared by `write` and the
    /// streaming `stream_write` to keep one source of truth for header bytes.
    fn build_header(&self, views: &[PointView]) -> Result<(Header, Vec<ExtraDim>), StageError> {
        validate_unique_spatial_reference(views, &self.filename)?;
        let path = Path::new(&self.filename);
        let should_compress = self.should_compress(path);
        let mut builder = self.initial_builder(views)?;
        add_pdal_vlrs(
            &mut builder,
            self.pdal_metadata_json.as_deref(),
            self.pdal_pipeline_json.as_deref(),
        );
        let extra_dims = resolve_extra_dims(
            views,
            self.point_format,
            &self.configured_extra_dims,
            self.reject_standard_extra_dims,
        )?;
        add_extra_bytes_vlr(&mut builder, &extra_dims)?;
        add_user_vlrs(&mut builder, &self.user_vlrs)?;
        add_forward_vlrs(&mut builder, &self.forward_vlrs);
        if self.enhanced_srs_vlrs {
            // When the caller supplies the VLR bytes explicitly (the C++
            // `writers.las` wrapper does this, computing each form via
            // `SpatialReference::getWKT1/getWKT2/getPROJJSON` and *omitting*
            // any form the CRS can't be expressed in -- e.g. a
            // DerivedProjectedCRS has no WKT1), honor that decision exactly and
            // don't regenerate the missing forms. Only the pure-Rust path,
            // which passes `a_srs` instead of byte VLRs, regenerates them here.
            let has_explicit_vlrs = self.srs_wkt2_vlr.is_some()
                || self.srs_projjson_vlr.is_some()
                || self.srs_wkt1_vlr.is_some();
            let generated_srs = if has_explicit_vlrs {
                None
            } else {
                generated_srs_vlrs(self, views)
            };
            add_enhanced_srs_vlrs(
                &mut builder,
                self.srs_wkt2_vlr
                    .as_deref()
                    .or(generated_srs.as_ref().map(|srs| srs.wkt2.as_bytes()))
                    .filter(|b| !b.is_empty()),
                self.srs_projjson_vlr
                    .as_deref()
                    .or(generated_srs.as_ref().map(|srs| srs.projjson.as_bytes()))
                    .filter(|b| !b.is_empty()),
                self.srs_wkt1_vlr
                    .as_deref()
                    .or(generated_srs.as_ref().map(|srs| srs.wkt.as_bytes()))
                    .filter(|b| !b.is_empty()),
            );
        } else {
            add_srs_vlr(&mut builder, views, self.a_srs.as_deref())?;
        }
        builder.point_format.is_compressed = should_compress;

        let header = builder
            .into_header()
            .map_err(|e| StageError(format!("Failed to create LAS header: {}", e)))?;
        Ok((header, extra_dims))
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
        if self.filename.contains('#') {
            for (idx, view) in views.iter().enumerate() {
                let mut writer = self.clone();
                writer.filename = numbered_filename(&self.filename, idx + 1);
                writer.write(std::slice::from_ref(view))?;
            }
            return Ok(());
        }

        let path = Path::new(&self.filename);
        let (header, extra_dims) = self.build_header(views)?;

        let file = File::create(path)
            .map(BufWriter::new)
            .map_err(|e| StageError(format!("Failed to create LAS/LAZ file: {}", e)))?;
        let mut writer = las::Writer::new(file, header)
            .map_err(|e| StageError(format!("Failed to create LAS/LAZ writer: {}", e)))?;

        write_las_points(
            &mut writer,
            views,
            &extra_dims,
            self.point_format,
            self.discard_high_return_numbers,
            self.minor_version.unwrap_or(2),
        )?;

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

    fn reset(&mut self) {
        self.stream = StreamSlot(None);
    }

    fn streamable(&self) -> bool {
        // Uncompressed single-file writes only. Compression (laz) and `#`
        // multi-file templating fall back to the materializing `write`.
        !self.filename.is_empty()
            && !self.filename.contains('#')
            && !self.should_compress(Path::new(&self.filename))
    }

    fn stream_write(&mut self, chunk: &PointView) -> Result<(), StageError> {
        // Defer creating the file/header until the first chunk so the header is
        // built from a representative view (matching how `write` derives it
        // from the full set). Empty chunks still carry layout/SRS, which matters
        // when a streamable filter drops every point.
        if self.stream.0.is_none() {
            let (header, extra_dims) = self.build_header(std::slice::from_ref(chunk))?;
            let file = File::create(Path::new(&self.filename))
                .map(BufWriter::new)
                .map_err(|e| StageError(format!("Failed to create LAS/LAZ file: {}", e)))?;
            let writer = las::Writer::new(file, header)
                .map_err(|e| StageError(format!("Failed to create LAS/LAZ writer: {}", e)))?;
            let transforms = *writer.header().transforms();
            self.stream.0 = Some(LasWriterStreamState {
                writer,
                extra_dims,
                transforms,
                bounds: None,
            });
        }

        let point_format = self.point_format;
        let discard = self.discard_high_return_numbers;
        let minor = self.minor_version.unwrap_or(2);
        let state = self.stream.0.as_mut().expect("stream initialized above");
        write_las_points(
            &mut state.writer,
            std::slice::from_ref(chunk),
            &state.extra_dims,
            point_format,
            discard,
            minor,
        )?;
        if !chunk.is_empty() {
            let chunk_bounds = pdal_header_bounds(std::slice::from_ref(chunk), &state.transforms);
            state.bounds = Some(merge_header_bounds(state.bounds.take(), chunk_bounds));
        }
        Ok(())
    }

    fn stream_finish(&mut self) -> Result<(), StageError> {
        let Some(mut state) = self.stream.0.take() else {
            // No chunk ever initialized the writer (all-empty stream): produce
            // the same empty-file output as the materializing path.
            return self.write(&[]);
        };
        state
            .writer
            .close()
            .map_err(|e| StageError(format!("Failed to close LAS writer: {}", e)))?;
        let path = Path::new(&self.filename);
        if let Some(bounds) = state.bounds {
            write_header_bounds(path, &bounds)?;
            patch_pdal_legacy_header_counts(
                path,
                self.point_format,
                self.minor_version.unwrap_or(2),
            )?;
        }
        Ok(())
    }
}

fn validate_unique_spatial_reference(
    views: &[PointView],
    filename: &str,
) -> Result<(), StageError> {
    let Some(first) = views.first().map(PointView::spatial_reference) else {
        return Ok(());
    };

    for srs in views.iter().skip(1).map(PointView::spatial_reference) {
        if srs == first
            || (!srs.is_empty()
                && !first.is_empty()
                && srs.epoch() == first.epoch()
                && pdal_native::srs::is_same(first.wkt(), srs.wkt(), first.epoch()))
        {
            continue;
        }
        return Err(StageError(format!(
            "writers.las: Attempting to write '{filename}' with multiple point spatial references."
        )));
    }

    Ok(())
}

mod write_helpers;
use write_helpers::*;
pub(crate) use write_helpers::{copc_extra_dims, point_from_view};

include!("las_writer_header_patch.rs");

#[cfg(test)]
mod tests;
