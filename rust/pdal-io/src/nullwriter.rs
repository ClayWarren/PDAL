use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::PointView;
use pdal_core::stage::StageError;

/// A writer that consumes points without producing any output.
/// Useful for testing pipeline execution and measuring point throughput.
pub struct NullWriter {
    view_count: u64,
    point_count: u64,
}

impl NullWriter {
    pub fn new(_options: &Options) -> Self {
        Self {
            view_count: 0,
            point_count: 0,
        }
    }

    pub fn view_count(&self) -> u64 {
        self.view_count
    }

    pub fn point_count(&self) -> u64 {
        self.point_count
    }
}

impl Writer for NullWriter {
    fn name(&self) -> &str {
        "writers.null"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        self.view_count += views.len() as u64;
        for view in views {
            self.point_count += view.len();
        }
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.null");
        node.add_value("view_count", MetadataValue::U64(self.view_count));
        node.add_value("point_count", MetadataValue::U64(self.point_count));
        node
    }

    fn streamable(&self) -> bool {
        true
    }

    fn stream_write(&mut self, chunk: &PointView) -> Result<(), StageError> {
        self.point_count += chunk.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::options::Options;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn make_test_view(count: u64) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);
        for _ in 0..count {
            view.add_point();
        }
        view
    }

    #[test]
    fn test_null_writer_counts_points() {
        let mut writer = NullWriter::new(&Options::new());
        let views = vec![make_test_view(10), make_test_view(20)];
        writer.write(&views).unwrap();

        assert_eq!(writer.view_count(), 2);
        assert_eq!(writer.point_count(), 30);
    }

    #[test]
    fn test_null_writer_handles_empty_views() {
        let mut writer = NullWriter::new(&Options::new());
        writer.write(&[]).unwrap();

        assert_eq!(writer.view_count(), 0);
        assert_eq!(writer.point_count(), 0);
    }

    #[test]
    fn test_null_writer_metadata() {
        let mut writer = NullWriter::new(&Options::new());
        let views = vec![make_test_view(5)];
        writer.write(&views).unwrap();

        let meta = writer.metadata();
        assert_eq!(meta.name(), "writers.null");
        let vc = meta.find_child("view_count").unwrap();
        assert_eq!(vc.value().unwrap().as_u64(), 1);
        let pc = meta.find_child("point_count").unwrap();
        assert_eq!(pc.value().unwrap().as_u64(), 5);
    }
}
