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
}
