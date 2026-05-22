pub fn looks_like_json(value: &str) -> bool {
    let value = value.trim();
    if value.len() < 2 {
        return false;
    }

    let Some(first) = value.chars().next() else {
        return false;
    };
    let Some(last) = value.chars().next_back() else {
        return false;
    };

    matches!((first, last), ('{', '}') | ('[', ']') | ('"', '"'))
}

pub fn trim_leading(value: &str) -> String {
    value
        .trim_start_matches(|c: char| c.is_ascii_whitespace())
        .to_string()
}

pub fn trim_trailing(value: &str) -> String {
    value
        .trim_end_matches(|c: char| c.is_ascii_whitespace())
        .to_string()
}

pub fn replace_all(value: &str, replace_what: &str, replace_with: &str) -> String {
    if replace_what.is_empty() {
        return value.to_string();
    }
    value.replace(replace_what, replace_with)
}

pub fn to_lower(value: &str) -> String {
    value.to_ascii_lowercase()
}

pub fn to_upper(value: &str) -> String {
    value.to_ascii_uppercase()
}

pub fn iequals(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

pub fn starts_with(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
}

pub fn split_char(value: &str, split: char) -> Vec<String> {
    if value.is_empty() {
        return Vec::new();
    }
    value.split(split).map(str::to_string).collect()
}

pub fn split2_char(value: &str, split: char) -> Vec<String> {
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(split)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\u{0000}' => out.push_str("\\u0000"),
            '\u{0001}' => out.push_str("\\u0001"),
            '\u{0002}' => out.push_str("\\u0002"),
            '\u{0003}' => out.push_str("\\u0003"),
            '\u{0004}' => out.push_str("\\u0004"),
            '\u{0005}' => out.push_str("\\u0005"),
            '\u{0006}' => out.push_str("\\u0006"),
            '\u{0007}' => out.push_str("\\u0007"),
            '\u{0008}' => out.push_str("\\u0008"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{000B}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\u{000E}' => out.push_str("\\u000E"),
            '\u{000F}' => out.push_str("\\u000F"),
            '\u{0010}' => out.push_str("\\u0010"),
            '\u{0011}' => out.push_str("\\u0011"),
            '\u{0012}' => out.push_str("\\u0012"),
            '\u{0013}' => out.push_str("\\u0013"),
            '\u{0014}' => out.push_str("\\u0014"),
            '\u{0015}' => out.push_str("\\u0015"),
            '\u{0016}' => out.push_str("\\u0016"),
            '\u{0017}' => out.push_str("\\u0017"),
            '\u{0018}' => out.push_str("\\u0018"),
            '\u{0019}' => out.push_str("\\u0019"),
            '\u{001A}' => out.push_str("\\u001A"),
            '\u{001B}' => out.push_str("\\u001B"),
            '\u{001C}' => out.push_str("\\u001C"),
            '\u{001D}' => out.push_str("\\u001D"),
            '\u{001E}' => out.push_str("\\u001E"),
            '\u{001F}' => out.push_str("\\u001F"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn escape_nonprinting_bytes(value: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for &byte in value {
        match byte {
            b'\n' => out.extend_from_slice(b"\\n"),
            0x07 => out.extend_from_slice(b"\\a"),
            0x08 => out.extend_from_slice(b"\\b"),
            b'\r' => out.extend_from_slice(b"\\r"),
            0x0B => out.extend_from_slice(b"\\v"),
            0..=31 => out.extend_from_slice(format!("\\x{byte:02x}").as_bytes()),
            _ => out.push(byte),
        }
    }
    out
}

pub fn normalize_longitude(mut longitude: f64) -> f64 {
    longitude %= 360.0;
    if longitude <= -180.0 {
        longitude += 360.0;
    } else if longitude > 180.0 {
        longitude -= 360.0;
    }
    longitude
}

pub fn compare_approx(v1: f64, v2: f64, tolerance: f64) -> bool {
    (v1 - v2).abs() <= tolerance.abs()
}

fn trim_fractional_zeros(value: &mut String) {
    if !value.contains('.') {
        return;
    }
    while value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
}

pub fn format_f64(value: f64, precision: u32) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    if value == 0.0 {
        return "0".to_string();
    }

    let precision = precision.max(1) as i32;
    let negative = value.is_sign_negative();
    let value = value.abs();
    let order = value.log10().floor() as i32;

    if order >= precision || order < -4 {
        let prec_usize = precision as usize;
        let mut s = format!("{value:.prec_usize$e}");
        trim_fractional_zeros(&mut s);
        return if negative { format!("-{s}") } else { s };
    }

    let decimals = (precision - order - 1).max(0) as usize;
    let mut s = if decimals == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.decimals$}")
    };
    trim_fractional_zeros(&mut s);
    if negative {
        format!("-{s}")
    } else {
        s
    }
}

pub fn format_i32(value: i32) -> String {
    value.to_string()
}

pub fn parse_i32(value: &str) -> Result<i32, String> {
    let bytes = value.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos == bytes.len() {
        return Err("empty input".to_string());
    }

    let start = pos;
    if bytes[pos] == b'+' || bytes[pos] == b'-' {
        pos += 1;
    }
    if pos == bytes.len() || !bytes[pos].is_ascii_digit() {
        return Err(format!("invalid integer value '{}'", value));
    }
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }

    let int_text = std::str::from_utf8(&bytes[start..pos])
        .map_err(|_| format!("invalid integer value '{}'", value))?;
    let parsed = int_text
        .parse::<i32>()
        .map_err(|_| format!("invalid integer value '{}'", value))?;

    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos != bytes.len() {
        return Err(format!(
            "Found '{}' after valid integral value of '{}'.",
            &value[pos..],
            &value[..pos]
        ));
    }

    Ok(parsed)
}

pub fn numeric_cast_f32_to_f64(value: f32) -> Option<f64> {
    Some(f64::from(value))
}

pub fn numeric_cast_f64_to_f32(value: f64) -> Option<f32> {
    if value.is_nan() {
        return Some(f32::NAN);
    }
    let max = f64::from(f32::MAX);
    let min = f64::from(f32::MIN);
    if value <= max && value >= min {
        Some(value as f32)
    } else {
        None
    }
}

pub fn parse_f64(value: &str) -> Result<f64, String> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("nan") {
        return Ok(f64::NAN);
    }
    trimmed
        .parse::<f64>()
        .map_err(|_| format!("invalid floating point value '{value}'"))
}

pub fn base64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);

        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

pub fn word_wrap(value: &str, line_length: usize, first_length: usize) -> Vec<String> {
    if value.is_empty() {
        return Vec::new();
    }

    let line_length = line_length.max(1);
    let mut len = if first_length == 0 {
        line_length
    } else {
        first_length.max(1)
    };
    let mut output = Vec::new();
    let mut line = String::new();

    for mut word in value.split_whitespace().map(str::to_string) {
        if line.len() + word.len() > len && !line.is_empty() {
            output.push(line.trim_end().to_string());
            len = line_length;
            line.clear();
        }

        while word.len() > len {
            output.push(word[..len].to_string());
            word = word[len..].to_string();
            len = line_length;
        }

        line.push_str(&word);
        line.push(' ');
    }

    let trimmed = line.trim_end();
    if !trimmed.is_empty() {
        output.push(trimmed.to_string());
    }
    output
}

pub fn word_wrap2(value: &str, line_length: usize, first_length: usize) -> Vec<String> {
    if value.is_empty() {
        return Vec::new();
    }

    let line_length = line_length.max(1);
    let mut len = if first_length == 0 {
        line_length
    } else {
        first_length.max(1)
    };
    let mut output = Vec::new();
    let mut start = 0usize;
    let bytes = value.as_bytes();

    loop {
        let mut end = (start + len).saturating_sub(1).min(bytes.len() - 1);
        if end + 1 == bytes.len() {
            if start != end + 1 {
                output.push(value[start..=end].to_string());
            }
            return output;
        }

        let mut pos = end;
        while pos > start {
            if bytes[pos].is_ascii_whitespace() && !bytes[pos + 1].is_ascii_whitespace() {
                end = pos;
                break;
            }
            pos -= 1;
        }

        if start != end + 1 {
            output.push(value[start..=end].to_string());
        }
        len = line_length;
        start = end + 1;
    }
}

pub fn simple_wordexp(value: &str) -> Vec<String> {
    let mut temp = String::new();
    let mut in_string = false;
    let mut escape = false;
    let mut args = Vec::new();

    for ch in value.chars() {
        if in_string {
            if escape {
                if ch != '"' && ch != '\\' {
                    temp.push('\\');
                }
                escape = false;
                temp.push(ch);
            } else if ch == '"' {
                in_string = false;
            } else if ch == '\\' {
                escape = true;
            } else {
                temp.push(ch);
            }
        } else if escape {
            escape = false;
            temp.push(ch);
        } else if ch == '"' {
            in_string = true;
        } else if ch == '\\' {
            escape = true;
        } else if ch.is_ascii_whitespace() {
            if !temp.is_empty() {
                args.push(std::mem::take(&mut temp));
            }
        } else {
            temp.push(ch);
        }
    }

    if !in_string && !temp.is_empty() {
        args.push(temp);
    }
    args
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub fn base64_decode(value: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut quartet = [0_u8; 4];
    let mut count = 0_usize;

    for byte in value.bytes() {
        if byte == b'=' {
            break;
        }
        let Some(decoded) = base64_value(byte) else {
            break;
        };
        quartet[count] = decoded;
        count += 1;

        if count == 4 {
            out.push((quartet[0] << 2) | ((quartet[1] & 0x30) >> 4));
            out.push(((quartet[1] & 0x0f) << 4) | ((quartet[2] & 0x3c) >> 2));
            out.push(((quartet[2] & 0x03) << 6) | quartet[3]);
            count = 0;
        }
    }

    if count > 1 {
        for item in quartet.iter_mut().skip(count) {
            *item = 0;
        }
        out.push((quartet[0] << 2) | ((quartet[1] & 0x30) >> 4));
        if count > 2 {
            out.push(((quartet[1] & 0x0f) << 4) | ((quartet[2] & 0x3c) >> 2));
        }
        if count > 3 {
            out.push(((quartet[2] & 0x03) << 6) | quartet[3]);
        }
    }

    out
}

pub fn charbuf_seekpos(pos: i64, offset: i64, len: i64, for_output: bool) -> Option<i64> {
    let adjusted = pos.checked_sub(offset)?;
    if adjusted < 0 {
        return None;
    }
    if for_output {
        (adjusted <= len).then_some(adjusted)
    } else {
        (adjusted < len).then_some(adjusted)
    }
}

pub fn charbuf_seekoff(off: i64, dir: u8, offset: i64, len: i64, current: i64) -> Option<i64> {
    let target = match dir {
        0 => off.checked_sub(offset)?,
        1 => current.checked_add(off)?,
        2 => len.checked_sub(off)?,
        _ => return None,
    };

    (0..=len).contains(&target).then_some(target)
}

use std::sync::{Mutex, OnceLock};

static RNG_STATE: OnceLock<Mutex<u64>> = OnceLock::new();

fn get_rng_state() -> &'static Mutex<u64> {
    RNG_STATE.get_or_init(|| Mutex::new(1))
}

pub fn get_env(key: &str) -> Option<String> {
    if key.is_empty() || key.contains('=') || key.contains('\0') {
        return None;
    }
    std::env::var(key).ok()
}

pub fn set_env(key: &str, value: &str) -> i32 {
    if key.is_empty() || key.contains('=') || key.contains('\0') || value.contains('\0') {
        return -1;
    }
    std::env::set_var(key, value);
    0
}

pub fn unset_env(key: &str) -> i32 {
    if key.is_empty() || key.contains('=') || key.contains('\0') {
        return -1;
    }
    std::env::remove_var(key);
    0
}

pub fn random_seed(seed: u32) {
    let mut state = get_rng_state().lock().unwrap();
    *state = seed as u64;
}

pub fn random(minimum: f64, maximum: f64) -> f64 {
    let mut state = get_rng_state().lock().unwrap();
    let next = state.wrapping_mul(1103515245).wrapping_add(12345);
    *state = next;

    let r = next as f64 / u64::MAX as f64;
    let val = minimum + r * (maximum - minimum);
    val.clamp(minimum, maximum)
}

#[cfg(test)]
mod tests {
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
}
