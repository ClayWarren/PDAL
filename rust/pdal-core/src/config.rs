pub fn version_integer(major: i32, minor: i32, patch: i32) -> i32 {
    major * 100 * 100 + minor * 100 + patch
}

pub fn full_version_string(version: &str, sha: &str) -> String {
    let git_version = if sha.eq_ignore_ascii_case("Release") {
        sha.to_string()
    } else {
        sha.chars().take(6).collect()
    };
    format!("{version} (git-version: {git_version})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_pdal_version_integer() {
        assert_eq!(version_integer(2, 10, 1), 21001);
        assert_eq!(version_integer(1, 9, 0), 10900);
    }

    #[test]
    fn formats_full_version_string_like_cpp() {
        assert_eq!(
            full_version_string("2.10.1", "abcdef123456"),
            "2.10.1 (git-version: abcdef)"
        );
        assert_eq!(
            full_version_string("2.10.1", "Release"),
            "2.10.1 (git-version: Release)"
        );
        assert_eq!(
            full_version_string("2.10.1", "release"),
            "2.10.1 (git-version: release)"
        );
    }
}
