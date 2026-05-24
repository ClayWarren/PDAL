use std::process::Command;

fn bin() -> String {
    env!("CARGO_BIN_EXE_pdal-rs").to_string()
}

fn repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn nitfwrap_wraps_and_unwraps_las() {
    let temp = tempfile::tempdir().unwrap();
    let input = repo().join("test/data/las/simple.las");
    let nitf = temp.path().join("simple.ntf");
    let out = temp.path().join("simple.las");

    let status = Command::new(bin())
        .args(["nitfwrap", input.to_str().unwrap(), nitf.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(bin())
        .args([
            "nitfwrap",
            "-u",
            nitf.to_str().unwrap(),
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(std::fs::read(out).unwrap(), std::fs::read(input).unwrap());
}

#[test]
fn nitfwrap_unwraps_existing_fixture() {
    let temp = tempfile::tempdir().unwrap();
    let input = repo().join("test/data/nitf/autzen-utm10.ntf");
    let out = temp.path().join("autzen.las");

    let status = Command::new(bin())
        .args([
            "nitfwrap",
            "--unwrap",
            input.to_str().unwrap(),
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        std::fs::read(out).unwrap(),
        std::fs::read(repo().join("test/data/nitf/autzen-utm10.las")).unwrap()
    );
}

#[test]
fn nitfwrap_supports_output_option() {
    let temp = tempfile::tempdir().unwrap();
    let input = repo().join("test/data/las/simple.las");
    let nitf = temp.path().join("from-option.ntf");
    let out = temp.path().join("from-option.las");

    let status = Command::new(bin())
        .args([
            "nitfwrap",
            input.to_str().unwrap(),
            "--output",
            nitf.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(bin())
        .args([
            "nitfwrap",
            "--unwrap",
            nitf.to_str().unwrap(),
            &format!("--output={}", out.display()),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(std::fs::read(out).unwrap(), std::fs::read(input).unwrap());
}

#[test]
fn nitfwrap_rejects_missing_input() {
    let output = Command::new(bin())
        .args(["nitfwrap", "missing.las"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Input file 'missing.las' doesn't exist."));
}

#[test]
fn nitfwrap_rejects_missing_output_value() {
    let output = Command::new(bin())
        .args(["nitfwrap", "-o"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("output option requires a filename"));
}

#[test]
fn nitfwrap_rejects_missing_input_and_extra_positionals() {
    let output = Command::new(bin()).args(["nitfwrap"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("usage: nitfwrap"));

    let output = Command::new(bin())
        .args(["nitfwrap", "a.las", "b.ntf", "c.ntf"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unexpected argument 'c.ntf'"));
}
