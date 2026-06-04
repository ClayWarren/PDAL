use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Deserialize)]
pub struct ArgSpec {
    pub name: String,
    #[serde(default)]
    pub short: Option<String>,
    pub kind: ArgKind,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub positional: bool,
    #[serde(default)]
    pub optional_positional: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    Bool,
    Double,
    Int,
    String,
    IntVec,
    StringVec,
    RegexVec,
    Json,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ParseResult {
    pub values: BTreeMap<String, Value>,
    pub remaining: Vec<String>,
}

#[derive(Clone, Debug)]
struct ArgState {
    spec: ArgSpec,
    value: Value,
    explicit: bool,
}

pub fn parse_program_args(
    specs: &[ArgSpec],
    input: &[String],
    simple: bool,
) -> Result<ParseResult, String> {
    validate_specs(specs)?;
    let mut states = initial_states(specs);
    let indexes = build_indexes(specs);
    let mut positionals = positional_order(specs)?;
    let mut remaining = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let token = &input[i];
        if token == "--" {
            if i + 1 >= input.len() {
                return Err("No argument found following '--'.".to_string());
            }
            for value in &input[i + 1..] {
                assign_positional(&mut states, &mut positionals, value, simple)?;
            }
            break;
        } else if let Some(rest) = token.strip_prefix("--") {
            match parse_long(rest, input, &mut i, &indexes, &mut states, &mut positionals) {
                Ok(()) => {}
                Err(_) if simple => remaining.push(token.clone()),
                Err(err) => return Err(err),
            }
        } else if is_short_option(token) {
            match parse_short(
                token,
                input,
                &mut i,
                &indexes,
                &mut states,
                &mut positionals,
            ) {
                Ok(()) => {}
                Err(_) if simple => remaining.push(token.clone()),
                Err(err) => return Err(err),
            }
        } else {
            assign_positional(&mut states, &mut positionals, token, simple)?;
        }
        i += 1;
    }

    ensure_required_positionals(&states)?;
    Ok(ParseResult {
        values: states
            .into_iter()
            .map(|(name, state)| (name, state.value))
            .collect(),
        remaining,
    })
}

fn validate_specs(specs: &[ArgSpec]) -> Result<(), String> {
    for spec in specs {
        if spec.name.is_empty() {
            return Err("No program argument provided.".to_string());
        }
        if let Some(short) = &spec.short {
            if short.chars().count() != 1 {
                return Err("Short argument not specified as single character".to_string());
            }
        }
        if spec.positional && spec.kind == ArgKind::Bool {
            return Err(format!(
                "Boolean argument '{}' can't be positional.",
                spec.name
            ));
        }
    }
    Ok(())
}

fn initial_states(specs: &[ArgSpec]) -> BTreeMap<String, ArgState> {
    specs
        .iter()
        .map(|spec| {
            let value = spec.default.clone().unwrap_or_else(|| match spec.kind {
                ArgKind::Bool => Value::Bool(false),
                ArgKind::IntVec | ArgKind::StringVec | ArgKind::RegexVec => json!([]),
                ArgKind::Json => Value::Null,
                ArgKind::Double => json!(0.0),
                ArgKind::Int => json!(0),
                ArgKind::String => Value::String(String::new()),
            });
            (
                spec.name.clone(),
                ArgState {
                    spec: spec.clone(),
                    value,
                    explicit: false,
                },
            )
        })
        .collect()
}

fn build_indexes(specs: &[ArgSpec]) -> HashMap<String, String> {
    let mut indexes = HashMap::new();
    for spec in specs {
        indexes.insert(format!("--{}", spec.name), spec.name.clone());
        if let Some(short) = &spec.short {
            indexes.insert(format!("-{short}"), spec.name.clone());
        }
        for alias in &spec.aliases {
            indexes.insert(format!("--{alias}"), spec.name.clone());
        }
    }
    indexes
}

fn positional_order(specs: &[ArgSpec]) -> Result<Vec<String>, String> {
    let mut seen_optional = false;
    let mut out = Vec::new();
    for spec in specs {
        if spec.optional_positional {
            seen_optional = true;
            out.push(spec.name.clone());
        } else if spec.positional {
            if seen_optional {
                return Err(format!(
                    "Found required positional argument '{}' after optional positional argument.",
                    spec.name
                ));
            }
            out.push(spec.name.clone());
        }
    }
    Ok(out)
}

fn parse_long(
    rest: &str,
    input: &[String],
    i: &mut usize,
    indexes: &HashMap<String, String>,
    states: &mut BTreeMap<String, ArgState>,
    positionals: &mut Vec<String>,
) -> Result<(), String> {
    if rest.is_empty() {
        return Err("No argument found following '--'.".to_string());
    }
    let (flag, explicit) = match rest.split_once('=') {
        Some((name, value)) => (format!("--{name}"), Some(value.to_string())),
        None => (format!("--{rest}"), None),
    };
    let Some(name) = indexes.get(&flag) else {
        return Err(format!("Unexpected argument '{}'.", rest));
    };
    let state = states.get_mut(name).expect("indexed arg exists");
    if state.spec.kind == ArgKind::Bool {
        if let Some(value) = explicit {
            match value.as_str() {
                "true" => state.value = Value::Bool(true),
                "false" => state.value = Value::Bool(false),
                _ => {
                    return Err(format!(
                        "Value '{}' provided for argument '{}' when 'true' or 'false' is expected.",
                        value, state.spec.name
                    ));
                }
            }
        } else {
            state.value = Value::Bool(!bool_default(&state.spec));
        }
        state.explicit = true;
    } else {
        let value = match explicit {
            Some(value) => value,
            None => next_value(input, i, &state.spec.name)?,
        };
        apply_value(state, &value)?;
    }
    mark_positional_satisfied(positionals, name);
    Ok(())
}

fn bool_default(spec: &ArgSpec) -> bool {
    spec.default
        .as_ref()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn parse_short(
    token: &str,
    input: &[String],
    i: &mut usize,
    indexes: &HashMap<String, String>,
    states: &mut BTreeMap<String, ArgState>,
    positionals: &mut Vec<String>,
) -> Result<(), String> {
    let mut chars = token
        .strip_prefix('-')
        .unwrap_or_default()
        .chars()
        .peekable();
    if chars.peek().is_none() {
        return Err("No argument found following '-'.".to_string());
    }
    while let Some(ch) = chars.next() {
        let flag = format!("-{ch}");
        let Some(name) = indexes.get(&flag) else {
            return Err(format!("Unexpected argument '-{ch}'."));
        };
        let state = states.get_mut(name).expect("indexed arg exists");
        if state.spec.kind == ArgKind::Bool {
            state.value = Value::Bool(true);
            state.explicit = true;
        } else {
            if chars.peek().is_some() {
                return Err(format!(
                    "Short option '{}' expects value but appears in option group '{}'.",
                    ch, token
                ));
            }
            let value = next_value(input, i, &state.spec.name)?;
            apply_value(state, &value)?;
        }
        mark_positional_satisfied(positionals, name);
    }
    Ok(())
}

fn is_short_option(token: &str) -> bool {
    token.starts_with('-')
        && !token.starts_with("--")
        && token.len() > 1
        && token.chars().nth(1).is_some_and(|ch| !ch.is_ascii_digit())
}

fn next_value(input: &[String], i: &mut usize, name: &str) -> Result<String, String> {
    *i += 1;
    let Some(value) = input.get(*i) else {
        return Err(format!(
            "Argument '{}' needs a value and none was provided.",
            name
        ));
    };
    if value.starts_with('-') && !is_number_like(value) {
        return Err(format!(
            "Argument '{}' needs a value and none was provided.",
            name
        ));
    }
    Ok(value.clone())
}

fn is_number_like(value: &str) -> bool {
    value
        .strip_prefix('-')
        .and_then(|rest| rest.chars().next())
        .is_some_and(|ch| ch.is_ascii_digit())
}

fn assign_positional(
    states: &mut BTreeMap<String, ArgState>,
    positionals: &mut Vec<String>,
    value: &str,
    simple: bool,
) -> Result<(), String> {
    let Some(name) = positionals.first().cloned() else {
        return Err(format!("Unexpected argument '{}'.", value));
    };
    let state = states.get_mut(&name).expect("positional arg exists");
    if !simple
        && state.spec.positional
        && matches!(
            state.spec.kind,
            ArgKind::IntVec | ArgKind::StringVec | ArgKind::RegexVec
        )
    {
        return Err(format!("Unexpected argument '{}'.", value));
    }
    apply_value(state, value)?;
    if !state.spec.optional_positional
        && !matches!(
            state.spec.kind,
            ArgKind::IntVec | ArgKind::StringVec | ArgKind::RegexVec
        )
    {
        positionals.remove(0);
    }
    Ok(())
}

fn ensure_required_positionals(states: &BTreeMap<String, ArgState>) -> Result<(), String> {
    for state in states.values() {
        if state.spec.positional
            && !matches!(
                state.spec.kind,
                ArgKind::IntVec | ArgKind::StringVec | ArgKind::RegexVec
            )
            && is_empty_default(&state.value, &state.spec.kind)
        {
            return Err(format!(
                "Missing value for positional argument '{}'.",
                state.spec.name
            ));
        }
    }
    Ok(())
}

fn is_empty_default(value: &Value, kind: &ArgKind) -> bool {
    match kind {
        ArgKind::String => value.as_str().unwrap_or_default().is_empty(),
        ArgKind::IntVec | ArgKind::StringVec | ArgKind::RegexVec => {
            value.as_array().is_none_or(Vec::is_empty)
        }
        _ => false,
    }
}

fn apply_value(state: &mut ArgState, value: &str) -> Result<(), String> {
    if !state.explicit
        && matches!(
            state.spec.kind,
            ArgKind::IntVec | ArgKind::StringVec | ArgKind::RegexVec
        )
    {
        state.value = json!([]);
    }
    state.explicit = true;
    match state.spec.kind {
        ArgKind::Bool => state.value = Value::Bool(true),
        ArgKind::Double => {
            let parsed = value
                .parse::<f64>()
                .map_err(|_| format!("Invalid value for argument '{}'.", state.spec.name))?;
            state.value = json_f64(parsed);
        }
        ArgKind::Int => {
            state.value = json!(value
                .parse::<i64>()
                .map_err(|_| { format!("Invalid value for argument '{}'.", state.spec.name) })?);
        }
        ArgKind::String => state.value = Value::String(value.to_string()),
        ArgKind::IntVec => {
            let parsed = value
                .parse::<i64>()
                .map_err(|_| format!("Invalid value for argument '{}'.", state.spec.name))?;
            push_array_value(&mut state.value, json!(parsed));
        }
        ArgKind::StringVec => {
            for item in value.split(',') {
                push_array_value(&mut state.value, json!(item));
            }
        }
        ArgKind::RegexVec => push_array_value(&mut state.value, json!(value)),
        ArgKind::Json => {
            state.value = serde_json::from_str(value)
                .map_err(|_| format!("Invalid value for argument '{}'.", state.spec.name))?;
        }
    }
    Ok(())
}

fn json_f64(value: f64) -> Value {
    if value.is_finite() {
        json!(value)
    } else {
        Value::String(value.to_string())
    }
}

fn mark_positional_satisfied(positionals: &mut Vec<String>, name: &str) {
    if positionals.first().is_some_and(|pos| pos == name) {
        positionals.remove(0);
    }
}

fn push_array_value(target: &mut Value, value: Value) {
    if !target.is_array() {
        *target = json!([]);
    }
    target.as_array_mut().expect("array value").push(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn str_arg() -> ArgSpec {
        ArgSpec {
            name: "foo".to_string(),
            short: Some("f".to_string()),
            kind: ArgKind::String,
            default: Some(json!("foo")),
            positional: false,
            optional_positional: false,
            aliases: Vec::new(),
        }
    }

    #[test]
    fn parses_short_groups_and_negative_values() {
        let specs = vec![
            str_arg(),
            ArgSpec {
                name: "baz".to_string(),
                short: Some("z".to_string()),
                kind: ArgKind::Bool,
                default: None,
                positional: false,
                optional_positional: false,
                aliases: Vec::new(),
            },
            ArgSpec {
                name: "vec".to_string(),
                short: None,
                kind: ArgKind::IntVec,
                default: None,
                positional: false,
                optional_positional: false,
                aliases: Vec::new(),
            },
        ];
        let input = vec![
            "-zf".to_string(),
            "hello".to_string(),
            "--vec=-1".to_string(),
            "--vec".to_string(),
            "2".to_string(),
        ];
        let parsed = parse_program_args(&specs, &input, false).unwrap();
        assert_eq!(parsed.values["foo"], json!("hello"));
        assert_eq!(parsed.values["baz"], json!(true));
        assert_eq!(parsed.values["vec"], json!([-1, 2]));
    }

    #[test]
    fn parse_simple_keeps_unknown_long_options() {
        let mut spec = str_arg();
        spec.positional = true;
        let input = vec![
            "--holy=Holy".to_string(),
            "--foo".to_string(),
            "Foo".to_string(),
            "--cow=Moo".to_string(),
        ];
        let parsed = parse_program_args(&[spec], &input, true).unwrap();
        assert_eq!(parsed.values["foo"], json!("Foo"));
        assert_eq!(parsed.remaining, vec!["--holy=Holy", "--cow=Moo"]);
    }

    #[test]
    fn long_bool_options_accept_explicit_values_and_invert_defaults() {
        let specs = vec![
            ArgSpec {
                name: "enabled".to_string(),
                short: None,
                kind: ArgKind::Bool,
                default: None,
                positional: false,
                optional_positional: false,
                aliases: Vec::new(),
            },
            ArgSpec {
                name: "disabled".to_string(),
                short: None,
                kind: ArgKind::Bool,
                default: Some(json!(true)),
                positional: false,
                optional_positional: false,
                aliases: Vec::new(),
            },
        ];

        let parsed = parse_program_args(
            &specs,
            &["--enabled=true".to_string(), "--disabled=false".to_string()],
            false,
        )
        .unwrap();
        assert_eq!(parsed.values["enabled"], json!(true));
        assert_eq!(parsed.values["disabled"], json!(false));

        let parsed = parse_program_args(
            &specs,
            &["--enabled".to_string(), "--disabled".to_string()],
            false,
        )
        .unwrap();
        assert_eq!(parsed.values["enabled"], json!(true));
        assert_eq!(parsed.values["disabled"], json!(false));

        assert!(parse_program_args(&specs, &["--enabled=maybe".to_string()], false).is_err());
    }

    #[test]
    fn parses_double_values_and_nonfinite_values() {
        let specs = vec![ArgSpec {
            name: "value".to_string(),
            short: None,
            kind: ArgKind::Double,
            default: None,
            positional: false,
            optional_positional: false,
            aliases: Vec::new(),
        }];

        let parsed =
            parse_program_args(&specs, &["--value=1.23456789012345".to_string()], false).unwrap();
        assert_eq!(parsed.values["value"], json!(1.23456789012345_f64));

        let parsed = parse_program_args(&specs, &["--value=NaN".to_string()], false).unwrap();
        assert_eq!(parsed.values["value"], json!("NaN"));

        assert!(parse_program_args(&specs, &["--value=not-a-number".to_string()], false).is_err());
    }
}
