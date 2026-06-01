pub fn normalize_longitude(mut longitude: f64) -> f64 {
    longitude %= 360.0;
    if longitude <= -180.0 {
        longitude += 360.0;
    } else if longitude > 180.0 {
        longitude -= 360.0;
    }
    longitude
}

pub fn compare_approx(v1: f64, v2: f64, tolerance: f64) -> bool {
    (v1 - v2).abs() <= tolerance.abs()
}

fn trim_trailing_zeros(s: &str) -> String {
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s.to_string()
    }
}

/// Format `value` to match C++ `std::setprecision(precision) << value` with
/// default float formatting (equivalent to `defaultfloat`).
///
/// C++ rules:
/// - NaN → "NaN"
/// - Inf → "Infinity" / "-Infinity"
/// - Zero → "0"
/// - Scientific notation when exponent < -4 or exponent >= precision
/// - Exponent zero-padded to 2 digits with sign (e.g. "e-05", "e+10")
/// - Trailing zeros and trailing decimal points removed from mantissa
pub fn format_f64(value: f64, precision: u32) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    if value == 0.0 {
        return "0".to_string();
    }

    let prec = precision.max(1) as usize;
    let abs_val = value.abs();
    let negative = value.is_sign_negative();

    // Format in scientific with (precision-1) digits after decimal
    // to get `precision` significant digits. This gives the correctly-rounded
    // representation that C++ defaultfloat would use.
    let sci = format!("{:.prec$e}", abs_val, prec = prec - 1);
    let e_pos = sci.rfind('e').unwrap();
    let mantissa = &sci[..e_pos];
    let exponent: i32 = sci[e_pos + 1..].parse().unwrap_or(0);

    // C++ defaultfloat rule: scientific if exponent < -4 or exponent >= precision
    let use_sci = exponent < -4 || exponent >= precision as i32;

    let result = if use_sci {
        let sign = if exponent >= 0 { "+" } else { "-" };
        let exp_str = format!("{}{:02}", sign, exponent.abs());
        let trimmed_mantissa = trim_trailing_zeros(mantissa);
        format!("{}e{}", trimmed_mantissa, exp_str)
    } else {
        let decimals = ((precision as i32) - 1 - exponent).max(0) as usize;
        let formatted = format!("{:.decimals$}", abs_val);
        trim_trailing_zeros(&formatted)
    };

    if negative {
        format!("-{result}")
    } else {
        result
    }
}

pub fn format_i32(value: i32) -> String {
    value.to_string()
}

pub fn parse_i32(value: &str) -> Result<i32, String> {
    let bytes = value.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos == bytes.len() {
        return Err("empty input".to_string());
    }

    let start = pos;
    if bytes[pos] == b'+' || bytes[pos] == b'-' {
        pos += 1;
    }
    if pos == bytes.len() || !bytes[pos].is_ascii_digit() {
        return Err(format!("invalid integer value '{}'", value));
    }
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }

    let int_text = std::str::from_utf8(&bytes[start..pos])
        .map_err(|_| format!("invalid integer value '{}'", value))?;
    let parsed = int_text
        .parse::<i32>()
        .map_err(|_| format!("invalid integer value '{}'", value))?;

    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos != bytes.len() {
        return Err(format!(
            "Found '{}' after valid integral value of '{}'.",
            &value[pos..],
            &value[..pos]
        ));
    }

    Ok(parsed)
}

pub fn numeric_cast_f32_to_f64(value: f32) -> Option<f64> {
    Some(f64::from(value))
}

pub fn numeric_cast_f64_to_f32(value: f64) -> Option<f32> {
    if value.is_nan() {
        return Some(f32::NAN);
    }
    let max = f64::from(f32::MAX);
    let min = f64::from(f32::MIN);
    if value <= max && value >= min {
        Some(value as f32)
    } else {
        None
    }
}

pub fn parse_f64(value: &str) -> Result<f64, String> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("nan") {
        return Ok(f64::NAN);
    }
    trimmed
        .parse::<f64>()
        .map_err(|_| format!("invalid floating point value '{value}'"))
}
