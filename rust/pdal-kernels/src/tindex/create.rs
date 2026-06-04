use super::{
    tindex_next_value, TindexCreateArgs, TindexParseResult, INVALID_TINDEX_FILTER_STAGE_MESSAGE,
};
use std::io::Read;

pub fn parse_tindex_create_args(args: &[String]) -> Result<TindexCreateArgs, TindexParseResult> {
    let mut parsed = TindexCreateArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        parse_tindex_create_arg(arg, &mut iter, &mut parsed)?;
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

fn parse_tindex_create_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut TindexCreateArgs,
) -> Result<(), TindexParseResult> {
    if parse_value_arg(arg, iter, parsed)? {
        return Ok(());
    }
    if parse_equals_arg(arg, parsed)? {
        return Ok(());
    }
    parse_filter_or_positional_arg(arg, parsed)
}

fn parse_value_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut TindexCreateArgs,
) -> Result<bool, TindexParseResult> {
    match arg {
        "--tindex" => parsed.tindex_file = tindex_next_value(iter, "--tindex")?.clone(),
        "--filelist" => {
            parsed.input_methods += 1;
            let path = tindex_next_value(iter, "--filelist")?;
            parsed.filelists.push(path.clone());
        }
        "--glob" | "--filespec" => {
            parsed.input_methods += 1;
            let pattern = tindex_next_value(iter, arg)?;
            parsed.files.extend(read_glob(pattern)?);
        }
        "--path_prefix" => parsed.path_prefix = Some(tindex_next_value(iter, arg)?.clone()),
        "--write_absolute_path" => parsed.write_absolute_path = true,
        "--lyr_name" => parsed.layer_name = tindex_next_value(iter, arg)?.clone(),
        "--tindex_name" => parsed.location_field = tindex_next_value(iter, arg)?.clone(),
        "-f" | "--ogrdriver" => parsed.driver_name = tindex_next_value(iter, arg)?.clone(),
        "--threads" | "--requests" | "--log" => {
            let _ = tindex_next_value(iter, arg)?;
        }
        "--t_srs" => parsed.target_srs = tindex_next_value(iter, arg)?.clone(),
        "--a_srs" => {
            parsed.assign_srs = tindex_next_value(iter, arg)?.clone();
            parsed.override_source_srs = true;
        }
        "--lco" => apply_layer_creation_option(parsed, tindex_next_value(iter, arg)?)?,
        "--stdin" | "-s" => {
            parsed.input_methods += 1;
            parsed.stdin_requested = true;
        }
        "--fast_boundary" => {
            parsed.rich_boundary_options = true;
            parsed.boundary.fast_boundary = true;
        }
        "--skip_different_srs" => parsed.skip_different_srs = true,
        "--threshold" | "--resolution" | "--edge_length" | "--sample_size" | "--simplify"
        | "--smooth" | "--where" => {
            let value = tindex_next_value(iter, arg)?;
            apply_boundary_value(parsed, arg, value)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn parse_equals_arg(arg: &str, parsed: &mut TindexCreateArgs) -> Result<bool, TindexParseResult> {
    if let Some(value) = arg.strip_prefix("--tindex=") {
        parsed.tindex_file = value.to_string();
    } else if let Some(value) = arg.strip_prefix("--filespec=") {
        parsed.input_methods += 1;
        if is_glob_pattern(value) {
            parsed.files.extend(read_glob(value)?);
        } else {
            parsed.files.push(value.to_string());
        }
    } else if let Some(pattern) = arg.strip_prefix("--glob=") {
        parsed.input_methods += 1;
        parsed.files.extend(read_glob(pattern)?);
    } else if let Some(path) = arg.strip_prefix("--filelist=") {
        parsed.input_methods += 1;
        parsed.filelists.push(path.to_string());
    } else if let Some(value) = arg.strip_prefix("--write_absolute_path=") {
        parsed.write_absolute_path = parse_bool(value, "--write_absolute_path")?;
    } else if arg
        .strip_prefix("--threads=")
        .or_else(|| arg.strip_prefix("--requests="))
        .or_else(|| arg.strip_prefix("--log="))
        .is_some()
    {
    } else if let Some(value) = arg.strip_prefix("--path_prefix=") {
        parsed.path_prefix = Some(value.to_string());
    } else if let Some(value) = arg.strip_prefix("--lyr_name=") {
        parsed.layer_name = value.to_string();
    } else if let Some(value) = arg.strip_prefix("--tindex_name=") {
        parsed.location_field = value.to_string();
    } else if let Some(value) = arg
        .strip_prefix("--ogrdriver=")
        .or_else(|| arg.strip_prefix("-f="))
    {
        parsed.driver_name = value.to_string();
    } else if let Some(value) = arg.strip_prefix("--t_srs=") {
        parsed.target_srs = value.to_string();
    } else if let Some(value) = arg.strip_prefix("--a_srs=") {
        parsed.assign_srs = value.to_string();
        parsed.override_source_srs = true;
    } else if let Some(value) = arg.strip_prefix("--lco=") {
        apply_layer_creation_option(parsed, value)?;
    } else if try_parse_boundary_eq_arg(parsed, arg)? {
    } else if let Some(value) = arg.strip_prefix("--skip_different_srs=") {
        parsed.skip_different_srs = parse_bool(value, "--skip_different_srs")?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn parse_filter_or_positional_arg(
    arg: &str,
    parsed: &mut TindexCreateArgs,
) -> Result<(), TindexParseResult> {
    if arg.starts_with("--filters.") {
        return Err(TindexParseResult::Error(
            INVALID_TINDEX_FILTER_STAGE_MESSAGE.to_string(),
        ));
    }
    if arg.starts_with('-') {
        return Err(TindexParseResult::Error(format!(
            "unknown tindex option '{arg}'"
        )));
    }
    if parsed.tindex_file.is_empty() {
        parsed.tindex_file = arg.to_string();
    } else if is_glob_pattern(arg) {
        parsed.input_methods += 1;
        parsed.files.extend(read_glob(arg)?);
    } else {
        parsed.files.push(arg.to_string());
    }
    Ok(())
}

fn apply_boundary_value(
    parsed: &mut TindexCreateArgs,
    arg: &str,
    value: &str,
) -> Result<(), TindexParseResult> {
    parsed.rich_boundary_options = true;
    match arg {
        "--threshold" => parsed.boundary.density = parse_int(value, arg)?,
        "--resolution" | "--edge_length" => parsed.boundary.edge_length = parse_float(value, arg)?,
        "--sample_size" => parsed.boundary.sample_size = parse_uint(value, arg)?,
        "--simplify" | "--smooth" => parsed.boundary.smooth = parse_bool(value, arg)?,
        "--where" => parsed.boundary.where_expr = Some(value.to_string()),
        _ => unreachable!("boundary option checked before dispatch"),
    }
    Ok(())
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
            Err(err) => {
                return Err(TindexParseResult::Error(format!(
                    "glob pattern '{pattern}' failed: {err}"
                )));
            }
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
