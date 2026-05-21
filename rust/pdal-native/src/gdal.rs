//! Minimal GDAL adapter using gdal-sys directly.

use gdal_sys::{
    CPLErr, GDALAccess, GDALDataType, GDALDatasetH, GDALGetRasterBand, GDALRWFlag, GDALRasterIO,
};
use gdal_sys::{OGRDataSourceH, OGRLayerH};
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

    pub fn create_float64(
        path: &str,
        driver_name: &str,
        width: i32,
        height: i32,
        band_count: i32,
        geo_transform: [f64; 6],
        srs_wkt: &str,
    ) -> Result<Self, String> {
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        let driver_name_c = CString::new(driver_name).map_err(|e| e.to_string())?;
        unsafe {
            let driver = gdal_sys::GDALGetDriverByName(driver_name_c.as_ptr());
            if driver.is_null() {
                return Err(format!("GDAL driver '{}' not found", driver_name));
            }
            let ds = gdal_sys::GDALCreate(
                driver,
                path_c.as_ptr(),
                width,
                height,
                band_count,
                GDALDataType::GDT_Float64,
                std::ptr::null_mut(),
            );
            if ds.is_null() {
                return Err(format!("Failed to create GDAL dataset: {}", path));
            }
            if gdal_sys::GDALSetGeoTransform(ds, geo_transform.as_ptr() as *mut f64)
                != CPLErr::CE_None
            {
                gdal_sys::GDALClose(ds);
                return Err("Failed to set GDAL geotransform".to_string());
            }
            if !srs_wkt.is_empty() {
                let srs_c = CString::new(srs_wkt).map_err(|e| e.to_string())?;
                if gdal_sys::GDALSetProjection(ds, srs_c.as_ptr()) != CPLErr::CE_None {
                    gdal_sys::GDALClose(ds);
                    return Err("Failed to set GDAL projection".to_string());
                }
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
                transform[1] = 1.0;
                transform[5] = 1.0;
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
        if buffer.len() != width * height {
            return Err("GDAL band buffer size does not match raster dimensions.".to_string());
        }
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

    pub fn write_band_f64(
        &mut self,
        band_idx: i32,
        width: usize,
        height: usize,
        buffer: &[f64],
        no_data: f64,
        description: &str,
    ) -> Result<(), String> {
        if buffer.len() != width * height {
            return Err("GDAL band buffer size does not match raster dimensions.".to_string());
        }
        let description_c = CString::new(description).map_err(|e| e.to_string())?;
        unsafe {
            let band = GDALGetRasterBand(self.ds, band_idx);
            if band.is_null() {
                return Err(format!("Failed to get band {}", band_idx));
            }
            if gdal_sys::GDALSetRasterNoDataValue(band, no_data) != CPLErr::CE_None {
                return Err(format!("Failed to set no-data value for band {}", band_idx));
            }
            gdal_sys::GDALSetDescription(
                band as gdal_sys::GDALMajorObjectH,
                description_c.as_ptr(),
            );

            let res = GDALRasterIO(
                band,
                GDALRWFlag::GF_Write,
                0,
                0,
                width as i32,
                height as i32,
                buffer.as_ptr() as *mut _,
                width as i32,
                height as i32,
                GDALDataType::GDT_Float64,
                0,
                0,
            );

            if res != CPLErr::CE_None {
                return Err("GDAL RasterIO write failed".to_string());
            }
            Ok(())
        }
    }

    pub fn set_metadata_item(&mut self, key: &str, value: &str) -> Result<(), String> {
        let key_c = CString::new(key).map_err(|e| e.to_string())?;
        let value_c = CString::new(value).map_err(|e| e.to_string())?;
        let domain_c = CString::new("").map_err(|e| e.to_string())?;
        unsafe {
            if gdal_sys::GDALSetMetadataItem(
                self.ds as gdal_sys::GDALMajorObjectH,
                key_c.as_ptr(),
                value_c.as_ptr(),
                domain_c.as_ptr(),
            ) != CPLErr::CE_None
            {
                return Err(format!("Failed to set GDAL metadata item '{key}'"));
            }
            Ok(())
        }
    }

    pub fn metadata_item(&self, key: &str) -> Option<String> {
        let key_c = CString::new(key).ok()?;
        let domain_c = CString::new("").ok()?;
        unsafe {
            let value = gdal_sys::GDALGetMetadataItem(
                self.ds as gdal_sys::GDALMajorObjectH,
                key_c.as_ptr(),
                domain_c.as_ptr(),
            );
            if !value.is_null() {
                return Some(CStr::from_ptr(value).to_string_lossy().into_owned());
            }

            // GDALGetMetadataItem returns null for empty values; scan the list instead.
            let items = gdal_sys::GDALGetMetadata(
                self.ds as gdal_sys::GDALMajorObjectH,
                domain_c.as_ptr(),
            );
            if items.is_null() {
                return None;
            }
            let count = gdal_sys::CSLCount(items);
            for idx in 0..count {
                let item = *items.offset(idx as isize);
                if item.is_null() {
                    continue;
                }
                let entry = CStr::from_ptr(item).to_string_lossy();
                if let Some((item_key, item_value)) = entry.split_once('=') {
                    if item_key == key {
                        return Some(item_value.to_string());
                    }
                }
            }
            None
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

pub type LayerHandle = OGRLayerH;

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

    pub fn create(path: &str, driver_name: &str) -> Result<Self, String> {
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        let driver_c = CString::new(driver_name).map_err(|e| e.to_string())?;
        unsafe {
            let driver = gdal_sys::OGRGetDriverByName(driver_c.as_ptr());
            if driver.is_null() {
                return Err(format!("OGR driver '{}' not found", driver_name));
            }
            let ds =
                gdal_sys::OGR_Dr_CreateDataSource(driver, path_c.as_ptr(), std::ptr::null_mut());
            if ds.is_null() {
                return Err(format!("Failed to create OGR datasource: {}", path));
            }
            Ok(Self { ds })
        }
    }

    pub fn open_or_create_layer(
        &self,
        layer_name: &str,
        srs_wkt: &str,
    ) -> Result<gdal_sys::OGRLayerH, String> {
        let layer_name_c = CString::new(layer_name).map_err(|e| e.to_string())?;
        unsafe {
            let mut layer = gdal_sys::OGR_DS_GetLayerByName(self.ds, layer_name_c.as_ptr());
            if layer.is_null() {
                let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
                if !srs_wkt.is_empty() {
                    let wkt_c = CString::new(srs_wkt).map_err(|e| e.to_string())?;
                    let mut wkt_ptr = wkt_c.as_ptr() as *mut std::os::raw::c_char;
                    gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr);
                }
                layer = gdal_sys::OGR_DS_CreateLayer(
                    self.ds,
                    layer_name_c.as_ptr(),
                    srs,
                    gdal_sys::OGRwkbGeometryType::wkbMultiPolygon,
                    std::ptr::null_mut(),
                );
                gdal_sys::OSRDestroySpatialReference(srs);
                if layer.is_null() {
                    return Err(format!("Failed to create OGR layer: {}", layer_name));
                }
            }
            Ok(layer)
        }
    }

    /// # Safety
    /// The `layer` must be a valid `OGRLayerH` pointer.
    pub unsafe fn create_string_field(
        layer: gdal_sys::OGRLayerH,
        name: &str,
    ) -> Result<(), String> {
        let name_c = CString::new(name).map_err(|e| e.to_string())?;
        unsafe {
            let field =
                gdal_sys::OGR_Fld_Create(name_c.as_ptr(), gdal_sys::OGRFieldType::OFTString);
            gdal_sys::OGR_L_CreateField(layer, field, 1);
            gdal_sys::OGR_Fld_Destroy(field);
        }
        Ok(())
    }

    /// # Safety
    /// The `layer` must be a valid `OGRLayerH` pointer.
    pub unsafe fn create_datetime_field(
        layer: gdal_sys::OGRLayerH,
        name: &str,
    ) -> Result<(), String> {
        let name_c = CString::new(name).map_err(|e| e.to_string())?;
        unsafe {
            let field =
                gdal_sys::OGR_Fld_Create(name_c.as_ptr(), gdal_sys::OGRFieldType::OFTDateTime);
            gdal_sys::OGR_L_CreateField(layer, field, 1);
            gdal_sys::OGR_Fld_Destroy(field);
        }
        Ok(())
    }

    /// # Safety
    /// The `layer` must be a valid `OGRLayerH` pointer.
    pub unsafe fn add_feature(
        layer: gdal_sys::OGRLayerH,
        wkt: &str,
        string_fields: &[(&str, &str)],
    ) -> Result<(), String> {
        let wkt_c = CString::new(wkt).map_err(|e| e.to_string())?;
        unsafe {
            let defn = gdal_sys::OGR_L_GetLayerDefn(layer);
            let feature = gdal_sys::OGR_F_Create(defn);

            for (name, value) in string_fields {
                let name_c = CString::new(*name).map_err(|e| e.to_string())?;
                let val_c = CString::new(*value).map_err(|e| e.to_string())?;
                let idx = gdal_sys::OGR_FD_GetFieldIndex(defn, name_c.as_ptr());
                if idx >= 0 {
                    gdal_sys::OGR_F_SetFieldString(feature, idx, val_c.as_ptr());
                }
            }

            let mut geom = std::ptr::null_mut();
            let mut wkt_ptr = wkt_c.as_ptr() as *mut std::os::raw::c_char;
            if gdal_sys::OGR_G_CreateFromWkt(&mut wkt_ptr, std::ptr::null_mut(), &mut geom)
                == CPLErr::CE_None
            {
                gdal_sys::OGR_F_SetGeometryDirectly(feature, geom);
            }

            let res = gdal_sys::OGR_L_CreateFeature(layer, feature);
            gdal_sys::OGR_F_Destroy(feature);

            if res != gdal_sys::OGRErr::OGRERR_NONE {
                return Err("Failed to create feature".to_string());
            }
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_tif(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("pdal-native-{name}-{}.tif", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn metadata_roundtrip_includes_empty_values_in_memory() {
        register_drivers();
        let path = temp_tif("metadata-empty");
        let mut raster = Raster::create_float64(
            path.to_str().unwrap(),
            "GTiff",
            1,
            1,
            1,
            [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
            "",
        )
        .unwrap();
        raster.write_band_f64(1, 1, 1, &[1.0], -9999.0, "Z").unwrap();
        raster
            .set_metadata_item("AREA_OR_PIXEL", "Pixel")
            .unwrap();
        raster.set_metadata_item("empty", "").unwrap();
        raster
            .set_metadata_item("equals", "some_more_equals===")
            .unwrap();
        assert_eq!(raster.metadata_item("empty").as_deref(), Some(""));

        drop(raster);

        let raster = Raster::open(path.to_str().unwrap()).unwrap();
        assert_eq!(
            raster.metadata_item("AREA_OR_PIXEL").as_deref(),
            Some("Pixel")
        );
        assert_eq!(
            raster.metadata_item("equals").as_deref(),
            Some("some_more_equals===")
        );
        // GTiff does not persist empty metadata values when the dataset is closed.
        assert!(raster.metadata_item("empty").is_none());
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
