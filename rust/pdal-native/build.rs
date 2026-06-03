use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let prefix = env::var("CONDA_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../../.pixi/envs/dev"));
    let prefix = prefix.canonicalize().unwrap_or(prefix);
    let include = prefix.join("include");
    let lib = prefix.join("lib");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .define("_REENTRANT", None)
        .define("__POSIX", None)
        .include(include.join("nitro/c++"))
        .include(include.join("nitro/c"))
        .file("src/nitf_bridge.cpp")
        .compile("pdal_native_nitf_bridge");

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include(&include)
        .file("src/geotiff_bridge.cpp")
        .compile("pdal_native_geotiff_bridge");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=geotiff");
    println!("cargo:rustc-link-lib=nitf-cpp");
    println!("cargo:rustc-link-lib=nitf-c");
    copy_runtime_library(&lib, &out_dir, "libgeos.3.14.1.dylib");
    copy_runtime_library(&lib, &out_dir, "libgeos.dylib");
    copy_runtime_library(&lib, &out_dir, "libgeos_c.1.dylib");
    copy_runtime_library(&lib, &out_dir, "libgeos_c.dylib");
    copy_runtime_library(&lib, &out_dir, "libgeotiff.5.dylib");
    copy_runtime_library(&lib, &out_dir, "libgeotiff.dylib");
    copy_runtime_library(&lib, &out_dir, "libproj.25.dylib");
    copy_runtime_library(&lib, &out_dir, "libproj.dylib");
    copy_runtime_library(&lib, &out_dir, "libgeotiff.so");
    copy_runtime_library(&lib, &out_dir, "libgeotiff.so.5");
    copy_runtime_library(&lib, &out_dir, "libproj.so");
    copy_runtime_library(&lib, &out_dir, "libproj.so.25");
    copy_runtime_library(&lib, &out_dir, "libgeos.so");
    copy_runtime_library(&lib, &out_dir, "libgeos_c.so");
    copy_runtime_library(&lib, &out_dir, "libnitf-cpp.dylib");
    copy_runtime_library(&lib, &out_dir, "libnitf-c.dylib");
    copy_runtime_library(&lib, &out_dir, "libnitf-cpp.so");
    copy_runtime_library(&lib, &out_dir, "libnitf-c.so");
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib.display());
    }
    println!("cargo:rerun-if-env-changed=CONDA_PREFIX");
    println!("cargo:rerun-if-changed=src/geotiff_bridge.cpp");
    println!("cargo:rerun-if-changed=src/nitf_bridge.cpp");
}

fn copy_runtime_library(lib: &Path, out_dir: &Path, name: &str) {
    let src = lib.join(name);
    if src.exists() {
        let _ = fs::copy(src, out_dir.join(name));
    }
}
