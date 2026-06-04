use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn simple_las() -> &'static str {
    "../../test/data/las/simple.las"
}

fn temp_file(name: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{name}-{stamp}.txt"))
}

#[test]
fn lasdump_writes_cpp_compatible_output_to_stdout() {
    let expected = pdal_io::lasdump::dump_las(std::path::Path::new(simple_las())).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["lasdump", simple_las()])
        .output()
        .expect("failed to execute pdal-rs");

    assert!(
        result.status.success(),
        "pdal-rs lasdump failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&result.stdout), expected);
    assert!(result.stderr.is_empty());
}

#[test]
#[ignore = "requires installed lasdump on PATH"]
fn installed_lasdump_matches_rust_lasdump() {
    let installed = Command::new("lasdump")
        .arg(simple_las())
        .output()
        .expect("failed to execute installed lasdump");
    assert!(
        installed.status.success(),
        "installed lasdump failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["lasdump", simple_las()])
        .output()
        .expect("failed to execute pdal-rs");
    assert!(
        rust.status.success(),
        "pdal-rs lasdump failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&installed.stdout)
    );
}

#[test]
fn lasdump_writes_output_file() {
    let output = temp_file("pdal-rs-lasdump");
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["lasdump", "-o", output.to_str().unwrap(), simple_las()])
        .output()
        .expect("failed to execute pdal-rs");

    assert!(
        result.status.success(),
        "pdal-rs lasdump failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    let text = std::fs::read_to_string(&output).unwrap();
    assert!(text.contains("File version: 1.2\n"));
    std::fs::remove_file(output).ok();
}

#[test]
fn lasdump_reports_usage_for_bad_arguments() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("lasdump")
        .output()
        .expect("failed to execute pdal-rs");

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("Usage: lasdump [-o <output filename>] <las/las file>"));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["lasdump", "--bogus", simple_las()])
        .output()
        .expect("failed to execute pdal-rs");

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("Usage: lasdump [-o <output filename>] <las/las file>"));
}

#[test]
fn lasdump_reports_file_errors() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["lasdump", "../../test/data/las/mvk-thin.las.wkt"])
        .output()
        .expect("failed to execute pdal-rs");

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("Not a LAS/LAZ file.  Invalid file signature."));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["lasdump", "-o", "/", simple_las()])
        .output()
        .expect("failed to execute pdal-rs");

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Couldn't open output file."));
}
