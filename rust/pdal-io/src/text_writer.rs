use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputType {
    Csv,
    GeoJson,
}

#[derive(Clone, Debug)]
struct DimSpec {
    id: DimId,
    precision: usize,
    name: String,
}

/// Text writer for CSV and simple GeoJSON point output.
pub struct TextWriter {
    filename: String,
    output_type: OutputType,
    callback: String,
    write_all_dims: bool,
    dim_order: String,
    write_header: bool,
    newline: String,
    delimiter: String,
    quote_header: bool,
    precision: usize,
    point_count: u64,
}

impl TextWriter {
    pub fn new(options: &Options) -> Self {
        let output_type = match options.get_str("format", "csv").to_lowercase().as_str() {
            "geojson" => OutputType::GeoJson,
            _ => OutputType::Csv,
        };

        Self {
            filename: options.get_str("filename", ""),
            output_type,
            callback: options.get_str("jscallback", ""),
            write_all_dims: options.get_bool("keep_unspecified", true),
            dim_order: options.get_str("order", ""),
            write_header: options.get_bool("write_header", true),
            newline: options.get_str("newline", "\n"),
            delimiter: options.get_str("delimiter", ","),
            quote_header: options.get_bool("quote_header", true),
            precision: options.get_u64("precision", 3) as usize,
            point_count: 0,
        }
    }

    fn dimension_specs(&self, layout: &PointLayout) -> Result<Vec<DimSpec>, StageError> {
        let mut specs = Vec::new();
        for item in self
            .dim_order
            .split(',')
            .filter(|item| !item.trim().is_empty())
        {
            specs.push(self.extract_dim(item, layout)?);
        }

        if self.dim_order.trim().is_empty() || self.write_all_dims {
            for idx in 0..layout.dim_count() {
                let Some((id, _ty)) = layout.dim_at(idx) else {
                    continue;
                };
                if !specs.iter().any(|spec| spec.id == *id) {
                    specs.push(DimSpec {
                        id: id.clone(),
                        precision: self.precision,
                        name: id.name().to_string(),
                    });
                }
            }
        }

        Ok(specs)
    }

    fn extract_dim(&self, text: &str, layout: &PointLayout) -> Result<DimSpec, StageError> {
        let mut parts = text.trim().split(':');
        let name = parts.next().unwrap_or("").trim();
        let precision = match parts.next() {
            Some(value) => value.parse::<usize>().map_err(|_| {
                StageError(format!("Can't convert dimension precision for '{text}'."))
            })?,
            None => self.precision,
        };
        if parts.next().is_some() {
            return Err(StageError(format!(
                "Invalid dimension specification '{text}'."
            )));
        }

        let id = DimId::from_name(name);
        if layout.dim(&id).is_none() {
            return Err(StageError(format!(
                "Dimension not found with name '{text}'."
            )));
        }

        Ok(DimSpec {
            id,
            precision,
            name: name.to_string(),
        })
    }

    fn write_csv(&mut self, views: &[PointView]) -> Result<String, StageError> {
        let Some(first) = views.first() else {
            return Ok(String::new());
        };
        let specs = self.dimension_specs(first.layout())?;
        let mut output = String::new();

        if self.write_header {
            for (idx, spec) in specs.iter().enumerate() {
                if idx > 0 {
                    output.push_str(&self.delimiter);
                }
                if self.quote_header {
                    output.push('"');
                    output.push_str(&spec.name);
                    output.push('"');
                } else {
                    output.push_str(&spec.name);
                }
            }
            output.push_str(&self.newline);
        }

        for view in views {
            for point in 0..view.len() {
                self.point_count += 1;
                for (idx, spec) in specs.iter().enumerate() {
                    if idx > 0 {
                        output.push_str(&self.delimiter);
                    }
                    output.push_str(&format_number(
                        view.get_f64(point, &spec.id),
                        spec.precision,
                    ));
                }
                output.push_str(&self.newline);
            }
        }

        Ok(output)
    }

    fn write_geojson(&mut self, views: &[PointView]) -> Result<String, StageError> {
        let Some(first) = views.first() else {
            return Ok(String::from(
                "{ \"type\": \"FeatureCollection\", \"features\": []}",
            ));
        };
        let specs = self.dimension_specs(first.layout())?;
        let x_dim = dim_spec_or_default(&specs, DimId::X, self.precision);
        let y_dim = dim_spec_or_default(&specs, DimId::Y, self.precision);
        let z_dim = dim_spec_or_default(&specs, DimId::Z, self.precision);

        let mut output = String::new();
        if !self.callback.is_empty() {
            output.push_str(&self.callback);
            output.push('(');
        }
        output.push_str("{ \"type\": \"FeatureCollection\", \"features\": [");

        let mut first_feature = true;
        for view in views {
            for point in 0..view.len() {
                self.point_count += 1;
                if !first_feature {
                    output.push(',');
                }
                first_feature = false;

                output.push_str(
                    "{ \"type\":\"Feature\",\"geometry\": { \"type\": \"Point\", \"coordinates\": [",
                );
                output.push_str(&format_number(
                    view.get_f64(point, &DimId::X),
                    x_dim.precision,
                ));
                output.push(',');
                output.push_str(&format_number(
                    view.get_f64(point, &DimId::Y),
                    y_dim.precision,
                ));
                output.push(',');
                output.push_str(&format_number(
                    view.get_f64(point, &DimId::Z),
                    z_dim.precision,
                ));
                output.push_str("]},\"properties\": {");

                for (idx, spec) in specs.iter().enumerate() {
                    if idx > 0 {
                        output.push(',');
                    }
                    output.push('"');
                    output.push_str(&spec.name);
                    output.push_str("\":\"");
                    output.push_str(&format_number(
                        view.get_f64(point, &spec.id),
                        spec.precision,
                    ));
                    output.push('"');
                }
                output.push_str("}}");
            }
        }

        output.push_str("]}");
        if !self.callback.is_empty() {
            output.push(')');
        }
        Ok(output)
    }
}

impl Writer for TextWriter {
    fn name(&self) -> &str {
        "writers.text"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "TextWriter requires a filename option.".to_string(),
            ));
        }

        let output = match self.output_type {
            OutputType::Csv => self.write_csv(views)?,
            OutputType::GeoJson => self.write_geojson(views)?,
        };

        fs::write(Path::new(&self.filename), output)
            .map_err(|_| StageError(format!("Couldn't open '{}' for output.", self.filename)))
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.text");
        node.add_value("filename", MetadataValue::String(self.filename.clone()));
        node.add_value("point_count", MetadataValue::U64(self.point_count));
        node
    }
}

fn dim_spec_or_default(specs: &[DimSpec], id: DimId, precision: usize) -> DimSpec {
    specs
        .iter()
        .find(|spec| spec.id == id)
        .cloned()
        .unwrap_or_else(|| DimSpec {
            name: id.name().to_string(),
            id,
            precision,
        })
}

fn format_number(value: f64, precision: usize) -> String {
    format!("{value:.precision$}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TextReader;
    use pdal_core::options::Options;
    use pdal_core::pipeline::{FilterWrapper, Pipeline, Reader};
    use pdal_core::point::{DimType, PointLayout, PointView};
    use pdal_filters::decimation::DecimationFilter;
    use std::rc::Rc;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn temp_path(name: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "pdal-rust-text-writer-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        path.display().to_string()
    }

    fn make_precision_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for values in [
            [1.0, 1.0, 1.0, 1.0],
            [
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_222_2,
                2.222_222_22,
            ],
            [3.33, 3.33, 3.33, 3.33],
        ] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, values[0]);
            view.set_f64(point, &DimId::Y, values[1]);
            view.set_f64(point, &DimId::Z, values[2]);
            view.set_f64(point, &DimId::Intensity, values[3]);
        }

        view
    }

    fn write_view(name: &str, view: &PointView, configure: impl FnOnce(&mut Options)) -> String {
        let filename = temp_path(name);
        let mut options = Options::new();
        options.add("filename", &filename);
        configure(&mut options);

        let mut writer = TextWriter::new(&options);
        writer.write(std::slice::from_ref(view)).unwrap();
        fs::read_to_string(filename).unwrap()
    }

    #[test]
    fn writes_csv_matching_existing_comma_fixture() {
        let mut options = Options::new();
        options.add("filename", data_path("text/utm17_1.txt"));
        let mut reader = TextReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let output = write_view("comma.csv", &view, |options| {
            options
                .add("order", "X,Y,Z")
                .add("quote_header", false)
                .add("precision", 2);
        });

        assert_eq!(
            output,
            fs::read_to_string(data_path("text/utm17_1.txt")).unwrap()
        );
    }

    #[test]
    fn writes_csv_matching_existing_space_fixture() {
        let mut options = Options::new();
        options.add("filename", data_path("text/utm17_2.txt"));
        let mut reader = TextReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let output = write_view("space.csv", &view, |options| {
            options
                .add("order", "X,Y,Z")
                .add("quote_header", false)
                .add("precision", 2)
                .add("delimiter", "  ");
        });

        assert_eq!(
            output,
            fs::read_to_string(data_path("text/utm17_2.txt")).unwrap()
        );
    }

    #[test]
    fn per_dimension_precision_matches_existing_behavior() {
        let output = write_view("precision.csv", &make_precision_view(), |options| {
            options
                .add("precision", 5)
                .add("order", "X:0,Y:0,Z:0,Intensity:0");
        });

        assert!(output.contains("1,1,1,1"));
        assert!(output.contains("2,2,2,2"));
        assert!(output.contains("3,3,3,3"));
    }

    #[test]
    fn writes_geojson_feature_collection() {
        let mut options = Options::new();
        options.add("filename", data_path("text/utm17_1.txt"));
        let mut reader = TextReader::new(&options);
        let view = reader.read().unwrap().pop().unwrap();

        let output = write_view("points.geojson", &view, |options| {
            options
                .add("format", "geojson")
                .add("order", "X,Y,Z")
                .add("precision", 2);
        });

        assert!(output.starts_with("{ \"type\": \"FeatureCollection\""));
        assert_eq!(output.matches("\"type\":\"Feature\"").count(), 10);
        assert!(output.contains("\"coordinates\": [289814.15,4320978.61,170.76]"));
    }

    #[test]
    fn reader_filter_writer_pipeline_writes_expected_text() {
        let input = data_path("text/utm17_1.txt");
        let output = temp_path("pipeline.txt");

        let mut reader_options = Options::new();
        reader_options.add("filename", input);
        let mut filter_options = Options::new();
        filter_options.add("step", 2);
        let mut writer_options = Options::new();
        writer_options
            .add("filename", &output)
            .add("order", "X,Y,Z")
            .add("quote_header", false)
            .add("precision", 2);

        let mut pipeline = Pipeline::new();
        let reader = pipeline.add_reader(
            "readers.text",
            Box::new(TextReader::new(&reader_options)),
            reader_options,
        );
        let filter = pipeline.add_stage(
            "filters.decimation",
            Box::new(FilterWrapper::new(DecimationFilter::new(&filter_options))),
            filter_options,
        );
        let writer = pipeline.add_writer(
            "writers.text",
            Box::new(TextWriter::new(&writer_options)),
            writer_options,
        );
        pipeline.add_dependency(filter, reader).unwrap();
        pipeline.add_dependency(writer, filter).unwrap();

        let result = pipeline.execute(Vec::new()).unwrap();
        assert!(result.is_empty());

        let written = fs::read_to_string(output).unwrap();
        let lines: Vec<_> = written.lines().collect();
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0], "X,Y,Z");
        assert_eq!(lines[1], "289814.15,4320978.61,170.76");
        assert_eq!(lines[5], "289818.01,4320980.38,170.61");
    }

    #[test]
    fn writer_errors_without_filename() {
        let mut writer = TextWriter::new(&Options::new());
        let view = make_precision_view();
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn writer_errors_on_bad_output_directory() {
        let mut options = Options::new();
        options.add("filename", "/no/such/dir/out.csv");
        let mut writer = TextWriter::new(&options);
        let view = make_precision_view();
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn writer_writes_geojson_with_empty_views() {
        let path = temp_path("empty-geojson.json");
        let mut options = Options::new();
        options.add("filename", &path).add("format", "geojson");
        let mut writer = TextWriter::new(&options);
        // Pass no views -> early-return ""
        writer.write(&[]).unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn writer_writes_geojson_with_jsonp_callback() {
        let path = temp_path("jsonp.json");
        let mut options = Options::new();
        options
            .add("filename", &path)
            .add("format", "geojson")
            .add("jscallback", "myCb");
        let mut writer = TextWriter::new(&options);
        let view = make_precision_view();
        writer.write(&[view]).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        assert!(written.starts_with("myCb("));
        assert!(written.ends_with(")"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn extract_dim_errors_on_bad_precision() {
        let mut options = Options::new();
        options
            .add("filename", "/tmp/x.csv")
            .add("order", "X:notanumber");
        let writer = TextWriter::new(&options);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let r = writer.dimension_specs(&layout);
        assert!(r.is_err());
    }

    #[test]
    fn extract_dim_errors_on_too_many_colons() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.csv").add("order", "X:2:3");
        let writer = TextWriter::new(&options);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let r = writer.dimension_specs(&layout);
        assert!(r.is_err());
    }

    #[test]
    fn extract_dim_errors_on_unknown_dim() {
        let mut options = Options::new();
        options
            .add("filename", "/tmp/x.csv")
            .add("order", "NotADim");
        let writer = TextWriter::new(&options);
        let layout = PointLayout::new();
        let r = writer.dimension_specs(&layout);
        assert!(r.is_err());
    }

    #[test]
    fn writer_name_is_writers_text() {
        let writer = TextWriter::new(&Options::new());
        assert_eq!(writer.name(), "writers.text");
    }

    #[test]
    fn writer_writes_csv_with_empty_views() {
        let path = temp_path("empty-csv.csv");
        let mut options = Options::new();
        options.add("filename", &path);
        let mut writer = TextWriter::new(&options);
        writer.write(&[]).unwrap();
        let _ = std::fs::remove_file(&path);
    }
}
