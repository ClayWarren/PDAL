use pdal_core::bounds::{parse_bounds2d, Bounds2D};
use std::io::Read;

pub const INVALID_TINDEX_FILTER_STAGE_MESSAGE: &str = "Argument references invalid/unused stage";

#[derive(Clone, Debug)]
pub struct BoundaryOptions {
    pub density: i32,
    pub edge_length: f64,
    pub sample_size: u32,
    pub smooth: bool,
    pub fast_boundary: bool,
    pub where_expr: Option<String>,
}

impl Default for BoundaryOptions {
    fn default() -> Self {
        Self {
            density: 15,
            edge_length: 0.0,
            sample_size: 5000,
            smooth: true,
            fast_boundary: false,
            where_expr: None,
        }
    }
}

impl BoundaryOptions {
    pub fn exact(&self) -> bool {
        !self.fast_boundary
    }
}

#[derive(Clone, Debug)]
pub struct TindexCreateArgs {
    pub tindex_file: String,
    pub files: Vec<String>,
    pub driver_name: String,
    pub target_srs: String,
    pub assign_srs: String,
    pub override_source_srs: bool,
    pub path_prefix: Option<String>,
    pub write_absolute_path: bool,
    pub layer_name: String,
    pub location_field: String,
    pub lco_description: Option<String>,
    pub rich_boundary_options: bool,
    pub boundary: BoundaryOptions,
    stdin_requested: bool,
    input_methods: u8,
    filelists: Vec<String>,
    pub skip_different_srs: bool,
    unsupported_input: bool,
}

impl Default for TindexCreateArgs {
    fn default() -> Self {
        Self {
            tindex_file: String::new(),
            files: Vec::new(),
            driver_name: "ESRI Shapefile".to_string(),
            target_srs: "EPSG:4326".to_string(),
            assign_srs: "EPSG:4326".to_string(),
            override_source_srs: false,
            path_prefix: None,
            write_absolute_path: false,
            layer_name: "pdal".to_string(),
            location_field: "location".to_string(),
            lco_description: None,
            rich_boundary_options: false,
            boundary: BoundaryOptions::default(),
            stdin_requested: false,
            input_methods: 0,
            filelists: Vec::new(),
            skip_different_srs: false,
            unsupported_input: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TindexParseResult {
    Error(String),
    Unsupported,
}

#[derive(Clone, Debug)]
pub struct TindexMergeArgs {
    pub tindex_file: String,
    pub output_file: String,
    pub location_field: String,
    pub target_srs: String,
    pub clip: Option<TindexMergeClip>,
}

#[derive(Clone, Debug)]
pub enum TindexMergeClip {
    Bounds { bounds: Bounds2D, value: String },
    Polygon { value: String },
}

pub fn print_tindex_usage() {
    println!("Usage:");
    println!("  pdal tindex create --tindex <output> <files...> [-f GeoJSON]");
    println!("  pdal tindex create --tindex <output> --filelist <path> [-f GeoJSON]");
    println!("  pdal tindex create --tindex <output> --glob <pattern> [-f GeoJSON]");
    println!("  pdal tindex merge --tindex <index> --filespec <output>");
}

pub fn parse_tindex_create_args(args: &[String]) -> Result<TindexCreateArgs, TindexParseResult> {
    let mut parsed = TindexCreateArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => parsed.tindex_file = tindex_next_value(&mut iter, "--tindex")?.clone(),
            "--filelist" => {
                parsed.input_methods += 1;
                let path = tindex_next_value(&mut iter, "--filelist")?;
                parsed.filelists.push(path.clone());
            }
            "--glob" => {
                parsed.input_methods += 1;
                let pattern = tindex_next_value(&mut iter, "--glob")?;
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
            "--simplify" => {
                parsed.rich_boundary_options = true;
                let value = tindex_next_value(&mut iter, arg)?;
                parsed.boundary.smooth = parse_bool(value, arg)?;
            }
            "--fast_boundary" => {
                parsed.rich_boundary_options = true;
                parsed.boundary.fast_boundary = true;
            }
            "--skip_different_srs" => {
                let value = tindex_next_value(&mut iter, arg)?;
                parsed.skip_different_srs = parse_bool(value, arg)?;
            }
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

pub fn parse_tindex_merge_args(args: &[String]) -> Result<TindexMergeArgs, TindexParseResult> {
    let mut tindex_file = None;
    let mut output_file = None;
    let mut location_field = "location".to_string();
    let mut target_srs = "EPSG:4326".to_string();
    let mut clip = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => tindex_file = Some(tindex_next_value(&mut iter, "--tindex")?.clone()),
            "--filespec" => output_file = Some(tindex_next_value(&mut iter, "--filespec")?.clone()),
            "--tindex_name" => {
                location_field = tindex_next_value(&mut iter, "--tindex_name")?.clone()
            }
            "--bounds" => {
                let value = tindex_next_value(&mut iter, "--bounds")?;
                clip = Some(parse_merge_bounds(value)?);
            }
            "--polygon" => {
                clip = Some(TindexMergeClip::Polygon {
                    value: tindex_next_value(&mut iter, "--polygon")?.clone(),
                });
            }
            "--t_srs" => {
                target_srs = tindex_next_value(&mut iter, "--t_srs")?.clone();
            }
            "--log" => {
                let _ = tindex_next_value(&mut iter, "--log")?;
            }
            "--lyr_name" | "--ogrdriver" | "-f" => {
                let _ = tindex_next_value(&mut iter, arg)?;
            }
            _ if let Some(value) = arg.strip_prefix("--bounds=") => {
                clip = Some(parse_merge_bounds(value)?);
            }
            _ if let Some(value) = arg.strip_prefix("--polygon=") => {
                clip = Some(TindexMergeClip::Polygon {
                    value: value.to_string(),
                });
            }
            _ if let Some(value) = arg.strip_prefix("--t_srs=") => {
                target_srs = value.to_string();
            }
            _ if arg.starts_with("--log=") => {}
            _ if arg.starts_with("--") => {
                return Err(TindexParseResult::Unsupported);
            }
            _ if tindex_file.is_none() => tindex_file = Some(arg.clone()),
            _ if output_file.is_none() => output_file = Some(arg.clone()),
            _ => {
                return Err(TindexParseResult::Error(
                    "too many merge arguments".to_string(),
                ))
            }
        }
    }

    let Some(tindex_file) = tindex_file else {
        return Err(TindexParseResult::Error(
            "merge requires --tindex <index>".to_string(),
        ));
    };
    let Some(output_file) = output_file else {
        return Err(TindexParseResult::Error(
            "merge requires --filespec <output>".to_string(),
        ));
    };
    Ok(TindexMergeArgs {
        tindex_file,
        output_file,
        location_field,
        target_srs,
        clip,
    })
}

fn parse_merge_bounds(value: &str) -> Result<TindexMergeClip, TindexParseResult> {
    let bounds = parse_bounds2d(value, 0)
        .map(|parsed| parsed.bounds)
        .map_err(|err| TindexParseResult::Error(format!("Invalid bounds: {err}")))?;
    Ok(TindexMergeClip::Bounds {
        bounds,
        value: value.to_string(),
    })
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
    } else if let Some(value) = arg.strip_prefix("--simplify=") {
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
        return Err(TindexParseResult::Unsupported);
    };
    if name.eq_ignore_ascii_case("DESCRIPTION") {
        args.lco_description = Some(option.to_string());
        Ok(())
    } else {
        Err(TindexParseResult::Unsupported)
    }
}

pub fn tindex_next_value<'a, I>(iter: &mut I, arg: &str) -> Result<&'a String, TindexParseResult>
where
    I: Iterator<Item = &'a String>,
{
    iter.next()
        .ok_or_else(|| TindexParseResult::Error(format!("{arg} requires a value")))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn create_accepts_positionals_and_defaults() {
        let parsed = parse_tindex_create_args(&strings(&["out.geojson", "in.las"])).unwrap();
        assert_eq!(parsed.tindex_file, "out.geojson");
        assert_eq!(parsed.files, vec!["in.las"]);
        assert_eq!(parsed.driver_name, "ESRI Shapefile");
        assert_eq!(parsed.target_srs, "EPSG:4326");
        assert_eq!(parsed.layer_name, "pdal");
        assert_eq!(parsed.location_field, "location");
    }

    #[test]
    fn create_tracks_boundary_and_srs_options() {
        let parsed = parse_tindex_create_args(&strings(&[
            "--tindex",
            "out.geojson",
            "--filespec=in.las",
            "--threshold=20",
            "--resolution=5.5",
            "--sample_size=100",
            "--simplify=false",
            "--fast_boundary=true",
            "--where=Classification == 2",
            "--a_srs=EPSG:3857",
            "--skip_different_srs=yes",
        ]))
        .unwrap();
        assert_eq!(parsed.files, vec!["in.las"]);
        assert!(parsed.rich_boundary_options);
        assert_eq!(parsed.boundary.density, 20);
        assert_eq!(parsed.boundary.edge_length, 5.5);
        assert_eq!(parsed.boundary.sample_size, 100);
        assert!(!parsed.boundary.smooth);
        assert!(!parsed.boundary.exact());
        assert_eq!(
            parsed.boundary.where_expr.as_deref(),
            Some("Classification == 2")
        );
        assert!(parsed.override_source_srs);
        assert_eq!(parsed.assign_srs, "EPSG:3857");
        assert!(parsed.skip_different_srs);
    }

    #[test]
    fn create_rejects_multiple_input_methods() {
        let Err(err) = parse_tindex_create_args(&strings(&[
            "--tindex",
            "out.geojson",
            "--filespec=a.las",
            "--filelist=list.txt",
        ])) else {
            panic!("expected multiple input methods to fail");
        };
        assert_eq!(
            err,
            TindexParseResult::Error(
                "Can't specify more than one source of tindex input files.".to_string()
            )
        );
    }

    #[test]
    fn merge_accepts_positionals_and_options() {
        let parsed = parse_tindex_merge_args(&strings(&[
            "--tindex",
            "idx.geojson",
            "--filespec",
            "out.las",
            "--tindex_name",
            "path",
            "--t_srs=EPSG:3857",
            "--bounds=([0,1],[2,3])",
        ]))
        .unwrap();
        assert_eq!(parsed.tindex_file, "idx.geojson");
        assert_eq!(parsed.output_file, "out.las");
        assert_eq!(parsed.location_field, "path");
        assert_eq!(parsed.target_srs, "EPSG:3857");
        match parsed.clip.unwrap() {
            TindexMergeClip::Bounds { bounds, value } => {
                assert_eq!(value, "([0,1],[2,3])");
                assert_eq!(bounds.minx, 0.0);
                assert_eq!(bounds.maxx, 1.0);
                assert_eq!(bounds.miny, 2.0);
                assert_eq!(bounds.maxy, 3.0);
            }
            TindexMergeClip::Polygon { .. } => panic!("expected bounds clip"),
        }
    }

    #[test]
    fn merge_tracks_polygon_without_native_geometry() {
        let parsed = parse_tindex_merge_args(&strings(&[
            "idx.geojson",
            "out.las",
            "--polygon=POLYGON ((0 0, 1 0, 1 1, 0 0))",
        ]))
        .unwrap();
        match parsed.clip.unwrap() {
            TindexMergeClip::Polygon { value } => {
                assert!(value.starts_with("POLYGON"));
            }
            TindexMergeClip::Bounds { .. } => panic!("expected polygon clip"),
        }
    }
}
