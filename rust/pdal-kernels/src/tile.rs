use pdal_core::kernel::{parse_stage_option, ParseStageResult};
use pdal_core::options::Options;
use std::ffi::CString;

pub enum TileKernelPlan {
    Run(TilePlan),
    Return(i32),
}

pub struct TilePlan {
    pub input: String,
    pub output: String,
    pub length: f64,
    pub origin_x: f64,
    pub origin_y: f64,
    pub buffer: f64,
    pub out_srs: Option<String>,
    pub writer_options: Options,
}

pub fn build_tile_plan(args: &[String]) -> TileKernelPlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.tile: Missing value for positional argument 'input'.");
            return TileKernelPlan::Return(1);
        }
        println!("Usage:");
        println!(
            "  pdal tile <input> <output-template> [--length=N] [--origin_x=X] [--origin_y=Y] [--buffer=N]"
        );
        return TileKernelPlan::Return(0);
    }

    let mut parsed = TileArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Err(code) = parse_tile_arg(arg, &mut iter, &mut parsed) {
            return TileKernelPlan::Return(code);
        }
    }

    let Some(input) = parsed.input else {
        eprintln!("PDAL: kernels.tile: Missing value for positional argument 'input'.");
        return TileKernelPlan::Return(1);
    };
    let Some(output) = parsed.output else {
        eprintln!("PDAL: kernels.tile: Missing value for positional argument 'output'.");
        return TileKernelPlan::Return(1);
    };
    if CString::new(input.as_str()).is_err() || CString::new(output.as_str()).is_err() {
        eprintln!("PDAL: kernels.tile: Path contains an interior NUL byte.");
        return TileKernelPlan::Return(1);
    }
    if output.matches('#').count() != 1 {
        eprintln!(
            "PDAL: kernels.tile: Output filename must contain a single '#' template placeholder."
        );
        return TileKernelPlan::Return(1);
    }

    TileKernelPlan::Run(TilePlan {
        input,
        output,
        length: parsed.length,
        origin_x: parsed.origin_x,
        origin_y: parsed.origin_y,
        buffer: parsed.buffer,
        out_srs: parsed.out_srs,
        writer_options: parsed.writer_options,
    })
}

struct TileArgs {
    input: Option<String>,
    output: Option<String>,
    length: f64,
    origin_x: f64,
    origin_y: f64,
    buffer: f64,
    out_srs: Option<String>,
    writer_options: Options,
}

impl Default for TileArgs {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            length: 1000.0,
            origin_x: f64::NAN,
            origin_y: f64::NAN,
            buffer: 0.0,
            out_srs: None,
            writer_options: Options::new(),
        }
    }
}

fn parse_tile_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut TileArgs,
) -> Result<(), i32> {
    if arg == "--input" || arg == "-i" {
        parsed.input = Some(next_value(arg, iter)?.to_string());
    } else if arg == "--output" || arg == "-o" {
        parsed.output = Some(next_value(arg, iter)?.to_string());
    } else if let Some(rest) = arg.strip_prefix("--") {
        parse_tile_option(rest, arg, iter, parsed)?;
    } else if parsed.input.is_none() {
        parsed.input = Some(arg.to_string());
    } else if parsed.output.is_none() {
        parsed.output = Some(arg.to_string());
    } else {
        eprintln!("PDAL: kernels.tile: Unexpected argument '{arg}'.");
        return Err(1);
    }
    Ok(())
}

fn parse_tile_option<'a>(
    rest: &str,
    original_arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut TileArgs,
) -> Result<(), i32> {
    let (key, value) = match rest.split_once('=') {
        Some(pair) => pair,
        None => (rest, next_value(original_arg, iter)?),
    };
    match key {
        "length" => parsed.length = parse_f64(key, value)?,
        "origin_x" => parsed.origin_x = parse_f64(key, value)?,
        "origin_y" => parsed.origin_y = parse_f64(key, value)?,
        "buffer" => parsed.buffer = parse_f64(key, value)?,
        "out_srs" => parsed.out_srs = Some(value.to_string()),
        _ => {
            let option_text = format!("--{key}={value}");
            let stage_option = parse_stage_option(&option_text, true);
            if stage_option.result == ParseStageResult::Ok && stage_option.stage == "writers.text" {
                parsed
                    .writer_options
                    .add(&stage_option.option, stage_option.value);
            } else {
                return Err(-1);
            }
        }
    }
    Ok(())
}

fn next_value<'a>(
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<&'a str, i32> {
    match iter.next() {
        Some(value) => Ok(value),
        None => {
            eprintln!("PDAL: kernels.tile: Missing value for option '{option}'.");
            Err(1)
        }
    }
}

fn parse_f64(key: &str, value: &str) -> Result<f64, i32> {
    match value.parse::<f64>() {
        Ok(value) => Ok(value),
        Err(_) => {
            eprintln!("PDAL: kernels.tile: Option '--{key}' expects a number.");
            Err(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(args: &[&str]) -> TilePlan {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        match build_tile_plan(&args) {
            TileKernelPlan::Run(plan) => plan,
            TileKernelPlan::Return(code) => panic!("unexpected return: {code}"),
        }
    }

    #[test]
    fn builds_default_tile_plan() {
        let plan = plan(&["in.las", "out#.las"]);
        assert_eq!(plan.input, "in.las");
        assert_eq!(plan.output, "out#.las");
        assert_eq!(plan.length, 1000.0);
        assert_eq!(plan.buffer, 0.0);
        assert!(plan.origin_x.is_nan());
        assert!(plan.origin_y.is_nan());
    }

    #[test]
    fn parses_numeric_options_and_srs() {
        let plan = plan(&[
            "--length=10",
            "--origin_x",
            "1.5",
            "--origin_y=2.5",
            "--buffer",
            "3.5",
            "--out_srs=EPSG:3857",
            "in.las",
            "out#.las",
        ]);
        assert_eq!(plan.length, 10.0);
        assert_eq!(plan.origin_x, 1.5);
        assert_eq!(plan.origin_y, 2.5);
        assert_eq!(plan.buffer, 3.5);
        assert_eq!(plan.out_srs.as_deref(), Some("EPSG:3857"));
    }

    #[test]
    fn accepts_writer_text_options() {
        let plan = plan(&["--writers.text.format=csv", "in.las", "out#.txt"]);
        assert_eq!(plan.writer_options.get_str("format", ""), "csv");
    }

    #[test]
    fn rejects_unknown_options_with_fallback_sentinel() {
        let args = vec![
            "--writers.las.minor_version=4".to_string(),
            "in.las".to_string(),
            "out#.las".to_string(),
        ];
        assert!(matches!(build_tile_plan(&args), TileKernelPlan::Return(-1)));
    }

    #[test]
    fn rejects_output_without_single_hash_placeholder() {
        let args = vec!["in.las".to_string(), "out.las".to_string()];
        assert!(matches!(build_tile_plan(&args), TileKernelPlan::Return(1)));

        let args = vec!["in.las".to_string(), "out##.las".to_string()];
        assert!(matches!(build_tile_plan(&args), TileKernelPlan::Return(1)));
    }
}
