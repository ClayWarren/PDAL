use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline, Reader};
use pdal_core::point::DimId;
use pdal_filters::decimation::DecimationFilter;
use pdal_io::pcd::{PcdReader, PcdWriter};
use pdal_io::pts::PtsReader;
use pdal_io::ptx::PtxReader;
use pdal_io::text::TextReader;
use pdal_io::text_writer::TextWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires installed pdal on PATH and reports timing only"]
fn installed_pdal_vs_rust_local_io_pipelines() {
    let iterations = std::env::var("PDAL_RUST_PERF_ITERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10)
        .max(1);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = make_temp_dir("perf-regression");
    let cases = [
        Case::Text {
            input: repo.join("test/data/text/utm17_1.txt"),
            precision: 2,
        },
        Case::Pcd {
            input: repo.join("test/data/pcd/utm17_space.pcd"),
            precision: 2,
        },
        Case::Pts {
            input: repo.join("test/data/pts/site_56_8.pts"),
            precision: 6,
        },
        Case::Ptx {
            input: repo.join("test/data/ptx/1.2-with-color.ptx"),
            precision: 6,
        },
    ];

    println!("case,installed_median_ms,rust_median_ms,ratio_rust_to_installed");
    for case in cases {
        let case_dir = temp.join(case.name());
        fs::create_dir_all(&case_dir).unwrap();

        let installed_reference = case_dir.join("installed-reference.out");
        let rust_reference = case_dir.join("rust-reference.out");
        run_installed(&case, &case_dir, &installed_reference);
        run_rust(&case, &rust_reference);
        assert_outputs_match(&case, &installed_reference, &rust_reference);

        let mut installed_times = Vec::with_capacity(iterations);
        let mut rust_times = Vec::with_capacity(iterations);
        for iteration in 0..iterations {
            let installed_output = case_dir.join(format!("installed-{iteration}.out"));
            installed_times.push(time(|| run_installed(&case, &case_dir, &installed_output)));

            let rust_output = case_dir.join(format!("rust-{iteration}.out"));
            rust_times.push(time(|| run_rust(&case, &rust_output)));
        }

        let installed = median(&mut installed_times);
        let rust = median(&mut rust_times);
        let ratio = rust.as_secs_f64() / installed.as_secs_f64();
        println!(
            "{},{:.3},{:.3},{:.2}",
            case.name(),
            millis(installed),
            millis(rust),
            ratio
        );
    }
}

enum Case {
    Text { input: PathBuf, precision: u64 },
    Pcd { input: PathBuf, precision: u64 },
    Pts { input: PathBuf, precision: u64 },
    Ptx { input: PathBuf, precision: u64 },
}

impl Case {
    fn name(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text-decimation",
            Self::Pcd { .. } => "pcd-decimation",
            Self::Pts { .. } => "pts-decimation",
            Self::Ptx { .. } => "ptx-decimation",
        }
    }

    fn input(&self) -> &Path {
        match self {
            Self::Text { input, .. }
            | Self::Pcd { input, .. }
            | Self::Pts { input, .. }
            | Self::Ptx { input, .. } => input,
        }
    }

    fn reader_name(&self) -> &'static str {
        match self {
            Self::Text { .. } => "readers.text",
            Self::Pcd { .. } => "readers.pcd",
            Self::Pts { .. } => "readers.pts",
            Self::Ptx { .. } => "readers.ptx",
        }
    }

    fn writer_name(&self) -> &'static str {
        match self {
            Self::Text { .. } => "writers.text",
            _ => "writers.pcd",
        }
    }

    fn precision(&self) -> u64 {
        match self {
            Self::Text { precision, .. }
            | Self::Pcd { precision, .. }
            | Self::Pts { precision, .. }
            | Self::Ptx { precision, .. } => *precision,
        }
    }
}

fn run_installed(case: &Case, dir: &Path, output: &Path) {
    let pipeline = dir.join(format!("{}.json", case.name()));
    fs::write(&pipeline, installed_pipeline_json(case, output)).unwrap();

    let result = Command::new("pdal")
        .arg("pipeline")
        .arg(&pipeline)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        result.status.success(),
        "installed pdal failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn installed_pipeline_json(case: &Case, output: &Path) -> String {
    let writer_options = match case {
        Case::Text { .. } => format!(
            r#""filename":"{}","order":"X,Y,Z","quote_header":false,"precision":{}"#,
            escape_json_path(output),
            case.precision()
        ),
        _ => format!(
            r#""filename":"{}","order":"X,Y,Z","precision":{}"#,
            escape_json_path(output),
            case.precision()
        ),
    };

    format!(
        r#"[
  {{"type":"{}","filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"type":"{}",{}}}
]
"#,
        case.reader_name(),
        escape_json_path(case.input()),
        case.writer_name(),
        writer_options
    )
}

fn run_rust(case: &Case, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", case.input().display());
    let mut filter_options = Options::new();
    filter_options.add("step", 2);
    let mut writer_options = Options::new();
    writer_options
        .add("filename", output.display())
        .add("order", "X,Y,Z")
        .add("precision", case.precision());

    let mut pipeline = Pipeline::new();
    let reader = match case {
        Case::Text { .. } => pipeline.add_reader(
            case.reader_name(),
            Box::new(TextReader::new(&reader_options)),
            reader_options,
        ),
        Case::Pcd { .. } => pipeline.add_reader(
            case.reader_name(),
            Box::new(PcdReader::new(&reader_options)),
            reader_options,
        ),
        Case::Pts { .. } => pipeline.add_reader(
            case.reader_name(),
            Box::new(PtsReader::new(&reader_options)),
            reader_options,
        ),
        Case::Ptx { .. } => pipeline.add_reader(
            case.reader_name(),
            Box::new(PtxReader::new(&reader_options)),
            reader_options,
        ),
    };
    let filter = pipeline.add_stage(
        "filters.decimation",
        Box::new(FilterWrapper::new(DecimationFilter::new(&filter_options))),
        filter_options,
    );

    if matches!(case, Case::Text { .. }) {
        writer_options.add("quote_header", false);
    }
    let writer = match case {
        Case::Text { .. } => pipeline.add_writer(
            case.writer_name(),
            Box::new(TextWriter::new(&writer_options)),
            writer_options,
        ),
        _ => pipeline.add_writer(
            case.writer_name(),
            Box::new(PcdWriter::new(&writer_options)),
            writer_options,
        ),
    };

    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

fn assert_outputs_match(case: &Case, installed: &Path, rust: &Path) {
    if matches!(case, Case::Text { .. }) {
        assert_eq!(
            fs::read_to_string(installed).unwrap(),
            fs::read_to_string(rust).unwrap()
        );
        return;
    }

    let installed = read_pcd(installed);
    let rust = read_pcd(rust);
    assert_eq!(rust.len(), installed.len());
    for point in 0..rust.len() {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            assert_eq!(rust.get_f64(point, &dim), installed.get_f64(point, &dim));
        }
    }
}

fn read_pcd(path: &Path) -> pdal_core::point::PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PcdReader::new(&options).read().unwrap().pop().unwrap()
}

fn time(run: impl FnOnce()) -> Duration {
    let start = Instant::now();
    run();
    start.elapsed()
}

fn median(values: &mut [Duration]) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
