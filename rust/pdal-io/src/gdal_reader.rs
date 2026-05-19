//! `readers.gdal` -- Read GDAL rasters as point clouds.
//!
//! Port of `io/GdalReader.cpp`. Creates a point for every pixel in the raster.

use pdal_core::gdal::Raster;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use std::rc::Rc;

pub struct GdalReader {
    filename: String,
    metadata: MetadataNode,
}

impl GdalReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
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
            return Err(StageError("GdalReader requires a filename option.".to_string()));
        }

        pdal_core::gdal::register_drivers();
        let raster = Raster::open(&self.filename).map_err(StageError)?;
        let width = raster.width();
        let height = raster.height();
        let band_count = raster.band_count();
        let transform = raster.get_geo_transform().map_err(StageError)?;
        let wkt_srs = raster.get_wkt_srs();

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);

        for b in 0..band_count {
            layout.register(DimId::Other(format!("band_{}", b + 1)), DimType::F64);
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
            raster.read_band(b + 1, width as usize, height as usize, &mut data).map_err(StageError)?;
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
                
                // Use first band as Z by default (PDAL behavior can be overridden but this is standard)
                let gz = bands_data[0][(y * width + x) as usize];

                view.set_f64(id, &DimId::X, gx);
                view.set_f64(id, &DimId::Y, gy);
                view.set_f64(id, &DimId::Z, gz);

                for b in 0..band_count {
                    let val = bands_data[b as usize][(y * width + x) as usize];
                    view.set_f64(id, &DimId::Other(format!("band_{}", b + 1)), val);
                }
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}
