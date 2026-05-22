#[cfg(test)]
mod tests {
    use super::*;
    use crate::las::LasReader;
    use pdal_core::pipeline::Reader;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn pdal_header_bounds_match_scaled_roundtrip() {
        let transforms = las::Vector {
            x: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
            y: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
            z: las::Transform {
                scale: 0.0001,
                offset: 0.0,
            },
        };
        let view = header_bbox_view();
        let bounds = pdal_header_bounds(&[view], &transforms);
        assert!((bounds.min.x - -136.8310).abs() < 1e-4);
        assert!((bounds.max.x - 194.1731).abs() < 1e-4);
        assert!((bounds.min.y - -165.4601).abs() < 1e-4);
        assert!((bounds.max.y - 165.5438).abs() < 1e-4);
        assert!((bounds.min.z - -20.4150).abs() < 1e-4);
        assert!((bounds.max.z - 310.5888).abs() < 1e-4);
    }

    #[test]
    fn writer_honors_global_encoding_option() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-global-encoding-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        options.add("minor_version", "3");
        options.add("dataformat_id", "3");
        options.add("global_encoding", "0");
        LasWriter::new(&options)
            .write(&[synthetic_point_view()])
            .unwrap();

        let reader = las::Reader::from_path(&temp).unwrap();
        let header = reader.header();
        assert_eq!(header.version(), las::Version::new(1, 3));
        assert!(!header.has_wkt_crs());

        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn writer_preserves_synthetic_flag_for_las10() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-synthetic-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        let mut writer = LasWriter::new(&options);
        writer.write(&[synthetic_point_view()]).unwrap();

        let mut reader = las::Reader::from_path(&temp).unwrap();
        let point = reader.points().next().unwrap().unwrap();
        assert!(point.is_synthetic);
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn quantize_scan_angle_matches_pdal_roundtrip() {
        let degrees = -16.998001098632812_f64;
        let target = (degrees / SCAN_ANGLE_SCALE_FACTOR).round() as i16;
        let quantized = quantize_scan_angle(7, degrees);
        let encoded = (f64::from(quantized) / SCAN_ANGLE_SCALE_FACTOR) as i16;
        assert_eq!(encoded, target);
        assert_eq!(target, -2833);
    }

    #[test]
    fn format7_laz_roundtrip_preserves_first_point_fields() {
        let source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/autzen_trim_7.las"
        );
        let output = std::env::temp_dir().join(format!(
            "pdal-format7-laz-{}-{}.laz",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let mut read_options = Options::new();
        read_options.add("filename", source);
        let input = LasReader::new(&read_options)
            .read()
            .expect("read source las");

        let mut write_options = Options::new();
        write_options.add("filename", output.display().to_string());
        write_options.add("dataformat_id", "7");
        write_options.add("minor_version", "4");
        write_options.add("compression", "true");
        LasWriter::new_laz(&write_options)
            .write(&input)
            .expect("write format 7 laz");

        let mut roundtrip_options = Options::new();
        roundtrip_options.add("filename", output.display().to_string());
        let output_views = LasReader::new(&roundtrip_options)
            .read()
            .expect("read written laz");

        let source_view = &input[0];
        let written_view = &output_views[0];
        assert_eq!(source_view.len(), written_view.len());

        let scan_channel = DimId::from_name("ScanChannel");
        for idx in 0..source_view.len().min(10) {
            for dim in [
                DimId::X,
                DimId::Y,
                DimId::Z,
                DimId::Intensity,
                DimId::ReturnNumber,
                DimId::NumberOfReturns,
                DimId::Classification,
                DimId::ScanAngleRank,
                DimId::GpsTime,
                DimId::Red,
                DimId::Green,
                DimId::Blue,
                scan_channel.clone(),
            ] {
                let left = source_view.get_f64(idx, &dim);
                let right = written_view.get_f64(idx, &dim);
                assert!(
                    (left - right).abs() <= 1e-9,
                    "point {idx} dim {:?}: {left} vs {right}",
                    dim
                );
            }
        }

        let _ = std::fs::remove_file(output);
    }

    fn header_bbox_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let coords = [
            (-136.8309503964847, -165.4601240504369, -20.415032985882097),
            (194.17314124182556, 165.54376758787334, 310.58878865242816),
        ];
        for (x, y, z) in coords {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, z);
        }
        view
    }

    fn synthetic_point_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::U8);
        layout.register(DimId::Synthetic, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let id = view.add_point();
        view.set_f64(id, &DimId::X, 1.0);
        view.set_f64(id, &DimId::Y, 2.0);
        view.set_f64(id, &DimId::Z, 3.0);
        view.set_f64(id, &DimId::Classification, 2.0);
        view.set_f64(id, &DimId::Synthetic, 1.0);
        view
    }

    #[test]
    fn writer_discards_high_return_numbers_when_requested() {
        let temp = std::env::temp_dir().join(format!(
            "pdal-las-writer-discard-{}-{}.las",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let mut options = Options::new();
        options.add("filename", temp.display().to_string());
        options.add("minor_version", "2");
        options.add("dataformat_id", "0");
        options.add("discard_high_return_numbers", "true");

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));

        let keep = view.add_point();
        view.set_f64(keep, &DimId::X, 1.0);
        view.set_f64(keep, &DimId::Y, 2.0);
        view.set_f64(keep, &DimId::Z, 3.0);
        view.set_f64(keep, &DimId::ReturnNumber, 5.0);
        view.set_f64(keep, &DimId::NumberOfReturns, 7.0);

        let drop = view.add_point();
        view.set_f64(drop, &DimId::X, 4.0);
        view.set_f64(drop, &DimId::Y, 5.0);
        view.set_f64(drop, &DimId::Z, 6.0);
        view.set_f64(drop, &DimId::ReturnNumber, 6.0);
        view.set_f64(drop, &DimId::NumberOfReturns, 7.0);

        LasWriter::new(&options)
            .write(&[view])
            .expect("write las with discard");

        let mut read_options = Options::new();
        read_options.add("filename", temp.display().to_string());
        let views = LasReader::new(&read_options)
            .read()
            .expect("read las with discard");
        assert_eq!(views[0].len(), 1);
        assert_eq!(views[0].get_f64(0, &DimId::ReturnNumber), 5.0);
        assert_eq!(views[0].get_f64(0, &DimId::NumberOfReturns), 5.0);

        let _ = std::fs::remove_file(temp);
    }
}
