/// Wrap text on whitespace without splitting words.
pub fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_not_wrapped() {
        assert_eq!(word_wrap("short", 20), vec!["short"]);
    }

    #[test]
    fn wraps_on_word_boundaries() {
        assert_eq!(
            word_wrap("alpha beta gamma delta", 12),
            vec!["alpha beta", "gamma delta"]
        );
    }

    #[test]
    fn preserves_long_words() {
        assert_eq!(
            word_wrap("supercalifragilistic", 5),
            vec!["supercalifragilistic"]
        );
    }
}
