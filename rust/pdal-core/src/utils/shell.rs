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
