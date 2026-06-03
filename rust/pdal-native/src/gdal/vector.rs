use super::register_drivers;
use gdal_sys::{CPLErr, OGRDataSourceH};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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

    pub fn open_with_options(
        path: &str,
        drivers: &[String],
        open_options: &[String],
    ) -> Result<Self, String> {
        if drivers.is_empty() && open_options.is_empty() {
            return Self::open(path);
        }
        register_drivers();
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        unsafe {
            let driver_list = csl_from_strings(drivers)?;
            let open_option_list = csl_from_strings(open_options)?;
            // GDAL_OF_READONLY (0) | GDAL_OF_VECTOR | GDAL_OF_VERBOSE_ERROR.
            let open_flags = 0x44;
            let ds = gdal_sys::GDALOpenEx(
                path_c.as_ptr(),
                open_flags,
                driver_list as *const *const _,
                open_option_list as *const *const _,
                std::ptr::null(),
            );
            gdal_sys::CSLDestroy(driver_list);
            gdal_sys::CSLDestroy(open_option_list);
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
        self.open_or_create_layer_with_options(layer_name, srs_wkt, &[])
    }

    pub fn open_or_create_layer_with_options(
        &self,
        layer_name: &str,
        srs_wkt: &str,
        layer_options: &[String],
    ) -> Result<gdal_sys::OGRLayerH, String> {
        let layer_name_c = CString::new(layer_name).map_err(|e| e.to_string())?;
        let option_strings = layer_options
            .iter()
            .map(|option| CString::new(option.as_str()).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut option_ptrs = option_strings
            .iter()
            .map(|option| option.as_ptr() as *mut c_char)
            .collect::<Vec<_>>();
        option_ptrs.push(std::ptr::null_mut());
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
                    option_ptrs.as_mut_ptr(),
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
        let layer = unsafe { gdal_sys::OGR_DS_GetLayer(self.ds, layer_idx) };
        if layer.is_null() {
            return Err("Failed to get layer".to_string());
        }
        self.get_int_features_from_layer(layer, column)
    }

    pub fn get_features_by_layer(
        &self,
        layer_name: &str,
        column: &str,
    ) -> Result<Vec<(String, i32)>, String> {
        if layer_name.is_empty() {
            return self.get_features(0, column);
        }
        let layer_name_c = CString::new(layer_name).map_err(|e| e.to_string())?;
        let layer = unsafe { gdal_sys::OGR_DS_GetLayerByName(self.ds, layer_name_c.as_ptr()) };
        if layer.is_null() {
            return Err(format!("Failed to get layer '{}'.", layer_name));
        }
        self.get_int_features_from_layer(layer, column)
    }

    pub fn get_features_by_sql(
        &self,
        sql: &str,
        column: &str,
    ) -> Result<Vec<(String, i32)>, String> {
        let sql_c = CString::new(sql).map_err(|e| e.to_string())?;
        unsafe {
            let layer = gdal_sys::OGR_DS_ExecuteSQL(
                self.ds,
                sql_c.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            if layer.is_null() {
                return Err(format!("Failed to execute OGR SQL '{}'.", sql));
            }
            let result = self.get_int_features_from_layer(layer, column);
            gdal_sys::OGR_DS_ReleaseResultSet(self.ds, layer);
            result
        }
    }

    fn get_int_features_from_layer(
        &self,
        layer: gdal_sys::OGRLayerH,
        column: &str,
    ) -> Result<Vec<(String, i32)>, String> {
        unsafe {
            let mut result = Vec::new();
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
        self.get_string_pair_features_from_layer(layer, column, "", "")
            .map(drop_optional_string_column)
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
            return self
                .get_string_pair_features_from_layer(layer, column, "", attribute_filter)
                .map(drop_optional_string_column);
        }
        let layer_name_c = CString::new(layer_name).map_err(|e| e.to_string())?;
        let layer = unsafe { gdal_sys::OGR_DS_GetLayerByName(self.ds, layer_name_c.as_ptr()) };
        if layer.is_null() {
            return Err(format!("Failed to get layer '{}'.", layer_name));
        }
        self.get_string_pair_features_from_layer(layer, column, "", attribute_filter)
            .map(drop_optional_string_column)
    }

    pub fn get_string_features_by_sql(
        &self,
        sql: &str,
        dialect: &str,
        column: &str,
        attribute_filter: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let sql_c = CString::new(sql).map_err(|e| e.to_string())?;
        let dialect_c = CString::new(dialect).map_err(|e| e.to_string())?;
        let dialect_ptr = if dialect.is_empty() {
            std::ptr::null()
        } else {
            dialect_c.as_ptr()
        };
        unsafe {
            let layer = gdal_sys::OGR_DS_ExecuteSQL(
                self.ds,
                sql_c.as_ptr(),
                std::ptr::null_mut(),
                dialect_ptr,
            );
            if layer.is_null() {
                return Err(format!("Failed to execute OGR SQL '{}'.", sql));
            }
            let result = self
                .get_string_pair_features_from_layer(layer, column, "", attribute_filter)
                .map(drop_optional_string_column);
            gdal_sys::OGR_DS_ReleaseResultSet(self.ds, layer);
            result
        }
    }

    pub fn get_string_pair_features_by_layer(
        &self,
        layer_name: &str,
        first_column: &str,
        second_column: &str,
        attribute_filter: &str,
    ) -> Result<Vec<(String, String, Option<String>)>, String> {
        if layer_name.is_empty() {
            let layer = unsafe { gdal_sys::OGR_DS_GetLayer(self.ds, 0) };
            if layer.is_null() {
                return Err("Failed to get layer".to_string());
            }
            return self.get_string_pair_features_from_layer(
                layer,
                first_column,
                second_column,
                attribute_filter,
            );
        }
        let layer_name_c = CString::new(layer_name).map_err(|e| e.to_string())?;
        let layer = unsafe { gdal_sys::OGR_DS_GetLayerByName(self.ds, layer_name_c.as_ptr()) };
        if layer.is_null() {
            return Err(format!("Failed to get layer '{}'.", layer_name));
        }
        self.get_string_pair_features_from_layer(
            layer,
            first_column,
            second_column,
            attribute_filter,
        )
    }

    pub fn get_string_pair_features_by_sql(
        &self,
        sql: &str,
        dialect: &str,
        first_column: &str,
        second_column: &str,
        attribute_filter: &str,
    ) -> Result<Vec<(String, String, Option<String>)>, String> {
        let sql_c = CString::new(sql).map_err(|e| e.to_string())?;
        let dialect_c = CString::new(dialect).map_err(|e| e.to_string())?;
        let dialect_ptr = if dialect.is_empty() {
            std::ptr::null()
        } else {
            dialect_c.as_ptr()
        };
        unsafe {
            let layer = gdal_sys::OGR_DS_ExecuteSQL(
                self.ds,
                sql_c.as_ptr(),
                std::ptr::null_mut(),
                dialect_ptr,
            );
            if layer.is_null() {
                return Err(format!("Failed to execute OGR SQL '{}'.", sql));
            }
            let result = self.get_string_pair_features_from_layer(
                layer,
                first_column,
                second_column,
                attribute_filter,
            );
            gdal_sys::OGR_DS_ReleaseResultSet(self.ds, layer);
            result
        }
    }

    fn get_string_pair_features_from_layer(
        &self,
        layer: gdal_sys::OGRLayerH,
        first_column: &str,
        second_column: &str,
        attribute_filter: &str,
    ) -> Result<Vec<(String, String, Option<String>)>, String> {
        let first_column_c = CString::new(first_column).map_err(|e| e.to_string())?;
        let second_column_c = CString::new(second_column).map_err(|e| e.to_string())?;
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

                let field_idx = gdal_sys::OGR_F_GetFieldIndex(feature, first_column_c.as_ptr());
                if field_idx < 0 {
                    gdal_sys::OGR_F_Destroy(feature);
                    return Err(format!("No column name '{}' was found.", first_column));
                }
                let second_field_idx = if second_column.is_empty() {
                    -1
                } else {
                    let idx = gdal_sys::OGR_F_GetFieldIndex(feature, second_column_c.as_ptr());
                    if idx < 0 {
                        gdal_sys::OGR_F_Destroy(feature);
                        return Err(format!("No column name '{}' was found.", second_column));
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
                        let value_ptr = gdal_sys::OGR_F_GetFieldAsString(feature, field_idx);
                        let value = if value_ptr.is_null() {
                            String::new()
                        } else {
                            std::ffi::CStr::from_ptr(value_ptr)
                                .to_string_lossy()
                                .into_owned()
                        };
                        let second = if second_field_idx < 0 {
                            None
                        } else {
                            let ptr = gdal_sys::OGR_F_GetFieldAsString(feature, second_field_idx);
                            if ptr.is_null() {
                                None
                            } else {
                                let value =
                                    std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
                                (!value.is_empty()).then_some(value)
                            }
                        };
                        result.push((wkt, value, second));
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
        let layer = unsafe { gdal_sys::OGR_DS_GetLayer(self.ds, layer_idx) };
        if layer.is_null() {
            return Err("Failed to get layer".to_string());
        }
        self.get_feature_wkts_from_layer(layer)
    }

    pub fn get_feature_wkts_by_layer(&self, layer_name: &str) -> Result<Vec<String>, String> {
        if layer_name.is_empty() {
            return self.get_feature_wkts(0);
        }
        let layer_name_c = CString::new(layer_name).map_err(|e| e.to_string())?;
        let layer = unsafe { gdal_sys::OGR_DS_GetLayerByName(self.ds, layer_name_c.as_ptr()) };
        if layer.is_null() {
            return Err(format!("Failed to get layer '{}'.", layer_name));
        }
        self.get_feature_wkts_from_layer(layer)
    }

    pub fn get_feature_wkts_by_sql(&self, sql: &str, dialect: &str) -> Result<Vec<String>, String> {
        self.get_feature_wkts_by_sql_with_filter(sql, dialect, "")
    }

    pub fn get_feature_wkts_by_sql_with_filter(
        &self,
        sql: &str,
        dialect: &str,
        filter_wkt: &str,
    ) -> Result<Vec<String>, String> {
        let sql_c = CString::new(sql).map_err(|e| e.to_string())?;
        let dialect_c = CString::new(dialect).map_err(|e| e.to_string())?;
        let dialect_ptr = if dialect.is_empty() {
            std::ptr::null()
        } else {
            dialect_c.as_ptr()
        };
        let filter_wkt_c = CString::new(filter_wkt).map_err(|e| e.to_string())?;
        unsafe {
            let filter_geometry = if filter_wkt.is_empty() {
                std::ptr::null_mut()
            } else {
                let mut geom = std::ptr::null_mut();
                let mut wkt_ptr = filter_wkt_c.as_ptr() as *mut std::ffi::c_char;
                if gdal_sys::OGR_G_CreateFromWkt(&mut wkt_ptr, std::ptr::null_mut(), &mut geom)
                    != gdal_sys::OGRErr::OGRERR_NONE
                {
                    return Err(format!(
                        "Failed to parse OGR SQL geometry filter: {filter_wkt}"
                    ));
                }
                geom
            };
            let layer =
                gdal_sys::OGR_DS_ExecuteSQL(self.ds, sql_c.as_ptr(), filter_geometry, dialect_ptr);
            if !filter_geometry.is_null() {
                gdal_sys::OGR_G_DestroyGeometry(filter_geometry);
            }
            if layer.is_null() {
                return Err(format!("Failed to execute OGR SQL '{}'.", sql));
            }
            let result = self.get_feature_wkts_from_layer(layer);
            gdal_sys::OGR_DS_ReleaseResultSet(self.ds, layer);
            result
        }
    }

    fn get_feature_wkts_from_layer(
        &self,
        layer: gdal_sys::OGRLayerH,
    ) -> Result<Vec<String>, String> {
        unsafe {
            let mut result = Vec::new();
            gdal_sys::OGR_L_ResetReading(layer);

            loop {
                let feature = gdal_sys::OGR_L_GetNextFeature(layer);
                if feature.is_null() {
                    break;
                }
                push_feature_wkt(feature, &mut result);
                gdal_sys::OGR_F_Destroy(feature);
            }
            Ok(result)
        }
    }

    pub fn geometry_column(&self, layer_idx: i32) -> Result<String, String> {
        unsafe {
            let layer = gdal_sys::OGR_DS_GetLayer(self.ds, layer_idx);
            if layer.is_null() {
                return Err("Failed to get layer".to_string());
            }
            let name = gdal_sys::OGR_L_GetGeometryColumn(layer);
            if name.is_null() {
                return Ok(String::new());
            }
            Ok(CStr::from_ptr(name).to_string_lossy().into_owned())
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

fn drop_optional_string_column(
    rows: Vec<(String, String, Option<String>)>,
) -> Vec<(String, String)> {
    rows.into_iter()
        .map(|(wkt, value, _)| (wkt, value))
        .collect()
}

unsafe fn csl_from_strings(values: &[String]) -> Result<*mut *mut std::ffi::c_char, String> {
    let mut list = std::ptr::null_mut();
    for value in values {
        let value_c = CString::new(value.as_str()).map_err(|e| e.to_string())?;
        list = gdal_sys::CSLAddString(list, value_c.as_ptr());
    }
    Ok(list)
}

unsafe fn push_feature_wkt(feature: gdal_sys::OGRFeatureH, result: &mut Vec<String>) {
    let geom = gdal_sys::OGR_F_GetGeometryRef(feature);
    if geom.is_null() {
        return;
    }
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
