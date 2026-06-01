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

pub fn extract_c_string(buffer: &[u8], offset: usize, count: usize) -> String {
    if count == 0 || offset >= buffer.len() {
        return String::new();
    }

    let end = offset.saturating_add(count).min(buffer.len());
    let bytes = &buffer[offset..end];
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
