//! C ABI for the `tile` kernel.

use crate::error::{clear_last_error, set_last_error};
use crate::metrics_abi::read_cloud;
use crate::registry::create_writer;
use pdal_core::options::Options;
use pdal_filters::splitter::SplitterFilter;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Tile a point cloud into a regular grid, writing one file per occupied
/// cell.
///
/// `output_template` must contain exactly one `#`, which is replaced by
/// `<xpos>_<ypos>` for each tile. `origin_x` and `origin_y` may be NaN, in
/// which case the first point's coordinate is used as the grid origin.
///
/// Returns the number of tiles written, or -1 on error with the message
/// available via `pdal_last_error`.
///
/// # Safety
///
/// `input_path` and `output_template` must be valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_tile(
    input_path: *const c_char,
    output_template: *const c_char,
    length: f64,
    origin_x: f64,
    origin_y: f64,
    buffer: f64,
) -> i32 {
    clear_last_error();
    if input_path.is_null() || output_template.is_null() {
        set_last_error("null argument to pdal_tile");
        return -1;
    }
    let input_path = CStr::from_ptr(input_path).to_string_lossy().into_owned();
    let template = CStr::from_ptr(output_template)
        .to_string_lossy()
        .into_owned();

    if template.matches('#').count() != 1 {
        set_last_error("tile: output template must contain exactly one '#' placeholder");
        return -1;
    }
    if length <= 0.0 || !length.is_finite() {
        set_last_error("tile: 'length' must be a positive number");
        return -1;
    }

    let writer_driver = match pdal_core::driver::infer_writer_driver(&template) {
        Some(driver) => driver,
        None => {
            set_last_error(format!(
                "tile: unable to infer a writer driver for '{template}'"
            ));
            return -1;
        }
    };

    let view = match read_cloud(&input_path) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return -1;
        }
    };
    if view.is_empty() {
        set_last_error("tile requires a non-empty point cloud");
        return -1;
    }

    let mut splitter = SplitterFilter::new(length, origin_x, origin_y, buffer);
    let tiles = match splitter.split(&view) {
        Ok(tiles) => tiles,
        Err(err) => {
            set_last_error(err.to_string());
            return -1;
        }
    };

    for ((xpos, ypos), tile_view) in &tiles {
        let filename = template.replacen('#', &format!("{xpos}_{ypos}"), 1);
        let mut options = Options::new();
        options.add("filename", filename.as_str());
        let mut writer = match create_writer(writer_driver, &options) {
            Ok(writer) => writer,
            Err(err) => {
                set_last_error(err.to_string());
                return -1;
            }
        };
        if let Err(err) = writer.write(std::slice::from_ref(tile_view)) {
            set_last_error(err.to_string());
            return -1;
        }
    }

    tiles.len() as i32
}
