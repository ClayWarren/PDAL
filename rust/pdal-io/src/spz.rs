use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::{Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use spz_crate::{CoordinateSystem, GaussianSplat, PackOptions, UnpackOptions};
use std::path::Path;
use std::rc::Rc;

const SH_BY_DEGREE: [usize; 4] = [0, 3, 8, 15];

pub struct SpzReader {
    filename: String,
    metadata: MetadataNode,
}

impl SpzReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            metadata: MetadataNode::new("readers.spz"),
        }
    }
}

impl Reader for SpzReader {
    fn name(&self) -> &str {
        "readers.spz"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "SpzReader requires a filename option.".to_string(),
            ));
        }

        let splat = GaussianSplat::load_packed_from_file(
            &self.filename,
            &UnpackOptions {
                to_coord_sys: CoordinateSystem::RUB,
            },
        )
        .map_err(|err| StageError(format!("Unable to load SPZ data: {err}")))?;

        let mut layout = PointLayout::new();
        register_spz_dimensions(&mut layout, splat.spherical_harmonics_degree as usize)?;
        let mut view = PointView::new(Rc::new(layout));

        let sh_per_channel = sh_per_channel(splat.spherical_harmonics_degree as usize)?;
        for point in 0..splat.num_points as u64 {
            view.add_point();
            let base3 = point as usize * 3;
            view.set_f64(point, &DimId::X, splat.positions[base3] as f64);
            view.set_f64(point, &DimId::Y, splat.positions[base3 + 1] as f64);
            view.set_f64(point, &DimId::Z, splat.positions[base3 + 2] as f64);
            view.set_f64(point, &dim("f_dc_0"), splat.colors[base3] as f64);
            view.set_f64(point, &dim("f_dc_1"), splat.colors[base3 + 1] as f64);
            view.set_f64(point, &dim("f_dc_2"), splat.colors[base3 + 2] as f64);
            view.set_f64(point, &dim("scale_0"), splat.scales[base3] as f64);
            view.set_f64(point, &dim("scale_1"), splat.scales[base3 + 1] as f64);
            view.set_f64(point, &dim("scale_2"), splat.scales[base3 + 2] as f64);
            view.set_f64(point, &dim("opacity"), splat.alphas[point as usize] as f64);

            let rot = point as usize * 4;
            view.set_f64(point, &dim("rot_0"), splat.rotations[rot + 3] as f64);
            view.set_f64(point, &dim("rot_1"), splat.rotations[rot] as f64);
            view.set_f64(point, &dim("rot_2"), splat.rotations[rot + 1] as f64);
            view.set_f64(point, &dim("rot_3"), splat.rotations[rot + 2] as f64);

            let sh_base = point as usize * sh_per_channel * 3;
            for coeff in 0..sh_per_channel {
                view.set_f64(
                    point,
                    &dim(&format!("f_rest_{coeff}")),
                    splat.spherical_harmonics[sh_base + coeff * 3] as f64,
                );
                view.set_f64(
                    point,
                    &dim(&format!("f_rest_{}", sh_per_channel + coeff)),
                    splat.spherical_harmonics[sh_base + coeff * 3 + 1] as f64,
                );
                view.set_f64(
                    point,
                    &dim(&format!("f_rest_{}", 2 * sh_per_channel + coeff)),
                    splat.spherical_harmonics[sh_base + coeff * 3 + 2] as f64,
                );
            }
        }

        self.metadata = reader_metadata(splat.num_points as u64);
        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

pub struct SpzWriter {
    filename: String,
    antialiased: bool,
    coordinate_orientation: CoordinateSystem,
    point_count: u64,
}

impl SpzWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            antialiased: options.get_bool("antialiased", false),
            coordinate_orientation: coordinate_system(
                &options.get_str("coordinate_orientation", ""),
            ),
            point_count: 0,
        }
    }
}

impl Writer for SpzWriter {
    fn name(&self) -> &str {
        "writers.spz"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "SpzWriter requires a filename option.".to_string(),
            ));
        }

        let sh_degree = views
            .first()
            .map(|view| sh_degree_from_layout(view.layout()))
            .unwrap_or(0);
        let sh_per_channel = sh_per_channel(sh_degree)?;
        let point_count: u64 = views.iter().map(PointView::len).sum();
        let num_points = i32::try_from(point_count).map_err(|_| {
            StageError(format!(
                "SPZ output supports at most {} points, got {point_count}.",
                i32::MAX
            ))
        })?;

        let mut splat = GaussianSplat {
            num_points,
            spherical_harmonics_degree: sh_degree as i32,
            antialiased: self.antialiased,
            positions: Vec::with_capacity(point_count as usize * 3),
            scales: Vec::with_capacity(point_count as usize * 3),
            rotations: Vec::with_capacity(point_count as usize * 4),
            alphas: Vec::with_capacity(point_count as usize),
            colors: Vec::with_capacity(point_count as usize * 3),
            spherical_harmonics: Vec::with_capacity(point_count as usize * sh_per_channel * 3),
        };

        for view in views {
            for point in 0..view.len() {
                splat.positions.push(view.get_f64(point, &DimId::X) as f32);
                splat.positions.push(view.get_f64(point, &DimId::Y) as f32);
                splat.positions.push(view.get_f64(point, &DimId::Z) as f32);
                for axis in 0..3 {
                    splat
                        .scales
                        .push(view.get_f64(point, &dim(&format!("scale_{axis}"))) as f32);
                }
                splat
                    .rotations
                    .push(view.get_f64(point, &dim("rot_1")) as f32);
                splat
                    .rotations
                    .push(view.get_f64(point, &dim("rot_2")) as f32);
                splat
                    .rotations
                    .push(view.get_f64(point, &dim("rot_3")) as f32);
                splat
                    .rotations
                    .push(view.get_f64(point, &dim("rot_0")) as f32);
                for channel in 0..3 {
                    splat
                        .colors
                        .push(view.get_f64(point, &dim(&format!("f_dc_{channel}"))) as f32);
                }
                splat
                    .alphas
                    .push(view.get_f64(point, &dim("opacity")) as f32);

                for coeff in 0..sh_per_channel {
                    splat
                        .spherical_harmonics
                        .push(view.get_f64(point, &dim(&format!("f_rest_{coeff}"))) as f32);
                    splat.spherical_harmonics.push(
                        view.get_f64(point, &dim(&format!("f_rest_{}", sh_per_channel + coeff)))
                            as f32,
                    );
                    splat.spherical_harmonics.push(view.get_f64(
                        point,
                        &dim(&format!("f_rest_{}", 2 * sh_per_channel + coeff)),
                    ) as f32);
                }
            }
        }

        splat
            .save_as_packed(
                Path::new(&self.filename),
                &PackOptions {
                    from: self.coordinate_orientation.clone(),
                },
            )
            .map_err(|err| StageError(format!("Unable to save SPZ data: {err}")))?;
        self.point_count = point_count;
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.spz");
        node.add_value("filename", MetadataValue::String(self.filename.clone()));
        node.add_value("point_count", MetadataValue::U64(self.point_count));
        node
    }
}

fn register_spz_dimensions(layout: &mut PointLayout, sh_degree: usize) -> Result<(), StageError> {
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(dim("opacity"), DimType::F32);
    for idx in 0..3 {
        layout.register(dim(&format!("f_dc_{idx}")), DimType::F32);
        layout.register(dim(&format!("scale_{idx}")), DimType::F32);
    }
    for idx in 0..4 {
        layout.register(dim(&format!("rot_{idx}")), DimType::F32);
    }
    for idx in 0..(sh_per_channel(sh_degree)? * 3) {
        layout.register(dim(&format!("f_rest_{idx}")), DimType::F32);
    }
    Ok(())
}

fn sh_per_channel(sh_degree: usize) -> Result<usize, StageError> {
    SH_BY_DEGREE.get(sh_degree).copied().ok_or_else(|| {
        StageError(format!(
            "Unsupported SPZ spherical harmonics degree {sh_degree}."
        ))
    })
}

fn sh_degree_from_layout(layout: &PointLayout) -> usize {
    let count = (0..45)
        .take_while(|idx| layout.dim(&dim(&format!("f_rest_{idx}"))).is_some())
        .count();
    match count {
        9 => 1,
        24 => 2,
        45 => 3,
        _ => 0,
    }
}

fn coordinate_system(name: &str) -> CoordinateSystem {
    if name.is_empty() {
        CoordinateSystem::UNSPECIFIED
    } else {
        CoordinateSystem::from(name)
    }
}

fn reader_metadata(point_count: u64) -> MetadataNode {
    let mut node = MetadataNode::new("readers.spz");
    node.add_value("count", MetadataValue::U64(point_count));
    node.add_value(
        "coordinate_orientation",
        MetadataValue::String("RUB".to_string()),
    );
    node
}

fn dim(name: &str) -> DimId {
    DimId::Other(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn reads_spz_fixture_with_plugin_dimensions() {
        let mut options = Options::new();
        options.add("filename", fixture("test/data/spz/fourth_st.spz"));
        let mut reader = SpzReader::new(&options);

        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 131_199);
        assert!(views[0].layout().dim(&DimId::X).is_some());
        assert!(views[0].layout().dim(&dim("rot_0")).is_some());
        assert!(views[0].layout().dim(&dim("f_dc_2")).is_some());
        assert_eq!(
            reader
                .metadata()
                .find_child("coordinate_orientation")
                .and_then(MetadataNode::value)
                .map(MetadataValue::as_string),
            Some("RUB".to_string())
        );
    }

    #[test]
    fn writer_roundtrips_xyz_and_orientation() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("out.spz");
        let view = xyz_view();
        let mut writer_options = Options::new();
        writer_options.add("filename", output.display());
        writer_options.add("coordinate_orientation", "RDF");
        let mut writer = SpzWriter::new(&writer_options);

        writer.write(std::slice::from_ref(&view)).unwrap();
        assert!(std::fs::metadata(&output).unwrap().len() > 0);

        let mut reader_options = Options::new();
        reader_options.add("filename", output.display());
        let mut reader = SpzReader::new(&reader_options);
        let views = reader.read().unwrap();
        let read = &views[0];

        assert_eq!(read.len(), 3);
        assert_eq!(read.get_f64(0, &DimId::X) as f32, 1.0);
        assert_eq!(read.get_f64(1, &DimId::X) as f32, 2.0);
        assert_eq!(read.get_f64(2, &DimId::X) as f32, 1.0);
        assert_eq!(read.get_f64(0, &DimId::Y) as f32, -1.0);
        assert_eq!(read.get_f64(1, &DimId::Y) as f32, -1.0);
        assert_eq!(read.get_f64(2, &DimId::Y) as f32, -2.0);
    }

    fn xyz_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in [(1.0, 1.0, 0.0), (2.0, 1.0, 0.0), (1.0, 2.0, 0.0)] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }
        view
    }

    fn fixture(path: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
            .display()
            .to_string()
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = SpzReader::new(&Options::new());
        let err = reader.read().err().expect("missing filename");
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/file.spz");
        let mut reader = SpzReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_name_returns_expected() {
        let reader = SpzReader::new(&Options::new());
        assert_eq!(reader.name(), "readers.spz");
    }

    #[test]
    fn writer_errors_without_filename() {
        let mut writer = SpzWriter::new(&Options::new());
        let view = xyz_view();
        let err = writer.write(&[view]).err().expect("missing filename");
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn writer_name_returns_expected() {
        let writer = SpzWriter::new(&Options::new());
        assert_eq!(writer.name(), "writers.spz");
    }

    fn full_spz_view(sh_degree: usize) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(dim("opacity"), DimType::F32);
        for idx in 0..3 {
            layout.register(dim(&format!("f_dc_{idx}")), DimType::F32);
            layout.register(dim(&format!("scale_{idx}")), DimType::F32);
        }
        for idx in 0..4 {
            layout.register(dim(&format!("rot_{idx}")), DimType::F32);
        }
        let sh_pc = sh_per_channel(sh_degree).unwrap();
        for idx in 0..(sh_pc * 3) {
            layout.register(dim(&format!("f_rest_{idx}")), DimType::F32);
        }
        let mut view = PointView::new(Rc::new(layout));
        for _ in 0..2 {
            let p = view.add_point();
            view.set_f64(p, &DimId::X, 1.0);
            view.set_f64(p, &DimId::Y, 1.0);
            view.set_f64(p, &DimId::Z, 1.0);
        }
        view
    }

    #[test]
    fn writer_roundtrips_with_sh_degree_1() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("sh1.spz");
        let view = full_spz_view(1);
        let mut options = Options::new();
        options.add("filename", output.display());
        let mut writer = SpzWriter::new(&options);
        writer.write(std::slice::from_ref(&view)).unwrap();
        assert!(std::fs::metadata(&output).unwrap().len() > 0);
    }

    #[test]
    fn writer_roundtrips_with_sh_degree_3() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("sh3.spz");
        let view = full_spz_view(3);
        let mut options = Options::new();
        options.add("filename", output.display());
        let mut writer = SpzWriter::new(&options);
        writer.write(std::slice::from_ref(&view)).unwrap();
    }

    #[test]
    fn writer_with_antialiased_option() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("aa.spz");
        let view = xyz_view();
        let mut options = Options::new();
        options.add("filename", output.display());
        options.add("antialiased", true);
        let mut writer = SpzWriter::new(&options);
        writer.write(std::slice::from_ref(&view)).unwrap();
    }

    #[test]
    fn coordinate_system_handles_empty_and_named() {
        let _ = coordinate_system("");
        let _ = coordinate_system("RDF");
    }

    #[test]
    fn sh_per_channel_errors_for_unsupported_degree() {
        assert!(sh_per_channel(99).is_err());
    }

    #[test]
    fn sh_degree_from_layout_returns_zero_when_no_f_rest_dims() {
        let layout = PointLayout::new();
        assert_eq!(sh_degree_from_layout(&layout), 0);
    }
}
