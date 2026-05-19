//! Minimal GDAL adapter using gdal-sys directly.

use gdal_sys::OGRDataSourceH;
use gdal_sys::{
    CPLErr, GDALAccess, GDALDataType, GDALDatasetH, GDALGetRasterBand, GDALRWFlag, GDALRasterIO,
};
use std::ffi::{CStr, CString};

pub struct Raster {
    ds: GDALDatasetH,
}

impl Raster {
    pub fn open(path: &str) -> Result<Self, String> {
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        unsafe {
            let ds = gdal_sys::GDALOpen(path_c.as_ptr(), GDALAccess::GA_ReadOnly);
            if ds.is_null() {
                return Err(format!("Failed to open GDAL dataset: {}", path));
            }
            Ok(Self { ds })
        }
    }

    pub fn width(&self) -> i32 {
        unsafe { gdal_sys::GDALGetRasterXSize(self.ds) }
    }

    pub fn height(&self) -> i32 {
        unsafe { gdal_sys::GDALGetRasterYSize(self.ds) }
    }

    pub fn band_count(&self) -> i32 {
        unsafe { gdal_sys::GDALGetRasterCount(self.ds) }
    }

    pub fn get_geo_transform(&self) -> Result<[f64; 6], String> {
        unsafe {
            let mut transform = [0.0f64; 6];
            if gdal_sys::GDALGetGeoTransform(self.ds, transform.as_mut_ptr()) != CPLErr::CE_None {
                return Err("Failed to get geo transform".to_string());
            }
            Ok(transform)
        }
    }

    pub fn get_wkt_srs(&self) -> String {
        unsafe {
            let srs = gdal_sys::GDALGetProjectionRef(self.ds);
            if srs.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(srs).to_string_lossy().into_owned()
            }
        }
    }

    pub fn read_band(
        &self,
        band_idx: i32,
        width: usize,
        height: usize,
        buffer: &mut [f64],
    ) -> Result<(), String> {
        unsafe {
            let band = GDALGetRasterBand(self.ds, band_idx);
            if band.is_null() {
                return Err(format!("Failed to get band {}", band_idx));
            }

            let res = GDALRasterIO(
                band,
                GDALRWFlag::GF_Read,
                0,
                0,
                width as i32,
                height as i32,
                buffer.as_mut_ptr() as *mut _,
                width as i32,
                height as i32,
                GDALDataType::GDT_Float64,
                0,
                0,
            );

            if res != CPLErr::CE_None {
                return Err("GDAL RasterIO failed".to_string());
            }
            Ok(())
        }
    }

    pub fn read_at(&self, x: f64, y: f64, buffer: &mut [f64]) -> Result<(), String> {
        unsafe {
            let mut transform = [0.0f64; 6];
            if gdal_sys::GDALGetGeoTransform(self.ds, transform.as_mut_ptr()) != CPLErr::CE_None {
                return Err("Failed to get geo transform".to_string());
            }

            let mut inv_transform = [0.0f64; 6];
            if gdal_sys::GDALInvGeoTransform(transform.as_mut_ptr(), inv_transform.as_mut_ptr())
                == 0
            {
                return Err("Failed to invert geo transform".to_string());
            }

            let pixel =
                (inv_transform[0] + x * inv_transform[1] + y * inv_transform[2]).floor() as i32;
            let line =
                (inv_transform[3] + x * inv_transform[4] + y * inv_transform[5]).floor() as i32;

            let width = gdal_sys::GDALGetRasterXSize(self.ds);
            let height = gdal_sys::GDALGetRasterYSize(self.ds);

            if pixel < 0 || pixel >= width || line < 0 || line >= height {
                return Err("Out of bounds".to_string());
            }

            let band_count = gdal_sys::GDALGetRasterCount(self.ds);
            for i in 0..band_count.min(buffer.len() as i32) {
                let band = GDALGetRasterBand(self.ds, i + 1);
                let res = GDALRasterIO(
                    band,
                    GDALRWFlag::GF_Read,
                    pixel,
                    line,
                    1,
                    1,
                    &mut buffer[i as usize] as *mut f64 as *mut _,
                    1,
                    1,
                    GDALDataType::GDT_Float64,
                    0,
                    0,
                );
                if res != CPLErr::CE_None {
                    return Err(format!("Failed to read pixel at band {}", i + 1));
                }
            }
            Ok(())
        }
    }
}

impl Drop for Raster {
    fn drop(&mut self) {
        unsafe {
            gdal_sys::GDALClose(self.ds);
        }
    }
}

pub struct Vector {
    ds: OGRDataSourceH,
}

impl Vector {
    pub fn open(path: &str) -> Result<Self, String> {
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        unsafe {
            let ds = gdal_sys::OGROpen(path_c.as_ptr(), 0, std::ptr::null_mut());
            if ds.is_null() {
                return Err(format!("Failed to open OGR datasource: {}", path));
            }
            Ok(Self { ds })
        }
    }

    pub fn get_features(&self, layer_idx: i32, column: &str) -> Result<Vec<(String, i32)>, String> {
        unsafe {
            let mut result = Vec::new();
            let layer = gdal_sys::OGR_DS_GetLayer(self.ds, layer_idx);
            if layer.is_null() {
                return Err("Failed to get layer".to_string());
            }
            gdal_sys::OGR_L_ResetReading(layer);

            loop {
                let feature = gdal_sys::OGR_L_GetNextFeature(layer);
                if feature.is_null() {
                    break;
                }

                let field_idx = if column.is_empty() {
                    1
                } else {
                    let column_c = CString::new(column).map_err(|e| e.to_string())?;
                    let idx = gdal_sys::OGR_F_GetFieldIndex(feature, column_c.as_ptr());
                    if idx < 0 {
                        gdal_sys::OGR_F_Destroy(feature);
                        return Err(format!("No column name '{}' was found.", column));
                    }
                    idx
                };

                let geom = gdal_sys::OGR_F_GetGeometryRef(feature);
                if !geom.is_null() {
                    let mut wkt_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
                    if gdal_sys::OGR_G_ExportToWkt(geom, &mut wkt_ptr) == CPLErr::CE_None {
                        let wkt = std::ffi::CStr::from_ptr(wkt_ptr)
                            .to_string_lossy()
                            .into_owned();
                        gdal_sys::VSIFree(wkt_ptr as *mut _);

                        let val = gdal_sys::OGR_F_GetFieldAsInteger(feature, field_idx);
                        result.push((wkt, val));
                    }
                }
                gdal_sys::OGR_F_Destroy(feature);
            }
            Ok(result)
        }
    }
}

impl Drop for Vector {
    fn drop(&mut self) {
        unsafe {
            gdal_sys::OGR_DS_Destroy(self.ds);
        }
    }
}

pub fn register_drivers() {
    unsafe {
        gdal_sys::GDALAllRegister();
        gdal_sys::OGRRegisterAll();
    }
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
