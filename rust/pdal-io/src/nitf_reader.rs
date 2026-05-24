//! `readers.nitf` — read NITF 2.1 lidar files via the Nitro native adapter.
//!
//! Strategy: use `pdal_native::nitf::lidar_segment` to find the embedded
//! LAS payload, then delegate point materialization to `LasReader` with the
//! discovered byte offset. NITF file/image/DES metadata is enumerated through
//! `pdal_native::nitf::read_metadata` and attached to the output.

use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::PointView;
use pdal_core::srs::SpatialReference;
use pdal_core::stage::StageError;

use crate::las::LasReader;

pub struct NitfReader {
    filename: String,
    spatial_reference_override: Option<String>,
    inner_options: Options,
    metadata: MetadataNode,
}

impl NitfReader {
    pub fn new(options: &Options) -> Self {
        let filename = options.get_str("filename", "");
        let spatial_reference_override = if options.has("spatialreference") {
            let v = options.get_str("spatialreference", "");
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        } else {
            None
        };
        Self {
            filename,
            spatial_reference_override,
            inner_options: options.clone(),
            metadata: MetadataNode::new("readers.nitf"),
        }
    }

    fn build_inner_reader(&self, start_offset: u64) -> LasReader {
        let mut opts = self.inner_options.clone();
        opts.add("start_offset", start_offset);
        LasReader::new(&opts)
    }

    fn populate_nitf_metadata(&mut self) -> Result<(), StageError> {
        let map = pdal_native::nitf::read_metadata(&self.filename)
            .map_err(|e| StageError(format!("readers.nitf: failed to read metadata: {}", e)))?;
        let mut node = MetadataNode::new("readers.nitf");
        for (key, value) in map {
            node.add_value(&key, MetadataValue::String(value));
        }
        self.metadata = node;
        Ok(())
    }
}

impl Reader for NitfReader {
    fn name(&self) -> &str {
        "readers.nitf"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "readers.nitf requires a filename option.".to_string(),
            ));
        }

        let (offset, _length) = pdal_native::nitf::lidar_segment(&self.filename)
            .map_err(|e| StageError(format!("readers.nitf: {}", e)))?;

        self.populate_nitf_metadata()?;

        let mut inner = self.build_inner_reader(offset);
        let mut views = inner.read()?;

        if let Some(srs) = &self.spatial_reference_override {
            let reference = SpatialReference::new(srs);
            for view in views.iter_mut() {
                view.set_spatial_reference(reference.clone());
            }
        }

        Ok(views)
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::DimId;
    use std::path::PathBuf;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn reader_for(path: &str) -> NitfReader {
        let mut opts = Options::default();
        opts.add("filename", path);
        NitfReader::new(&opts)
    }

    #[test]
    fn reads_autzen_nitf_with_same_point_data_as_las() {
        let nitf_path = repo().join("test/data/nitf/autzen-utm10.ntf");
        let las_path = repo().join("test/data/nitf/autzen-utm10.las");

        let mut nitf_opts = Options::default();
        nitf_opts.add("filename", nitf_path.to_str().unwrap());
        nitf_opts.add("count", 750u64);
        let mut nitf_reader = NitfReader::new(&nitf_opts);
        let nitf_view = nitf_reader.read().unwrap().pop().unwrap();

        let mut las_opts = Options::default();
        las_opts.add("filename", las_path.to_str().unwrap());
        las_opts.add("count", 750u64);
        let mut las_reader = LasReader::new(&las_opts);
        let las_view = las_reader.read().unwrap().pop().unwrap();

        assert_eq!(nitf_view.len(), las_view.len());
        for idx in 0..nitf_view.len() {
            assert_eq!(
                nitf_view.get_f64(idx, &DimId::X),
                las_view.get_f64(idx, &DimId::X)
            );
            assert_eq!(
                nitf_view.get_f64(idx, &DimId::Y),
                las_view.get_f64(idx, &DimId::Y)
            );
            assert_eq!(
                nitf_view.get_f64(idx, &DimId::Z),
                las_view.get_f64(idx, &DimId::Z)
            );
        }
    }

    #[test]
    fn exposes_nitf_header_metadata() {
        let nitf_path = repo().join("test/data/nitf/autzen-utm10.ntf");
        let mut reader = reader_for(nitf_path.to_str().unwrap());
        let _ = reader.read().unwrap();
        let meta = reader.metadata();
        let fdt = meta
            .find_child("FH.FDT")
            .and_then(|n| n.value())
            .map(|v| v.as_string())
            .expect("FH.FDT metadata present");
        assert_eq!(fdt, "20120323002946");
        let igeolo = meta
            .find_child("IM:0.IGEOLO")
            .and_then(|n| n.value())
            .map(|v| v.as_string())
            .expect("IM:0.IGEOLO present");
        assert_eq!(
            igeolo,
            "440344N1230429W440344N1230346W440300N1230346W440300N1230429W"
        );
    }
}
