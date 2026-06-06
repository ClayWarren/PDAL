//! C ABI for the `tile` kernel.

use crate::error::{clear_last_error, set_last_error};
use crate::metrics_abi::read_cloud;
use crate::registry::create_writer;
use pdal_core::options::Options;
use pdal_core::point::PointView;
use pdal_core::stage::Filter;
use pdal_core::utils::{expand_local_glob, has_glob_pattern};
use pdal_filters::reprojection::ReprojectionFilter;
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

    let writer_options = Options::new();
    match tile_file(TileRequest {
        input_path: &input_path,
        output_template: &template,
        length,
        origin_x,
        origin_y,
        buffer,
        out_srs: None,
        writer_options: &writer_options,
    }) {
        Ok(count) => count,
        Err(err) => {
            set_last_error(err.to_string());
            -1
        }
    }
}

pub(crate) struct TileRequest<'a> {
    pub input_path: &'a str,
    pub output_template: &'a str,
    pub length: f64,
    pub origin_x: f64,
    pub origin_y: f64,
    pub buffer: f64,
    pub out_srs: Option<&'a str>,
    pub writer_options: &'a Options,
}

pub(crate) fn tile_file(request: TileRequest<'_>) -> Result<i32, pdal_core::stage::StageError> {
    if request.output_template.matches('#').count() != 1 {
        return Err(pdal_core::stage::StageError(
            "tile: output template must contain exactly one '#' placeholder".to_string(),
        ));
    }
    if request.length <= 0.0 || !request.length.is_finite() {
        return Err(pdal_core::stage::StageError(
            "tile: 'length' must be a positive number".to_string(),
        ));
    }

    let writer_driver = pdal_core::driver::infer_writer_driver(request.output_template)
        .ok_or_else(|| {
            pdal_core::stage::StageError(format!(
                "tile: unable to infer a writer driver for '{}'",
                request.output_template
            ))
        })?;

    let view = read_tile_input(request.input_path, request.out_srs)?;
    if view.is_empty() {
        return Err(pdal_core::stage::StageError(
            "tile requires a non-empty point cloud".to_string(),
        ));
    }

    let mut splitter = SplitterFilter::new(
        request.length,
        request.origin_x,
        request.origin_y,
        request.buffer,
    );
    let tiles = splitter.split(&view)?;

    for ((xpos, ypos), tile_view) in &tiles {
        let filename = request
            .output_template
            .replacen('#', &format!("{xpos}_{ypos}"), 1);
        let mut options = request.writer_options.clone();
        options.add("filename", filename.as_str());
        let mut writer = create_writer(writer_driver, &options)?;
        writer.write(std::slice::from_ref(tile_view))?;
    }

    Ok(tiles.len() as i32)
}

fn read_tile_input(
    path: &str,
    out_srs: Option<&str>,
) -> Result<PointView, pdal_core::stage::StageError> {
    if !has_glob_pattern(path) {
        let view = read_cloud(path)?;
        return reproject_tile_view(view, out_srs);
    }

    let mut output: Option<PointView> = None;
    for file in expand_local_glob(path).map_err(pdal_core::stage::StageError)? {
        let file = file.to_string_lossy();
        let view = read_cloud(&file).map_err(|err| {
            pdal_core::stage::StageError(format!("tile: failed to read '{file}': {err}"))
        })?;
        let view = reproject_tile_view(view, out_srs)?;
        append_tile_view(&mut output, &view, path)?;
    }
    output.ok_or_else(|| {
        pdal_core::stage::StageError(format!(
            "tile: glob pattern '{path}' did not match any files"
        ))
    })
}

fn reproject_tile_view(
    view: PointView,
    out_srs: Option<&str>,
) -> Result<PointView, pdal_core::stage::StageError> {
    let Some(out_srs) = out_srs.filter(|srs| !srs.is_empty()) else {
        return Ok(view);
    };
    let mut reprojection = ReprojectionFilter::new(out_srs, None, true);
    let mut views = reprojection.run_one(&view)?;
    Ok(views.remove(0))
}

fn append_tile_view(
    output: &mut Option<PointView>,
    view: &PointView,
    path: &str,
) -> Result<(), pdal_core::stage::StageError> {
    let Some(merged) = output else {
        *output = Some(view.clone());
        return Ok(());
    };
    if merged.layout().dim_count() != view.layout().dim_count()
        || merged.layout().point_size() != view.layout().point_size()
    {
        return Err(pdal_core::stage::StageError(format!(
            "tile: glob '{path}' produced incompatible point layouts"
        )));
    }
    for idx in 0..view.len() {
        merged.append_point(view, idx);
    }
    Ok(())
}
