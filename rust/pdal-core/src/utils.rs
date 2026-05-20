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

#[cfg(test)]
mod tests {
    use super::*;

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
}
