use std::path::PathBuf;

pub fn has_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

pub fn expand_local_glob(pattern: &str) -> Result<Vec<PathBuf>, String> {
    if pattern.starts_with('~') {
        return Err("PDAL does not support shell expansion".to_string());
    }

    if !has_glob_pattern(pattern) {
        let path = PathBuf::from(pattern);
        if path.exists() {
            return Ok(vec![path]);
        }
        return Err(format!("glob pattern '{pattern}' did not match any files"));
    }

    let glob_error = match glob::glob(pattern) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries {
                files.push(entry.map_err(|err| format!("glob pattern '{pattern}' failed: {err}"))?);
            }
            files.sort();
            if !files.is_empty() {
                return Ok(files);
            }
            None
        }
        Err(err) => Some(err.to_string()),
    };

    let first_wildcard = pattern
        .find(['*', '?', '['])
        .ok_or_else(|| format!("glob pattern '{pattern}' did not match any files"))?;
    let prefix = &pattern[..first_wildcard];
    let Some(sep) = prefix.rfind(['/', '\\']) else {
        return expand_final_component(".", pattern);
    };
    let parent = &pattern[..=sep];
    let file_pattern = &pattern[sep + 1..];
    if has_glob_pattern(parent) {
        return Err(format!(
            "glob pattern '{pattern}' contains unsupported directory wildcards"
        ));
    }
    expand_final_component(parent, file_pattern).map_err(|_| {
        glob_error.unwrap_or_else(|| format!("glob pattern '{pattern}' did not match any files"))
    })
}

fn expand_final_component(parent: &str, file_pattern: &str) -> Result<Vec<PathBuf>, String> {
    if file_pattern.contains('[') {
        return Err(format!(
            "glob pattern '{parent}{file_pattern}' did not match any files"
        ));
    }

    let mut files = Vec::new();
    let include_hidden = file_pattern.starts_with('.');
    let entries = std::fs::read_dir(parent)
        .map_err(|err| format!("glob parent '{parent}' could not be read: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("glob parent '{parent}' failed: {err}"))?;
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        if !include_hidden && filename.starts_with('.') {
            continue;
        }
        if wildcard_match(file_pattern, &filename) {
            files.push(entry.path());
        }
    }
    files.sort();
    if files.is_empty() {
        return Err(format!(
            "glob pattern '{parent}{file_pattern}' did not match any files"
        ));
    }
    Ok(files)
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let mut matched = vec![vec![false; text.len() + 1]; pattern.len() + 1];
    matched[0][0] = true;

    for i in 1..=pattern.len() {
        if pattern[i - 1] == b'*' {
            matched[i][0] = matched[i - 1][0];
        }
    }

    for i in 1..=pattern.len() {
        for j in 1..=text.len() {
            matched[i][j] = match pattern[i - 1] {
                b'*' => matched[i - 1][j] || matched[i][j - 1],
                b'?' => matched[i - 1][j - 1],
                ch => ch == text[j - 1] && matched[i - 1][j - 1],
            };
        }
    }

    matched[pattern.len()][text.len()]
}
