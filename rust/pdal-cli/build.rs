fn main() {
    let lib_dir = std::env::var("PDAL_LIB_DIR").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        format!("{}/../../build/lib", manifest_dir)
    });

    println!("cargo:rustc-link-search=native={}", lib_dir);
    println!("cargo:rustc-link-lib=pdalcpp");
    println!("cargo:rustc-link-lib=static=pdal_kernel_capi");
    println!("cargo:rustc-link-lib=c++");

    // Set rpath so the binary can find libpdalcpp.dylib at runtime.
    if std::env::consts::OS == "macos" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir);
    }
}
