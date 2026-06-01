use super::*;

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "pdal-analysis-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn tindex_filelist_and_plain_arguments_parse() {
    let list = temp_file("filelist", " a.las\n\nb.las \n");
    let args = vec![
        "--tindex".to_string(),
        "out.geojson".to_string(),
        "--filelist".to_string(),
        list.to_string_lossy().into_owned(),
        "--path_prefix".to_string(),
        "data".to_string(),
        "--write_absolute_path".to_string(),
        "--lyr_name".to_string(),
        "tiles".to_string(),
        "--tindex_name".to_string(),
        "location_path".to_string(),
        "--ogrdriver".to_string(),
        "GeoJSON".to_string(),
        "--fast_boundary".to_string(),
        "c.las".to_string(),
    ];

    let parsed = parse_tindex_create_args(&args).unwrap();

    assert_eq!(parsed.tindex_file, "out.geojson");
    assert_eq!(parsed.files, vec!["a.las", "b.las", "c.las"]);
    assert_eq!(parsed.path_prefix.as_deref(), Some("data"));
    assert!(parsed.write_absolute_path);
    assert_eq!(parsed.layer_name, "tiles");
    assert_eq!(parsed.location_field, "location_path");
    assert_eq!(parsed.driver_name, "GeoJSON");
    let _ = std::fs::remove_file(list);
}

#[test]
fn tindex_glob_and_validation_errors_are_reported() {
    let first = temp_file("glob-a", "");
    let second = temp_file("glob-b", "");
    let pattern = first
        .with_file_name("pdal-analysis-glob-*")
        .to_string_lossy()
        .into_owned();

    let mut files = read_tindex_glob(&pattern).unwrap();
    files.sort();
    assert!(files.iter().any(|path| path == &first.to_string_lossy()));
    assert!(files.iter().any(|path| path == &second.to_string_lossy()));

    assert!(read_tindex_filelist("/definitely/not/here").is_err());
    assert!(read_tindex_glob("[").is_err());
    assert!(read_tindex_glob("/definitely/not/here/*").is_err());
    assert!(parse_tindex_create_args(&[]).is_err());
    assert!(
        parse_tindex_create_args(&["--tindex".to_string(), "out.geojson".to_string()]).is_err()
    );

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
}

#[test]
fn tindex_bounds_and_location_helpers_cover_error_paths() {
    let good = serde_json::json!({
        "bounds_2d": {
            "minx": 1.0,
            "miny": 2.0,
            "maxx": 3.0,
            "maxy": 4.0
        }
    });
    assert_eq!(
        tindex_bounds("good.las", &good).unwrap(),
        (1.0, 2.0, 3.0, 4.0)
    );
    assert!(tindex_bounds("missing.las", &serde_json::json!({})).is_err());
    assert!(tindex_bounds(
        "bad-minx.las",
        &serde_json::json!({"bounds_2d": {"minx": "x", "miny": 2.0, "maxx": 3.0, "maxy": 4.0}})
    )
    .is_err());
    assert!(tindex_bounds(
        "bad-maxx.las",
        &serde_json::json!({"bounds_2d": {"minx": 1.0, "miny": 2.0, "maxx": "x", "maxy": 4.0}})
    )
    .is_err());
    assert!(tindex_bounds(
        "bad-miny.las",
        &serde_json::json!({"bounds_2d": {"minx": 1.0, "miny": "x", "maxx": 3.0, "maxy": 4.0}})
    )
    .is_err());
    assert!(tindex_bounds(
        "bad-maxy.las",
        &serde_json::json!({"bounds_2d": {"minx": 1.0, "miny": 2.0, "maxx": 3.0, "maxy": "x"}})
    )
    .is_err());

    let path = temp_file("location", "");
    assert_eq!(
        tindex_location("relative.las", false).unwrap(),
        "relative.las"
    );
    assert_eq!(
        tindex_location(path.to_str().unwrap(), true).unwrap(),
        path.canonicalize().unwrap().to_string_lossy()
    );
    assert!(tindex_location("/definitely/not/here.las", true).is_err());
    let _ = std::fs::remove_file(path);
}
