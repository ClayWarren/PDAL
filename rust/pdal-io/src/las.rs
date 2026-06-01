//! `readers.las` and `readers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasReader.cpp` using the `las` Rust crate.

use crate::source;
use byteorder::{LittleEndian, ReadBytesExt};
use chrono::Datelike;
use las::point::ScanDirection;
use las::Header;
use las_crs::{get_epsg_from_geotiff_crs, ParseEpsgCRS};
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

const SCAN_ANGLE_SCALE_FACTOR: f64 = 0.006;
const VLR_HEADER_SIZE: u64 = 54;
const TRANSFORM_USER_ID: &str = "LASF_Projection";
const PDAL_USER_ID: &str = "PDAL";
const LIBLAS_USER_ID: &str = "liblas";
const WKT_RECORD_ID: u16 = 2112;
const WKT2_RECORD_ID: u16 = 4224;
const PROJJSON_RECORD_ID: u16 = 4225;
const GEOTIFF_DIRECTORY_RECORD_ID: u16 = 34735;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SrsVlrKind {
    Wkt1,
    Geotiff,
    Proj,
    Wkt2,
}

#[derive(Clone)]
struct ConfiguredExtraDim {
    name: String,
    type_name: String,
}

pub struct LasReader {
    filename: String,
    start: u64,
    count: Option<u64>,
    nosrs: bool,
    ignore_missing_vlrs: bool,
    start_offset: u64,
    start_length: u64,
    configured_extra_dims: Vec<ConfiguredExtraDim>,
    srs_vlr_order: Vec<SrsVlrKind>,
    metadata: MetadataNode,
    /// Streaming state for the chunked `stream_next` path (simple local-file
    /// reads only); `None` until the first chunk is pulled.
    stream: Option<LasStreamState>,
}

impl Clone for LasReader {
    fn clone(&self) -> Self {
        Self {
            filename: self.filename.clone(),
            start: self.start,
            count: self.count,
            nosrs: self.nosrs,
            ignore_missing_vlrs: self.ignore_missing_vlrs,
            start_offset: self.start_offset,
            start_length: self.start_length,
            configured_extra_dims: self.configured_extra_dims.clone(),
            srs_vlr_order: self.srs_vlr_order.clone(),
            metadata: self.metadata.clone(),
            // A clone is a fresh reader; in-progress streaming state is not shared.
            stream: None,
        }
    }
}

/// Open reader plus derived layout used by the chunked streaming read path.
struct LasStreamState {
    reader: las::Reader,
    layout: Rc<PointLayout>,
    point_format: u8,
    extra_dims: Vec<ExtraDim>,
    srs: pdal_core::srs::SpatialReference,
    remaining: u64,
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
            ignore_missing_vlrs: options.get_bool("ignore_missing_vlrs", false),
            // Non-zero: skip this many bytes before the LAS header. Used by
            // `readers.nitf` to expose the embedded LAS payload.
            start_offset: options.get_u64("start_offset", 0),
            start_length: options.get_u64("start_length", 0),
            configured_extra_dims: configured_extra_dims_from_options(options),
            srs_vlr_order: srs_vlr_order_from_options(options),
            metadata: MetadataNode::new("readers.las"),
            stream: None,
        }
    }

    /// Open the file and derive layout/SRS for the streaming read path. Mirrors
    /// the plain `read_standard_reader` setup so chunked reads produce the same
    /// points as `read()`.
    fn stream_init(&mut self) -> Result<(), StageError> {
        let path = Path::new(&self.filename);
        let mut reader = las::Reader::from_path(path)
            .map_err(|e| StageError(format!("Failed to open LAS file: {}", e)))?;
        let header = reader.header();
        let point_count = header.number_of_points();
        let point_format = header.point_format().to_u8().unwrap_or(3);
        if self.start >= point_count && point_count > 0 {
            return Err(StageError(format!(
                "LAS start point {} is outside the file's {} points.",
                self.start, point_count
            )));
        }
        self.add_metadata(header);
        let (layout, extra_dims) = las_layout(header, &self.configured_extra_dims)?;
        let layout = Rc::new(layout);
        let mut probe = PointView::new(Rc::clone(&layout));
        self.set_spatial_reference(&mut probe, header)?;
        let srs = probe.spatial_reference().clone();
        let remaining = self.count.unwrap_or(point_count.saturating_sub(self.start));

        // `header` borrow of `reader` ends here, so we can advance the reader.
        if self.start > 0 {
            reader
                .read_points(self.start)
                .map_err(|e| StageError(format!("Failed to seek LAS start: {}", e)))?;
        }
        self.stream = Some(LasStreamState {
            reader,
            layout,
            point_format,
            extra_dims,
            srs,
            remaining,
        });
        Ok(())
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
        if let Some(date) = header.date() {
            self.metadata
                .add_value("creation_year", MetadataValue::U64(date.year() as u64));
            self.metadata
                .add_value("creation_doy", MetadataValue::U64(date.ordinal() as u64));
        }
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

        // Surface VLRs as metadata, matching C++ las::addVlrMetadata (called per
        // VLR from LasReader). Without this the pure-Rust read path (e.g.
        // `pdal info --metadata`) drops all VLR records.
        for (index, vlr) in header
            .vlrs()
            .iter()
            .chain(header.evlrs().iter())
            .enumerate()
        {
            self.add_vlr_metadata(vlr, index);
        }
    }

    fn add_vlr_metadata(&mut self, vlr: &las::Vlr, index: usize) {
        // C++ las::addVlrMetadata skips VLRs larger than 1 MB.
        const DATA_LEN_MAX: usize = 1_000_000;
        if vlr.data.len() > DATA_LEN_MAX {
            return;
        }

        // PDAL metadata/pipeline VLRs carry (NUL-terminated) JSON, not opaque
        // bytes. Match the C++ pdal_metadata / pdal_pipeline naming.
        const PDAL_USER_ID: &str = "PDAL";
        const PDAL_METADATA_RECORD_ID: u16 = 12;
        const PDAL_PIPELINE_RECORD_ID: u16 = 13;
        if vlr.user_id == PDAL_USER_ID
            && (vlr.record_id == PDAL_METADATA_RECORD_ID
                || vlr.record_id == PDAL_PIPELINE_RECORD_ID)
        {
            let name = if vlr.record_id == PDAL_METADATA_RECORD_ID {
                "pdal_metadata"
            } else {
                "pdal_pipeline"
            };
            let bytes: Vec<u8> = vlr.data.iter().copied().filter(|&b| b != 0).collect();
            let json = String::from_utf8_lossy(&bytes).into_owned();
            self.metadata.add_value(name, MetadataValue::String(json));
            return;
        }

        let mut node = MetadataNode::new(format!("vlr_{index}"));
        node.add_value(
            "data",
            MetadataValue::String(pdal_core::utils::base64_encode(&vlr.data)),
        );
        node.add_value("user_id", MetadataValue::String(vlr.user_id.clone()));
        node.add_value("record_id", MetadataValue::U64(vlr.record_id as u64));
        node.add_value(
            "description",
            MetadataValue::String(vlr.description.clone()),
        );
        self.metadata.add_child(node);
    }

    fn set_spatial_reference(
        &self,
        view: &mut PointView,
        header: &Header,
    ) -> Result<(), StageError> {
        if self.nosrs {
            return Ok(());
        }

        if let Some(srs) = resolve_spatial_reference_from_vlrs(header, &self.srs_vlr_order)? {
            view.set_spatial_reference(srs);
            return Ok(());
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
        Ok(())
    }

    fn read_points(
        &self,
        reader: &mut las::Reader,
        point_count: u64,
        point_format: u8,
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
            append_point(view, &point, point_format, extra_dims)?;
        }
        Ok(())
    }

    /// Read only the points whose indices fall inside `ranges`. Returns the
    /// materialized view; callers can read `metadata()` afterward. Used by
    /// `readers.copc` for `resolution`-pruned execution. Each range is
    /// `(start_point_idx, count)`; behavior is the union of all ranges, which
    /// must be sorted and disjoint.
    ///
    /// Implementation note: `laz::LasZipDecompressor::seek` is buggy for
    /// variable-size chunks (which COPC always uses) — its delta formula
    /// assumes uniform chunk sizes. We side-step it by streaming the LAZ
    /// points sequentially and dropping records outside the kept ranges.
    pub fn read_ranges(&mut self, ranges: &[(u64, u64)]) -> Result<PointView, StageError> {
        let mut reader = if is_vsi_path(&self.filename) {
            let data = read_vsi_file(&self.filename)?;
            las::Reader::new(Cursor::new(data))
                .map_err(|e| StageError(format!("Failed to open LAS/LAZ VSI data: {}", e)))?
        } else {
            let path = Path::new(&self.filename);
            las::Reader::from_path(path)
                .map_err(|e| StageError(format!("Failed to open LAS/LAZ file: {}", e)))?
        };
        let header = reader.header();
        let point_format = header.point_format().to_u8().unwrap_or(3);
        self.add_metadata(header);
        let (layout, extra_dims) = las_layout(header, &self.configured_extra_dims)?;
        let mut view = PointView::new(Rc::new(layout));
        self.set_spatial_reference(&mut view, header)?;

        if ranges.is_empty() {
            return Ok(view);
        }
        let max_end = ranges.iter().map(|(s, c)| s + c).max().unwrap_or(0);
        let mut range_idx = 0usize;
        for (idx, point_result) in (0_u64..).zip(reader.points()) {
            if idx >= max_end {
                break;
            }
            let point = point_result
                .map_err(|e| StageError(format!("Failed to read LAS/LAZ point: {}", e)))?;
            while range_idx < ranges.len() {
                let (start, count) = ranges[range_idx];
                if idx >= start + count {
                    range_idx += 1;
                } else {
                    break;
                }
            }
            if range_idx < ranges.len() {
                let (start, count) = ranges[range_idx];
                if idx >= start && idx < start + count {
                    append_point(&mut view, &point, point_format, &extra_dims)?;
                }
            }
        }
        Ok(view)
    }

    fn read_standard_reader(&mut self, reader: &mut las::Reader) -> Result<PointView, StageError> {
        let header = reader.header();
        let point_count = header.number_of_points();
        let point_format = header.point_format().to_u8().unwrap_or(3);
        if self.start >= point_count && point_count > 0 {
            return Err(StageError(format!(
                "LAS start point {} is outside the file's {} points.",
                self.start, point_count
            )));
        }

        self.add_metadata(header);
        let (layout, extra_dims) = las_layout(header, &self.configured_extra_dims)?;

        let mut view = PointView::new(Rc::new(layout));
        self.set_spatial_reference(&mut view, header)?;
        self.read_points(reader, point_count, point_format, &mut view, &extra_dims)?;
        Ok(view)
    }

    fn read_points_from_stream<R: Read + Seek>(
        &self,
        read: &mut R,
        header: &Header,
        point_format: u8,
        view: &mut PointView,
        extra_dims: &[ExtraDim],
    ) -> Result<(), StageError> {
        let point_count = header.number_of_points();
        let take_count = self.count.unwrap_or(point_count.saturating_sub(self.start));
        let format = header.point_format();

        for _ in 0..self.start {
            las::raw::Point::read_from(&mut *read, format)
                .map_err(|e| StageError(format!("Failed to read LAS point: {}", e)))?;
        }
        for _ in 0..take_count {
            let raw_point = las::raw::Point::read_from(&mut *read, format)
                .map_err(|e| StageError(format!("Failed to read LAS point: {}", e)))?;
            let point = las::Point::new(raw_point, header.transforms());
            append_point(view, &point, point_format, extra_dims)?;
        }
        Ok(())
    }
}

const COPC_SIGNATURE_OFFSET: u64 = 377;

/// Return true when the LAS file at `path` contains a COPC VLR signature.
pub fn detect_copc(path: &Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.seek(SeekFrom::Start(COPC_SIGNATURE_OFFSET)).is_err() {
        return false;
    }
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature).is_ok() && signature == *b"copc"
}

fn is_vsi_path(filename: &str) -> bool {
    filename.starts_with("/vsi")
        || filename.starts_with("http://")
        || filename.starts_with("https://")
}

fn read_vsi_file(filename: &str) -> Result<Vec<u8>, StageError> {
    let vsi_path = if filename.starts_with("http://") || filename.starts_with("https://") {
        format!("/vsicurl/{filename}")
    } else {
        filename.to_string()
    };
    let mut file = pdal_native::vsi::VsiFile::open(&vsi_path)
        .map_err(|err| StageError(format!("Failed to open LAS VSI path: {err}")))?;
    let len = file
        .len()
        .map_err(|err| StageError(format!("Failed to size LAS VSI path: {err}")))?;
    file.read_exact_at(0, len as usize)
        .map_err(|err| StageError(format!("Failed to read LAS VSI path: {err}")))
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
        if filename_has_glob(&self.filename) {
            return self.read_glob();
        }

        let path = Path::new(&self.filename);
        let view = if is_vsi_path(&self.filename) {
            let data = read_vsi_file(&self.filename)?;
            let cursor = Cursor::new(data);
            let mut reader = las::Reader::new(cursor)
                .map_err(|e| StageError(format!("Failed to open LAS VSI path: {e}")))?;
            self.read_standard_reader(&mut reader)?
        } else if self.start_offset > 0 {
            // NITF-embedded LAS: open the file, shift past the wrapper bytes,
            // and let the standard `las` reader walk the embedded payload.
            let file = File::open(path)
                .map_err(|e| StageError(format!("Failed to open LAS file: {}", e)))?;
            let shifted = if self.start_length > 0 {
                crate::shift_reader::ShiftReader::with_length(
                    file,
                    self.start_offset,
                    self.start_length,
                )
            } else {
                crate::shift_reader::ShiftReader::new(file, self.start_offset)
            }
            .map_err(|e| StageError(format!("Failed to seek LAS start offset: {}", e)))?;
            let buffered = BufReader::new(shifted);
            let mut reader = las::Reader::new(buffered)
                .map_err(|e| StageError(format!("Failed to open embedded LAS: {}", e)))?;
            let header = reader.header();
            let point_count = header.number_of_points();
            let point_format = header.point_format().to_u8().unwrap_or(3);
            if self.start >= point_count && point_count > 0 {
                return Err(StageError(format!(
                    "LAS start point {} is outside the file's {} points.",
                    self.start, point_count
                )));
            }
            self.add_metadata(header);
            let (layout, extra_dims) = las_layout(header, &self.configured_extra_dims)?;
            let mut view = PointView::new(Rc::new(layout));
            self.set_spatial_reference(&mut view, header)?;
            self.read_points(
                &mut reader,
                point_count,
                point_format,
                &mut view,
                &extra_dims,
            )?;
            view
        } else if self.ignore_missing_vlrs {
            let file = source::open_seek(&self.filename)
                .map_err(|e| StageError(format!("Failed to open LAS file: {}", e)))?;
            let mut read = BufReader::new(file);
            let raw_header = las::raw::Header::read_from(&mut read)
                .map_err(|e| StageError(format!("Failed to read LAS header: {}", e)))?;
            let header = read_header_lenient(&mut read, raw_header)?;
            let point_count = header.number_of_points();
            let point_format = header.point_format().to_u8().unwrap_or(3);
            if self.start >= point_count && point_count > 0 {
                return Err(StageError(format!(
                    "LAS start point {} is outside the file's {} points.",
                    self.start, point_count
                )));
            }

            self.add_metadata(&header);
            let (layout, extra_dims) = las_layout(&header, &self.configured_extra_dims)?;

            let mut view = PointView::new(Rc::new(layout));
            self.set_spatial_reference(&mut view, &header)?;
            self.read_points_from_stream(&mut read, &header, point_format, &mut view, &extra_dims)?;
            view
        } else {
            let mut reader = las::Reader::from_path(path)
                .map_err(|e| StageError(format!("Failed to open LAS file: {}", e)))?;

            self.read_standard_reader(&mut reader)?
        };

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }

    fn reset(&mut self) {
        self.stream = None;
    }

    fn streamable(&self) -> bool {
        // Only the plain local-file standard read path streams; glob, VSI,
        // NITF-embedded (start_offset), and lenient (ignore_missing_vlrs) reads
        // fall back to the materializing `read()`.
        !self.filename.is_empty()
            && !filename_has_glob(&self.filename)
            && !is_vsi_path(&self.filename)
            && self.start_offset == 0
            && self.start_length == 0
            && !self.ignore_missing_vlrs
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.stream.is_none() {
            self.stream_init()?;
        }
        let state = self.stream.as_mut().expect("stream initialized above");
        if state.remaining == 0 {
            return Ok(None);
        }
        let take = (capacity.max(1) as u64).min(state.remaining);
        let points = state
            .reader
            .read_points(take)
            .map_err(|e| StageError(format!("Failed to read LAS point: {}", e)))?;
        if points.is_empty() {
            state.remaining = 0;
            return Ok(None);
        }

        let mut view = PointView::new(Rc::clone(&state.layout));
        view.set_spatial_reference(state.srs.clone());
        for point in &points {
            append_point(&mut view, point, state.point_format, &state.extra_dims)?;
        }

        let read = points.len() as u64;
        state.remaining = state.remaining.saturating_sub(read);
        if read < take {
            state.remaining = 0; // short read => end of file
        }
        Ok(Some(view))
    }
}

impl LasReader {
    fn read_glob(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.start_offset > 0 || self.start_length > 0 {
            return Err(StageError(
                "LAS filename globbing is not supported with start offsets.".to_string(),
            ));
        }

        let mut paths = glob::glob(&self.filename)
            .map_err(|err| StageError(format!("Invalid LAS filename glob: {err}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| StageError(format!("Failed to expand LAS filename glob: {err}")))?;
        paths.sort();
        if paths.is_empty() {
            return Err(StageError(format!(
                "LAS filename glob '{}' matched no files.",
                self.filename
            )));
        }

        let mut merged: Option<PointView> = None;
        for path in paths {
            let mut reader = self.clone();
            reader.filename = path.display().to_string();
            let mut views = reader.read()?;
            let view = views
                .drain(..)
                .next()
                .ok_or_else(|| StageError(format!("'{}' produced no points.", path.display())))?;
            append_glob_view(&mut merged, &view, &path.display().to_string())?;
            self.metadata = reader.metadata;
        }

        Ok(merged.into_iter().collect())
    }
}

fn filename_has_glob(filename: &str) -> bool {
    filename.contains('*') || filename.contains('?') || filename.contains('[')
}

fn append_glob_view(
    output: &mut Option<PointView>,
    view: &PointView,
    path: &str,
) -> Result<(), StageError> {
    let Some(output) = output else {
        *output = Some(view.clone());
        return Ok(());
    };
    ensure_same_layout(output, view, path)?;
    for idx in 0..view.len() {
        output.append_point(view, idx);
    }
    Ok(())
}

fn ensure_same_layout(
    reference: &PointView,
    view: &PointView,
    path: &str,
) -> Result<(), StageError> {
    if reference.layout().dim_count() != view.layout().dim_count()
        || reference.layout().point_size() != view.layout().point_size()
    {
        return Err(StageError(format!(
            "'{path}' produced point views with incompatible layouts"
        )));
    }
    for idx in 0..reference.layout().dim_count() {
        if reference.layout().dim_at(idx) != view.layout().dim_at(idx) {
            return Err(StageError(format!(
                "'{path}' produced point views with incompatible layouts"
            )));
        }
    }
    Ok(())
}

enum VlrReadResult {
    Ok(las::Vlr),
    Stop,
}

fn read_vlr_lenient<R: Read + Seek>(read: &mut R, point_offset: u64) -> VlrReadResult {
    let vlr_start = match read.stream_position() {
        Ok(position) => position,
        Err(_) => return VlrReadResult::Stop,
    };

    let mut header_buf = [0u8; VLR_HEADER_SIZE as usize];
    if read.read_exact(&mut header_buf).is_err() {
        return VlrReadResult::Stop;
    }

    let data_len = u64::from(u16::from_le_bytes([header_buf[20], header_buf[21]]));
    if vlr_start + VLR_HEADER_SIZE + data_len > point_offset {
        return VlrReadResult::Stop;
    }

    let mut data = vec![0u8; data_len as usize];
    if read.read_exact(&mut data).is_err() {
        return VlrReadResult::Stop;
    }

    let mut user_id = [0u8; 16];
    user_id.copy_from_slice(&header_buf[2..18]);
    let mut description = [0u8; 32];
    description.copy_from_slice(&header_buf[22..54]);
    let raw_vlr = las::raw::Vlr {
        reserved: u16::from_le_bytes([header_buf[0], header_buf[1]]),
        user_id,
        record_id: u16::from_le_bytes([header_buf[18], header_buf[19]]),
        record_length_after_header: las::raw::vlr::RecordLength::Vlr(u16::from_le_bytes([
            header_buf[20],
            header_buf[21],
        ])),
        description,
        data,
    };
    VlrReadResult::Ok(las::Vlr::new(raw_vlr))
}

fn read_header_lenient<R: Read + Seek>(
    read: &mut R,
    raw_header: las::raw::Header,
) -> Result<Header, StageError> {
    let point_format = las::point::Format::new(raw_header.point_data_record_format)
        .map_err(|e| StageError(format!("Invalid LAS point format: {}", e)))?;
    if point_format.is_compressed {
        return Err(StageError(
            "ignore_missing_vlrs is not supported for LAZ files.".to_string(),
        ));
    }

    let mut position = u64::from(raw_header.header_size);
    let vlr_count = raw_header.number_of_variable_length_records;
    let offset_to_point_data = u64::from(raw_header.offset_to_point_data);
    let offset_to_end_of_points = raw_header.offset_to_end_of_points();
    let evlr = raw_header.evlr;

    let mut builder = las::Builder::new(raw_header)
        .map_err(|e| StageError(format!("Failed to build LAS header: {}", e)))?;

    for _ in 0..vlr_count {
        match read_vlr_lenient(read, offset_to_point_data) {
            VlrReadResult::Ok(vlr) => {
                position += vlr.len(false) as u64;
                builder.vlrs.push(vlr);
            }
            VlrReadResult::Stop => break,
        }
    }

    match position.cmp(&offset_to_point_data) {
        Ordering::Less => {
            read.by_ref()
                .take(offset_to_point_data - position)
                .read_to_end(&mut builder.vlr_padding)
                .map_err(|e| StageError(format!("Failed to read LAS VLR padding: {}", e)))?;
        }
        Ordering::Equal => {}
        Ordering::Greater => {
            return Err(StageError(format!(
                "LAS offset to point data ({}) is too small.",
                offset_to_point_data
            )));
        }
    }

    read.seek(SeekFrom::Start(offset_to_end_of_points))
        .map_err(|e| StageError(format!("Failed to seek LAS point data: {}", e)))?;
    if let Some(evlr) = evlr {
        if !builder.point_format.is_compressed {
            match evlr.start_of_first_evlr.cmp(&offset_to_end_of_points) {
                Ordering::Less => {
                    return Err(StageError(format!(
                        "LAS offset to EVLRs ({}) is too small.",
                        evlr.start_of_first_evlr
                    )));
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    let n = evlr.start_of_first_evlr - offset_to_end_of_points;
                    read.by_ref()
                        .take(n)
                        .read_to_end(&mut builder.point_padding)
                        .map_err(|e| {
                            StageError(format!("Failed to read LAS point padding: {}", e))
                        })?;
                }
            }
        }
        read.seek(SeekFrom::Start(evlr.start_of_first_evlr))
            .map_err(|e| StageError(format!("Failed to seek LAS EVLRs: {}", e)))?;
        let evlr = las::raw::Vlr::read_from(read.by_ref(), true)
            .map(las::Vlr::new)
            .map_err(|e| StageError(format!("Failed to read LAS EVLR: {}", e)))?;
        builder.evlrs.push(evlr);
    }

    read.seek(SeekFrom::Start(offset_to_point_data))
        .map_err(|e| StageError(format!("Failed to seek LAS point data: {}", e)))?;

    if let Some(version) = builder.minimum_supported_version() {
        if version > builder.version {
            builder.version = version;
        }
    }

    builder
        .into_header()
        .map_err(|e| StageError(format!("Failed to finalize LAS header: {}", e)))
}

fn append_point(
    view: &mut PointView,
    point: &las::Point,
    point_format: u8,
    extra_dims: &[ExtraDim],
) -> Result<(), StageError> {
    let id = view.add_point();
    set_standard_dims(view, id, point, point_format);
    set_optional_dims(view, id, point);
    set_extra_dims(view, id, point, extra_dims)
}

mod read_helpers;
use read_helpers::*;

#[cfg(test)]
mod tests;
