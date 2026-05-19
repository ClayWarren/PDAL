pub fn level_string(level: i32) -> &'static str {
    match level {
        0 => "Error",
        1 => "Warning",
        2 => "Info",
        _ => "Debug",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_strings_match_cpp_contract() {
        assert_eq!(level_string(0), "Error");
        assert_eq!(level_string(1), "Warning");
        assert_eq!(level_string(2), "Info");
        assert_eq!(level_string(3), "Debug");
        assert_eq!(level_string(8), "Debug");
    }
}
