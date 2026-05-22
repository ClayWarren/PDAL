use pdal_core::options::Options;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::HashMap;

type Voxel = (i32, i32, i32);
type Coord = (f64, f64, f64);

pub struct SampleFilter {
    pub cell: Option<f64>,
    pub radius: Option<f64>,
    pub dimension_name: Option<String>,
    pub origin_x: Option<f64>,
    pub origin_y: Option<f64>,
    pub origin_z: Option<f64>,

    // runtime state
    cell_val: f64,
    radius_sqr: f64,
    origin: Option<(f64, f64, f64)>,
    populated_voxels: HashMap<Voxel, Vec<Coord>>,
}

impl SampleFilter {
    pub fn new(ops: &Options) -> Result<Self, String> {
        let cell = if ops.has("cell") {
            Some(ops.get_f64("cell", 0.0))
        } else {
            None
        };
        let radius = if ops.has("radius") {
            Some(ops.get_f64("radius", 0.0))
        } else {
            None
        };
        if cell.is_some() && radius.is_some() {
            return Err("Must set only one of 'cell' or 'radius'.".into());
        }
        if cell.is_none() && radius.is_none() {
            return Err("Must set 'cell' or 'radius' but not both.".into());
        }
        let dimension_name = if ops.has("dimension") {
            Some(ops.get_str("dimension", ""))
        } else {
            None
        };
        let origin_x = if ops.has("origin_x") {
            Some(ops.get_f64("origin_x", 0.0))
        } else {
            None
        };
        let origin_y = if ops.has("origin_y") {
            Some(ops.get_f64("origin_y", 0.0))
        } else {
            None
        };
        let origin_z = if ops.has("origin_z") {
            Some(ops.get_f64("origin_z", 0.0))
        } else {
            None
        };

        Ok(Self {
            cell,
            radius,
            dimension_name,
            origin_x,
            origin_y,
            origin_z,
            cell_val: 0.0,
            radius_sqr: 0.0,
            origin: None,
            populated_voxels: HashMap::new(),
        })
    }

    pub fn prepare_runtime(&mut self) {
        let mut r = self.radius.unwrap_or(0.0);
        let mut c = self.cell.unwrap_or(0.0);

        if self.cell.is_some() {
            r = c / 2.0 * 3.0f64.sqrt();
        } else if self.radius.is_some() {
            c = 2.0 * r / 3.0f64.sqrt();
        }

        self.cell_val = c;
        self.radius_sqr = r * r;
        self.populated_voxels.clear();
        self.origin = None;
    }

    fn voxelize(&mut self, x: f64, y: f64, z: f64) -> bool {
        if self.populated_voxels.is_empty() {
            let ox = self.origin_x.unwrap_or(x);
            let oy = self.origin_y.unwrap_or(y);
            let oz = self.origin_z.unwrap_or(y); // replicate C++ Copy-Paste Bug (y instead of z)
            self.origin = Some((ox, oy, oz));
        }

        let (ox, oy, oz) = self.origin.unwrap();

        let vx = ((x - ox) / self.cell_val).floor() as i32;
        let vy = ((y - oy) / self.cell_val).floor() as i32;
        let vz = ((z - oz) / self.cell_val).floor() as i32;

        let v = (vx, vy, vz);

        // Check center voxel
        if let Some(coords) = self.populated_voxels.get(&v) {
            for &coord in coords {
                let dx = coord.0 - x;
                let dy = coord.1 - y;
                let dz = coord.2 - z;
                let dist_sqr = dx * dx + dy * dy + dz * dz;
                if dist_sqr < self.radius_sqr {
                    return false;
                }
            }
        }

        // Check neighbors
        for xi in (vx - 1)..=(vx + 1) {
            for yi in (vy - 1)..=(vy + 1) {
                for zi in (vz - 1)..=(vz + 1) {
                    let candidate = (xi, yi, zi);
                    if candidate == v {
                        continue;
                    }
                    if let Some(coords) = self.populated_voxels.get(&candidate) {
                        for &coord in coords {
                            let dx = coord.0 - x;
                            let dy = coord.1 - y;
                            let dz = coord.2 - z;
                            let dist_sqr = dx * dx + dy * dy + dz * dz;
                            if dist_sqr < self.radius_sqr {
                                return false;
                            }
                        }
                    }
                }
            }
        }

        // Insert
        self.populated_voxels.entry(v).or_default().push((x, y, z));
        true
    }
}

impl Filter for SampleFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.sample"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        self.prepare_runtime();

        let mut output = PointView::new(view.layout().clone());
        let size = view.len();

        let opt_dim = self
            .dimension_name
            .as_ref()
            .map(|name| DimId::from_name(name));

        if let Some(ref dim) = opt_dim {
            // we keep all points, but set the flag
            for i in 0..size {
                let x = view.get_f64(i, &DimId::X);
                let y = view.get_f64(i, &DimId::Y);
                let z = view.get_f64(i, &DimId::Z);

                let keep = self.voxelize(x, y, z);
                output.append_point(view, i);
                let out_idx = output.len() - 1;
                output.set_f64(out_idx, dim, if keep { 1.0 } else { 0.0 });
            }
        } else {
            // standard culling
            for i in 0..size {
                let x = view.get_f64(i, &DimId::X);
                let y = view.get_f64(i, &DimId::Y);
                let z = view.get_f64(i, &DimId::Z);

                if self.voxelize(x, y, z) {
                    output.append_point(view, i);
                }
            }
        }

        Ok(vec![output])
    }
}

impl Streamable for SampleFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {
        self.populated_voxels.clear();
        self.origin = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn options(pairs: &[(&str, &str)]) -> Options {
        let mut options = Options::new();
        for (key, value) in pairs {
            options.add(key, *value);
        }
        options
    }

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Flag, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn radius_mode_keeps_points_outside_the_poisson_radius() {
        let input = view(&[(0.0, 0.0, 0.0), (0.25, 0.0, 0.0), (2.0, 0.0, 0.0)]);
        let mut filter = SampleFilter::new(&options(&[("radius", "1.0")])).unwrap();

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(out.len(), 2);
        assert_eq!(out.get_f64(0, &DimId::X), 0.0);
        assert_eq!(out.get_f64(1, &DimId::X), 2.0);
    }

    #[test]
    fn dimension_mode_marks_every_point_instead_of_culling() {
        let input = view(&[(0.0, 0.0, 0.0), (0.25, 0.0, 0.0), (2.0, 0.0, 0.0)]);
        let mut filter =
            SampleFilter::new(&options(&[("radius", "1.0"), ("dimension", "Flag")])).unwrap();

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(out.len(), 3);
        assert_eq!(out.get_f64(0, &DimId::Flag), 1.0);
        assert_eq!(out.get_f64(1, &DimId::Flag), 0.0);
        assert_eq!(out.get_f64(2, &DimId::Flag), 1.0);
    }

    #[test]
    fn cell_option_derives_radius_and_origin_options_anchor_voxels() {
        let input = view(&[(10.0, 10.0, 10.0), (10.1, 10.1, 10.1), (20.0, 20.0, 20.0)]);
        let mut filter = SampleFilter::new(&options(&[
            ("cell", "2.0"),
            ("origin_x", "0.0"),
            ("origin_y", "0.0"),
            ("origin_z", "0.0"),
        ]))
        .unwrap();

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(out.len(), 2);
        assert_eq!(out.get_f64(0, &DimId::X), 10.0);
        assert_eq!(out.get_f64(1, &DimId::X), 20.0);

        filter.reset();
        assert!(filter.populated_voxels.is_empty());
        assert!(filter.origin.is_none());
    }

    #[test]
    fn streaming_is_not_supported() {
        let mut filter = SampleFilter::new(&options(&[("radius", "1.0")])).unwrap();
        let mut input = view(&[(0.0, 0.0, 0.0)]);

        assert!(!filter.process_one(&mut input, 0));
    }
}
