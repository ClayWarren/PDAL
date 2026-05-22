use super::*;
use std::ffi::CString;

fn cstr(value: &str) -> CString {
    CString::new(value).unwrap()
}

#[test]
fn handle_filename_template_locates_placeholder() {
    unsafe {
        let mut pos: usize = 0;

        let none = cstr("output.las");
        assert!(pdal_writer_handle_filename_template(
            none.as_ptr(),
            &mut pos
        ));
        assert_eq!(pos, PDAL_WRITER_NO_TEMPLATE);

        let templated = cstr("out_#.las");
        assert!(pdal_writer_handle_filename_template(
            templated.as_ptr(),
            &mut pos
        ));
        assert_eq!(pos, 4);
    }
}

#[test]
fn handle_filename_template_rejects_invalid_templates() {
    unsafe {
        let mut pos: usize = 0;

        let suffix = cstr("output.la#s");
        assert!(!pdal_writer_handle_filename_template(
            suffix.as_ptr(),
            &mut pos
        ));

        let doubled = cstr("out_#_#.las");
        assert!(!pdal_writer_handle_filename_template(
            doubled.as_ptr(),
            &mut pos
        ));

        // A null filename or output pointer fails cleanly.
        assert!(!pdal_writer_handle_filename_template(
            std::ptr::null(),
            &mut pos
        ));
        let ok = cstr("out_#.las");
        assert!(!pdal_writer_handle_filename_template(
            ok.as_ptr(),
            std::ptr::null_mut()
        ));
    }
}

#[test]
fn replace_tags_substitutes_uuid_tags() {
    unsafe {
        let untagged = cstr("output_#.las");
        let result = take_string(pdal_writer_replace_tags(untagged.as_ptr()));
        assert_eq!(result, "output_#.las");

        let tagged = cstr("#_#uuid#_foo.txt");
        let result = take_string(pdal_writer_replace_tags(tagged.as_ptr()));
        assert!(result.starts_with("#_"));
        assert!(result.ends_with("_foo.txt"));
        let uuid = &result["#_".len()..result.len() - "_foo.txt".len()];
        assert_eq!(uuid.len(), 36);
        assert!(uuid
            .chars()
            .all(|c| c == '-' || (c.is_ascii_hexdigit() && !c.is_ascii_uppercase())));

        assert!(pdal_writer_replace_tags(std::ptr::null()).is_null());
    }
}
