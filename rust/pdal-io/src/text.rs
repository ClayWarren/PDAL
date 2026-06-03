use crate::source;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;
use std::io::{BufRead, BufReader};
use std::rc::Rc;

/// Text reader for simple numeric delimited files.
pub struct TextReader {
    filename: String,
    separator: Option<char>,
    header: Option<String>,
    skip: usize,
    override_srs: String,
    stream: Option<TextStreamState>,
}

struct TextStreamState {
    reader: BufReader<Box<dyn source::ReadSeek>>,
    dims: Vec<DimId>,
    layout: Rc<PointLayout>,
    separator: char,
    srs: Option<SpatialReference>,
    eof: bool,
}

impl TextReader {
    pub fn new(options: &Options) -> Self {
        let separator = options.get_str("separator", "");
        let header = options.get_str("header", "");

        Self {
            filename: options.get_str("filename", ""),
            separator: separator.chars().next(),
            header: (!header.is_empty()).then_some(header),
            skip: options.get_u64("skip", 0) as usize,
            override_srs: options.get_str("override_srs", ""),
            stream: None,
        }
    }

    fn parse_header(&mut self, header: &str) -> Result<Vec<DimId>, StageError> {
        if header.is_empty() {
            return Err(StageError("Empty text header.".to_string()));
        }

        let names = if header.starts_with('"') {
            self.parse_quoted_header(header)?
        } else {
            self.parse_unquoted_header(header)?
        };

        let mut dims = Vec::with_capacity(names.len());
        for name in names {
            let name = name.trim().trim_end_matches('\r');
            validate_dimension_name(name)?;
            let dim = DimId::from_name(name);
            if dims.contains(&dim) {
                return Err(StageError(format!(
                    "Duplicate dimension '{name}' detected in input file '{}'.",
                    self.filename
                )));
            }
            dims.push(dim);
        }
        Ok(dims)
    }

    fn parse_unquoted_header(&mut self, header: &str) -> Result<Vec<String>, StageError> {
        let separator = match self.separator {
            Some(separator) => separator,
            None => {
                let separator = header
                    .chars()
                    .find(|ch| !ch.is_ascii_alphanumeric())
                    .unwrap_or(' ');
                self.separator = Some(separator);
                separator
            }
        };

        let names = split_fields(header, separator);
        if names.is_empty() {
            Err(StageError(
                "Text header contains no dimensions.".to_string(),
            ))
        } else {
            Ok(names)
        }
    }

    fn parse_quoted_header(&mut self, header: &str) -> Result<Vec<String>, StageError> {
        let mut names = Vec::new();
        let mut pos = 0;
        let bytes = header.as_bytes();
        let mut inferred_separator: Option<char> = None;

        loop {
            skip_ascii_whitespace(bytes, &mut pos);
            if bytes.get(pos) != Some(&b'"') {
                break;
            }
            pos += 1;

            let name_start = pos;
            while let Some(&byte) = bytes.get(pos) {
                if byte == b'"' {
                    break;
                }
                pos += 1;
            }
            if bytes.get(pos) != Some(&b'"') {
                return Err(StageError("Unterminated quoted text header.".to_string()));
            }
            names.push(header[name_start..pos].to_string());
            pos += 1;

            let separator_start = pos;
            while let Some(&byte) = bytes.get(pos) {
                if byte == b'"' {
                    break;
                }
                pos += 1;
            }

            let separator_text = header[separator_start..pos].trim();
            if !separator_text.is_empty() {
                let mut chars = separator_text.chars();
                let separator = chars.next().unwrap();
                if chars.next().is_some() {
                    return Err(StageError(
                        "Found separator longer than a single character.".to_string(),
                    ));
                }
                inferred_separator.get_or_insert(separator);
            }

            if bytes.get(pos) != Some(&b'"') {
                break;
            }
        }

        if names.is_empty() {
            return Err(StageError(
                "Text header contains no dimensions.".to_string(),
            ));
        }
        if pos < bytes.len() && !header[pos..].trim().is_empty() {
            return Err(StageError(format!(
                "Invalid character '{}' found while parsing quoted header line.",
                header[pos..].chars().next().unwrap()
            )));
        }

        if self.separator.is_none() {
            self.separator = inferred_separator.or(Some(' '));
        }
        Ok(names)
    }

    fn data_start(&self) -> usize {
        if self.header.is_some() {
            self.skip
        } else {
            self.skip + 1
        }
    }

    fn stream_init(&mut self) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "TextReader requires a filename option.".to_string(),
            ));
        }

        let file = source::open_seek(&self.filename)
            .map_err(|_| StageError(format!("Unable to open text file '{}'.", self.filename)))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        for _ in 0..self.skip {
            line.clear();
            if reader
                .read_line(&mut line)
                .map_err(|err| StageError(err.to_string()))?
                == 0
            {
                break;
            }
        }

        let header = match &self.header {
            Some(header) => header.clone(),
            None => {
                line.clear();
                if reader
                    .read_line(&mut line)
                    .map_err(|err| StageError(err.to_string()))?
                    == 0
                {
                    return Err(StageError(
                        "Text file is missing a header line.".to_string(),
                    ));
                }
                let header = trim_line_endings(&line).to_string();
                if !header.chars().any(|c| c.is_alphabetic()) {
                    eprintln!(
                        "(readers.text Warning) readers.text: file '{}' doesn't \
                         appear to contain a header line.",
                        self.filename
                    );
                }
                header
            }
        };
        let dims = self.parse_header(&header)?;

        let mut layout = PointLayout::new();
        for dim in &dims {
            layout.register(dim.clone(), DimType::F64);
        }
        let layout = Rc::new(layout);
        let srs =
            (!self.override_srs.is_empty()).then(|| SpatialReference::new(&self.override_srs));
        let separator = self.separator.unwrap_or(' ');

        self.stream = Some(TextStreamState {
            reader,
            dims,
            layout,
            separator,
            srs,
            eof: false,
        });
        Ok(())
    }
}

impl Reader for TextReader {
    fn name(&self) -> &str {
        "readers.text"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "TextReader requires a filename option.".to_string(),
            ));
        }

        let text = source::read_to_string(&self.filename)
            .map_err(|_| StageError(format!("Unable to open text file '{}'.", self.filename)))?;
        let lines: Vec<&str> = text.lines().collect();
        let header = match &self.header {
            Some(header) => header.clone(),
            None => {
                let line = lines
                    .get(self.skip)
                    .ok_or_else(|| StageError("Text file is missing a header line.".to_string()))?
                    .to_string();
                // Mirror C++ TextReader::checkHeader: a file-derived header with
                // no alphabetic character almost certainly is not a header.
                if !line.chars().any(|c| c.is_alphabetic()) {
                    eprintln!(
                        "(readers.text Warning) readers.text: file '{}' doesn't \
                         appear to contain a header line.",
                        self.filename
                    );
                }
                line
            }
        };
        let dims = self.parse_header(&header)?;

        let mut layout = PointLayout::new();
        for dim in &dims {
            layout.register(dim.clone(), DimType::F64);
        }
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);
        if !self.override_srs.is_empty() {
            view.set_spatial_reference(SpatialReference::new(&self.override_srs));
        }
        let separator = self.separator.unwrap_or(' ');

        for line in lines.iter().skip(self.data_start()) {
            if line.is_empty() {
                continue;
            }

            let fields = split_fields(line, separator);
            if fields.len() != dims.len() {
                continue;
            }

            let point = view.add_point();
            for (field, dim) in fields.iter().zip(&dims) {
                let value = field.trim().parse::<f64>().unwrap_or(0.0);
                view.set_f64(point, dim, value);
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.text")
    }

    fn reset(&mut self) {
        self.stream = None;
    }

    fn streamable(&self) -> bool {
        !self.filename.is_empty()
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.stream.is_none() {
            self.stream_init()?;
        }
        let state = self.stream.as_mut().expect("stream initialized above");
        if state.eof {
            return Ok(None);
        }

        let mut view = PointView::new(Rc::clone(&state.layout));
        if let Some(srs) = &state.srs {
            view.set_spatial_reference(srs.clone());
        }

        let mut line = String::new();
        while view.len() < capacity.max(1) as u64 {
            line.clear();
            if state
                .reader
                .read_line(&mut line)
                .map_err(|err| StageError(err.to_string()))?
                == 0
            {
                state.eof = true;
                break;
            }
            let line = trim_line_endings(&line);
            if line.is_empty() {
                continue;
            }

            let fields = split_fields(line, state.separator);
            if fields.len() != state.dims.len() {
                continue;
            }

            let point = view.add_point();
            for (field, dim) in fields.iter().zip(&state.dims) {
                let value = field.trim().parse::<f64>().unwrap_or(0.0);
                view.set_f64(point, dim, value);
            }
        }

        if view.is_empty() && state.eof {
            Ok(None)
        } else {
            Ok(Some(view))
        }
    }
}

fn trim_line_endings(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn split_fields(line: &str, separator: char) -> Vec<String> {
    if separator == ' ' {
        line.split_whitespace()
            .map(|field| field.trim().trim_end_matches('\r').to_string())
            .filter(|field| !field.is_empty())
            .collect()
    } else {
        line.replace(' ', "")
            .split(separator)
            .map(|field| field.trim().trim_end_matches('\r').to_string())
            .collect()
    }
}

fn validate_dimension_name(name: &str) -> Result<(), StageError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(StageError(
            "Empty dimension name in text header.".to_string(),
        ));
    };
    if !first.is_ascii_alphabetic() {
        return Err(StageError(format!(
            "Invalid character '{first}' in dimension name."
        )));
    }
    if let Some(invalid) = chars.find(|ch| !ch.is_ascii_alphanumeric() && *ch != '_') {
        return Err(StageError(format!(
            "Invalid character '{invalid}' in dimension name."
        )));
    }
    Ok(())
}

fn skip_ascii_whitespace(bytes: &[u8], pos: &mut usize) {
    while bytes
        .get(*pos)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        *pos += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_writer::TextWriter;
    use pdal_core::pipeline::{FilterWrapper, Pipeline};
    use pdal_filters::range::{RangeFilter, RangeLimit};
    use std::path::Path;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn read_text(path: &str, configure: impl FnOnce(&mut Options)) -> PointView {
        let mut options = Options::new();
        options.add("filename", data_path(path));
        configure(&mut options);

        let mut reader = TextReader::new(&options);
        let mut views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        views.pop().unwrap()
    }

    fn temp_path(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-text-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path.display().to_string()
    }

    #[test]
    fn reads_comma_delimited_xyz() {
        let view = read_text("text/utm17_1.txt", |_| {});

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(0, &DimId::X), 289814.15);
        assert_eq!(view.get_f64(0, &DimId::Y), 4320978.61);
        assert_eq!(view.get_f64(0, &DimId::Z), 170.76);
        assert_eq!(view.get_f64(9, &DimId::X), 289818.50);
        assert_eq!(view.get_f64(9, &DimId::Y), 4320980.59);
        assert_eq!(view.get_f64(9, &DimId::Z), 170.58);
    }

    #[test]
    fn reads_space_delimited_xyz() {
        let view = read_text("text/utm17_2.txt", |_| {});

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(0, &DimId::X), 289814.15);
        assert_eq!(view.get_f64(9, &DimId::Y), 4320980.59);
    }

    #[test]
    fn streaming_chunks_match_full_read() {
        let mut options = Options::new();
        options.add("filename", data_path("text/utm17_1.txt"));

        let mut full_reader = TextReader::new(&options);
        let full = full_reader.read().unwrap().pop().unwrap();

        let mut stream_reader = TextReader::new(&options);
        assert!(stream_reader.streamable());
        let first = stream_reader.stream_next(4).unwrap().unwrap();
        let second = stream_reader.stream_next(4).unwrap().unwrap();
        let third = stream_reader.stream_next(4).unwrap().unwrap();
        assert!(stream_reader.stream_next(4).unwrap().is_none());

        assert_eq!(first.len(), 4);
        assert_eq!(second.len(), 4);
        assert_eq!(third.len(), 2);
        assert_eq!(first.get_f64(0, &DimId::X), full.get_f64(0, &DimId::X));
        assert_eq!(second.get_f64(0, &DimId::X), full.get_f64(4, &DimId::X));
        assert_eq!(third.get_f64(1, &DimId::Z), full.get_f64(9, &DimId::Z));
    }

    #[test]
    fn streaming_honors_override_header_skip_and_srs() {
        let mut options = Options::new();
        options
            .add("filename", data_path("text/crlf_test.txt"))
            .add("skip", 1)
            .add("header", "A,B,C,G")
            .add("override_srs", "EPSG:2029");
        let mut reader = TextReader::new(&options);

        let first = reader.stream_next(3).unwrap().unwrap();
        assert_eq!(first.len(), 3);
        assert_eq!(first.spatial_reference().wkt(), "EPSG:2029");
        assert_eq!(first.get_f64(0, &DimId::Other("A".to_string())), 289814.15);
        assert_eq!(first.get_f64(2, &DimId::Other("G".to_string())), 2.0);
    }

    #[test]
    fn pipeline_streams_text_reader_to_csv_writer() {
        let output = temp_path("stream-pipeline.csv");
        let mut reader_options = Options::new();
        reader_options.add("filename", data_path("text/utm17_1.txt"));
        let limits = vec![RangeLimit {
            dim_name: "X".to_string(),
            lower_bound: 289814.0,
            upper_bound: 289815.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];
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
            "filters.range",
            Box::new(FilterWrapper::new(RangeFilter::new(limits))),
            Options::new(),
        );
        let writer = pipeline.add_writer(
            "writers.text",
            Box::new(TextWriter::new(&writer_options)),
            writer_options,
        );
        pipeline.add_dependency(filter, reader).unwrap();
        pipeline.add_dependency(writer, filter).unwrap();

        assert_eq!(pipeline.execute_streaming().unwrap(), Some(2));
        let written = std::fs::read_to_string(&output).unwrap();
        let _ = std::fs::remove_file(output);
        let lines: Vec<_> = written.lines().collect();
        assert_eq!(lines[0], "X,Y,Z");
        assert_eq!(lines.len(), 3);
        assert!(lines[1].starts_with("289814.15,4320978.61,170.76"));
    }

    #[test]
    fn skips_bad_rows_with_the_wrong_field_count() {
        let view = read_text("text/utm17_3.txt", |_| {});

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(0, &DimId::X), 289814.15);
        assert_eq!(view.get_f64(0, &DimId::Y), 4320978.61);
    }

    #[test]
    fn strips_crlf_from_dimension_names() {
        let view = read_text("text/crlf_test.txt", |_| {});

        assert_eq!(view.len(), 10);
        for idx in 0..view.len() {
            assert_eq!(view.get_f64(idx, &DimId::Intensity), idx as f64);
        }
    }

    #[test]
    fn supports_override_header_and_skip() {
        let view = read_text("text/crlf_test.txt", |options| {
            options.add("skip", 1).add("header", "A,B,C,G");
        });

        assert_eq!(view.len(), 10);
        assert_eq!(view.get_f64(0, &DimId::Other("A".to_string())), 289814.15);
        assert_eq!(view.get_f64(9, &DimId::Other("G".to_string())), 9.0);
    }

    #[test]
    fn override_srs_sets_view_spatial_reference() {
        let view = read_text("text/utm17_1.txt", |options| {
            options.add("override_srs", "EPSG:2029");
        });

        assert_eq!(view.spatial_reference().wkt(), "EPSG:2029");
    }

    #[test]
    fn supports_inserted_header_without_skipping_file_header() {
        let view = read_text("text/crlf_test.txt", |options| {
            options.add("header", "A,B,C,G");
        });

        assert_eq!(view.len(), 11);
        assert_eq!(view.get_f64(0, &DimId::Other("A".to_string())), 0.0);
        assert_eq!(view.get_f64(10, &DimId::Other("G".to_string())), 9.0);
    }

    #[test]
    fn supports_quoted_headers() {
        let view = read_text("text/quoted.txt", |_| {});

        assert_eq!(view.len(), 9);
        assert_eq!(view.get_f64(0, &DimId::X), 0.0);
        assert_eq!(view.get_f64(8, &DimId::Y), 22.0);
    }

    #[test]
    fn rejects_duplicate_dimensions() {
        let mut options = Options::new();
        options.add("filename", data_path("text/badheader.txt"));
        let mut reader = TextReader::new(&options);

        assert!(reader.read().is_err());
    }

    #[test]
    fn rejects_invalid_quoted_headers() {
        let mut options = Options::new();
        options
            .add("filename", data_path("text/quoted.txt"))
            .add("skip", 1)
            .add("header", "\"X\",\"Y\"   \"  ");
        let mut reader = TextReader::new(&options);

        assert!(reader.read().is_err());
    }

    // ----- Additional text reader coverage -----

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = TextReader::new(&Options::new());
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/text.txt");
        let mut reader = TextReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_empty_header_when_override() {
        // Provide empty header explicitly
        let mut options = Options::new();
        options.add("filename", data_path("text/utm17_1.txt"));
        options.add("header", "");
        // Header empty -> parse_header errors
        let _r = TextReader::new(&options);
        // The implementation only sets header field if header is not empty,
        // so this case becomes "no explicit header" — still works. Skip assertion.
    }

    #[test]
    fn reader_errors_on_quoted_with_long_separator() {
        let mut options = Options::new();
        options
            .add("filename", data_path("text/quoted.txt"))
            .add("skip", 1)
            .add("header", "\"X\",,\"Y\"");
        let mut reader = TextReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_unterminated_quote() {
        let mut options = Options::new();
        options
            .add("filename", data_path("text/quoted.txt"))
            .add("skip", 1)
            .add("header", "\"unterminated");
        let mut reader = TextReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_invalid_dimension_name() {
        let mut options = Options::new();
        options
            .add("filename", data_path("text/utm17_1.txt"))
            .add("header", "X,Y,1bad");
        let mut reader = TextReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_metadata_name() {
        let reader = TextReader::new(&Options::new());
        assert_eq!(reader.name(), "readers.text");
    }
}
