//! `writers.copc` -- the COPC writer stage.
//!
//! Ties the COPC subsystem together: compute data stats, size the octree
//! [`Grid`], bin points into finest-level cells, build the octree with
//! [`Pyramid`], generate the SRS/eb VLRs, and assemble the file with
//! [`output::write_copc`]. Port of the orchestration in
//! `io/private/copcwriter/BuPyramid.cpp` + `io/CopcWriter.cpp`.

use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;

use crate::las_writer::copc_extra_dims;

use super::cell_manager::CellManager;
use super::common::ROOT_CELL_COUNT;
use super::grid::Grid;
use super::output::{self, CopcWriteParams, RawVlr};
use super::pyramid::Pyramid;

const WKT_RECORD_ID: u16 = 2112;
const WKT2_RECORD_ID: u16 = 4224;
const PROJJSON_RECORD_ID: u16 = 4225;
const TRANSFORM_USER_ID: &str = "LASF_Projection";
const LIBLAS_USER_ID: &str = "liblas";
const PDAL_USER_ID: &str = "PDAL";

pub struct CopcWriter {
    options: Options,
}

impl CopcWriter {
    pub fn new(options: &Options) -> Self {
        CopcWriter {
            options: options.clone(),
        }
    }

    fn point_format(&self) -> u8 {
        ["dataformat_id", "format", "point_format"]
            .into_iter()
            .find_map(|key| {
                self.options
                    .value(key)
                    .and_then(|v| v.trim().parse::<u8>().ok())
            })
            .unwrap_or(3)
    }
}

/// Conforming data extents plus GPS-time range, gathered over all input points.
struct Stats {
    bounds: [f64; 6], // minx, miny, minz, maxx, maxy, maxz
    gps_min: f64,
    gps_max: f64,
    total: u64,
}

fn gather_stats(views: &[PointView]) -> Stats {
    let mut minx = f64::INFINITY;
    let mut miny = f64::INFINITY;
    let mut minz = f64::INFINITY;
    let mut maxx = f64::NEG_INFINITY;
    let mut maxy = f64::NEG_INFINITY;
    let mut maxz = f64::NEG_INFINITY;
    let mut gps_min = f64::INFINITY;
    let mut gps_max = f64::NEG_INFINITY;
    let mut total = 0u64;

    for view in views {
        let has_gps = view.layout().dim(&DimId::GpsTime).is_some();
        for idx in 0..view.len() {
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            let z = view.get_f64(idx, &DimId::Z);
            minx = minx.min(x);
            miny = miny.min(y);
            minz = minz.min(z);
            maxx = maxx.max(x);
            maxy = maxy.max(y);
            maxz = maxz.max(z);
            if has_gps {
                let g = view.get_f64(idx, &DimId::GpsTime);
                gps_min = gps_min.min(g);
                gps_max = gps_max.max(g);
            }
            total += 1;
        }
    }

    if total == 0 {
        return Stats {
            bounds: [0.0; 6],
            gps_min: 0.0,
            gps_max: 0.0,
            total: 0,
        };
    }
    Stats {
        bounds: [minx, miny, minz, maxx, maxy, maxz],
        gps_min: if gps_min.is_finite() { gps_min } else { 0.0 },
        gps_max: if gps_max.is_finite() { gps_max } else { 0.0 },
        total,
    }
}

impl Writer for CopcWriter {
    fn name(&self) -> &str {
        "writers.copc"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        let filename = self.options.get_str("filename", "");
        if filename.is_empty() {
            return Err(StageError(
                "CopcWriter requires a filename option.".to_string(),
            ));
        }
        let point_format = self.point_format();

        let (extra_dims, eb_vlr_data) = copc_extra_dims(&self.options, views, point_format)?;
        let num_extra_bytes: u16 = extra_dims.iter().map(|d| d.size).sum::<usize>() as u16;

        let stats = gather_stats(views);
        let conforming = pdal_core::bounds::Bounds3D {
            minx: stats.bounds[0],
            miny: stats.bounds[1],
            minz: stats.bounds[2],
            maxx: stats.bounds[3],
            maxy: stats.bounds[4],
            maxz: stats.bounds[5],
        };

        // Octree grid over the cubic bounds.
        let grid = Grid::new(conforming, stats.total as usize);
        let cube = grid.cubic_bounds();
        let auto_offset = grid.offset();

        // scale/offset come from options (scale defaults to 0.01; offset
        // defaults to the data-centered value), matching writers.copc.
        let scale = [
            self.options.get_f64("scale_x", 0.01),
            self.options.get_f64("scale_y", 0.01),
            self.options.get_f64("scale_z", 0.01),
        ];
        let offset = [
            self.options.get_f64("offset_x", auto_offset[0]),
            self.options.get_f64("offset_y", auto_offset[1]),
            self.options.get_f64("offset_z", auto_offset[2]),
        ];

        let halfsize = (cube.maxx - cube.minx) / 2.0;
        let center = [
            cube.minx + halfsize,
            cube.miny + halfsize,
            cube.minz + halfsize,
        ];
        let spacing = (2.0 * halfsize) / ROOT_CELL_COUNT as f64;

        // Bin every point into its finest-level cell.
        let template = views
            .first()
            .cloned()
            .map(|v| v.make_new())
            .unwrap_or_else(|| {
                PointView::new(std::rc::Rc::new(pdal_core::point::PointLayout::new()))
            });
        let mut cells = CellManager::new(template);
        for view in views {
            for idx in 0..view.len() {
                let x = view.get_f64(idx, &DimId::X);
                let y = view.get_f64(idx, &DimId::Y);
                let z = view.get_f64(idx, &DimId::Z);
                let key = grid.key(x, y, z);
                cells.get(key).append_point(view, idx);
            }
        }

        // Build the octree.
        let result = Pyramid::new(cube, 1234).run(cells);

        // SRS VLRs.
        let extra_vlrs = self.srs_vlrs(views);
        let mut extra_vlrs = extra_vlrs;
        if let Some(data) = eb_vlr_data {
            extra_vlrs.push(RawVlr {
                user_id: "LASF_Spec".to_string(),
                record_id: 4,
                description: "Extra Bytes Record".to_string(),
                data,
            });
        }

        let params = CopcWriteParams {
            point_format,
            num_extra_bytes,
            extra_dims,
            scale,
            offset,
            bounds: stats.bounds,
            center,
            halfsize,
            spacing,
            gpstime_min: stats.gps_min,
            gpstime_max: stats.gps_max,
            file_source_id: self.options.get_u64("filesource_id", 0) as u16,
            global_encoding: self.options.get_u64("global_encoding", 0) as u16,
            creation_day: self.options.get_u64("creation_doy", 0) as u16,
            creation_year: self.options.get_u64("creation_year", 0) as u16,
            system_id: "PDAL".to_string(),
            software_id: "pdal-rs (copc)".to_string(),
            extra_vlrs,
        };

        output::write_copc(&filename, &params, &result.chunks, &result.child_counts)
            .map_err(StageError)?;
        Ok(())
    }
}

impl CopcWriter {
    /// SRS VLRs from `a_srs` (or the view's SRS). With `enhanced_srs_vlrs`,
    /// writes WKT2 (4224) + PROJJSON (4225) + WKT1 (2112, two variants);
    /// otherwise just WKT1.
    fn srs_vlrs(&self, views: &[PointView]) -> Vec<RawVlr> {
        let a_srs = self.options.get_str("a_srs", "");
        let srs_text = if !a_srs.is_empty() {
            a_srs
        } else {
            views
                .first()
                .map(|v| v.spatial_reference())
                .filter(|s| !s.is_empty())
                .map(|s| s.wkt().to_string())
                .unwrap_or_default()
        };
        if srs_text.is_empty() {
            return Vec::new();
        }
        let Ok(srs) = pdal_native::srs::user_input_to_wkt(&srs_text) else {
            return Vec::new();
        };
        let enhanced = self.options.get_bool("enhanced_srs_vlrs", false);

        let nt = |s: &str| {
            let mut b = s.as_bytes().to_vec();
            b.push(0);
            b
        };
        let mut vlrs = Vec::new();
        if enhanced {
            if !srs.wkt2.is_empty() {
                vlrs.push(RawVlr {
                    user_id: TRANSFORM_USER_ID.to_string(),
                    record_id: WKT2_RECORD_ID,
                    description: "PDAL WKT2 Record".to_string(),
                    data: nt(&srs.wkt2),
                });
            }
            if !srs.projjson.is_empty() {
                vlrs.push(RawVlr {
                    user_id: PDAL_USER_ID.to_string(),
                    record_id: PROJJSON_RECORD_ID,
                    description: "PDAL PROJJSON Record".to_string(),
                    data: nt(&srs.projjson),
                });
            }
        }
        // WKT1 (record 2112), in both the Transform and liblas variants, as the
        // C++ writer does.
        let wkt1 = if srs.wkt.is_empty() {
            srs.wkt2.clone()
        } else {
            srs.wkt.clone()
        };
        if !wkt1.is_empty() {
            vlrs.push(RawVlr {
                user_id: TRANSFORM_USER_ID.to_string(),
                record_id: WKT_RECORD_ID,
                description: "OGC Transformation Record".to_string(),
                data: nt(&wkt1),
            });
            vlrs.push(RawVlr {
                user_id: LIBLAS_USER_ID.to_string(),
                record_id: WKT_RECORD_ID,
                description: "OGR variant of OpenGIS WKT SRS".to_string(),
                data: nt(&wkt1),
            });
        }
        vlrs
    }
}
