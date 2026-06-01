/// Compare two files byte-by-byte, returning the number of differing bytes.
/// If either file does not exist or fails to open, returns u32::MAX.
pub fn diff_files(
    file1: &str,
    file2: &str,
    ignorable_starts: &[u32],
    ignorable_lengths: &[u32],
) -> u32 {
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::path::Path;

    let path1 = Path::new(file1);
    let path2 = Path::new(file2);
    if !path1.exists() || !path2.exists() {
        return u32::MAX;
    }

    let f1 = match File::open(path1) {
        Ok(f) => BufReader::new(f),
        Err(_) => return u32::MAX,
    };
    let f2 = match File::open(path2) {
        Ok(f) => BufReader::new(f),
        Err(_) => return u32::MAX,
    };

    let mut bytes1 = f1.bytes();
    let mut bytes2 = f2.bytes();
    let mut num_diffs = 0u32;
    let mut i = 0u32;

    loop {
        let b1 = bytes1.next();
        let b2 = bytes2.next();

        match (b1, b2) {
            (Some(Ok(p)), Some(Ok(q))) => {
                if p != q {
                    let mut is_ignorable = false;
                    for (&start, &len) in ignorable_starts.iter().zip(ignorable_lengths) {
                        let end = start.saturating_add(len);
                        if i >= start && i < end {
                            is_ignorable = true;
                            break;
                        }
                    }
                    if !is_ignorable {
                        num_diffs += 1;
                    }
                }
            }
            (None, None) => break,
            (Some(Ok(_)), None) | (None, Some(Ok(_))) => {
                num_diffs += 1;
                break;
            }
            _ => {
                num_diffs += 1;
                break;
            }
        }
        i += 1;
    }

    num_diffs
}

/// Compare two text files line-by-line, stripping CRLF and returning the number of differing lines.
/// If either file does not exist or fails to open, returns u32::MAX.
pub fn diff_text_files(file1: &str, file2: &str, ignore_line: i32) -> u32 {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    let path1 = Path::new(file1);
    let path2 = Path::new(file2);
    if !path1.exists() || !path2.exists() {
        return u32::MAX;
    }

    let f1 = match File::open(path1) {
        Ok(f) => f,
        Err(_) => return u32::MAX,
    };
    let f2 = match File::open(path2) {
        Ok(f) => f,
        Err(_) => return u32::MAX,
    };

    let mut reader1 = BufReader::new(f1);
    let mut reader2 = BufReader::new(f2);
    let mut num_diffs = 0u32;
    let mut curr_line = 1i32;

    loop {
        let mut line1 = String::new();
        let mut line2 = String::new();

        let len1 = reader1.read_line(&mut line1).unwrap_or(0);
        let len2 = reader2.read_line(&mut line2).unwrap_or(0);

        if len1 == 0 && len2 == 0 {
            break;
        }

        if curr_line == ignore_line {
            curr_line += 1;
            continue;
        }

        if len1 == 0 && len2 > 0 {
            num_diffs += 1;
            loop {
                let mut rest2 = String::new();
                if reader2.read_line(&mut rest2).unwrap_or(0) == 0 {
                    break;
                }
                num_diffs += 1;
            }
            break;
        } else if len1 > 0 && len2 == 0 {
            num_diffs += 1;
            loop {
                let mut rest1 = String::new();
                if reader1.read_line(&mut rest1).unwrap_or(0) == 0 {
                    break;
                }
                num_diffs += 1;
            }
            break;
        }

        let clean1 = line1.replace(['\r', '\n'], "");
        let clean2 = line2.replace(['\r', '\n'], "");

        if clean1 != clean2 {
            num_diffs += 1;
        }

        curr_line += 1;
    }

    num_diffs
}
