use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let nitf_enabled = env::var_os("CARGO_FEATURE_NITF").is_some();
    let prefix = env::var("CONDA_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../../.pixi/envs/dev"));
    let prefix = prefix.canonicalize().unwrap_or(prefix);
    let (include, lib) = if cfg!(target_os = "windows") {
        (
            prefix.join("Library").join("include"),
            prefix.join("Library").join("lib"),
        )
    } else {
        (prefix.join("include"), prefix.join("lib"))
    };
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));

    if nitf_enabled {
        let nitro_cpp_include = nitro_cpp_include(&include, &out_dir);
        cc::Build::new()
            .cpp(true)
            .std("c++17")
            .define("_REENTRANT", None)
            .define("__POSIX", None)
            .include(&include)
            .include(nitro_cpp_include)
            .include(include.join("nitro/c"))
            .file("src/nitf_bridge.cpp")
            .compile("pdal_native_nitf_bridge");
    }

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include(&include)
        .file("src/geotiff_bridge.cpp")
        .compile("pdal_native_geotiff_bridge");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=geotiff");
    link_library(
        &lib,
        &out_dir,
        "xml2",
        &["libxml2.so", "libxml2.dylib", "xml2.lib"],
    );
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
    copy_runtime_library(&lib, &out_dir, "libxml2.so");
    copy_runtime_library(&lib, &out_dir, "libxml2.so.2");
    copy_runtime_library(&lib, &out_dir, "libproj.so");
    copy_runtime_library(&lib, &out_dir, "libproj.so.25");
    copy_runtime_library(&lib, &out_dir, "libgeos.so");
    copy_runtime_library(&lib, &out_dir, "libgeos_c.so");
    if nitf_enabled {
        println!("cargo:rustc-link-lib=nitf-cpp");
        println!("cargo:rustc-link-lib=nitf-c");
        copy_runtime_library(&lib, &out_dir, "libnitf-cpp.dylib");
        copy_runtime_library(&lib, &out_dir, "libnitf-c.dylib");
        copy_runtime_library(&lib, &out_dir, "libnitf-cpp.so");
        copy_runtime_library(&lib, &out_dir, "libnitf-c.so");
    }
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib.display());
    }
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath-link,{}", lib.display());
    }
    println!("cargo:rerun-if-env-changed=CONDA_PREFIX");
    println!("cargo:rerun-if-changed=src/geotiff_bridge.cpp");
    if nitf_enabled {
        println!("cargo:rerun-if-changed=src/nitf_bridge.cpp");
    }
}

fn copy_runtime_library(lib: &Path, out_dir: &Path, name: &str) {
    let src = lib.join(name);
    if src.exists() {
        let _ = fs::copy(src, out_dir.join(name));
    }
}

fn nitro_cpp_include(include: &Path, out_dir: &Path) -> PathBuf {
    let source = include.join("nitro/c++");
    if cfg!(target_os = "windows") && source.exists() {
        let mirror = out_dir.join("nitro_cpp_include");
        let _ = fs::remove_dir_all(&mirror);
        if copy_dir_all(&source, &mirror).is_ok() {
            return mirror;
        }
    }
    source
}

fn copy_dir_all(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = target.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn link_library(lib: &Path, out_dir: &Path, name: &str, unversioned_names: &[&str]) {
    if unversioned_names.iter().any(|name| lib.join(name).exists()) {
        println!("cargo:rustc-link-lib={name}");
        return;
    }

    if let Some(candidate) = versioned_library_candidates(lib, name).into_iter().next() {
        if cfg!(target_os = "linux") {
            if name != "xml2" {
                println!("cargo:rustc-link-arg={}", candidate.display());
                return;
            }
        } else {
            let link_name = out_dir.join(format!("lib{name}.so"));
            let _ = fs::copy(candidate, &link_name);
            println!("cargo:rustc-link-search=native={}", out_dir.display());
            println!("cargo:rustc-link-lib={name}");
            return;
        }
    }

    println!("cargo:rustc-link-lib={name}");
}

fn versioned_library_candidates(lib: &Path, name: &str) -> Vec<PathBuf> {
    if cfg!(target_os = "linux") && name == "xml2" {
        let candidate = lib.join("libxml2.so.2");
        if candidate.exists() {
            return vec![candidate];
        }
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(lib) else {
        return Vec::new();
    };
    let prefix = format!("lib{name}.so.");
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect();
    candidates.sort();
    candidates
}
