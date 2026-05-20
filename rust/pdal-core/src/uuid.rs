pub fn parse_uuid(input: &str) -> Option<[u8; 16]> {
    let bytes = input.as_bytes();
    if bytes.len() != 36 {
        return None;
    }

    for &idx in &[8, 13, 18, 23] {
        if bytes[idx] != b'-' {
            return None;
        }
    }

    let hex_positions = [
        0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 14, 15, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35,
    ];
    let mut out = [0u8; 16];
    for (i, pair) in hex_positions.chunks_exact(2).enumerate() {
        let high = hex_value(bytes[pair[0]])?;
        let low = hex_value(bytes[pair[1]])?;
        out[i] = (high << 4) | low;
    }
    Some(out)
}

pub fn unparse_uuid(bytes: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

pub fn random_v4_uuid_bytes() -> Result<[u8; 16], getrandom::Error> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(bytes)
}

pub fn is_null_uuid(bytes: &[u8; 16]) -> bool {
    bytes.iter().all(|&b| b == 0)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_unparses_canonical_uuid_text() {
        let text = "5CE0E9A5-6015-FEC5-AADF-A328AE398115";
        let bytes = parse_uuid(text).unwrap();
        assert_eq!(
            bytes,
            [
                0x5c, 0xe0, 0xe9, 0xa5, 0x60, 0x15, 0xfe, 0xc5, 0xaa, 0xdf, 0xa3, 0x28, 0xae, 0x39,
                0x81, 0x15
            ]
        );
        assert_eq!(unparse_uuid(&bytes), text);
    }

    #[test]
    fn rejects_malformed_uuid_text() {
        assert!(parse_uuid("foo").is_none());
        assert!(parse_uuid("5CE0E9A5_6015-FEC5-AADF-A328AE398115").is_none());
        assert!(parse_uuid("5CE0E9A5-6015-FEC5-AADF-A328AE39811Z").is_none());
    }

    #[test]
    fn random_uuid_sets_version_and_variant_bits() {
        let bytes = random_v4_uuid_bytes().unwrap();
        let text = unparse_uuid(&bytes);
        assert_eq!(text.as_bytes()[14], b'4');
        assert!(matches!(text.as_bytes()[19], b'8'..=b'B'));
    }
}
