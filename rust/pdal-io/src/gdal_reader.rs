//! `readers.gdal` -- Read GDAL rasters as point clouds.
//!
//! Port of `io/GdalReader.cpp`. Creates a point for every pixel in the raster.

use pdal_core::gdal::Raster;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use std::rc::Rc;

pub struct GdalReader {
    filename: String,
    header: String,
    metadata: MetadataNode,
    stream: Option<GdalStreamState>,
}

struct GdalStreamState {
    raster: Raster,
    width: i32,
    height: i32,
    band_count: i32,
    transform: [f64; 6],
    dims: Vec<DimId>,
    layout: Rc<PointLayout>,
    wkt_srs: String,
    next_row: i32,
}

impl GdalReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            header: options.get_str("header", ""),
            metadata: MetadataNode::new("readers.gdal"),
            stream: None,
        }
    }
}

impl Reader for GdalReader {
    fn name(&self) -> &str {
        "readers.gdal"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "GdalReader requires a filename option.".to_string(),
            ));
        }

        let state = self.open_stream_state()?;
        let mut view = stream_view(&state);
        let mut bands_data = Vec::with_capacity(state.band_count as usize);
        for b in 0..state.band_count {
            let mut data = vec![0.0f64; (state.width * state.height) as usize];
            state
                .raster
                .read_band(
                    b + 1,
                    state.width as usize,
                    state.height as usize,
                    &mut data,
                )
                .map_err(StageError)?;
            bands_data.push(data);
        }

        for y in 0..state.height {
            for x in 0..state.width {
                let id = view.add_point();

                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;

                let gx = state.transform[0] + px * state.transform[1] + py * state.transform[2];
                let gy = state.transform[3] + px * state.transform[4] + py * state.transform[5];

                view.set_f64(id, &DimId::X, gx);
                view.set_f64(id, &DimId::Y, gy);

                for b in 0..state.band_count {
                    let val = bands_data[b as usize][(y * state.width + x) as usize];
                    view.set_f64(id, &state.dims[b as usize], val);
                }
            }
        }

        Ok(vec![view])
    }

    fn streamable(&self) -> bool {
        !self.filename.is_empty()
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.stream.is_none() {
            self.stream = Some(self.open_stream_state()?);
        }
        let state = self.stream.as_mut().expect("stream initialized");
        if state.next_row >= state.height {
            return Ok(None);
        }

        let rows = ((capacity.max(1) + state.width as usize - 1) / state.width as usize)
            .max(1)
            .min((state.height - state.next_row) as usize);
        let y_start = state.next_row;
        state.next_row += rows as i32;

        let mut band_windows = Vec::with_capacity(state.band_count as usize);
        for band in 0..state.band_count {
            let mut data = vec![0.0; state.width as usize * rows];
            state
                .raster
                .read_band_window(
                    band + 1,
                    0,
                    y_start as usize,
                    state.width as usize,
                    rows,
                    &mut data,
                )
                .map_err(StageError)?;
            band_windows.push(data);
        }

        let mut view = stream_view(state);
        for row in 0..rows {
            let y = y_start + row as i32;
            for x in 0..state.width {
                let id = view.add_point();
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                view.set_f64(
                    id,
                    &DimId::X,
                    state.transform[0] + px * state.transform[1] + py * state.transform[2],
                );
                view.set_f64(
                    id,
                    &DimId::Y,
                    state.transform[3] + px * state.transform[4] + py * state.transform[5],
                );
                for band in 0..state.band_count {
                    let value =
                        band_windows[band as usize][row * state.width as usize + x as usize];
                    view.set_f64(id, &state.dims[band as usize], value);
                }
            }
        }

        Ok(Some(view))
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

impl GdalReader {
    fn open_stream_state(&mut self) -> Result<GdalStreamState, StageError> {
        pdal_core::gdal::register_drivers();
        let raster = Raster::open(&self.filename).map_err(StageError)?;
        let width = raster.width();
        let height = raster.height();
        let band_count = raster.band_count();
        if width <= 0 || height <= 0 || band_count <= 0 {
            return Err(StageError("Invalid GDAL raster dimensions.".to_string()));
        }
        let transform = raster.get_geo_transform().map_err(StageError)?;
        let wkt_srs = raster.get_wkt_srs();
        let dim_names = self.dimension_names(band_count)?;
        self.metadata = reader_metadata(&self.filename, width, height, band_count);

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);

        let dims: Vec<DimId> = dim_names
            .iter()
            .map(|name| DimId::from_name(name))
            .collect();
        for dim in &dims {
            layout.register(dim.clone(), DimType::F64);
        }

        Ok(GdalStreamState {
            raster,
            width,
            height,
            band_count,
            transform,
            dims,
            layout: Rc::new(layout),
            wkt_srs,
            next_row: 0,
        })
    }

    fn dimension_names(&self, band_count: i32) -> Result<Vec<String>, StageError> {
        if self.header.is_empty() {
            return Ok((1..=band_count).map(|idx| format!("band_{idx}")).collect());
        }

        let names: Vec<String> = self
            .header
            .split(',')
            .map(str::trim)
            .map(str::to_string)
            .collect();
        if names.len() != band_count as usize {
            return Err(StageError(
                "Dimension names are not the same count as raster bands.".to_string(),
            ));
        }
        Ok(names)
    }
}

fn stream_view(state: &GdalStreamState) -> PointView {
    let mut view = PointView::new(Rc::clone(&state.layout));
    if !state.wkt_srs.is_empty() {
        view.set_spatial_reference(SpatialReference::new(&state.wkt_srs));
    }
    view
}

fn reader_metadata(filename: &str, width: i32, height: i32, band_count: i32) -> MetadataNode {
    let mut metadata = MetadataNode::new("readers.gdal");
    let mut raster = MetadataNode::new("raster");
    raster.add_value("filename", MetadataValue::String(filename.to_string()));
    raster.add_value("width", MetadataValue::I64(i64::from(width)));
    raster.add_value("height", MetadataValue::I64(i64::from(height)));
    raster.add_value("band_count", MetadataValue::I64(i64::from(band_count)));
    metadata.add_child(raster);
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dimension_names_match_pdal_band_names() {
        let reader = GdalReader::new(&Options::new());

        assert_eq!(
            reader.dimension_names(3).unwrap(),
            vec!["band_1", "band_2", "band_3"]
        );
    }

    #[test]
    fn header_dimension_names_are_trimmed_and_count_checked() {
        let mut options = Options::new();
        options.add("header", "Intensity, Userdata, Z");
        let reader = GdalReader::new(&options);

        assert_eq!(
            reader.dimension_names(3).unwrap(),
            vec!["Intensity", "Userdata", "Z"]
        );
        assert!(reader.dimension_names(2).is_err());
    }

    #[test]
    fn streaming_chunks_match_full_read() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reader.tif");
        {
            pdal_core::gdal::register_drivers();
            let mut raster = Raster::create_float64(
                path.to_str().unwrap(),
                "GTiff",
                3,
                2,
                2,
                [10.0, 2.0, 0.0, 20.0, 0.0, -2.0],
                "",
            )
            .unwrap();
            raster
                .write_band_f64(1, 3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], -9999.0, "a")
                .unwrap();
            raster
                .write_band_f64(2, 3, 2, &[11.0, 12.0, 13.0, 14.0, 15.0, 16.0], -9999.0, "b")
                .unwrap();
        }

        let mut options = Options::new();
        options.add("filename", path.display());
        let mut full_reader = GdalReader::new(&options);
        let full = full_reader.read().unwrap().remove(0);

        let mut stream_reader = GdalReader::new(&options);
        assert!(stream_reader.streamable());
        let first = stream_reader.stream_next(3).unwrap().unwrap();
        let second = stream_reader.stream_next(3).unwrap().unwrap();
        assert!(stream_reader.stream_next(3).unwrap().is_none());

        assert_eq!(first.len(), 3);
        assert_eq!(second.len(), 3);
        for idx in 0..3 {
            for dim in [
                DimId::X,
                DimId::Y,
                DimId::from_name("band_1"),
                DimId::from_name("band_2"),
            ] {
                assert_eq!(first.get_f64(idx, &dim), full.get_f64(idx, &dim));
                assert_eq!(second.get_f64(idx, &dim), full.get_f64(idx + 3, &dim));
            }
        }
    }
}
