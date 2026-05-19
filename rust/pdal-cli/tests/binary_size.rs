use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const STARTUP_ITERATIONS: usize = 20;

#[test]
#[ignore = "requires installed pdal on PATH and reports binary sizes only"]
fn installed_pdal_vs_rust_cli_binary_size() {
    let installed = installed_pdal_path();
    let rust = Path::new(env!("CARGO_BIN_EXE_pdal-rs")).to_path_buf();

    let installed_size = file_size(&installed);
    let rust_size = file_size(&rust);

    println!("binary,path,size_bytes,size_mib");
    println!(
        "installed_pdal,{}, {}, {:.3}",
        installed.display(),
        installed_size,
        mib(installed_size)
    );
    println!(
        "pdal_rs,{}, {}, {:.3}",
        rust.display(),
        rust_size,
        mib(rust_size)
    );
    println!(
        "ratio_rust_to_installed,{:.3}",
        rust_size as f64 / installed_size as f64
    );
}

#[test]
#[ignore = "requires installed pdal on PATH and reports startup timings only"]
fn installed_pdal_vs_rust_cli_startup_time() {
    let installed = installed_pdal_path();
    let rust = Path::new(env!("CARGO_BIN_EXE_pdal-rs")).to_path_buf();

    let mut installed_times = Vec::with_capacity(STARTUP_ITERATIONS);
    let mut rust_times = Vec::with_capacity(STARTUP_ITERATIONS);

    run_version(&installed);
    run_version(&rust);
    for _ in 0..STARTUP_ITERATIONS {
        installed_times.push(time(|| run_version(&installed)));
        rust_times.push(time(|| run_version(&rust)));
    }

    let installed_median = median(&mut installed_times);
    let rust_median = median(&mut rust_times);

    println!("binary,path,median_startup_ms");
    println!(
        "installed_pdal,{}, {:.3}",
        installed.display(),
        millis(installed_median)
    );
    println!("pdal_rs,{}, {:.3}", rust.display(), millis(rust_median));
    println!(
        "ratio_rust_to_installed,{:.3}",
        rust_median.as_secs_f64() / installed_median.as_secs_f64()
    );
}

fn installed_pdal_path() -> PathBuf {
    let output = Command::new("which")
        .arg("pdal")
        .output()
        .expect("failed to locate installed pdal with which");
    assert!(
        output.status.success(),
        "installed pdal not found on PATH\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path)
        .unwrap_or_else(|err| panic!("failed to stat {}: {err}", path.display()))
        .len()
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn run_version(binary: &Path) {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .unwrap_or_else(|err| panic!("failed to execute {}: {err}", binary.display()));
    assert!(
        output.status.success(),
        "{} --version failed\nstdout:\n{}\nstderr:\n{}",
        binary.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
