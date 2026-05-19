#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseStageResult {
    Ok,
    Invalid,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedStageOption {
    pub result: ParseStageResult,
    pub stage: String,
    pub option: String,
    pub value: String,
}

pub fn parse_stage_option(input: &str, allow_stage_prefix: bool) -> ParsedStageOption {
    let mut parsed = ParsedStageOption {
        result: ParseStageResult::Unknown,
        stage: String::new(),
        option: String::new(),
        value: String::new(),
    };

    let Some(option_text) = input.strip_prefix("--") else {
        return parsed;
    };
    if option_text.is_empty() {
        return parsed;
    }

    let mut pos = take_while(option_text, 0, u8::is_ascii_lowercase);
    let stage_type = &option_text[..pos];
    if !is_stage_prefix(stage_type, allow_stage_prefix)
        || option_text.as_bytes().get(pos) != Some(&b'.')
    {
        return parsed;
    }
    pos += 1;

    if !parse_stage_name(option_text, &mut pos) {
        return parsed;
    }
    parsed.stage = option_text[..pos].to_string();
    if option_text.as_bytes().get(pos) != Some(&b'.') {
        parsed.stage.clear();
        return parsed;
    }
    pos += 1;

    let option_start = pos;
    pos += parse_option_name(option_text, pos);
    parsed.option = option_text[option_start..pos].to_string();

    if pos >= option_text.len() {
        parsed.result = ParseStageResult::Ok;
        return parsed;
    }

    if option_text.as_bytes()[pos] == b'=' {
        parsed.value = option_text[pos + 1..].to_string();
        if !parsed.value.is_empty() {
            parsed.result = ParseStageResult::Ok;
            return parsed;
        }
    }
    parsed.result = ParseStageResult::Invalid;
    parsed
}

fn is_stage_prefix(stage_type: &str, allow_stage_prefix: bool) -> bool {
    matches!(stage_type, "readers" | "writers" | "filters")
        || (allow_stage_prefix && stage_type == "stage")
}

fn parse_stage_name(input: &str, pos: &mut usize) -> bool {
    if input
        .as_bytes()
        .get(*pos)
        .is_none_or(|c| !c.is_ascii_alphabetic())
    {
        return false;
    }
    *pos += 1;
    *pos = take_while(input, *pos, |c| c.is_ascii_alphanumeric() || *c == b'_');
    true
}

fn parse_option_name(input: &str, pos: usize) -> usize {
    if input
        .as_bytes()
        .get(pos)
        .is_none_or(|c| !c.is_ascii_lowercase())
    {
        return 0;
    }
    let end = take_while(input, pos + 1, |c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_'
    });
    end - pos
}

fn take_while(input: &str, start: usize, predicate: impl Fn(&u8) -> bool) -> usize {
    let mut pos = start;
    while input.as_bytes().get(pos).is_some_and(&predicate) {
        pos += 1;
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stage_options_like_cpp_kernel() {
        let parsed = parse_stage_option("--readers.p2g.foobar=baz", false);
        assert_eq!(parsed.result, ParseStageResult::Ok);
        assert_eq!(parsed.stage, "readers.p2g");
        assert_eq!(parsed.option, "foobar");
        assert_eq!(parsed.value, "baz");

        assert_eq!(
            parse_stage_option("--readers.2pg.foobar=baz", false).result,
            ParseStageResult::Unknown
        );
        assert_eq!(
            parse_stage_option("--read1ers.las.foobar=baz", false).result,
            ParseStageResult::Unknown
        );

        let parsed = parse_stage_option("--readers.p2g.foobar", false);
        assert_eq!(parsed.result, ParseStageResult::Ok);
        assert_eq!(parsed.value, "");

        assert_eq!(
            parse_stage_option("--readers.p2g.foobar=", false).result,
            ParseStageResult::Invalid
        );
        assert_eq!(
            parse_stage_option("--readers.p2g.foobar!", false).result,
            ParseStageResult::Invalid
        );
    }

    #[test]
    fn stage_prefix_is_opt_in_for_pipeline_kernel() {
        assert_eq!(
            parse_stage_option("--stage.tag.option=value", false).result,
            ParseStageResult::Unknown
        );
        assert_eq!(
            parse_stage_option("--stage.tag.option=value", true).result,
            ParseStageResult::Ok
        );
    }
}
