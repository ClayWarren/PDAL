mod base64;
mod charbuf;
mod command;
mod diff;
mod env;
mod glob;
mod numeric;
mod shell;
mod strings;
mod wrap;

pub use base64::{base64_decode, base64_encode};
pub use charbuf::{charbuf_seekoff, charbuf_seekpos, extract_c_string};
pub use command::run_shell_command;
pub use diff::{diff_files, diff_text_files};
pub use env::{get_env, random, random_seed, set_env, unset_env};
pub use glob::{expand_local_glob, has_glob_pattern};
pub use numeric::{
    compare_approx, format_f64, format_i32, normalize_longitude, numeric_cast_f32_to_f64,
    numeric_cast_f64_to_f32, parse_f64, parse_i32,
};
pub use shell::simple_wordexp;
pub use strings::{
    escape_json, escape_nonprinting_bytes, iequals, looks_like_json, replace_all, split2_char,
    split_char, starts_with, to_lower, to_upper, trim_leading, trim_trailing,
};
pub use wrap::{word_wrap, word_wrap2};

#[cfg(test)]
mod tests;
