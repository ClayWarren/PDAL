use pdal_core::point::{DimId, PointView};
use std::collections::HashMap;

pub fn get_kept_indices(view: &PointView, resolution: f64, output_type: &str) -> Vec<u64> {
    if view.is_empty() {
        return Vec::new();
    }

    // 1. Calculate Bounds
    let mut minx = f64::MAX;
    let mut maxx = -f64::MAX;
    let mut miny = f64::MAX;
    let mut maxy = -f64::MAX;

    for idx in 0..view.len() {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        if x < minx {
            minx = x;
        }
        if x > maxx {
            maxx = x;
        }
        if y < miny {
            miny = y;
        }
        if y > maxy {
            maxy = y;
        }
    }

    // 2. Map of (col, row) -> point index
    let mut grid: HashMap<(i32, i32), i64> = HashMap::new();

    // Initialize grid matching createGrid
    let d_width = ((maxx - minx) / resolution).floor() as i32 + 2;
    let d_height = ((maxy - miny) / resolution).floor() as i32 + 2;
    for r in 0..d_height {
        for c in 0..d_width {
            grid.insert((c, r), -1);
        }
    }

    // 3. Process each point
    for idx in 0..view.len() {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);

        let mut col = ((x - minx) / resolution) as i32;
        let mut row = ((y - miny) / resolution) as i32;

        if x < minx + col as f64 * resolution {
            col -= 1;
        }
        if y < miny + row as f64 * resolution {
            row -= 1;
        }
        if x >= minx + (col + 1) as f64 * resolution {
            col += 1;
        }
        if y >= miny + (row + 1) as f64 * resolution {
            row += 1;
        }

        let key = (col, row);
        if let Some(&ref_idx) = grid.get(&key) {
            if ref_idx == -1 {
                grid.insert(key, idx as i64);
            } else {
                let z_ref = view.get_f64(ref_idx as u64, &DimId::Z);
                let keep = if output_type == "min" {
                    z < z_ref
                } else {
                    z > z_ref
                };
                if keep {
                    grid.insert(key, idx as i64);
                }
            }
        } else {
            // fallback for points slightly outside bounds / initialization
            grid.insert(key, idx as i64);
        }
    }

    // 4. Extract non-empty indices
    let mut kept = Vec::new();
    for &val in grid.values() {
        if val != -1 {
            kept.push(val as u64);
        }
    }
    kept
}
