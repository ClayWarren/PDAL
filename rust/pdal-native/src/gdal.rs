//! Minimal GDAL adapter using gdal-sys directly.

mod raster;
mod vector;
mod vector_writer;

use std::ffi::{CStr, CString};

pub use raster::Raster;
pub use vector::Vector;
pub use vector_writer::{VectorFieldType, VectorFieldValue, VectorPointWriter};

pub type LayerHandle = gdal_sys::OGRLayerH;

pub fn register_drivers() {
    // GDAL's driver-manager registration mutates global state and is not safe
    // to run concurrently from multiple threads; guard it so it happens exactly
    // once even when several stages register drivers in parallel.
    static REGISTER: std::sync::Once = std::sync::Once::new();
    REGISTER.call_once(|| unsafe {
        gdal_sys::GDALAllRegister();
        gdal_sys::OGRRegisterAll();
    });
}

pub fn version() -> String {
    version_info("RELEASE_NAME")
}

pub fn version_info(key: &str) -> String {
    let Ok(key) = CString::new(key) else {
        return String::new();
    };
    unsafe {
        let value = gdal_sys::GDALVersionInfo(key.as_ptr());
        if value.is_null() {
            String::new()
        } else {
            CStr::from_ptr(value).to_string_lossy().into_owned()
        }
    }
}

#[cfg(test)]
mod tests;
