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
