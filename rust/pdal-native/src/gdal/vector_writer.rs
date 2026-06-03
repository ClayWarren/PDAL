use super::register_drivers;
use gdal_sys::{CPLErr, OGRDataSourceH, OGRLayerH};
use std::ffi::CString;
use std::os::raw::c_char;

pub struct VectorPointWriter {
    ds: OGRDataSourceH,
    layer: OGRLayerH,
}

#[derive(Clone, Copy, Debug)]
pub enum VectorFieldType {
    Integer,
    Integer64,
    Real,
}

#[derive(Clone, Copy, Debug)]
pub enum VectorFieldValue {
    Integer(i32),
    Integer64(i64),
    Real(f64),
}

impl VectorPointWriter {
    pub fn create(path: &str, driver_name: &str, srs_wkt: &str) -> Result<Self, String> {
        Self::create_point(path, driver_name, srs_wkt, false)
    }

    pub fn create_point(
        path: &str,
        driver_name: &str,
        srs_wkt: &str,
        measured: bool,
    ) -> Result<Self, String> {
        Self::create_point_with_options(path, driver_name, srs_wkt, measured, &[])
    }

    pub fn create_point_with_options(
        path: &str,
        driver_name: &str,
        srs_wkt: &str,
        measured: bool,
        layer_options: &[String],
    ) -> Result<Self, String> {
        Self::create_with_geometry(
            path,
            driver_name,
            srs_wkt,
            if measured {
                gdal_sys::OGRwkbGeometryType::wkbPointZM
            } else {
                gdal_sys::OGRwkbGeometryType::wkbPoint25D
            },
            layer_options,
        )
    }

    pub fn create_multipoint(path: &str, driver_name: &str, srs_wkt: &str) -> Result<Self, String> {
        Self::create_multipoint_with_options(path, driver_name, srs_wkt, &[])
    }

    pub fn create_multipoint_with_options(
        path: &str,
        driver_name: &str,
        srs_wkt: &str,
        layer_options: &[String],
    ) -> Result<Self, String> {
        Self::create_with_geometry(
            path,
            driver_name,
            srs_wkt,
            gdal_sys::OGRwkbGeometryType::wkbMultiPoint25D,
            layer_options,
        )
    }

    /// Create a polygon-geometry writer with the layer named like the C++
    /// `density::OGR` writer (`wkbMultiPolygon` layer geometry, so it can hold
    /// both per-hexagon `wkbPolygon` density features and a `wkbMultiPolygon`
    /// boundary feature). `layer_name` mirrors the C++ `lyr_name`/`layerName`.
    pub fn create_polygon(
        path: &str,
        driver_name: &str,
        srs_wkt: &str,
        layer_name: &str,
    ) -> Result<Self, String> {
        Self::create_polygon_with_options(path, driver_name, srs_wkt, layer_name, &[])
    }

    pub fn create_polygon_with_options(
        path: &str,
        driver_name: &str,
        srs_wkt: &str,
        layer_name: &str,
        layer_options: &[String],
    ) -> Result<Self, String> {
        Self::create_with_geometry_named(
            path,
            driver_name,
            srs_wkt,
            layer_name,
            gdal_sys::OGRwkbGeometryType::wkbMultiPolygon,
            layer_options,
        )
    }

    fn create_with_geometry(
        path: &str,
        driver_name: &str,
        srs_wkt: &str,
        geometry_type: gdal_sys::OGRwkbGeometryType::Type,
        layer_options: &[String],
    ) -> Result<Self, String> {
        Self::create_with_geometry_named(
            path,
            driver_name,
            srs_wkt,
            "points",
            geometry_type,
            layer_options,
        )
    }

    fn create_with_geometry_named(
        path: &str,
        driver_name: &str,
        srs_wkt: &str,
        layer_name: &str,
        geometry_type: gdal_sys::OGRwkbGeometryType::Type,
        layer_options: &[String],
    ) -> Result<Self, String> {
        register_drivers();
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        let driver_c = CString::new(driver_name).map_err(|e| e.to_string())?;
        let layer_c = CString::new(layer_name).map_err(|e| e.to_string())?;
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
            let driver = gdal_sys::OGRGetDriverByName(driver_c.as_ptr());
            if driver.is_null() {
                return Err(format!("OGR driver '{}' not found", driver_name));
            }
            let ds =
                gdal_sys::OGR_Dr_CreateDataSource(driver, path_c.as_ptr(), std::ptr::null_mut());
            if ds.is_null() {
                return Err(format!("Failed to create OGR datasource: {}", path));
            }

            let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
            if !srs_wkt.is_empty() {
                let wkt_c = CString::new(srs_wkt).map_err(|e| e.to_string())?;
                let mut wkt_ptr = wkt_c.as_ptr() as *mut std::os::raw::c_char;
                if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != CPLErr::CE_None {
                    gdal_sys::OSRDestroySpatialReference(srs);
                    gdal_sys::OGR_DS_Destroy(ds);
                    return Err("Can't initialise OGR SRS".to_string());
                }
            }

            let layer = gdal_sys::OGR_DS_CreateLayer(
                ds,
                layer_c.as_ptr(),
                srs,
                geometry_type,
                option_ptrs.as_mut_ptr(),
            );
            gdal_sys::OSRDestroySpatialReference(srs);
            if layer.is_null() {
                gdal_sys::OGR_DS_Destroy(ds);
                return Err("Can't create OGR layer".to_string());
            }

            Ok(Self { ds, layer })
        }
    }

    pub fn create_field(&self, name: &str, field_type: VectorFieldType) -> Result<(), String> {
        let name_c = CString::new(name).map_err(|e| e.to_string())?;
        unsafe {
            let ogr_type = match field_type {
                VectorFieldType::Integer => gdal_sys::OGRFieldType::OFTInteger,
                VectorFieldType::Integer64 => gdal_sys::OGRFieldType::OFTInteger64,
                VectorFieldType::Real => gdal_sys::OGRFieldType::OFTReal,
            };
            let field = gdal_sys::OGR_Fld_Create(name_c.as_ptr(), ogr_type);
            if field.is_null() {
                return Err(format!("Can't create OGR field definition: {name}"));
            }
            let result = gdal_sys::OGR_L_CreateField(self.layer, field, 1);
            gdal_sys::OGR_Fld_Destroy(field);
            if result != gdal_sys::OGRErr::OGRERR_NONE {
                return Err(format!("Can't create OGR field: {name}"));
            }
        }
        Ok(())
    }

    pub fn write_point(
        &self,
        x: f64,
        y: f64,
        z: f64,
        measure: Option<f64>,
        fields: &[VectorFieldValue],
    ) -> Result<(), String> {
        unsafe {
            let defn = gdal_sys::OGR_L_GetLayerDefn(self.layer);
            if defn.is_null() {
                return Err("Can't get OGR layer definition".to_string());
            }
            let feature = gdal_sys::OGR_F_Create(defn);
            if feature.is_null() {
                return Err("Can't create OGR feature".to_string());
            }
            let geometry = gdal_sys::OGR_G_CreateGeometry(if measure.is_some() {
                gdal_sys::OGRwkbGeometryType::wkbPointZM
            } else {
                gdal_sys::OGRwkbGeometryType::wkbPoint25D
            });
            if geometry.is_null() {
                gdal_sys::OGR_F_Destroy(feature);
                return Err("Can't create OGR point geometry".to_string());
            }
            if let Some(measure) = measure {
                gdal_sys::OGR_G_SetPointZM(geometry, 0, x, y, z, measure);
            } else {
                gdal_sys::OGR_G_SetPoint(geometry, 0, x, y, z);
            }
            if gdal_sys::OGR_F_SetGeometryDirectly(feature, geometry)
                != gdal_sys::OGRErr::OGRERR_NONE
            {
                gdal_sys::OGR_G_DestroyGeometry(geometry);
                gdal_sys::OGR_F_Destroy(feature);
                return Err("Can't set OGR feature geometry".to_string());
            }
            for (idx, value) in fields.iter().enumerate() {
                match value {
                    VectorFieldValue::Integer(value) => {
                        gdal_sys::OGR_F_SetFieldInteger(feature, idx as i32, *value);
                    }
                    VectorFieldValue::Integer64(value) => {
                        gdal_sys::OGR_F_SetFieldInteger64(feature, idx as i32, *value);
                    }
                    VectorFieldValue::Real(value) => {
                        gdal_sys::OGR_F_SetFieldDouble(feature, idx as i32, *value);
                    }
                }
            }
            let result = gdal_sys::OGR_L_CreateFeature(self.layer, feature);
            gdal_sys::OGR_F_Destroy(feature);
            if result != gdal_sys::OGRErr::OGRERR_NONE {
                return Err("Can't create OGR feature".to_string());
            }
        }
        Ok(())
    }

    pub fn write_multipoint(&self, points: &[(f64, f64, f64)]) -> Result<(), String> {
        unsafe {
            let defn = gdal_sys::OGR_L_GetLayerDefn(self.layer);
            if defn.is_null() {
                return Err("Can't get OGR layer definition".to_string());
            }
            let feature = gdal_sys::OGR_F_Create(defn);
            if feature.is_null() {
                return Err("Can't create OGR feature".to_string());
            }
            let geometry =
                gdal_sys::OGR_G_CreateGeometry(gdal_sys::OGRwkbGeometryType::wkbMultiPoint25D);
            if geometry.is_null() {
                gdal_sys::OGR_F_Destroy(feature);
                return Err("Can't create OGR multipoint geometry".to_string());
            }
            for (x, y, z) in points {
                let point =
                    gdal_sys::OGR_G_CreateGeometry(gdal_sys::OGRwkbGeometryType::wkbPoint25D);
                if point.is_null() {
                    gdal_sys::OGR_G_DestroyGeometry(geometry);
                    gdal_sys::OGR_F_Destroy(feature);
                    return Err("Can't create OGR point geometry".to_string());
                }
                gdal_sys::OGR_G_SetPoint(point, 0, *x, *y, *z);
                if gdal_sys::OGR_G_AddGeometryDirectly(geometry, point)
                    != gdal_sys::OGRErr::OGRERR_NONE
                {
                    gdal_sys::OGR_G_DestroyGeometry(point);
                    gdal_sys::OGR_G_DestroyGeometry(geometry);
                    gdal_sys::OGR_F_Destroy(feature);
                    return Err("Can't append point to OGR multipoint".to_string());
                }
            }
            if gdal_sys::OGR_F_SetGeometryDirectly(feature, geometry)
                != gdal_sys::OGRErr::OGRERR_NONE
            {
                gdal_sys::OGR_G_DestroyGeometry(geometry);
                gdal_sys::OGR_F_Destroy(feature);
                return Err("Can't set OGR feature geometry".to_string());
            }
            let result = gdal_sys::OGR_L_CreateFeature(self.layer, feature);
            gdal_sys::OGR_F_Destroy(feature);
            if result != gdal_sys::OGRErr::OGRERR_NONE {
                return Err("Can't create OGR feature".to_string());
            }
        }
        Ok(())
    }

    /// Write one `wkbPolygon` feature from a single exterior ring, mirroring the
    /// C++ `collectHexagon` density path. The ring is closed automatically if
    /// the caller's last vertex does not repeat the first.
    pub fn write_polygon(
        &self,
        ring: &[(f64, f64)],
        fields: &[VectorFieldValue],
    ) -> Result<(), String> {
        unsafe {
            let polygon = gdal_sys::OGR_G_CreateGeometry(gdal_sys::OGRwkbGeometryType::wkbPolygon);
            if polygon.is_null() {
                return Err("Can't create OGR polygon geometry".to_string());
            }
            let linear_ring =
                gdal_sys::OGR_G_CreateGeometry(gdal_sys::OGRwkbGeometryType::wkbLinearRing);
            if linear_ring.is_null() {
                gdal_sys::OGR_G_DestroyGeometry(polygon);
                return Err("Can't create OGR linear ring geometry".to_string());
            }
            for (x, y) in ring {
                gdal_sys::OGR_G_AddPoint_2D(linear_ring, *x, *y);
            }
            // Close the ring if the caller did not already repeat the first point.
            if let (Some(first), Some(last)) = (ring.first(), ring.last()) {
                if first != last {
                    gdal_sys::OGR_G_AddPoint_2D(linear_ring, first.0, first.1);
                }
            }
            if gdal_sys::OGR_G_AddGeometryDirectly(polygon, linear_ring)
                != gdal_sys::OGRErr::OGRERR_NONE
            {
                gdal_sys::OGR_G_DestroyGeometry(linear_ring);
                gdal_sys::OGR_G_DestroyGeometry(polygon);
                return Err("Can't add ring to OGR polygon".to_string());
            }
            self.write_geometry_feature(polygon, fields)
        }
    }

    /// Write one feature whose geometry is parsed from a WKT string (used for
    /// the boundary `MULTIPOLYGON`, which already comes from the hex grid as
    /// WKT). Mirrors the C++ `writeBoundary` single-feature output.
    pub fn write_geometry_wkt(&self, wkt: &str, fields: &[VectorFieldValue]) -> Result<(), String> {
        let wkt_c = CString::new(wkt).map_err(|e| e.to_string())?;
        unsafe {
            let mut wkt_ptr = wkt_c.as_ptr() as *mut std::os::raw::c_char;
            let mut geom: gdal_sys::OGRGeometryH = std::ptr::null_mut();
            if gdal_sys::OGR_G_CreateFromWkt(&mut wkt_ptr, std::ptr::null_mut(), &mut geom)
                != gdal_sys::OGRErr::OGRERR_NONE
                || geom.is_null()
            {
                return Err(format!("Can't parse OGR geometry from WKT: {wkt}"));
            }
            self.write_geometry_feature(geom, fields)
        }
    }

    /// Attach `geometry` (taken by ownership) to a new feature with `fields` and
    /// append it to the layer. On any error the geometry/feature are destroyed.
    unsafe fn write_geometry_feature(
        &self,
        geometry: gdal_sys::OGRGeometryH,
        fields: &[VectorFieldValue],
    ) -> Result<(), String> {
        let defn = gdal_sys::OGR_L_GetLayerDefn(self.layer);
        if defn.is_null() {
            gdal_sys::OGR_G_DestroyGeometry(geometry);
            return Err("Can't get OGR layer definition".to_string());
        }
        let feature = gdal_sys::OGR_F_Create(defn);
        if feature.is_null() {
            gdal_sys::OGR_G_DestroyGeometry(geometry);
            return Err("Can't create OGR feature".to_string());
        }
        for (idx, value) in fields.iter().enumerate() {
            match value {
                VectorFieldValue::Integer(value) => {
                    gdal_sys::OGR_F_SetFieldInteger(feature, idx as i32, *value);
                }
                VectorFieldValue::Integer64(value) => {
                    gdal_sys::OGR_F_SetFieldInteger64(feature, idx as i32, *value);
                }
                VectorFieldValue::Real(value) => {
                    gdal_sys::OGR_F_SetFieldDouble(feature, idx as i32, *value);
                }
            }
        }
        if gdal_sys::OGR_F_SetGeometryDirectly(feature, geometry) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OGR_G_DestroyGeometry(geometry);
            gdal_sys::OGR_F_Destroy(feature);
            return Err("Can't set OGR feature geometry".to_string());
        }
        let result = gdal_sys::OGR_L_CreateFeature(self.layer, feature);
        gdal_sys::OGR_F_Destroy(feature);
        if result != gdal_sys::OGRErr::OGRERR_NONE {
            return Err("Can't create OGR feature".to_string());
        }
        Ok(())
    }
}

impl Drop for VectorPointWriter {
    fn drop(&mut self) {
        unsafe {
            gdal_sys::OGR_DS_Destroy(self.ds);
        }
    }
}
