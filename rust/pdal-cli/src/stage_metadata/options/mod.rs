mod filters;
mod readers;
mod writers;

use serde_json::json;

pub(crate) fn stage_options(stage_name: &str) -> Vec<serde_json::Value> {
    if stage_name.starts_with("readers.") {
        return readers::options(stage_name);
    }
    if stage_name.starts_with("filters.") {
        return filters::options(stage_name);
    }
    if stage_name.starts_with("writers.") {
        return writers::options(stage_name);
    }
    Vec::new()
}

fn filename() -> serde_json::Value {
    json!({"arg": "filename", "description": "Input or output filename."})
}

fn option(arg: &str, description: &str, default: Option<serde_json::Value>) -> serde_json::Value {
    let mut value = json!({"arg": arg, "description": description});
    if let Some(default) = default {
        value["default"] = default;
    }
    value
}
