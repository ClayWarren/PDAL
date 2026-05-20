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
}

impl GdalReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            header: options.get_str("header", ""),
            metadata: MetadataNode::new("readers.gdal"),
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

        pdal_core::gdal::register_drivers();
        let raster = Raster::open(&self.filename).map_err(StageError)?;
        let width = raster.width();
        let height = raster.height();
        let band_count = raster.band_count();
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

        let mut view = PointView::new(Rc::new(layout));
        if !wkt_srs.is_empty() {
            view.set_spatial_reference(SpatialReference::new(&wkt_srs));
        }

        // We read band by band for efficiency, or pixel by pixel?
        // PDAL's GdalReader reads in blocks. For this slice, we will do a simple full-read.
        let mut bands_data = Vec::with_capacity(band_count as usize);
        for b in 0..band_count {
            let mut data = vec![0.0f64; (width * height) as usize];
            raster
                .read_band(b + 1, width as usize, height as usize, &mut data)
                .map_err(StageError)?;
            bands_data.push(data);
        }

        for y in 0..height {
            for x in 0..width {
                let id = view.add_point();

                // Pixel center coordinates
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;

                let gx = transform[0] + px * transform[1] + py * transform[2];
                let gy = transform[3] + px * transform[4] + py * transform[5];

                view.set_f64(id, &DimId::X, gx);
                view.set_f64(id, &DimId::Y, gy);

                for b in 0..band_count {
                    let val = bands_data[b as usize][(y * width + x) as usize];
                    view.set_f64(id, &dims[b as usize], val);
                }
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

impl GdalReader {
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
}
