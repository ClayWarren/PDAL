use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::rc::Rc;

/// Mode for generating synthetic point data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FauxMode {
    /// All points at the same location (bounds min).
    Constant,
    /// Points linearly spaced between bounds min and max.
    Ramp,
    /// Points from a uniform random distribution within bounds.
    Uniform,
    /// Points from a normal distribution with given mean/stdev.
    Normal,
}

impl FauxMode {
    fn from_str(s: &str) -> Self {
        match s {
            "constant" => FauxMode::Constant,
            "ramp" => FauxMode::Ramp,
            "uniform" => FauxMode::Uniform,
            "normal" => FauxMode::Normal,
            _ => FauxMode::Ramp,
        }
    }
}

/// 3D bounding box.
#[derive(Clone, Copy, Debug)]
pub struct Box3d {
    pub minx: f64,
    pub miny: f64,
    pub minz: f64,
    pub maxx: f64,
    pub maxy: f64,
    pub maxz: f64,
}

impl Default for Box3d {
    fn default() -> Self {
        Self {
            minx: 0.0,
            miny: 0.0,
            minz: 0.0,
            maxx: 1.0,
            maxy: 1.0,
            maxz: 1.0,
        }
    }
}

impl Box3d {
    fn del(&self, count: u64) -> (f64, f64, f64) {
        if count > 1 {
            (
                (self.maxx - self.minx) / (count - 1) as f64,
                (self.maxy - self.miny) / (count - 1) as f64,
                (self.maxz - self.minz) / (count - 1) as f64,
            )
        } else {
            (0.0, 0.0, 0.0)
        }
    }
}

/// A simple linear congruential generator for reproducible synthetic data.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn uniform(&mut self, min: f64, max: f64) -> f64 {
        let r = (self.next() as f64) / (u64::MAX as f64);
        min + r * (max - min)
    }

    fn normal(&mut self, mean: f64, stdev: f64) -> f64 {
        let u1 = (self.next() as f64 + 1.0) / (u64::MAX as f64 + 2.0);
        let u2 = (self.next() as f64) / (u64::MAX as f64);
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        mean + stdev * z
    }
}

/// Synthetic point data reader.
pub struct FauxReader {
    mode: FauxMode,
    count: u64,
    bounds: Box3d,
    seed: u64,
    mean_x: f64,
    mean_y: f64,
    mean_z: f64,
    stdev_x: f64,
    stdev_y: f64,
    stdev_z: f64,
}

impl FauxReader {
    pub fn new(options: &Options) -> Self {
        let mode_str = options.get_str("mode", "ramp");
        let mode = FauxMode::from_str(&mode_str);

        let bounds = Box3d {
            minx: options.get_f64("minx", 0.0),
            miny: options.get_f64("miny", 0.0),
            minz: options.get_f64("minz", 0.0),
            maxx: options.get_f64("maxx", 1.0),
            maxy: options.get_f64("maxy", 1.0),
            maxz: options.get_f64("maxz", 1.0),
        };

        Self {
            mode,
            count: options.get_u64("count", 10),
            bounds,
            seed: options.get_u64("seed", 42),
            mean_x: options.get_f64("mean_x", 0.0),
            mean_y: options.get_f64("mean_y", 0.0),
            mean_z: options.get_f64("mean_z", 0.0),
            stdev_x: options.get_f64("stdev_x", 1.0),
            stdev_y: options.get_f64("stdev_y", 1.0),
            stdev_z: options.get_f64("stdev_z", 1.0),
        }
    }

    fn generate_point(&self, idx: u64, rng: &mut SimpleRng) -> (f64, f64, f64) {
        match self.mode {
            FauxMode::Constant => (self.bounds.minx, self.bounds.miny, self.bounds.minz),
            FauxMode::Ramp => {
                let (dx, dy, dz) = self.bounds.del(self.count);
                (
                    self.bounds.minx + dx * idx as f64,
                    self.bounds.miny + dy * idx as f64,
                    self.bounds.minz + dz * idx as f64,
                )
            }
            FauxMode::Uniform => (
                rng.uniform(self.bounds.minx, self.bounds.maxx),
                rng.uniform(self.bounds.miny, self.bounds.maxy),
                rng.uniform(self.bounds.minz, self.bounds.maxz),
            ),
            FauxMode::Normal => (
                rng.normal(self.mean_x, self.stdev_x),
                rng.normal(self.mean_y, self.stdev_y),
                rng.normal(self.mean_z, self.stdev_z),
            ),
        }
    }
}

impl Reader for FauxReader {
    fn name(&self) -> &str {
        "readers.faux"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::OffsetTime, DimType::F64);
        let layout = Rc::new(layout);

        let mut view = PointView::new(layout);
        let mut rng = SimpleRng::new(self.seed);

        for i in 0..self.count {
            view.add_point();
            let (x, y, z) = self.generate_point(i, &mut rng);
            view.set_f64(i, &DimId::X, x);
            view.set_f64(i, &DimId::Y, y);
            view.set_f64(i, &DimId::Z, z);
            view.set_f64(i, &DimId::OffsetTime, i as f64);
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("readers.faux");
        node.add_value("count", MetadataValue::U64(self.count));
        node.add_value("mode", MetadataValue::String(format!("{:?}", self.mode)));
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::options::Options;

    #[test]
    fn test_ramp_mode() {
        let mut opts = Options::new();
        opts.add("count", "10")
            .add("mode", "ramp")
            .add("minx", "0.0")
            .add("maxx", "9.0")
            .add("miny", "0.0")
            .add("maxy", "9.0")
            .add("minz", "1.0")
            .add("maxz", "10.0");

        let mut reader = FauxReader::new(&opts);
        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 10);

        for i in 0..10u64 {
            let x = views[0].get_f64(i, &DimId::X);
            let y = views[0].get_f64(i, &DimId::Y);
            let z = views[0].get_f64(i, &DimId::Z);
            assert!((x - i as f64).abs() < 1e-10);
            assert!((y - i as f64).abs() < 1e-10);
            assert!((z - (i + 1) as f64).abs() < 1e-10);
        }
    }

    #[test]
    fn test_constant_mode() {
        let mut opts = Options::new();
        opts.add("count", "5")
            .add("mode", "constant")
            .add("minx", "1.5")
            .add("miny", "2.5")
            .add("minz", "3.5");

        let mut reader = FauxReader::new(&opts);
        let views = reader.read().unwrap();
        assert_eq!(views[0].len(), 5);

        for i in 0..5u64 {
            assert!((views[0].get_f64(i, &DimId::X) - 1.5).abs() < 1e-10);
            assert!((views[0].get_f64(i, &DimId::Y) - 2.5).abs() < 1e-10);
            assert!((views[0].get_f64(i, &DimId::Z) - 3.5).abs() < 1e-10);
        }
    }

    #[test]
    fn test_uniform_mode() {
        let mut opts = Options::new();
        opts.add("count", "100")
            .add("mode", "uniform")
            .add("minx", "0.0")
            .add("maxx", "10.0")
            .add("miny", "0.0")
            .add("maxy", "10.0")
            .add("minz", "0.0")
            .add("maxz", "10.0")
            .add("seed", "12345");

        let mut reader = FauxReader::new(&opts);
        let views = reader.read().unwrap();
        assert_eq!(views[0].len(), 100);

        for i in 0..100u64 {
            let x = views[0].get_f64(i, &DimId::X);
            let y = views[0].get_f64(i, &DimId::Y);
            let z = views[0].get_f64(i, &DimId::Z);
            assert!(x >= 0.0 && x <= 10.0);
            assert!(y >= 0.0 && y <= 10.0);
            assert!(z >= 0.0 && z <= 10.0);
        }
    }

    #[test]
    fn test_normal_mode() {
        let mut opts = Options::new();
        opts.add("count", "100")
            .add("mode", "normal")
            .add("mean_x", "50.0")
            .add("mean_y", "50.0")
            .add("mean_z", "50.0")
            .add("stdev_x", "1.0")
            .add("stdev_y", "1.0")
            .add("stdev_z", "1.0")
            .add("seed", "99999");

        let mut reader = FauxReader::new(&opts);
        let views = reader.read().unwrap();
        assert_eq!(views[0].len(), 100);

        let mut sum_x = 0.0;
        for i in 0..100u64 {
            sum_x += views[0].get_f64(i, &DimId::X);
        }
        let mean_x = sum_x / 100.0;
        assert!((mean_x - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_offset_time_is_sequential() {
        let mut opts = Options::new();
        opts.add("count", "5").add("mode", "ramp");

        let mut reader = FauxReader::new(&opts);
        let views = reader.read().unwrap();

        for i in 0..5u64 {
            let t = views[0].get_f64(i, &DimId::OffsetTime);
            assert!((t - i as f64).abs() < 1e-10);
        }
    }
}
