use super::{
    tindex_next_value, TindexCreateArgs, TindexParseResult, INVALID_TINDEX_FILTER_STAGE_MESSAGE,
};
use std::io::Read;

pub fn parse_tindex_create_args(args: &[String]) -> Result<TindexCreateArgs, TindexParseResult> {
    let mut parsed = TindexCreateArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => parsed.tindex_file = tindex_next_value(&mut iter, "--tindex")?.clone(),
            _ if let Some(value) = arg.strip_prefix("--tindex=") => {
                parsed.tindex_file = value.to_string();
            }
            "--filelist" => {
                parsed.input_methods += 1;
                let path = tindex_next_value(&mut iter, "--filelist")?;
                parsed.filelists.push(path.clone());
            }
            "--glob" | "--filespec" => {
                parsed.input_methods += 1;
                let pattern = tindex_next_value(&mut iter, arg)?;
                parsed.files.extend(read_glob(pattern)?);
            }
            "--path_prefix" => {
                parsed.path_prefix = Some(tindex_next_value(&mut iter, arg)?.clone())
            }
            "--write_absolute_path" => parsed.write_absolute_path = true,
            "--lyr_name" => parsed.layer_name = tindex_next_value(&mut iter, arg)?.clone(),
            "--tindex_name" => parsed.location_field = tindex_next_value(&mut iter, arg)?.clone(),
            "-f" | "--ogrdriver" => parsed.driver_name = tindex_next_value(&mut iter, arg)?.clone(),
            "--threads" | "--requests" => {
                let _ = tindex_next_value(&mut iter, arg)?;
            }
            "--t_srs" => parsed.target_srs = tindex_next_value(&mut iter, arg)?.clone(),
            "--a_srs" => {
                parsed.assign_srs = tindex_next_value(&mut iter, arg)?.clone();
                parsed.override_source_srs = true;
            }
            "--lco" => {
                apply_layer_creation_option(&mut parsed, tindex_next_value(&mut iter, arg)?)?
            }
            "--log" => {
                let _ = tindex_next_value(&mut iter, "--log")?;
            }
            "--stdin" | "-s" => {
                parsed.input_methods += 1;
                parsed.stdin_requested = true;
            }
            "--threshold" => {
                parsed.rich_boundary_options = true;
                let value = tindex_next_value(&mut iter, arg)?;
                parsed.boundary.density = parse_int(value, arg)?;
            }
            "--resolution" | "--edge_length" => {
                parsed.rich_boundary_options = true;
                let value = tindex_next_value(&mut iter, arg)?;
                parsed.boundary.edge_length = parse_float(value, arg)?;
            }
            "--sample_size" => {
                parsed.rich_boundary_options = true;
                let value = tindex_next_value(&mut iter, arg)?;
                parsed.boundary.sample_size = parse_uint(value, arg)?;
            }
            "--simplify" | "--smooth" => {
                parsed.rich_boundary_options = true;
                let value = tindex_next_value(&mut iter, arg)?;
                parsed.boundary.smooth = parse_bool(value, arg)?;
            }
            "--fast_boundary" => {
                parsed.rich_boundary_options = true;
                parsed.boundary.fast_boundary = true;
            }
            "--skip_different_srs" => parsed.skip_different_srs = true,
            "--where" => {
                parsed.rich_boundary_options = true;
                parsed.boundary.where_expr = Some(tindex_next_value(&mut iter, arg)?.clone());
            }
            _ if let Some(value) = arg.strip_prefix("--filespec=") => {
                parsed.input_methods += 1;
                if is_glob_pattern(value) {
                    parsed.files.extend(read_glob(value)?);
                } else {
                    parsed.files.push(value.to_string());
                }
            }
            _ if let Some(pattern) = arg.strip_prefix("--glob=") => {
                parsed.input_methods += 1;
                parsed.files.extend(read_glob(pattern)?);
            }
            _ if let Some(path) = arg.strip_prefix("--filelist=") => {
                parsed.input_methods += 1;
                parsed.filelists.push(path.to_string());
            }
            _ if let Some(value) = arg.strip_prefix("--write_absolute_path=") => {
                parsed.write_absolute_path = matches!(
                    value.to_ascii_lowercase().as_str(),
                    "true" | "1" | "yes" | "on"
                );
            }
            _ if arg
                .strip_prefix("--threads=")
                .or_else(|| arg.strip_prefix("--requests="))
                .is_some() => {}
            _ if let Some(value) = arg.strip_prefix("--path_prefix=") => {
                parsed.path_prefix = Some(value.to_string());
            }
            _ if let Some(value) = arg.strip_prefix("--t_srs=") => {
                parsed.target_srs = value.to_string();
            }
            _ if let Some(value) = arg.strip_prefix("--a_srs=") => {
                parsed.assign_srs = value.to_string();
                parsed.override_source_srs = true;
            }
            _ if arg.starts_with("--log=") => {}
            _ if let Some(value) = arg.strip_prefix("--lco=") => {
                apply_layer_creation_option(&mut parsed, value)?;
            }
            _ if try_parse_boundary_eq_arg(&mut parsed, arg)? => {}
            _ if let Some(value) = arg.strip_prefix("--skip_different_srs=") => {
                parsed.skip_different_srs = parse_bool(value, "--skip_different_srs")?;
            }
            _ if arg.starts_with("--filters.hexbin.smooth") => {
                return Err(TindexParseResult::Error(
                    INVALID_TINDEX_FILTER_STAGE_MESSAGE.to_string(),
                ));
            }
            _ if arg.starts_with("--filters.") => {
                return Err(TindexParseResult::Error(
                    INVALID_TINDEX_FILTER_STAGE_MESSAGE.to_string(),
                ));
            }
            _ if arg.starts_with('-') => {
                return Err(TindexParseResult::Error(format!(
                    "unknown tindex option '{arg}'"
                )));
            }
            _ if parsed.tindex_file.is_empty() => parsed.tindex_file = arg.clone(),
            _ if is_glob_pattern(arg) => {
                parsed.input_methods += 1;
                parsed.files.extend(read_glob(arg)?);
            }
            _ => parsed.files.push(arg.clone()),
        }
    }
    if parsed.input_methods > 1 {
        return Err(TindexParseResult::Error(
            "Can't specify more than one source of tindex input files.".to_string(),
        ));
    }
    if parsed.path_prefix.is_some() && parsed.write_absolute_path {
        return Err(TindexParseResult::Error(
            "Can't specify both --write_absolute_path and --path_prefix options.".to_string(),
        ));
    }
    if parsed.unsupported_input {
        return Err(TindexParseResult::Unsupported);
    }
    for path in &parsed.filelists {
        parsed.files.extend(read_filelist(path)?);
    }
    if parsed.stdin_requested {
        parsed.files.extend(read_stdin_files()?);
    }
    if parsed.tindex_file.is_empty() {
        return Err(TindexParseResult::Error(
            "tindex create requires --tindex <output>".to_string(),
        ));
    }
    if parsed.files.is_empty() {
        return Err(TindexParseResult::Error(
            "tindex create needs at least one input file".to_string(),
        ));
    }
    Ok(parsed)
}

fn try_parse_boundary_eq_arg(
    parsed: &mut TindexCreateArgs,
    arg: &str,
) -> Result<bool, TindexParseResult> {
    if let Some(value) = arg.strip_prefix("--threshold=") {
        parsed.rich_boundary_options = true;
        parsed.boundary.density = parse_int(value, "--threshold")?;
    } else if let Some(value) = arg
        .strip_prefix("--resolution=")
        .or_else(|| arg.strip_prefix("--edge_length="))
    {
        parsed.rich_boundary_options = true;
        parsed.boundary.edge_length = parse_float(value, "--resolution")?;
    } else if let Some(value) = arg.strip_prefix("--sample_size=") {
        parsed.rich_boundary_options = true;
        parsed.boundary.sample_size = parse_uint(value, "--sample_size")?;
    } else if let Some(value) = arg
        .strip_prefix("--simplify=")
        .or_else(|| arg.strip_prefix("--smooth="))
    {
        parsed.rich_boundary_options = true;
        parsed.boundary.smooth = parse_bool(value, "--simplify")?;
    } else if let Some(value) = arg.strip_prefix("--fast_boundary=") {
        parsed.rich_boundary_options = true;
        parsed.boundary.fast_boundary = parse_bool(value, "--fast_boundary")?;
    } else if let Some(value) = arg.strip_prefix("--where=") {
        parsed.rich_boundary_options = true;
        parsed.boundary.where_expr = Some(value.to_string());
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn apply_layer_creation_option(
    args: &mut TindexCreateArgs,
    value: &str,
) -> Result<(), TindexParseResult> {
    let Some((name, option)) = value.split_once('=') else {
        return Err(TindexParseResult::Error(format!(
            "--lco requires NAME=VALUE, got '{value}'"
        )));
    };
    args.lco_options.push(value.to_string());
    if name.eq_ignore_ascii_case("DESCRIPTION") {
        args.lco_description = Some(option.to_string());
    }
    Ok(())
}

fn parse_int(value: &str, arg: &str) -> Result<i32, TindexParseResult> {
    value.parse::<i32>().map_err(|_| {
        TindexParseResult::Error(format!("{arg} requires an integer value, got '{value}'"))
    })
}

fn parse_uint(value: &str, arg: &str) -> Result<u32, TindexParseResult> {
    value.parse::<u32>().map_err(|_| {
        TindexParseResult::Error(format!(
            "{arg} requires a non-negative integer value, got '{value}'"
        ))
    })
}

fn parse_float(value: &str, arg: &str) -> Result<f64, TindexParseResult> {
    value.parse::<f64>().map_err(|_| {
        TindexParseResult::Error(format!("{arg} requires a numeric value, got '{value}'"))
    })
}

fn parse_bool(value: &str, arg: &str) -> Result<bool, TindexParseResult> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(TindexParseResult::Error(format!(
            "{arg} requires a boolean value, got '{value}'"
        ))),
    }
}

fn read_glob(pattern: &str) -> Result<Vec<String>, TindexParseResult> {
    let entries = glob::glob(pattern).map_err(|err| TindexParseResult::Error(format!("{err}")))?;
    let mut files = Vec::new();
    for entry in entries {
        match entry {
            Ok(path) => files.push(path.to_string_lossy().into_owned()),
            Err(_) => return Err(TindexParseResult::Unsupported),
        }
    }
    if files.is_empty() {
        return Err(TindexParseResult::Error(format!(
            "glob pattern '{pattern}' did not match any files"
        )));
    }
    Ok(files)
}

fn is_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

fn read_stdin_files() -> Result<Vec<String>, TindexParseResult> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).map_err(|err| {
        TindexParseResult::Error(format!("unable to read stdin file list: {err}"))
    })?;
    let files = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err(TindexParseResult::Error(
            "stdin contained no tindex input files".to_string(),
        ));
    }
    Ok(files)
}

fn read_filelist(path: &str) -> Result<Vec<String>, TindexParseResult> {
    let input = std::fs::read_to_string(path).map_err(|err| {
        TindexParseResult::Error(format!("unable to read filelist '{path}': {err}"))
    })?;
    let files = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err(TindexParseResult::Error(format!(
            "filelist '{path}' contained no tindex input files"
        )));
    }
    Ok(files)
}
