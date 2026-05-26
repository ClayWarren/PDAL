//! `readers.las` and `readers.laz` -- ASPRS LAS and LAZ formats.
//!
//! Port of `io/LasReader.cpp` using the `las` Rust crate.

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

#[derive(Clone)]
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
        let path = Path::new(&self.filename);
        let mut reader = las::Reader::from_path(path)
            .map_err(|e| StageError(format!("Failed to open LAS/LAZ file: {}", e)))?;
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
            let file = File::open(path)
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

fn srs_vlr_order_from_options(options: &Options) -> Vec<SrsVlrKind> {
    let spec = options.get_str("srs_vlr_order", "");
    if spec.trim().is_empty() {
        return Vec::new();
    }

    spec.split(',')
        .filter_map(|part| parse_srs_vlr_kind(part.trim()))
        .collect()
}

fn parse_srs_vlr_kind(name: &str) -> Option<SrsVlrKind> {
    match name.to_ascii_lowercase().as_str() {
        "wkt1" => Some(SrsVlrKind::Wkt1),
        "geotiff" => Some(SrsVlrKind::Geotiff),
        "projjson" => Some(SrsVlrKind::Proj),
        "wkt2" | "wkt" => Some(SrsVlrKind::Wkt2),
        _ => None,
    }
}

fn header_must_use_wkt(header: &Header) -> bool {
    header.version().minor >= 4 || header.point_format().is_extended
}

fn default_srs_vlr_order(header: &Header) -> Vec<SrsVlrKind> {
    if header_must_use_wkt(header) {
        vec![SrsVlrKind::Wkt2, SrsVlrKind::Proj, SrsVlrKind::Wkt1]
    } else {
        vec![SrsVlrKind::Wkt2, SrsVlrKind::Proj, SrsVlrKind::Geotiff]
    }
}

fn find_vlr<'a>(header: &'a Header, user_id: &str, record_id: u16) -> Option<&'a las::Vlr> {
    header
        .vlrs()
        .iter()
        .chain(header.evlrs().iter())
        .find(|vlr| vlr.user_id == user_id && vlr.record_id == record_id)
}

fn vlr_as_string(data: &[u8]) -> String {
    let len = data
        .iter()
        .rposition(|byte| *byte != 0)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    String::from_utf8_lossy(&data[..len]).trim().to_string()
}

fn resolve_spatial_reference_from_vlrs(
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

fn spatial_reference_from_geotiff_vlrs(
    header: &Header,
) -> Result<Option<pdal_core::srs::SpatialReference>, StageError> {
    let geotiff = header
        .get_geotiff_crs()
        .map_err(|err| StageError(format!("Could not create an SRS: {err}")))?
        .ok_or_else(|| StageError("Could not create an SRS: missing GeoTIFF keys.".to_string()))?;
    let crs = get_epsg_from_geotiff_crs(&geotiff)
        .map_err(|err| StageError(format!("Could not create an SRS: {err}")))?;
    Ok(Some(pdal_core::srs::SpatialReference::new(&format!(
        "EPSG:{}",
        crs.get_horizontal()
    ))))
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

fn las_layout(
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

fn extra_dims_from_configured(
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
    if header.point_format().is_extended {
        layout.register(DimId::from_name("ScanChannel"), DimType::U8);
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

fn scan_angle_degrees(point: &las::Point, point_format: u8) -> f64 {
    if point_format >= 6 {
        let scaled = (f64::from(point.scan_angle) / SCAN_ANGLE_SCALE_FACTOR).round() as i16;
        f64::from(scaled) * SCAN_ANGLE_SCALE_FACTOR
    } else {
        f64::from(point.scan_angle)
    }
}

fn set_standard_dims(view: &mut PointView, id: u64, point: &las::Point, point_format: u8) {
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

#[cfg(test)]
include!("las_tests.rs");
