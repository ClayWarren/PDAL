use super::super::*;

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

fn data_path(path: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data")
        .join(path)
        .display()
        .to_string()
}

fn data_file(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data")
        .join(path)
}

unsafe fn preview_options(entries: &[(&str, String)]) -> *mut pdal_core::options::Options {
    let options = pdal_options_create();
    for (key, value) in entries {
        pdal_options_add_str(options, cstring(key).as_ptr(), cstring(value).as_ptr());
    }
    options
}

#[test]
fn ept_preview_returns_bounds_count_srs_and_dim_names() {
    unsafe {
        let path = data_path("ept/lone-star-laszip/ept.json");
        let path_c = cstring(&path);
        let handle = pdal_ept_reader_preview_create(path_c.as_ptr());
        assert!(!handle.is_null());

        assert_eq!(pdal_ept_reader_preview_point_count(handle), 518862);

        let mut minx = 0.0;
        let mut miny = 0.0;
        let mut minz = 0.0;
        let mut maxx = 0.0;
        let mut maxy = 0.0;
        let mut maxz = 0.0;
        assert!(pdal_ept_reader_preview_bounds(
            handle, &mut minx, &mut miny, &mut minz, &mut maxx, &mut maxy, &mut maxz,
        ));
        assert_eq!(minx, 515368.0);
        assert_eq!(maxz, 2339.0);

        let srs = take_string(pdal_ept_reader_preview_srs_wkt(handle));
        assert!(srs.contains("NAD83 / UTM zone 12N"));

        let count = pdal_ept_reader_preview_dim_count(handle);
        // 14 schema dims + 4 laszip class flags.
        assert_eq!(count, 18);

        let mut names: Vec<String> = (0..count)
            .map(|i| take_string(pdal_ept_reader_preview_dim_name(handle, i)))
            .collect();
        names.sort();
        assert!(names.contains(&"X".to_string()));
        assert!(names.contains(&"Withheld".to_string()));
        assert!(names.contains(&"OriginId".to_string()));

        // Out-of-range dim index returns null.
        assert!(pdal_ept_reader_preview_dim_name(handle, count).is_null());

        pdal_ept_reader_preview_destroy(handle);
    }
}

#[test]
fn ept_preview_options_handle_bounds_polygon_and_ogr_filters() {
    unsafe {
        let path = data_path("ept/1.2-with-color/ept.json");
        let selection = std::fs::read_to_string(data_file("autzen/autzen-selection.wkt")).unwrap();
        let source_srs = std::fs::read_to_string(data_file("autzen/autzen-srs.wkt")).unwrap();
        let attributes = data_path("autzen/attributes.json");

        let bounds_options = preview_options(&[
            ("filename", path.clone()),
            ("source_srs", source_srs.clone()),
            (
                "bounds",
                "([636577.1, 637297.4225], [850571.42, 851489.34])".to_string(),
            ),
            ("polygon", selection.clone()),
            ("polygon_srs", "EPSG:3644".to_string()),
        ]);
        let bounds_preview = pdal_ept_reader_preview_create_with_reader_options(bounds_options);
        assert!(!bounds_preview.is_null());
        assert_eq!(pdal_ept_reader_preview_point_count(bounds_preview), 1065);
        pdal_ept_reader_preview_destroy(bounds_preview);
        pdal_options_destroy(bounds_options);

        let polygon_options = preview_options(&[
            ("filename", path.clone()),
            ("source_srs", source_srs.clone()),
            ("polygon", selection),
            ("polygon_srs", "EPSG:3644".to_string()),
        ]);
        let polygon_preview = pdal_ept_reader_preview_create_with_reader_options(polygon_options);
        assert!(!polygon_preview.is_null());
        let polygon_count = pdal_ept_reader_preview_point_count(polygon_preview);
        assert_eq!(polygon_count, 1065);
        pdal_ept_reader_preview_destroy(polygon_preview);
        pdal_options_destroy(polygon_options);

        let ogr = format!(
            r#"{{"type":"ogr","drivers":["GeoJSON"],"datasource":"{}","sql":"select \"_ogr_geometry_\" from attributes"}}"#,
            attributes
        );
        let ogr_options =
            preview_options(&[("filename", path), ("source_srs", source_srs), ("ogr", ogr)]);
        let ogr_preview = pdal_ept_reader_preview_create_with_reader_options(ogr_options);
        assert!(!ogr_preview.is_null());
        let ogr_count = pdal_ept_reader_preview_point_count(ogr_preview);
        assert_eq!(ogr_count, 1065);
        pdal_ept_reader_preview_destroy(ogr_preview);
        pdal_options_destroy(ogr_options);
    }
}

#[test]
fn ept_preview_options_apply_origin_filter() {
    unsafe {
        let options = preview_options(&[
            ("filename", data_path("ept/ellipsoid-binary/ept.json")),
            ("origin", "ellipsoid".to_string()),
        ]);
        let preview = pdal_ept_reader_preview_create_with_reader_options(options);
        assert!(!preview.is_null());
        assert_eq!(pdal_ept_reader_preview_point_count(preview), 100000);

        let mut minx = 0.0;
        let mut miny = 0.0;
        let mut minz = 0.0;
        let mut maxx = 0.0;
        let mut maxy = 0.0;
        let mut maxz = 0.0;
        assert!(pdal_ept_reader_preview_bounds(
            preview, &mut minx, &mut miny, &mut minz, &mut maxx, &mut maxy, &mut maxz,
        ));
        assert_eq!(minx, -8242746.01);
        assert_eq!(maxx, -8242445.99);
        assert_eq!(minz, -50.01);
        assert_eq!(maxz, 50.01);

        pdal_ept_reader_preview_destroy(preview);
        pdal_options_destroy(options);
    }
}

#[test]
fn ept_preview_options_apply_resolution_limit() {
    unsafe {
        let path = data_path("ept/lone-star-laszip/ept.json");
        let path_c = cstring(&path);
        let resolution = cstring("0.1");
        let handle =
            pdal_ept_reader_preview_create_with_options(path_c.as_ptr(), resolution.as_ptr());
        assert!(!handle.is_null());
        assert_eq!(pdal_ept_reader_preview_point_count(handle), 479269);
        pdal_ept_reader_preview_destroy(handle);
    }
}

#[test]
fn ept_preview_options_apply_same_srs_bounds() {
    unsafe {
        let path = data_path("ept/lone-star-laszip/ept.json");
        let path_c = cstring(&path);
        let bounds = cstring("([515380,515400],[4918350,4918370])");
        let handle = pdal_ept_reader_preview_create_with_bounds(
            path_c.as_ptr(),
            std::ptr::null(),
            bounds.as_ptr(),
        );
        assert!(!handle.is_null());
        assert_eq!(pdal_ept_reader_preview_point_count(handle), 430376);

        let mut minx = 0.0;
        let mut miny = 0.0;
        let mut minz = 0.0;
        let mut maxx = 0.0;
        let mut maxy = 0.0;
        let mut maxz = 0.0;
        assert!(pdal_ept_reader_preview_bounds(
            handle, &mut minx, &mut miny, &mut minz, &mut maxx, &mut maxy, &mut maxz,
        ));
        assert_eq!(minx, 515380.0);
        assert_eq!(miny, 4918350.0);
        assert_eq!(minz, 2322.0);
        assert_eq!(maxx, 515400.0);
        assert_eq!(maxy, 4918370.0);
        assert_eq!(maxz, 2339.0);

        pdal_ept_reader_preview_destroy(handle);
    }
}

#[test]
fn ept_preview_options_apply_transformed_bounds() {
    unsafe {
        let path = data_path("ept/lone-star-laszip/ept.json");
        let path_c = cstring(&path);
        let bounds = cstring("([515380,515400],[4918350,4918370]) / EPSG:26912");
        let handle = pdal_ept_reader_preview_create_with_bounds(
            path_c.as_ptr(),
            std::ptr::null(),
            bounds.as_ptr(),
        );
        assert!(!handle.is_null());
        assert_eq!(pdal_ept_reader_preview_point_count(handle), 430376);
        let mut minx = 0.0;
        let mut miny = 0.0;
        let mut minz = 0.0;
        let mut maxx = 0.0;
        let mut maxy = 0.0;
        let mut maxz = 0.0;
        assert!(pdal_ept_reader_preview_bounds(
            handle, &mut minx, &mut miny, &mut minz, &mut maxx, &mut maxy, &mut maxz,
        ));
        assert_eq!(minx, 515380.0);
        assert_eq!(miny, 4918350.0);
        assert_eq!(minz, 2322.0);
        assert_eq!(maxx, 515400.0);
        assert_eq!(maxy, 4918370.0);
        assert_eq!(maxz, 2339.0);
        pdal_ept_reader_preview_destroy(handle);
    }
}

#[test]
fn ept_preview_options_clip_cross_srs_bounds() {
    unsafe {
        let path = data_path("ept/lone-star-laszip/ept.json");
        let path_c = cstring(&path);
        let bounds = cstring(
            "([-110.806808465464,-110.806556642360], \
             [44.418280204485,44.418459837981], \
             [2322,2339]) / EPSG:4326",
        );
        let handle = pdal_ept_reader_preview_create_with_bounds(
            path_c.as_ptr(),
            std::ptr::null(),
            bounds.as_ptr(),
        );
        assert!(!handle.is_null());
        assert_eq!(pdal_ept_reader_preview_point_count(handle), 430376);

        let mut minx = 0.0;
        let mut miny = 0.0;
        let mut minz = 0.0;
        let mut maxx = 0.0;
        let mut maxy = 0.0;
        let mut maxz = 0.0;
        assert!(pdal_ept_reader_preview_bounds(
            handle, &mut minx, &mut miny, &mut minz, &mut maxx, &mut maxy, &mut maxz,
        ));
        assert!((minx - 515380.0).abs() < 0.1);
        assert!((miny - 4918350.0).abs() < 1e-5);
        assert_eq!(minz, 2322.0);
        assert!((maxx - 515400.0).abs() < 0.1);
        assert!((maxy - 4918370.0).abs() < 1e-5);
        assert_eq!(maxz, 2339.0);

        pdal_ept_reader_preview_destroy(handle);
    }
}

#[test]
fn ept_preview_rejects_null_and_missing_files() {
    unsafe {
        assert!(pdal_ept_reader_preview_create(std::ptr::null()).is_null());

        // Missing file -> null + last error set.
        let bad = cstring("/this/path/definitely/does/not/exist/ept.json");
        let handle = pdal_ept_reader_preview_create(bad.as_ptr());
        assert!(handle.is_null());
        let last = pdal_last_error();
        assert!(!last.is_null());
        let msg = std::ffi::CStr::from_ptr(last).to_string_lossy();
        assert!(msg.contains("Can't open"));
    }
}

#[test]
fn ept_preview_accessors_handle_null_handle() {
    unsafe {
        // Each accessor should tolerate a null handle.
        assert_eq!(pdal_ept_reader_preview_point_count(std::ptr::null()), 0);
        assert_eq!(pdal_ept_reader_preview_dim_count(std::ptr::null()), 0);
        assert!(pdal_ept_reader_preview_srs_wkt(std::ptr::null()).is_null());
        assert!(pdal_ept_reader_preview_dim_name(std::ptr::null(), 0).is_null());

        // bounds returns false for null handle (and shouldn't crash).
        let mut a = 0.0;
        let mut b = 0.0;
        let mut c = 0.0;
        let mut d = 0.0;
        let mut e = 0.0;
        let mut f = 0.0;
        assert!(!pdal_ept_reader_preview_bounds(
            std::ptr::null(),
            &mut a,
            &mut b,
            &mut c,
            &mut d,
            &mut e,
            &mut f
        ));

        // Destroy null is a no-op.
        pdal_ept_reader_preview_destroy(std::ptr::null_mut());
    }
}

#[test]
fn ogr_writer_validate_reports_unprefixed_messages() {
    unsafe {
        // Happy path returns null (no error).
        assert!(pdal_ogr_writer_validate(1, 0).is_null());
        assert!(pdal_ogr_writer_validate(3, 0).is_null());

        // multicount = 0 -> error.
        let err = take_string(pdal_ogr_writer_validate(0, 0));
        assert!(err.contains("multicount must be greater than 0"));

        // multicount > 1 with attr_dims -> error.
        let err = take_string(pdal_ogr_writer_validate(3, 2));
        assert!(err.contains("multicount > 1 incompatible with attr_dims"));
    }
}

#[test]
fn ogr_writer_dim_not_found_formats_message_or_returns_null_for_null_input() {
    unsafe {
        let err = take_string(pdal_ogr_writer_dim_not_found(cstring("Bananas").as_ptr()));
        assert_eq!(err, "Dimension 'Bananas' (attr_dims) not found.");

        assert!(pdal_ogr_writer_dim_not_found(std::ptr::null()).is_null());
    }
}
