use pdal_core::kernel::{parse_stage_option, ParseStageResult};

pub(crate) fn apply_writer_stage_option(
    arg: &str,
    writer_options: &mut serde_json::Map<String, serde_json::Value>,
) -> bool {
    let parsed = parse_stage_option(arg, true);
    if parsed.result != ParseStageResult::Ok || !parsed.stage.starts_with("writers.") {
        return false;
    }
    writer_options.insert(parsed.option, parse_option_value(&parsed.value));
    true
}

pub(crate) fn parse_option_value(value: &str) -> serde_json::Value {
    if let Ok(number) = value.parse::<u64>() {
        serde_json::json!(number)
    } else if let Ok(number) = value.parse::<f64>() {
        serde_json::json!(number)
    } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        serde_json::json!(value.eq_ignore_ascii_case("true"))
    } else {
        serde_json::json!(value)
    }
}

#[derive(Debug)]
pub(crate) struct CliStageOption {
    pub(crate) stage: String,
    pub(crate) key: String,
    pub(crate) value: String,
}

pub(crate) fn parse_cli_stage_option(arg: &str) -> Option<CliStageOption> {
    let spec = arg.strip_prefix("--")?;
    let (lhs, value) = spec.split_once('=')?;
    let (stage, key) = lhs.rsplit_once('.')?;
    Some(CliStageOption {
        stage: stage.to_string(),
        key: key.to_string(),
        value: value.to_string(),
    })
}
