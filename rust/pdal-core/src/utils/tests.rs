use super::*;

#[test]
fn test_environment_variable_helpers() {
    let var_name = "PDAL_RUST_TEST_VAR";
    assert_eq!(get_env(var_name), None);
    assert_eq!(set_env(var_name, "value1"), 0);
    assert_eq!(get_env(var_name), Some("value1".to_string()));
    assert_eq!(set_env(var_name, "value2"), 0);
    assert_eq!(get_env(var_name), Some("value2".to_string()));
    assert_eq!(unset_env(var_name), 0);
    assert_eq!(get_env(var_name), None);

    // Invalid key checks
    assert_eq!(set_env("", "val"), -1);
    assert_eq!(set_env("A=B", "val"), -1);
    assert_eq!(set_env("A\0B", "val"), -1);
    assert_eq!(set_env("A", "val\0"), -1);
    assert_eq!(get_env(""), None);
    assert_eq!(get_env("A=B"), None);
    assert_eq!(get_env("A\0B"), None);
}

#[test]
fn compare_approx_respects_tolerance() {
    assert!(!compare_approx(1.001, 1.0, 0.0001));
    assert!(compare_approx(1.001, 1.0, 0.01));
    assert!(compare_approx(10.0, 12.0, 2.0));
}

#[test]
fn formats_nan_inf_and_numbers() {
    assert_eq!(format_f64(f64::NAN, 10), "NaN");
    assert_eq!(format_f64(f64::INFINITY, 10), "Infinity");
    assert_eq!(format_f64(-f64::INFINITY, 10), "-Infinity");
    assert_eq!(format_f64(1.2365, 10), "1.2365");
    assert_eq!(format_i32(12_365_565), "12365565");
}

#[test]
fn numeric_cast_matches_cpp_utils() {
    let nan_f32 = f32::NAN;
    assert!(numeric_cast_f32_to_f64(nan_f32).unwrap().is_nan());
    assert!(numeric_cast_f64_to_f32(f64::NAN).unwrap().is_nan());
    assert_eq!(numeric_cast_f32_to_f64(1.5).unwrap(), 1.5);

    let too_large = f64::from(f32::MAX) * 2.0;
    assert!(numeric_cast_f64_to_f32(too_large).is_none());
    assert!(numeric_cast_f64_to_f32(f64::from(f32::MAX) / 2.0).is_some());
}

#[test]
fn parses_numeric_strings_like_cpp_utils() {
    assert_eq!(parse_i32("12345").unwrap(), 12345);
    assert!(parse_i32("12345.123").is_err());
    assert_eq!(parse_f64("12345.34").unwrap(), 12345.34);
    assert_eq!(parse_f64("12345").unwrap(), 12345.0);
    assert!(parse_f64("foo").is_err());
    assert!(parse_f64("12345.34abc").is_err());
    assert!(parse_f64("NaN").unwrap().is_nan());
}

#[test]
fn test_random_helpers() {
    random_seed(42);
    let first = random(0.0, 100.0);
    assert!((0.0..=100.0).contains(&first));

    random_seed(42);
    let second = random(0.0, 100.0);
    assert_eq!(first, second); // Seed determinism

    let mut sum = 0.0;
    for _ in 0..100 {
        let val = random(-10.0, 10.0);
        assert!((-10.0..=10.0).contains(&val));
        sum += val;
    }
    let avg = sum / 100.0;
    assert!((-5.0..=5.0).contains(&avg));
}

#[test]
fn identifies_json_like_strings() {
    assert!(looks_like_json(r#" {"path":"file.laz"} "#));
    assert!(looks_like_json(" [1, 2, 3] "));
    assert!(looks_like_json(r#" "value" "#));

    assert!(!looks_like_json(""));
    assert!(!looks_like_json("{"));
    assert!(!looks_like_json("file.laz"));
    assert!(!looks_like_json("{not closed"));
}

#[test]
fn encodes_and_decodes_base64() {
    assert_eq!(base64_encode(&[]), "");
    assert_eq!(base64_decode(""), Vec::<u8>::new());
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_decode("Zg=="), b"f");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_decode("Zm8="), b"fo");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_decode("Zm9v"), b"foo");
    assert_eq!(base64_decode("Z"), Vec::<u8>::new());
    assert_eq!(
        base64_encode(&[0x00, 0x01, 0x02, 0xfd, 0xfe, 0xff]),
        "AAEC/f7/"
    );
    assert_eq!(
        base64_decode("AAEC/f7/"),
        [0x00, 0x01, 0x02, 0xfd, 0xfe, 0xff]
    );
}

#[test]
fn trims_and_escapes_strings() {
    assert_eq!(trim_leading("  \t value"), "value");
    assert_eq!(trim_trailing("value  \t "), "value");
    assert_eq!(replace_all(" This  is ", " ", "\""), "\"This\"\"is\"");
    assert_eq!(
        escape_json("\u{0001}\t\u{000C}\n\\\"\u{0016}"),
        "\\u0001\\t\\f\\n\\\\\\\"\\u0016"
    );
    assert_eq!(
        escape_nonprinting_bytes(b"CTRL: \n\x07\x08\r\x0b\x12\x0e\x01"),
        b"CTRL: \\n\\a\\b\\r\\v\\x12\\x0e\\x01"
    );
    assert_eq!(normalize_longitude(181.0), -179.0);
    assert_eq!(normalize_longitude(-181.0), 179.0);
}

#[test]
fn computes_charbuf_seek_positions() {
    assert_eq!(charbuf_seekpos(3, 0, 5, false), Some(3));
    assert_eq!(charbuf_seekpos(5, 0, 5, false), None);
    assert_eq!(charbuf_seekpos(5, 0, 5, true), Some(5));
    assert_eq!(charbuf_seekpos(12, 10, 5, true), Some(2));

    assert_eq!(charbuf_seekoff(2, 0, 10, 5, 0), None);
    assert_eq!(charbuf_seekoff(12, 0, 10, 5, 0), Some(2));
    assert_eq!(charbuf_seekoff(1, 1, 10, 5, 3), Some(4));
    assert_eq!(charbuf_seekoff(2, 2, 10, 5, 0), Some(3));
}

#[test]
fn wraps_words_like_cpp_utils() {
    assert_eq!(
        word_wrap(
            "This   is   a    test    1234567890abcdefghij1234 a   ",
            10,
            12
        ),
        vec!["This is a", "test", "1234567890", "abcdefghij", "1234 a"]
    );
    assert_eq!(
        word_wrap2(
            "This   is   a    test    1234567890abcdefghij1234 a   ",
            10,
            12
        ),
        vec![
            "This   is   ",
            "a    ",
            "test    ",
            "1234567890",
            "abcdefghij",
            "1234 a   "
        ]
    );
}

#[test]
fn expands_simple_shell_words_like_cpp_utils() {
    assert_eq!(
        simple_wordexp("fo\"o\\n= \"b\\\"   ar\" \"b"),
        vec!["foo\\n= b\"", "ar b"]
    );
    assert_eq!(
        simple_wordexp("a b   c   def \"ghi jkl\""),
        vec!["a", "b", "c", "def", "ghi jkl"]
    );
}

#[test]
fn test_diff_files_and_diff_text_files() {
    use std::io::Write;
    let temp_dir = std::env::temp_dir();
    let file1_path = temp_dir.join("pdal_rust_test_diff_1.txt");
    let file2_path = temp_dir.join("pdal_rust_test_diff_2.txt");

    // Write some text of equal length (19 bytes each)
    {
        let mut f1 = std::fs::File::create(&file1_path).unwrap();
        f1.write_all(b"hello world\nline 2\n").unwrap();
        let mut f2 = std::fs::File::create(&file2_path).unwrap();
        f2.write_all(b"hello world\nline 3\n").unwrap();
    }

    // diff_files check
    let d = diff_files(
        file1_path.to_str().unwrap(),
        file2_path.to_str().unwrap(),
        &[],
        &[],
    );
    assert!(d > 0);

    // diff_files with ignorable region (character '2' vs '3' starts at byte 17, length 1)
    let d_ign = diff_files(
        file1_path.to_str().unwrap(),
        file2_path.to_str().unwrap(),
        &[17],
        &[1],
    );
    assert_eq!(d_ign, 0);

    // diff_text_files check
    let dt = diff_text_files(
        file1_path.to_str().unwrap(),
        file2_path.to_str().unwrap(),
        -1,
    );
    assert_eq!(dt, 1);

    // diff_text_files with line ignore
    let dt_ign = diff_text_files(
        file1_path.to_str().unwrap(),
        file2_path.to_str().unwrap(),
        2,
    );
    assert_eq!(dt_ign, 0);

    // Clean up
    let _ = std::fs::remove_file(&file1_path);
    let _ = std::fs::remove_file(&file2_path);
}

#[test]
fn looks_like_json_handles_all_branches() {
    assert!(!looks_like_json(""));
    assert!(!looks_like_json("a"));
    assert!(looks_like_json("{x}"));
    assert!(looks_like_json("[1]"));
    assert!(looks_like_json("\"str\""));
    assert!(!looks_like_json("hello"));
    assert!(!looks_like_json("(1,2)"));
}

#[test]
fn trim_leading_trailing_round_trip() {
    assert_eq!(trim_leading("  hi"), "hi");
    assert_eq!(trim_trailing("hi  "), "hi");
    assert_eq!(trim_leading(""), "");
    assert_eq!(trim_trailing(""), "");
}

#[test]
fn replace_all_handles_empty_pattern() {
    assert_eq!(replace_all("hello", "", "X"), "hello");
    assert_eq!(replace_all("a-b-c", "-", "_"), "a_b_c");
}

#[test]
fn case_helpers_match() {
    assert_eq!(to_lower("ABC"), "abc");
    assert_eq!(to_upper("abc"), "ABC");
    assert!(iequals("abc", "ABC"));
    assert!(!iequals("abc", "abd"));
    assert!(starts_with("hello", "he"));
    assert!(!starts_with("hi", "x"));
}

#[test]
fn split_helpers_handle_empty() {
    assert!(split_char("", ',').is_empty());
    assert!(split2_char("", ',').is_empty());
    assert_eq!(split_char("a,b,", ','), vec!["a", "b", ""]);
    assert_eq!(split2_char("a,,b", ','), vec!["a", "b"]);
}

#[test]
fn escape_json_covers_all_control_chars() {
    let mut input = String::new();
    for ch in 0u32..0x20 {
        if let Some(c) = char::from_u32(ch) {
            input.push(c);
        }
    }
    input.push('"');
    input.push('\\');
    input.push('a');
    let out = escape_json(&input);
    assert!(out.contains("\\u0000"));
    assert!(out.contains("\\t"));
    assert!(out.contains("\\n"));
    assert!(out.contains("\\r"));
    assert!(out.contains("\\b"));
    assert!(out.contains("\\f"));
    assert!(out.contains("\\\""));
    assert!(out.contains("\\\\"));
    assert!(out.ends_with('a'));
}

#[test]
fn escape_nonprinting_bytes_covers_branches() {
    let out = escape_nonprinting_bytes(b"\n\x07\x08\r\x0B\x01a");
    assert!(out.starts_with(b"\\n\\a\\b\\r\\v\\x01"));
    assert!(out.ends_with(b"a"));
}

#[test]
fn normalize_longitude_wraps_to_180_range() {
    assert_eq!(normalize_longitude(0.0), 0.0);
    assert_eq!(normalize_longitude(180.0), 180.0);
    assert_eq!(normalize_longitude(190.0), -170.0);
    assert_eq!(normalize_longitude(-190.0), 170.0);
    let v = normalize_longitude(720.5);
    assert!(v.abs() < 1.0);
}

#[test]
fn compare_approx_branches() {
    assert!(compare_approx(1.0, 1.0001, 0.001));
    assert!(!compare_approx(1.0, 1.5, 0.1));
    assert!(compare_approx(0.0, 0.0, 0.0));
}

#[test]
fn format_f64_special_values() {
    assert_eq!(format_f64(f64::NAN, 6), "NaN");
    assert_eq!(format_f64(f64::INFINITY, 6), "Infinity");
    assert_eq!(format_f64(f64::NEG_INFINITY, 6), "-Infinity");
    assert_eq!(format_f64(0.0, 6), "0");
}

#[test]
fn format_f64_uses_scientific_when_appropriate() {
    let sci = format_f64(0.000001, 6);
    assert!(sci.contains('e'));
    let big = format_f64(1.23456789e10, 6);
    assert!(big.contains('e'));
    let normal = format_f64(123.456, 6);
    assert!(!normal.contains('e'));
}

#[test]
fn format_f64_handles_negative() {
    let v = format_f64(-1.5, 6);
    assert!(v.starts_with('-'));
}

#[test]
fn trim_trailing_zeros_handles_branches() {
    // Direct via format_f64 to exercise the helper
    assert_eq!(format_f64(2.0, 3), "2");
    // No decimal => returned as-is via trim_trailing_zeros's else branch
    let v = format_f64(1.0e10, 3);
    assert!(v.contains('e'));
}

#[test]
fn parse_i32_handles_empty_and_whitespace() {
    assert!(parse_i32("").is_err());
    assert!(parse_i32("   ").is_err());
    assert!(parse_i32("-").is_err()); // sign with no digits
    assert_eq!(parse_i32("  +12  ").unwrap(), 12);
    assert_eq!(parse_i32("  -12  ").unwrap(), -12);
    assert!(parse_i32("12x").is_err()); // trailing junk
    assert!(parse_i32("notnum").is_err());
}

#[test]
fn numeric_cast_f64_to_f32_handles_overflow_and_nan() {
    assert!(numeric_cast_f64_to_f32(f64::NAN).unwrap().is_nan());
    // out of f32 range
    assert!(numeric_cast_f64_to_f32(1e40).is_none());
    assert!(numeric_cast_f64_to_f32(-1e40).is_none());
    // normal value
    assert!((numeric_cast_f64_to_f32(1.0).unwrap() - 1.0).abs() < 1e-6);
}

#[test]
fn word_wrap_empty_returns_empty() {
    assert!(word_wrap("", 10, 5).is_empty());
    assert!(word_wrap2("", 10, 5).is_empty());
}

#[test]
fn word_wrap_handles_first_length_zero() {
    // first_length=0 -> uses line_length
    let v = word_wrap("hello world", 10, 0);
    assert!(!v.is_empty());
}

#[test]
fn word_wrap2_handles_first_length_zero() {
    let v = word_wrap2("hello world", 10, 0);
    assert!(!v.is_empty());
}

#[test]
fn base64_decode_handles_plus_and_slash() {
    let bytes = base64_decode("ab+/");
    assert!(!bytes.is_empty());
}

#[test]
fn base64_decode_handles_invalid_chars() {
    // Invalid characters break out of the loop
    let bytes = base64_decode("!@#$");
    assert!(bytes.is_empty());
}

#[test]
fn env_helpers_handle_invalid_keys() {
    // Empty key returns None / -1
    assert_eq!(get_env(""), None);
    assert_eq!(set_env("", "v"), -1);
    assert_eq!(unset_env(""), -1);
    // Key with '='
    assert_eq!(get_env("a=b"), None);
    assert_eq!(set_env("a=b", "v"), -1);
    assert_eq!(unset_env("a=b"), -1);
}

#[test]
fn random_is_in_range() {
    random_seed(42);
    let v = random(0.0, 1.0);
    assert!((0.0..=1.0).contains(&v));
}

#[test]
fn diff_files_returns_max_for_missing_files() {
    assert_eq!(
        diff_files("/no/such/file_a", "/no/such/file_b", &[], &[]),
        u32::MAX
    );
}

#[test]
fn diff_text_files_returns_max_for_missing_files() {
    assert_eq!(diff_text_files("/no/such/a", "/no/such/b", -1), u32::MAX);
}

#[test]
fn diff_text_files_handles_extra_lines_in_first() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let p1 = dir.join("pdal-rust-diff-extra-1.txt");
    let p2 = dir.join("pdal-rust-diff-extra-2.txt");
    std::fs::File::create(&p1)
        .unwrap()
        .write_all(b"a\nb\nc\nd\n")
        .unwrap();
    std::fs::File::create(&p2)
        .unwrap()
        .write_all(b"a\n")
        .unwrap();
    let d = diff_text_files(p1.to_str().unwrap(), p2.to_str().unwrap(), -1);
    assert!(d > 0);
    let _ = std::fs::remove_file(&p1);
    let _ = std::fs::remove_file(&p2);
}

#[test]
fn diff_text_files_handles_extra_lines_in_second() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let p1 = dir.join("pdal-rust-diff-extra-3.txt");
    let p2 = dir.join("pdal-rust-diff-extra-4.txt");
    std::fs::File::create(&p1)
        .unwrap()
        .write_all(b"a\n")
        .unwrap();
    std::fs::File::create(&p2)
        .unwrap()
        .write_all(b"a\nb\nc\nd\n")
        .unwrap();
    let d = diff_text_files(p1.to_str().unwrap(), p2.to_str().unwrap(), -1);
    assert!(d > 0);
    let _ = std::fs::remove_file(&p1);
    let _ = std::fs::remove_file(&p2);
}

#[test]
fn charbuf_seekpos_branches() {
    // Negative result
    assert!(charbuf_seekpos(0, 10, 100, false).is_none());
    // Within range for output
    assert_eq!(charbuf_seekpos(10, 0, 10, true), Some(10));
    // Equal-to-len for input is rejected
    assert!(charbuf_seekpos(10, 0, 10, false).is_none());
}

#[test]
fn charbuf_seekoff_branches() {
    assert_eq!(charbuf_seekoff(5, 0, 0, 10, 0), Some(5));
    assert_eq!(charbuf_seekoff(2, 1, 0, 10, 3), Some(5));
    assert_eq!(charbuf_seekoff(2, 2, 0, 10, 0), Some(8));
    assert!(charbuf_seekoff(0, 99, 0, 10, 0).is_none());
    // Out of range
    assert!(charbuf_seekoff(100, 0, 0, 10, 0).is_none());
}

#[test]
fn run_shell_command_returns_output() {
    let (status, output) = run_shell_command("echo hi");
    assert_eq!(status, 0);
    assert!(output.contains("hi"));
}
