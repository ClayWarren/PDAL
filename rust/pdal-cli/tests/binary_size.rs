use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
