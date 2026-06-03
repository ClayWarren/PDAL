//! `writers.raster` -- write raster attachments from point views.

use pdal_core::gdal::RasterDataType;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::PointView;
use pdal_core::raster::RasterData;
use pdal_core::stage::StageError;

pub struct RasterWriter {
    filename: String,
    driver_name: String,
    raster_names: Vec<String>,
    data_type: RasterDataType,
    data_type_error: Option<String>,
    gdal_options: Vec<String>,
    no_data: f64,
}

impl RasterWriter {
    pub fn new(options: &Options) -> Self {
        let (data_type, data_type_error) = parse_data_type(&options.get_str("data_type", "double"));
        Self {
            filename: options.get_str("filename", ""),
            driver_name: options.get_str("gdaldriver", "GTiff"),
            raster_names: raster_names(options),
            data_type,
            data_type_error,
            gdal_options: parse_gdal_options(options),
            no_data: options.get_f64("nodata", f64::NAN),
        }
    }
}

impl Writer for RasterWriter {
    fn name(&self) -> &str {
        "writers.raster"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "RasterWriter requires a filename option.".to_string(),
            ));
        }
        if let Some(error) = &self.data_type_error {
            return Err(StageError(error.clone()));
        }

        let rasters = self.collect_rasters(views)?;
        let Some(first) = rasters.first() else {
            return Ok(());
        };
        let limits = first.limits();
        let rasters: Vec<&RasterData> = rasters
            .into_iter()
            .filter(|raster| raster.limits() == limits)
            .collect();
        if rasters.is_empty() {
            return Ok(());
        }

        let geo_transform = [
            limits.x_origin,
            limits.edge_length,
            0.0,
            limits.y_origin + limits.edge_length * limits.height as f64,
            0.0,
            -limits.edge_length,
        ];
        let srs_wkt = views
            .iter()
            .map(PointView::spatial_reference)
            .find(|srs| !srs.is_empty())
            .map(|srs| srs.wkt().to_string())
            .unwrap_or_default();

        pdal_core::gdal::register_drivers();
        let mut output = pdal_core::gdal::Raster::create_typed_with_options(
            &self.filename,
            &self.driver_name,
            limits.width as i32,
            limits.height as i32,
            rasters.len() as i32,
            geo_transform,
            &srs_wkt,
            self.data_type,
            &self.gdal_options,
        )
        .map_err(StageError)?;

        for (idx, raster) in rasters.iter().enumerate() {
            let no_data = output_no_data(self.no_data);
            let band_data = convert_no_data(raster.data(), raster.initializer(), no_data);
            output
                .write_band_f64(
                    idx as i32 + 1,
                    limits.width,
                    limits.height,
                    &band_data,
                    no_data,
                    raster.name(),
                )
                .map_err(StageError)?;
        }
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.raster");
        node.add_value("filename", MetadataValue::String(self.filename.clone()));
        node
    }
}

impl RasterWriter {
    fn collect_rasters<'a>(
        &self,
        views: &'a [PointView],
    ) -> Result<Vec<&'a RasterData>, StageError> {
        if self.raster_names.is_empty() {
            return Ok(views
                .iter()
                .find_map(|view| view.raster(""))
                .into_iter()
                .collect());
        }

        let mut rasters = Vec::new();
        for name in &self.raster_names {
            let mut found = false;
            for view in views {
                if let Some(raster) = view.raster(name) {
                    rasters.push(raster);
                    found = true;
                }
            }
            if !found {
                return Err(StageError(format!("Raster '{name}' not found.")));
            }
        }
        Ok(rasters)
    }
}

fn output_no_data(requested: f64) -> f64 {
    if requested.is_nan() {
        -9999.0
    } else {
        requested
    }
}

fn convert_no_data(values: &[f64], source_no_data: f64, output_no_data: f64) -> Vec<f64> {
    values
        .iter()
        .map(|value| {
            if *value == source_no_data || (value.is_nan() && source_no_data.is_nan()) {
                output_no_data
            } else {
                *value
            }
        })
        .collect()
}

fn raster_names(options: &Options) -> Vec<String> {
    options
        .values("rasters")
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_gdal_options(options: &Options) -> Vec<String> {
    options
        .values("gdalopts")
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_data_type(value: &str) -> (RasterDataType, Option<String>) {
    match value.to_ascii_lowercase().as_str() {
        "double" | "float64" => (RasterDataType::Float64, None),
        "float" | "float32" => (RasterDataType::Float32, None),
        "int8" | "signed8" => (RasterDataType::Int8, None),
        "uint8" | "unsigned8" | "byte" => (RasterDataType::UInt8, None),
        "int16" | "int16_t" | "signed16" => (RasterDataType::Int16, None),
        "uint16" | "uint16_t" | "unsigned16" => (RasterDataType::UInt16, None),
        "int32" | "int32_t" | "signed32" | "int" => (RasterDataType::Int32, None),
        "uint32" | "uint32_t" | "unsigned32" => (RasterDataType::UInt32, None),
        "int64" | "int64_t" | "signed64" => (RasterDataType::Int64, None),
        "uint64" | "uint64_t" | "unsigned64" => (RasterDataType::UInt64, None),
        other => (
            RasterDataType::Float64,
            Some(format!(
                "Unsupported raster writer data_type '{other}' for the Rust-backed path."
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout};
    use pdal_core::raster::{RasterData, RasterLimits};
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;

    fn temp_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "pdal-rust-raster-writer-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_file(&path);
        path
    }

    fn view_with_raster(name: &str, data: Vec<f64>) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let limits = RasterLimits::new(10.0, 20.0, 2, 2, 1.0);
        view.add_raster(RasterData::from_data(name, limits, data, -9999.0).unwrap());
        view
    }

    #[test]
    fn writes_default_raster_attachment() {
        let output = temp_path("default.tif");
        let view = view_with_raster("", vec![1.0, 2.0, 3.0, 4.0]);
        let mut options = Options::new();
        options.add("filename", output.display());
        let mut writer = RasterWriter::new(&options);

        writer.write(&[view]).unwrap();

        let raster = pdal_core::gdal::Raster::open(output.to_str().unwrap()).unwrap();
        assert_eq!(raster.width(), 2);
        assert_eq!(raster.height(), 2);
        let mut values = vec![0.0; 4];
        raster.read_band(1, 2, 2, &mut values).unwrap();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn default_nan_no_data_writes_cpp_float64_default() {
        let output = temp_path("nan-default.tif");
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let limits = RasterLimits::new(10.0, 20.0, 2, 2, 1.0);
        view.add_raster(
            RasterData::from_data("", limits, vec![1.0, f64::NAN, 3.0, f64::NAN], f64::NAN)
                .unwrap(),
        );
        let mut options = Options::new();
        options.add("filename", output.display());
        let mut writer = RasterWriter::new(&options);

        writer.write(&[view]).unwrap();

        let raster = pdal_core::gdal::Raster::open(output.to_str().unwrap()).unwrap();
        let mut values = vec![0.0; 4];
        raster.read_band(1, 2, 2, &mut values).unwrap();
        assert_eq!(values, vec![1.0, -9999.0, 3.0, -9999.0]);
    }

    #[test]
    fn named_rasters_write_as_multiple_bands() {
        let output = temp_path("named.tif");
        let mut view = view_with_raster("a", vec![1.0, 2.0, 3.0, 4.0]);
        let limits = RasterLimits::new(10.0, 20.0, 2, 2, 1.0);
        view.add_raster(
            RasterData::from_data("b", limits, vec![5.0, 6.0, 7.0, 8.0], -9999.0).unwrap(),
        );
        let mut options = Options::new();
        options.add("filename", output.display());
        options.add("rasters", "a,b");
        let mut writer = RasterWriter::new(&options);

        writer.write(&[view]).unwrap();

        let raster = pdal_core::gdal::Raster::open(output.to_str().unwrap()).unwrap();
        assert_eq!(raster.band_count(), 2);
        let mut values = vec![0.0; 4];
        raster.read_band(2, 2, 2, &mut values).unwrap();
        assert_eq!(values, vec![5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn writes_requested_raster_data_type() {
        let output = temp_path("typed.tif");
        let view = view_with_raster("", vec![1.0, 2.0, 3.0, 4.0]);
        let mut options = Options::new();
        options
            .add("filename", output.display())
            .add("data_type", "float")
            .add("gdalopts", "COMPRESS=LZW");
        let mut writer = RasterWriter::new(&options);

        writer.write(&[view]).unwrap();

        let raster = pdal_core::gdal::Raster::open(output.to_str().unwrap()).unwrap();
        assert_eq!(raster.band_type_name(1).unwrap(), "Float32");
        assert_eq!(
            raster.metadata_item_domain("COMPRESSION", "IMAGE_STRUCTURE"),
            Some("LZW".to_string())
        );
    }

    #[test]
    fn rejects_unsupported_raster_data_type() {
        let output = temp_path("bad-type.tif");
        let view = view_with_raster("", vec![1.0, 2.0, 3.0, 4.0]);
        let mut options = Options::new();
        options
            .add("filename", output.display())
            .add("data_type", "mystery");
        let mut writer = RasterWriter::new(&options);

        let err = writer.write(&[view]).unwrap_err();

        assert!(err.0.contains("Unsupported raster writer data_type"));
    }

    #[test]
    fn requested_missing_raster_is_an_error() {
        let output = temp_path("missing.tif");
        let view = view_with_raster("a", vec![1.0, 2.0, 3.0, 4.0]);
        let mut options = Options::new();
        options.add("filename", output.display());
        options.add("rasters", "missing");
        let mut writer = RasterWriter::new(&options);

        assert!(writer.write(&[view]).is_err());
    }
}
