use pdal_core::raster::RasterLimits;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_raster_limits_t {
    pub x_origin: f64,
    pub y_origin: f64,
    pub width: u64,
    pub height: u64,
    pub edge_length: f64,
}

impl From<RasterLimits> for pdal_raster_limits_t {
    fn from(value: RasterLimits) -> Self {
        Self {
            x_origin: value.x_origin,
            y_origin: value.y_origin,
            width: value.width as u64,
            height: value.height as u64,
            edge_length: value.edge_length,
        }
    }
}

impl From<pdal_raster_limits_t> for RasterLimits {
    fn from(value: pdal_raster_limits_t) -> Self {
        RasterLimits::new(
            value.x_origin,
            value.y_origin,
            value.width as usize,
            value.height as usize,
            value.edge_length,
        )
    }
}

#[no_mangle]
pub extern "C" fn pdal_raster_limits_valid(limits: pdal_raster_limits_t) -> bool {
    limits.width > 0 && limits.height > 0
}

#[no_mangle]
pub unsafe extern "C" fn pdal_raster_limits_x_cell(
    limits: pdal_raster_limits_t,
    x: f64,
    out_ok: *mut bool,
) -> i32 {
    raster_cell((x - limits.x_origin) / limits.edge_length, out_ok)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_raster_limits_y_cell(
    limits: pdal_raster_limits_t,
    y: f64,
    out_ok: *mut bool,
) -> i32 {
    raster_cell((y - limits.y_origin) / limits.edge_length, out_ok)
}

#[no_mangle]
pub extern "C" fn pdal_raster_limits_x_cell_pos(limits: pdal_raster_limits_t, x: u64) -> f64 {
    limits.x_origin + (x as f64 + 0.5) * limits.edge_length
}

#[no_mangle]
pub extern "C" fn pdal_raster_limits_y_cell_pos(limits: pdal_raster_limits_t, y: u64) -> f64 {
    limits.y_origin + (y as f64 + 0.5) * limits.edge_length
}

unsafe fn raster_cell(cell: f64, out_ok: *mut bool) -> i32 {
    let floored = cell.floor();
    let ok = floored >= f64::from(i32::MIN) && floored + 1.0 <= f64::from(i32::MAX);
    if let Some(out_ok) = out_ok.as_mut() {
        *out_ok = ok;
    }
    floored as i32
}
