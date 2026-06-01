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
