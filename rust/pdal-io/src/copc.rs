//! `readers.copc` -- local COPC full-file slice.
//!
//! This currently reuses the LAS/LAZ reader for local COPC files and applies
//! simple post-read filters. COPC hierarchy traversal, bounds pruning,
//! resolution queries, remote reads, and writer behavior are deferred.

use crate::las::LasReader;
use pdal_core::bounds::{parse_bounds2d, parse_bounds3d, Bounds2D, Bounds3D};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::StageError;

pub struct CopcReader {
    inner: LasReader,
    bounds: String,
    metadata: MetadataNode,
}

impl CopcReader {
    pub fn new(options: &Options) -> Self {
        Self {
            inner: LasReader::new(options),
            bounds: options.get_str("bounds", ""),
            metadata: MetadataNode::new("readers.copc"),
        }
    }
}

impl Reader for CopcReader {
    fn name(&self) -> &str {
        "readers.copc"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        let bounds = self.bounds_filter()?;
        let views = self
            .inner
            .read()?
            .into_iter()
            .map(|view| apply_bounds(view, bounds.as_ref()))
            .filter(|view| !view.is_empty())
            .collect();
        self.metadata = self.inner.metadata();
        Ok(views)
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

impl CopcReader {
    fn bounds_filter(&self) -> Result<Option<QueryBounds>, StageError> {
        if self.bounds.is_empty() {
            return Ok(None);
        }
        if let Ok(parsed) = parse_bounds3d(&self.bounds, 0) {
            return Ok(Some(QueryBounds::Three(parsed.bounds)));
        }
        parse_bounds2d(&self.bounds, 0)
            .map(|parsed| Some(QueryBounds::Two(parsed.bounds)))
            .map_err(|err| StageError(format!("Invalid COPC bounds option: {err}")))
    }
}

enum QueryBounds {
    Two(Bounds2D),
    Three(Bounds3D),
}

impl QueryBounds {
    fn contains(&self, view: &PointView, idx: PointId) -> bool {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        match self {
            QueryBounds::Two(bounds) => bounds.contains_point(x, y),
            QueryBounds::Three(bounds) => bounds.contains_point(x, y, view.get_f64(idx, &DimId::Z)),
        }
    }
}

fn apply_bounds(view: PointView, bounds: Option<&QueryBounds>) -> PointView {
    let Some(bounds) = bounds else {
        return view;
    };
    let mut output = view.make_new();
    for idx in 0..view.len() {
        if bounds.contains(&view, idx) {
            output.append_point(&view, idx);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn data_path(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data")
            .join(path)
    }

    #[test]
    fn reads_local_copc_with_las_path() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        let mut reader = CopcReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 518862);
    }

    #[test]
    fn applies_2d_bounds_filter() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        options.add("bounds", "([515380,515400],[4918350,4918370])");
        let mut reader = CopcReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 354211);
        for idx in 0..views[0].len() {
            let x = views[0].get_f64(idx, &DimId::X);
            let y = views[0].get_f64(idx, &DimId::Y);
            assert!((515380.0..=515400.0).contains(&x));
            assert!((4918350.0..=4918370.0).contains(&y));
        }
    }

    #[test]
    fn applies_3d_bounds_filter() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        options.add("bounds", "([515380,515400],[4918350,4918370],[2320,2325])");
        let mut reader = CopcReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 45930);
        for idx in 0..views[0].len() {
            let x = views[0].get_f64(idx, &DimId::X);
            let y = views[0].get_f64(idx, &DimId::Y);
            let z = views[0].get_f64(idx, &DimId::Z);
            assert!((515380.0..=515400.0).contains(&x));
            assert!((4918350.0..=4918370.0).contains(&y));
            assert!((2320.0..=2325.0).contains(&z));
        }
    }
}
