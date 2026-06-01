use super::register_drivers;
use gdal_sys::{CPLErr, OGRDataSourceH};
use std::ffi::CString;

pub struct Vector {
    ds: OGRDataSourceH,
}

impl Vector {
    pub fn open(path: &str) -> Result<Self, String> {
        register_drivers();
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
        register_drivers();
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

    pub fn get_string_features(
        &self,
        layer_idx: i32,
        column: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let layer = unsafe { gdal_sys::OGR_DS_GetLayer(self.ds, layer_idx) };
        if layer.is_null() {
            return Err("Failed to get layer".to_string());
        }
        self.get_string_features_from_layer(layer, column, "")
    }

    pub fn get_string_features_by_layer(
        &self,
        layer_name: &str,
        column: &str,
        attribute_filter: &str,
    ) -> Result<Vec<(String, String)>, String> {
        if layer_name.is_empty() {
            let layer = unsafe { gdal_sys::OGR_DS_GetLayer(self.ds, 0) };
            if layer.is_null() {
                return Err("Failed to get layer".to_string());
            }
            return self.get_string_features_from_layer(layer, column, attribute_filter);
        }
        let layer_name_c = CString::new(layer_name).map_err(|e| e.to_string())?;
        let layer = unsafe { gdal_sys::OGR_DS_GetLayerByName(self.ds, layer_name_c.as_ptr()) };
        if layer.is_null() {
            return Err(format!("Failed to get layer '{}'.", layer_name));
        }
        self.get_string_features_from_layer(layer, column, attribute_filter)
    }

    fn get_string_features_from_layer(
        &self,
        layer: gdal_sys::OGRLayerH,
        column: &str,
        attribute_filter: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let column_c = CString::new(column).map_err(|e| e.to_string())?;
        let filter_c = CString::new(attribute_filter).map_err(|e| e.to_string())?;
        unsafe {
            let mut result = Vec::new();
            gdal_sys::OGR_L_ResetReading(layer);
            if !attribute_filter.is_empty()
                && gdal_sys::OGR_L_SetAttributeFilter(layer, filter_c.as_ptr())
                    != gdal_sys::OGRErr::OGRERR_NONE
            {
                return Err(format!(
                    "Unable to set attribute filter '{}'.",
                    attribute_filter
                ));
            }

            loop {
                let feature = gdal_sys::OGR_L_GetNextFeature(layer);
                if feature.is_null() {
                    break;
                }

                let field_idx = gdal_sys::OGR_F_GetFieldIndex(feature, column_c.as_ptr());
                if field_idx < 0 {
                    gdal_sys::OGR_F_Destroy(feature);
                    return Err(format!("No column name '{}' was found.", column));
                }

                let geom = gdal_sys::OGR_F_GetGeometryRef(feature);
                if !geom.is_null() {
                    let mut wkt_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
                    if gdal_sys::OGR_G_ExportToWkt(geom, &mut wkt_ptr) == CPLErr::CE_None {
                        let wkt = std::ffi::CStr::from_ptr(wkt_ptr)
                            .to_string_lossy()
                            .into_owned();
                        gdal_sys::VSIFree(wkt_ptr as *mut _);
                        let value_ptr = gdal_sys::OGR_F_GetFieldAsString(feature, field_idx);
                        let value = if value_ptr.is_null() {
                            String::new()
                        } else {
                            std::ffi::CStr::from_ptr(value_ptr)
                                .to_string_lossy()
                                .into_owned()
                        };
                        result.push((wkt, value));
                    }
                }
                gdal_sys::OGR_F_Destroy(feature);
            }
            if !attribute_filter.is_empty() {
                gdal_sys::OGR_L_SetAttributeFilter(layer, std::ptr::null());
            }
            Ok(result)
        }
    }

    pub fn get_feature_wkts(&self, layer_idx: i32) -> Result<Vec<String>, String> {
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

                let geom = gdal_sys::OGR_F_GetGeometryRef(feature);
                if !geom.is_null() {
                    let mut wkt_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
                    if gdal_sys::OGR_G_ExportToWkt(geom, &mut wkt_ptr) == CPLErr::CE_None {
                        result.push(
                            std::ffi::CStr::from_ptr(wkt_ptr)
                                .to_string_lossy()
                                .into_owned(),
                        );
                        gdal_sys::VSIFree(wkt_ptr as *mut _);
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
