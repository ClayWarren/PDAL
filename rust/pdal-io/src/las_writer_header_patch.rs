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

