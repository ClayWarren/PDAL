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
