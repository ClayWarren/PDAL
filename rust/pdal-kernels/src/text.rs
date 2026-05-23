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
    fn wraps_multiple_short_words_across_many_lines() {
        assert_eq!(
            word_wrap("one two three four five six seven", 9),
            vec!["one two", "three", "four five", "six seven"]
        );
        assert_eq!(
            word_wrap("north east south west", 6),
            vec!["north", "east", "south", "west"]
        );
    }

    #[test]
    fn preserves_long_words() {
        assert_eq!(
            word_wrap("supercalifragilistic", 5),
            vec!["supercalifragilistic"]
        );
    }

    #[test]
    fn exact_width_words_stay_on_the_current_line() {
        assert_eq!(word_wrap("abc def", 7), vec!["abc def"]);
        assert_eq!(word_wrap("abc def ghi", 7), vec!["abc def", "ghi"]);
    }

    #[test]
    fn empty_and_whitespace_text_return_no_rendered_lines() {
        assert_eq!(word_wrap("", 10), vec![""]);
        assert_eq!(word_wrap("   ", 1), Vec::<String>::new());
    }
}
