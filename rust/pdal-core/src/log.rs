pub fn level_string(level: i32) -> &'static str {
    match level {
        0 => "Error",
        1 => "Warning",
        2 => "Info",
        _ => "Debug",
    }
}

/// Compose the leader prefix that C++ `Log::get(level)` emits before each line.
///
/// Mirrors `pdal::Log::get` formatting: `"(LEADER LEVEL) "` with an optional
/// `" S.SSS"` elapsed-seconds segment when `timing` is true, followed by one
/// tab per debug sub-level (Debug1..Debug5).
pub fn format_prefix(leader: &str, level: i32, timing: bool, elapsed_seconds: f64) -> String {
    let mut out = String::with_capacity(32);
    out.push('(');
    out.push_str(leader);
    if !leader.is_empty() {
        out.push(' ');
    }
    out.push_str(level_string(level));
    if timing {
        out.push(' ');
        out.push_str(&format!("{:.3}", elapsed_seconds));
    }
    out.push_str(") ");

    let debug_native: i32 = 3; // LogLevel::Debug
    let tabs = if level < debug_native {
        0
    } else {
        (level - debug_native) as usize
    };
    for _ in 0..tabs {
        out.push('\t');
    }
    out
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

    #[test]
    fn prefix_without_timing() {
        assert_eq!(format_prefix("PDAL", 3, false, 0.0), "(PDAL Debug) ");
        assert_eq!(format_prefix("PDAL", 0, false, 0.0), "(PDAL Error) ");
        assert_eq!(format_prefix("PDAL", 2, false, 0.0), "(PDAL Info) ");
    }

    #[test]
    fn prefix_with_timing_three_decimals() {
        assert_eq!(
            format_prefix("PDAL", 3, true, 0.1234),
            "(PDAL Debug 0.123) "
        );
        assert_eq!(format_prefix("PDAL", 3, true, 1.0), "(PDAL Debug 1.000) ");
    }

    #[test]
    fn prefix_empty_leader_skips_space() {
        assert_eq!(format_prefix("", 1, false, 0.0), "(Warning) ");
    }

    #[test]
    fn prefix_indents_debug_sub_levels() {
        assert_eq!(format_prefix("PDAL", 4, false, 0.0), "(PDAL Debug) \t");
        assert_eq!(
            format_prefix("PDAL", 8, false, 0.0),
            "(PDAL Debug) \t\t\t\t\t"
        );
    }

    #[test]
    fn prefix_does_not_indent_below_debug() {
        assert_eq!(format_prefix("PDAL", 0, false, 0.0), "(PDAL Error) ");
        assert_eq!(format_prefix("PDAL", 2, false, 0.0), "(PDAL Info) ");
    }
}
