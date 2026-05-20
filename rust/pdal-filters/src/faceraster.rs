//! `filters.faceraster` -- rasterize triangular mesh faces.

use pdal_core::options::Options;
use pdal_core::point::{DimId, PointId, PointView, Triangle};
use pdal_core::raster::{RasterData, RasterLimits};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct FaceRasterFilter {
    resolution: f64,
    origin_x: f64,
    origin_y: f64,
    width: usize,
    height: usize,
    fixed_arg_count: usize,
    no_data: f64,
    max_triangle_edge_length: f64,
    mesh_name: String,
}

impl FaceRasterFilter {
    pub fn new(options: &Options) -> Self {
        Self {
            resolution: options.get_f64("resolution", 1.0),
            origin_x: options.get_f64("origin_x", 0.0),
            origin_y: options.get_f64("origin_y", 0.0),
            width: options.get_u64("width", 0) as usize,
            height: options.get_u64("height", 0) as usize,
            fixed_arg_count: ["origin_x", "origin_y", "width", "height"]
                .into_iter()
                .filter(|key| options.has(key))
                .count(),
            no_data: options.get_f64("nodata", f64::NAN),
            max_triangle_edge_length: options.get_f64("max_triangle_edge_length", f64::INFINITY),
            mesh_name: options.get_str("mesh", ""),
        }
    }

    fn limits(&self, input: &PointView) -> Result<RasterLimits, StageError> {
        if self.resolution <= 0.0 {
            return Err(StageError(
                "FaceRasterFilter resolution must be positive.".to_string(),
            ));
        }
        if self.fixed_arg_count != 0 && self.fixed_arg_count != 4 {
            return Err(StageError(
                "Must specify all or none of 'origin_x', 'origin_y', 'width' and 'height'."
                    .to_string(),
            ));
        }
        if self.fixed_arg_count == 4 {
            return Ok(RasterLimits::new(
                self.origin_x,
                self.origin_y,
                self.width,
                self.height,
                self.resolution,
            ));
        }

        let bounds = input
            .calculate_bounds_2d()
            .ok_or_else(|| StageError("Unable to compute raster limits.".to_string()))?;
        let half_edge = self.resolution / 2.0;
        let x_origin = bounds.minx - half_edge;
        let y_origin = bounds.miny - half_edge;
        let width = (((bounds.maxx - x_origin) / self.resolution) + 1.0) as usize;
        let height = (((bounds.maxy - y_origin) / self.resolution) + 1.0) as usize;
        Ok(RasterLimits::new(
            x_origin,
            y_origin,
            width,
            height,
            self.resolution,
        ))
    }
}

impl Filter for FaceRasterFilter {
    fn name(&self) -> &str {
        "filters.faceraster"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        if input.raster("faceraster").is_some() {
            return Err(StageError("Raster already exists".to_string()));
        }
        let Some(mesh) = input.mesh_named(&self.mesh_name) else {
            return Err(StageError(format!(
                "Mesh '{}' does not exist.",
                self.mesh_name
            )));
        };

        let limits = self.limits(input)?;
        let mut raster = RasterData::new("faceraster", limits, self.no_data);
        for triangle in mesh.triangles() {
            rasterize_triangle(input, &mut raster, triangle, self.max_triangle_edge_length);
        }

        let mut output = input.clone();
        output.add_raster(raster);
        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for FaceRasterFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

fn rasterize_triangle(
    input: &PointView,
    raster: &mut RasterData,
    triangle: &Triangle,
    max_edge_length: f64,
) {
    let p1 = point(input, triangle.a);
    let p2 = point(input, triangle.b);
    let p3 = point(input, triangle.c);
    if !max_edge_length.is_infinite()
        && (edge_length(p1, p2) > max_edge_length
            || edge_length(p2, p3) > max_edge_length
            || edge_length(p1, p3) > max_edge_length)
    {
        return;
    }

    let xmin = p1.0.min(p2.0).min(p3.0);
    let xmax = p1.0.max(p2.0).max(p3.0);
    let ymin = p1.1.min(p2.1).min(p3.1);
    let ymax = p1.1.max(p2.1).max(p3.1);
    let limits = raster.limits().clone();
    let half_edge = limits.edge_length / 2.0;
    let edge_bit = limits.edge_length * 0.000001;

    let ax = clamp_cell(limits.x_cell(xmin + half_edge - edge_bit), limits.width);
    let bx = clamp_cell(limits.x_cell(xmax + half_edge), limits.width);
    let ay = clamp_cell(limits.y_cell(ymin + half_edge - edge_bit), limits.height);
    let by = clamp_cell(limits.y_cell(ymax + half_edge), limits.height);

    for xi in ax..bx {
        for yi in ay..by {
            let current = raster.get_cell(xi, yi);
            if raster.initializer().is_nan() {
                if !current.is_nan() {
                    continue;
                }
            } else if current != raster.initializer() {
                continue;
            }

            let x = limits.x_cell_pos(xi);
            let y = limits.y_cell_pos(yi);
            let value = barycentric_interpolation(p1, p2, p3, x, y);
            if value.is_finite() {
                raster.set_cell(xi, yi, value);
            }
        }
    }
}

fn point(view: &PointView, idx: u64) -> (f64, f64, f64) {
    (
        view.get_f64(idx, &DimId::X),
        view.get_f64(idx, &DimId::Y),
        view.get_f64(idx, &DimId::Z),
    )
}

fn edge_length(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

fn clamp_cell(value: isize, upper: usize) -> usize {
    value.clamp(0, upper as isize) as usize
}

fn mag2(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1)
}

fn barycentric_interpolation(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    p3: (f64, f64, f64),
    x: f64,
    y: f64,
) -> f64 {
    let area_total = ((p2.0 - p1.0) * (p3.1 - p2.1)) - ((p2.1 - p1.1) * (p3.0 - p2.0));
    if area_total == 0.0 {
        return f64::INFINITY;
    }

    let sign_total = area_total.is_sign_negative();
    let almost_zero = 1e-14;
    let mut area12 = (p2.0 - p1.0) * (y - p1.1) - (p2.1 - p1.1) * (x - p1.0);
    if area12 != 0.0 && area12.is_sign_negative() != sign_total {
        let magsum = mag2(p1.0, p1.1, p2.0, p2.1) + mag2(p1.0, p1.1, x, y);
        if (area12 / magsum).abs() > almost_zero {
            return f64::INFINITY;
        }
        area12 = 0.0;
    }

    let mut area23 = (p3.0 - p2.0) * (y - p2.1) - (p3.1 - p2.1) * (x - p2.0);
    if area23 != 0.0 && area23.is_sign_negative() != sign_total {
        let magsum = mag2(p3.0, p3.1, p2.0, p2.1) + mag2(p3.0, p3.1, x, y);
        if (area23 / magsum).abs() > almost_zero {
            return f64::INFINITY;
        }
        area23 = 0.0;
    }

    let mut area31 = (p1.0 - p3.0) * (y - p3.1) - (p1.1 - p3.1) * (x - p3.0);
    if area31 != 0.0 && area31.is_sign_negative() != sign_total {
        let magsum = mag2(p3.0, p3.1, p1.0, p1.1) + mag2(p3.0, p3.1, x, y);
        if (area31 / magsum).abs() > almost_zero {
            return f64::INFINITY;
        }
        area31 = 0.0;
    }

    (area12 * p3.2 + area23 * p1.2 + area31 * p2.2) / area_total
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn triangle_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in [(0.0, 0.0, 0.0), (2.0, 0.0, 2.0), (0.0, 2.0, 4.0)] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, x);
            view.set_f64(point, &DimId::Y, y);
            view.set_f64(point, &DimId::Z, z);
        }
        view.create_mesh().add(0, 1, 2);
        view
    }

    #[test]
    fn rasterizes_triangle_centers_into_faceraster_attachment() {
        let mut options = Options::new();
        options.add("resolution", 1.0);
        options.add("origin_x", 0.0);
        options.add("origin_y", 0.0);
        options.add("width", 2);
        options.add("height", 2);
        options.add("nodata", -9999.0);
        let mut filter = FaceRasterFilter::new(&options);

        let output = filter.run_one(&triangle_view()).unwrap().pop().unwrap();
        let raster = output.raster("faceraster").unwrap();

        assert_eq!(raster.data(), &[3.5, -9999.0, 1.5, 2.5]);
    }

    #[test]
    fn max_triangle_edge_length_skips_large_faces() {
        let mut options = Options::new();
        options.add("resolution", 1.0);
        options.add("origin_x", 0.0);
        options.add("origin_y", 0.0);
        options.add("width", 2);
        options.add("height", 2);
        options.add("nodata", -9999.0);
        options.add("max_triangle_edge_length", 1.0);
        let mut filter = FaceRasterFilter::new(&options);

        let output = filter.run_one(&triangle_view()).unwrap().pop().unwrap();

        assert_eq!(output.raster("faceraster").unwrap().data(), &[-9999.0; 4]);
    }

    #[test]
    fn mesh_option_selects_a_named_mesh() {
        let mut view = triangle_view();
        view.create_named_mesh("empty").unwrap();

        let mut options = Options::new();
        options.add("resolution", 1.0);
        options.add("origin_x", 0.0);
        options.add("origin_y", 0.0);
        options.add("width", 2);
        options.add("height", 2);
        options.add("nodata", -9999.0);
        options.add("mesh", "empty");
        let mut filter = FaceRasterFilter::new(&options);

        let output = filter.run_one(&view).unwrap().pop().unwrap();

        assert_eq!(output.raster("faceraster").unwrap().data(), &[-9999.0; 4]);
    }
}
