use gdal_sys::{
    CPLErr, GDALAccess, GDALDataType, GDALDatasetH, GDALGetRasterBand, GDALRWFlag, GDALRasterIO,
};
use std::ffi::{CStr, CString};

pub struct Raster {
    ds: GDALDatasetH,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterDataType {
    Float64,
    Float32,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
}

impl RasterDataType {
    fn gdal_type(self) -> gdal_sys::GDALDataType::Type {
        match self {
            RasterDataType::Float64 => GDALDataType::GDT_Float64,
            RasterDataType::Float32 => GDALDataType::GDT_Float32,
            RasterDataType::Int8 => GDALDataType::GDT_Int8,
            RasterDataType::UInt8 => GDALDataType::GDT_UInt8,
            RasterDataType::Int16 => GDALDataType::GDT_Int16,
            RasterDataType::UInt16 => GDALDataType::GDT_UInt16,
            RasterDataType::Int32 => GDALDataType::GDT_Int32,
            RasterDataType::UInt32 => GDALDataType::GDT_UInt32,
            RasterDataType::Int64 => GDALDataType::GDT_Int64,
            RasterDataType::UInt64 => GDALDataType::GDT_UInt64,
        }
    }
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
        Self::create(
            path,
            driver_name,
            width,
            height,
            band_count,
            geo_transform,
            srs_wkt,
            GDALDataType::GDT_Float64,
        )
    }

    pub fn create_int32(
        path: &str,
        driver_name: &str,
        width: i32,
        height: i32,
        band_count: i32,
        geo_transform: [f64; 6],
        srs_wkt: &str,
    ) -> Result<Self, String> {
        Self::create(
            path,
            driver_name,
            width,
            height,
            band_count,
            geo_transform,
            srs_wkt,
            GDALDataType::GDT_Int32,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_typed(
        path: &str,
        driver_name: &str,
        width: i32,
        height: i32,
        band_count: i32,
        geo_transform: [f64; 6],
        srs_wkt: &str,
        pixel_type: RasterDataType,
    ) -> Result<Self, String> {
        Self::create(
            path,
            driver_name,
            width,
            height,
            band_count,
            geo_transform,
            srs_wkt,
            pixel_type.gdal_type(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn create(
        path: &str,
        driver_name: &str,
        width: i32,
        height: i32,
        band_count: i32,
        geo_transform: [f64; 6],
        srs_wkt: &str,
        pixel_type: gdal_sys::GDALDataType::Type,
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
                pixel_type,
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

    pub fn band_type_name(&self, band_idx: i32) -> Result<String, String> {
        unsafe {
            let band = GDALGetRasterBand(self.ds, band_idx);
            if band.is_null() {
                return Err(format!("Failed to get band {}", band_idx));
            }
            let data_type = gdal_sys::GDALGetRasterDataType(band);
            let name = gdal_sys::GDALGetDataTypeName(data_type);
            if name.is_null() {
                return Err(format!(
                    "Failed to get data type name for band {}",
                    band_idx
                ));
            }
            Ok(CStr::from_ptr(name).to_string_lossy().into_owned())
        }
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

    pub fn read_band_window(
        &self,
        band_idx: i32,
        x_offset: usize,
        y_offset: usize,
        width: usize,
        height: usize,
        buffer: &mut [f64],
    ) -> Result<(), String> {
        if buffer.len() != width * height {
            return Err("GDAL band buffer size does not match raster window.".to_string());
        }
        unsafe {
            let band = GDALGetRasterBand(self.ds, band_idx);
            if band.is_null() {
                return Err(format!("Failed to get band {}", band_idx));
            }

            let res = GDALRasterIO(
                band,
                GDALRWFlag::GF_Read,
                x_offset as i32,
                y_offset as i32,
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

    pub fn write_band_i32(
        &mut self,
        band_idx: i32,
        width: usize,
        height: usize,
        buffer: &[i32],
        no_data: i32,
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
            if gdal_sys::GDALSetRasterNoDataValue(band, no_data as f64) != CPLErr::CE_None {
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
                GDALDataType::GDT_Int32,
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
            let items =
                gdal_sys::GDALGetMetadata(self.ds as gdal_sys::GDALMajorObjectH, domain_c.as_ptr());
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
