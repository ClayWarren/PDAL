//! `filters.crop` -- filter points inside or outside a bounding box or polygon.

use pdal_core::geometry::Geometry;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct CropFilter {
    outside: bool,
    bounds: Vec<(f64, f64, f64, f64, f64, f64)>, // minx, miny, minz, maxx, maxy, maxz
    polygons: Vec<Geometry>,
    centers: Vec<(f64, f64, f64)>,
    distance: f64,
}

impl CropFilter {
    pub fn new(
        outside: bool,
        bounds: Vec<(f64, f64, f64, f64, f64, f64)>,
        polygons_wkt: Vec<String>,
        centers: Vec<(f64, f64, f64)>,
        distance: f64,
    ) -> Result<Self, StageError> {
        let mut polygons = Vec::new();
        for wkt in polygons_wkt {
            polygons.push(Geometry::from_wkt(&wkt).map_err(StageError)?);
        }
        Ok(CropFilter {
            outside,
            bounds,
            polygons,
            centers,
            distance,
        })
    }

    fn check_point(&self, x: f64, y: f64, z: f64) -> bool {
        let mut inside = false;

        // Check bounds
        for b in &self.bounds {
            if x >= b.0 && x <= b.3 && y >= b.1 && y <= b.4 && z >= b.2 && z <= b.5 {
                inside = true;
                break;
            }
        }

        if !inside {
            // Check polygons
            for p in &self.polygons {
                if p.contains(x, y) {
                    inside = true;
                    break;
                }
            }
        }

        if !inside {
            // Check centers
            let dist2 = self.distance * self.distance;
            for c in &self.centers {
                let dx = x - c.0;
                let dy = y - c.1;
                let dz = z - c.2;
                if dx * dx + dy * dy + dz * dz <= dist2 {
                    inside = true;
                    break;
                }
            }
        }

        self.outside != inside
    }
}

impl Filter for CropFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.crop"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = input.make_new();
        for idx in 0..input.len() {
            if self.process_one(&mut input.clone(), idx) {
                output.append_point(input, idx);
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for CropFilter {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);

        self.check_point(x, y, z)
    }

    fn reset(&mut self) {}
}
