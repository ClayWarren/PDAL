//! `readers.copc` -- local + remote COPC reader.
//!
//! `read()` still reuses the LAS/LAZ reader (with VSI byte-range support for
//! http(s)/`/vsi*` paths) for full-file materialization. `preview()` walks the
//! COPC hierarchy directly, applying 2D/3D bounds and resolution pruning to
//! match the C++ `CopcReader::inspect()` semantics for the remote/sample-mode
//! use case.

use std::fs::File;
use std::io::BufReader;

use crate::copc_hierarchy::{
    self, CopcInfo, CopcPreview, LasBounds, QueryBounds as HierarchyBounds,
};
use crate::las::LasReader;
use pdal_core::bounds::{parse_bounds2d, parse_bounds3d, Bounds2D, Bounds3D};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::StageError;

pub struct CopcReader {
    inner: LasReader,
    filename: String,
    bounds: String,
    resolution: f64,
    metadata: MetadataNode,
}

impl CopcReader {
    pub fn new(options: &Options) -> Self {
        Self {
            inner: LasReader::new(options),
            filename: options.get_str("filename", ""),
            bounds: options.get_str("bounds", ""),
            resolution: options.get_f64("resolution", 0.0),
            metadata: MetadataNode::new("readers.copc"),
        }
    }

    /// Hierarchy-driven preview: returns (point_count, bounds[6]) after
    /// applying the bounds and resolution options. Mirrors the C++
    /// `CopcReader::inspect()` count/bbox behavior for sample-mode queries.
    pub fn preview(&self) -> Result<CopcPreview, StageError> {
        let query = self
            .bounds_filter()
            .map_err(|e| StageError(e.0))?
            .map(|b| match b {
                QueryBounds::Two(b) => HierarchyBounds::Two(b),
                QueryBounds::Three(b) => HierarchyBounds::Three(b),
            });
        let resolution = self.resolution;
        with_byte_source(&self.filename, |reader| {
            let (info, full_bounds) = copc_hierarchy::read_copc_info(reader)?;
            copc_hierarchy::walk_preview(reader, &info, full_bounds, query.as_ref(), resolution)
        })
        .map_err(StageError)
    }

    pub fn copc_info(&self) -> Result<(CopcInfo, LasBounds), StageError> {
        with_byte_source(&self.filename, |reader| {
            copc_hierarchy::read_copc_info(reader)
        })
        .map_err(StageError)
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

fn is_vsi_path(filename: &str) -> bool {
    filename.starts_with("/vsi")
        || filename.starts_with("http://")
        || filename.starts_with("https://")
}

fn with_byte_source<T>(
    filename: &str,
    mut f: impl FnMut(&mut dyn ReadSeek) -> Result<T, String>,
) -> Result<T, String> {
    if filename.is_empty() {
        return Err("COPC: missing filename option".to_string());
    }
    if is_vsi_path(filename) {
        let vsi_path = if filename.starts_with("http://") || filename.starts_with("https://") {
            format!("/vsicurl/{filename}")
        } else {
            filename.to_string()
        };
        let mut vsi = pdal_native::vsi::VsiFile::open(&vsi_path)
            .map_err(|e| format!("COPC: failed to open VSI path {vsi_path}: {e}"))?;
        f(&mut vsi)
    } else {
        let file =
            File::open(filename).map_err(|e| format!("COPC: failed to open {filename}: {e}"))?;
        let mut buf = BufReader::new(file);
        f(&mut buf)
    }
}

/// Combined Read+Seek so we can pass either `BufReader<File>` or `VsiFile`
/// through a single trait object.
pub trait ReadSeek: std::io::Read + std::io::Seek {}
impl<T: std::io::Read + std::io::Seek + ?Sized> ReadSeek for T {}

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

    /// Network smoke test mirroring the C++ `pdal_io_copc_remote_reader_test`:
    /// hits the canonical autzen-classified COPC over https and `/vsicurl/`
    /// and expects the 61201/2D-bounds preview after bounds + resolution
    /// pruning. Ignored by default to keep CI hermetic.
    #[test]
    #[ignore = "network smoke for remote COPC preview"]
    fn preview_remote_autzen_matches_cpp_expectation() {
        for filename in [
            "https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz",
            "/vsicurl/https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz",
        ] {
            let mut options = Options::new();
            options.add("filename", filename.to_string());
            options.add("bounds", "([635700,637000],[848900,853300])");
            options.add("resolution", 1000.0_f64);
            let reader = CopcReader::new(&options);
            let preview = reader.preview().unwrap();
            assert_eq!(preview.point_count, 61_201, "url={filename}");
            assert!(preview.bounds.min_x >= 635_700.0);
            assert!(preview.bounds.max_x <= 637_000.0);
            assert!(preview.bounds.min_y >= 848_900.0);
            assert!(preview.bounds.max_y <= 853_300.0);
        }
    }

    #[test]
    fn preview_unfiltered_local_matches_full_count() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        let reader = CopcReader::new(&options);
        let preview = reader.preview().unwrap();
        assert_eq!(preview.point_count, 518_862);
    }

    #[test]
    fn preview_with_2d_bounds_prunes_count_below_full() {
        let mut options = Options::new();
        options.add("filename", data_path("copc/lone-star.copc.laz").display());
        options.add("bounds", "([515380,515400],[4918350,4918370])");
        let reader = CopcReader::new(&options);
        let preview = reader.preview().unwrap();
        assert!(preview.point_count > 0);
        assert!(preview.point_count < 518_862);
        assert!(preview.bounds.min_x >= 515380.0);
        assert!(preview.bounds.max_x <= 515400.0);
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
