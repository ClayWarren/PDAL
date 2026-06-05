//! `writers.nitf` — write NITF 2.1 LIDARA files via the Nitro native adapter.
//!
//! Strategy: have `LasWriter` produce a LAS payload to a temporary file, then
//! invoke `pdal_native::nitf::write` to wrap that payload in a NITF record with
//! the writer's NITF-specific options (FTITLE, AIMIDB/ACFTB, security, etc.).
//! Multi-view inputs use the same `#` filename templating as PDAL's FlexWriter.

use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{Bounds3D, PointView};
use pdal_core::stage::StageError;
use pdal_core::writer::{handle_filename_template, FilenameTemplate};
use std::path::{Path, PathBuf};

use crate::las_writer::LasWriter;

pub struct NitfWriter {
    filename: String,
    template: FilenameTemplate,
    nitf_opts: pdal_native::nitf::NitfWriteOptions,
    las_options: Options,
    metadata: MetadataNode,
}

impl NitfWriter {
    pub fn new(options: &Options) -> Result<Self, StageError> {
        let filename = options.get_str("filename", "");
        let template = if filename.is_empty() {
            FilenameTemplate::NoPlaceholder
        } else {
            handle_filename_template(&filename).map_err(StageError)?
        };

        let nitf_opts = pdal_native::nitf::NitfWriteOptions {
            file_title: optional(options, "ftitle"),
            complexity_level: optional(options, "clevel"),
            system_type: optional(options, "stype"),
            origin_station_id: optional(options, "ostaid"),
            file_class: optional(options, "fsclas"),
            origin_name: optional(options, "oname"),
            origin_phone: optional(options, "ophone"),
            fsclsy: optional(options, "fsclsy"),
            fsctlh: optional(options, "fsctlh"),
            fscltx: optional(options, "fscltx"),
            image_security_class: optional(options, "isclas"),
            image_date_time: optional(options, "idatim"),
            image_id2: optional(options, "iid2"),
            aimidb: collect_list(options, "aimidb"),
            acftb: collect_list(options, "acftb"),
            ..Default::default()
        };

        // LAS writer options are everything except the NITF-only options.
        // Pass them through as-is; LasWriter will pick what it understands.
        let las_options = options.clone();

        Ok(Self {
            filename,
            template,
            nitf_opts,
            las_options,
            metadata: MetadataNode::new("writers.nitf"),
        })
    }

    fn write_one(&self, view: &PointView, output_path: &Path) -> Result<(), StageError> {
        if view.is_empty() {
            return Err(StageError(
                "writers.nitf cannot write an empty view.".to_string(),
            ));
        }

        let temp_dir = tempfile::tempdir()
            .map_err(|e| StageError(format!("writers.nitf: failed to create tempdir: {}", e)))?;
        let las_path = temp_dir.path().join("payload.las");

        let mut las_opts = self.las_options.clone();
        las_opts.add("filename", las_path.to_string_lossy().to_string());
        // Defensive: ensure we always write uncompressed for the NITF payload.
        las_opts.add("compression", false);
        let mut las_writer = LasWriter::new(&las_opts);
        las_writer.write(std::slice::from_ref(view))?;

        let mut nitf_opts = self.nitf_opts.clone();
        let bounds = view.calculate_bounds_3d().unwrap_or(Bounds3D {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        });
        let (minx, miny, maxx, maxy) = reproject_bounds_to_dd(view, &bounds);
        nitf_opts.minx = minx;
        nitf_opts.miny = miny;
        nitf_opts.maxx = maxx;
        nitf_opts.maxy = maxy;

        // Default file title to the output filename if the option is unset.
        if nitf_opts.file_title.as_deref().unwrap_or("").is_empty() {
            if let Some(name) = output_path.file_name().and_then(|n| n.to_str()) {
                nitf_opts.file_title = Some(name.to_string());
            }
        }

        pdal_native::nitf::write(
            las_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            &nitf_opts,
        )
        .map_err(|e| StageError(format!("writers.nitf: {}", e)))?;
        Ok(())
    }
}

fn optional(options: &Options, key: &str) -> Option<String> {
    if options.has(key) {
        let v = options.get_str(key, "");
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    } else {
        None
    }
}

fn collect_list(options: &Options, key: &str) -> Vec<String> {
    if !options.has(key) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let len = options.len();
    for idx in 0..len {
        if let Some((k, v)) = options.entry(idx) {
            if k == key {
                out.push(v.to_string());
            }
        }
    }
    out
}

fn reproject_bounds_to_dd(view: &PointView, bounds: &Bounds3D) -> (f64, f64, f64, f64) {
    let srs = view.spatial_reference();
    let wkt = srs.wkt();
    if wkt.is_empty() {
        return (bounds.minx, bounds.miny, bounds.maxx, bounds.maxy);
    }
    let target_wkt = match pdal_native::srs::user_input_to_wkt("EPSG:4326") {
        Ok(input) => input.wkt,
        Err(_) => return (bounds.minx, bounds.miny, bounds.maxx, bounds.maxy),
    };
    if target_wkt.is_empty() {
        return (bounds.minx, bounds.miny, bounds.maxx, bounds.maxy);
    }
    let transform =
        match pdal_native::srs::GdalSrsTransform::new(wkt, 0.0, &target_wkt, 0.0, &[], &[]) {
            Ok(t) => t,
            Err(_) => return (bounds.minx, bounds.miny, bounds.maxx, bounds.maxy),
        };
    let mut minx = bounds.minx;
    let mut miny = bounds.miny;
    let mut maxx = bounds.maxx;
    let mut maxy = bounds.maxy;
    let mut z0 = 0.0;
    let mut z1 = 0.0;
    if !transform.transform_xyz(&mut minx, &mut miny, &mut z0)
        || !transform.transform_xyz(&mut maxx, &mut maxy, &mut z1)
    {
        return (bounds.minx, bounds.miny, bounds.maxx, bounds.maxy);
    }
    (
        minx.min(maxx),
        miny.min(maxy),
        minx.max(maxx),
        miny.max(maxy),
    )
}

impl Writer for NitfWriter {
    fn name(&self) -> &str {
        "writers.nitf"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "writers.nitf requires a filename option.".to_string(),
            ));
        }
        if views.is_empty() {
            return Err(StageError(
                "writers.nitf received no input views.".to_string(),
            ));
        }

        match self.template {
            FilenameTemplate::NoPlaceholder => {
                if views.len() == 1 {
                    self.write_one(&views[0], &PathBuf::from(&self.filename))?;
                } else {
                    let temp_dir = tempfile::tempdir().map_err(|e| {
                        StageError(format!("writers.nitf: failed to create tempdir: {}", e))
                    })?;
                    let las_path = temp_dir.path().join("payload.las");
                    let mut las_opts = self.las_options.clone();
                    las_opts.add("filename", las_path.to_string_lossy().to_string());
                    las_opts.add("compression", false);
                    let mut las_writer = LasWriter::new(&las_opts);
                    las_writer.write(views)?;

                    let mut combined: Option<Bounds3D> = None;
                    for view in views {
                        if let Some(b) = view.calculate_bounds_3d() {
                            combined = Some(match combined {
                                None => b,
                                Some(prev) => Bounds3D {
                                    minx: prev.minx.min(b.minx),
                                    maxx: prev.maxx.max(b.maxx),
                                    miny: prev.miny.min(b.miny),
                                    maxy: prev.maxy.max(b.maxy),
                                    minz: prev.minz.min(b.minz),
                                    maxz: prev.maxz.max(b.maxz),
                                },
                            });
                        }
                    }
                    let bounds = combined.unwrap_or(Bounds3D {
                        minx: 0.0,
                        maxx: 0.0,
                        miny: 0.0,
                        maxy: 0.0,
                        minz: 0.0,
                        maxz: 0.0,
                    });
                    let mut nitf_opts = self.nitf_opts.clone();
                    let (minx, miny, maxx, maxy) = reproject_bounds_to_dd(&views[0], &bounds);
                    nitf_opts.minx = minx;
                    nitf_opts.miny = miny;
                    nitf_opts.maxx = maxx;
                    nitf_opts.maxy = maxy;
                    if nitf_opts.file_title.as_deref().unwrap_or("").is_empty() {
                        if let Some(name) = Path::new(&self.filename)
                            .file_name()
                            .and_then(|n| n.to_str())
                        {
                            nitf_opts.file_title = Some(name.to_string());
                        }
                    }
                    pdal_native::nitf::write(
                        las_path.to_str().unwrap(),
                        &self.filename,
                        &nitf_opts,
                    )
                    .map_err(|e| StageError(format!("writers.nitf: {}", e)))?;
                }
            }
            FilenameTemplate::Placeholder(hash_pos) => {
                let prefix = &self.filename[..hash_pos];
                let suffix = &self.filename[hash_pos + 1..];
                for (i, view) in views.iter().enumerate() {
                    let output = format!("{}{}{}", prefix, i + 1, suffix);
                    self.write_one(view, &PathBuf::from(output))?;
                }
            }
        }
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

#[cfg(all(test, feature = "nitf"))]
mod tests {
    use super::*;
    use crate::las::LasReader;
    use crate::nitf_reader::NitfReader;
    use pdal_core::pipeline::Reader;
    use pdal_core::point::DimId;
    use std::path::PathBuf;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn read_las(path: &Path) -> PointView {
        let mut opts = Options::default();
        opts.add("filename", path.to_str().unwrap());
        LasReader::new(&opts).read().unwrap().pop().unwrap()
    }

    #[test]
    fn writes_nitf_round_trip_matches_input_xyz() {
        let temp = tempfile::tempdir().unwrap();
        let input_las = repo().join("test/data/las/1.2-with-color.las");
        let nitf_out = temp.path().join("temp_nitf.ntf");

        let source = read_las(&input_las);

        let mut write_opts = Options::default();
        write_opts.add("filename", nitf_out.to_str().unwrap());
        write_opts.add("idatim", "20110516183337");
        write_opts.add("fsclas", "S");
        write_opts.add("ophone", "5155554628");
        write_opts.add("oname", "Howard Butler");
        write_opts.add("ftitle", "LiDAR from somewhere");

        let mut writer = NitfWriter::new(&write_opts).unwrap();
        writer.write(std::slice::from_ref(&source)).unwrap();
        assert!(nitf_out.exists());

        let mut read_opts = Options::default();
        read_opts.add("filename", nitf_out.to_str().unwrap());
        let mut reader = NitfReader::new(&read_opts);
        let read_back = reader.read().unwrap().pop().unwrap();

        assert_eq!(read_back.len(), source.len());
        for idx in 0..source.len() {
            assert_eq!(
                read_back.get_f64(idx, &DimId::X),
                source.get_f64(idx, &DimId::X)
            );
            assert_eq!(
                read_back.get_f64(idx, &DimId::Y),
                source.get_f64(idx, &DimId::Y)
            );
            assert_eq!(
                read_back.get_f64(idx, &DimId::Z),
                source.get_f64(idx, &DimId::Z)
            );
        }

        let meta = reader.metadata();
        let ftitle = meta
            .find_child("FH.FTITLE")
            .and_then(|n| n.value())
            .map(|v| v.as_string());
        assert_eq!(ftitle.as_deref(), Some("LiDAR from somewhere"));
        let idatim = meta
            .find_child("IM:0.IDATIM")
            .and_then(|n| n.value())
            .map(|v| v.as_string());
        assert_eq!(idatim.as_deref(), Some("20110516183337"));
    }

    #[test]
    fn rejects_overlong_ftitle() {
        let temp = tempfile::tempdir().unwrap();
        let input_las = repo().join("test/data/nitf/autzen-utm10.las");
        let source = read_las(&input_las);
        let nitf_out = temp.path().join("overlong.ntf");

        let mut opts = Options::default();
        opts.add("filename", nitf_out.to_str().unwrap());
        let long_title = "a".repeat(120);
        opts.add("ftitle", long_title);
        let mut writer = NitfWriter::new(&opts).unwrap();
        let err = writer.write(std::slice::from_ref(&source)).unwrap_err();
        assert!(err.to_string().contains("FTITLE"));
    }
}
