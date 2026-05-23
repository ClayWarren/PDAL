fn pdal_header_bounds(views: &[PointView], transforms: &las::Vector<Transform>) -> las::Bounds {
    let mut min = [i32::MAX; 3];
    let mut max = [i32::MIN; 3];

    for view in views {
        for i in 0..view.len() {
            let coords = [
                (view.get_f64(i, &DimId::X), &transforms.x),
                (view.get_f64(i, &DimId::Y), &transforms.y),
                (view.get_f64(i, &DimId::Z), &transforms.z),
            ];
            for (axis, (coord, transform)) in coords.into_iter().enumerate() {
                let scaled = pdal_scaled_i32(coord, transform);
                min[axis] = min[axis].min(scaled);
                max[axis] = max[axis].max(scaled);
            }
        }
    }

    las::Bounds {
        min: las::Vector {
            x: pdal_from_scaled(min[0], &transforms.x),
            y: pdal_from_scaled(min[1], &transforms.y),
            z: pdal_from_scaled(min[2], &transforms.z),
        },
        max: las::Vector {
            x: pdal_from_scaled(max[0], &transforms.x),
            y: pdal_from_scaled(max[1], &transforms.y),
            z: pdal_from_scaled(max[2], &transforms.z),
        },
    }
}

fn patch_pdal_legacy_header_counts(
    path: &Path,
    point_format: u8,
    minor_version: u8,
) -> Result<(), StageError> {
    if point_format < 6 || minor_version < 4 {
        return Ok(());
    }

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| StageError(format!("Failed to reopen LAS/LAZ file: {}", e)))?;
    file.seek(SeekFrom::Start(LEGACY_POINT_COUNT_OFFSET))
        .map_err(|e| StageError(e.to_string()))?;
    file.write_u32::<LittleEndian>(0)
        .map_err(|e| StageError(e.to_string()))?;
    file.seek(SeekFrom::Start(LEGACY_POINTS_BY_RETURN_OFFSET))
        .map_err(|e| StageError(e.to_string()))?;
    for _ in 0..5 {
        file.write_u32::<LittleEndian>(0)
            .map_err(|e| StageError(e.to_string()))?;
    }
    Ok(())
}

fn patch_pdal_header_bounds(
    path: &Path,
    transforms: &las::Vector<Transform>,
    views: &[PointView],
) -> Result<(), StageError> {
    let bounds = pdal_header_bounds(views, transforms);
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| StageError(format!("Failed to reopen LAS/LAZ file: {}", e)))?;
    file.seek(SeekFrom::Start(HEADER_MAX_X_OFFSET))
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.max.x)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.min.x)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.max.y)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.min.y)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.max.z)
        .map_err(|e| StageError(e.to_string()))?;
    file.write_f64::<LittleEndian>(bounds.min.z)
        .map_err(|e| StageError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod header_patch_tests {
    use super::*;

    fn single_point_view(x: f64, y: f64, z: f64) -> PointView {
        let mut layout = pdal_core::point::PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = std::rc::Rc::new(layout);
        let mut view = PointView::new(layout);
        let id = view.add_point();
        view.set_f64(id, &DimId::X, x);
        view.set_f64(id, &DimId::Y, y);
        view.set_f64(id, &DimId::Z, z);
        view
    }

    #[test]
    fn pdal_header_bounds_single_point() {
        let views = vec![single_point_view(1.0, 2.0, 3.0)];
        let xform = las::Transform {
            scale: 0.01,
            offset: 0.0,
        };
        let bounds = pdal_header_bounds(&views, &las::Vector {
            x: xform,
            y: xform,
            z: xform,
        });
        assert!((bounds.min.x - 1.0).abs() < 0.01);
        assert!((bounds.max.x - 1.0).abs() < 0.01);
        assert!((bounds.min.y - 2.0).abs() < 0.01);
        assert!((bounds.max.y - 2.0).abs() < 0.01);
    }

    #[test]
    fn pdal_header_bounds_multiple_points() {
        let views = vec![
            single_point_view(1.0, 10.0, 100.0),
            single_point_view(5.0, 20.0, 50.0),
        ];
        let xform = las::Transform {
            scale: 0.01,
            offset: 0.0,
        };
        let bounds = pdal_header_bounds(&views, &las::Vector {
            x: xform,
            y: xform,
            z: xform,
        });
        assert!((bounds.min.x - 1.0).abs() < 0.01);
        assert!((bounds.max.x - 5.0).abs() < 0.01);
        assert!((bounds.min.y - 10.0).abs() < 0.01);
        assert!((bounds.max.y - 20.0).abs() < 0.01);
    }

    #[test]
    fn pdal_header_bounds_with_offset() {
        let views = vec![single_point_view(100.0, 200.0, 300.0)];
        let xform = las::Transform {
            scale: 0.001,
            offset: 50.0,
        };
        let bounds = pdal_header_bounds(&views, &las::Vector {
            x: xform,
            y: xform,
            z: xform,
        });
        let expected = 100.0;
        assert!((bounds.min.x - expected).abs() < 0.001);
    }
}

