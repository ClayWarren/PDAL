use std::path::Path;

pub fn valid_plugin_name(
    path: &str,
    types: &[&str],
    dynamic_lib_extension: &str,
) -> Option<String> {
    let mut file = Path::new(path).file_name()?.to_string_lossy().into_owned();
    let leader = "libpdal_plugin_";
    file = file.strip_prefix(leader)?.to_string();

    let type_end = file.find('_')?;
    if type_end >= file.len() - 1 {
        return None;
    }
    let plugin_type = file[..type_end].to_string();
    if !types.iter().any(|ty| ty == &plugin_type) {
        return None;
    }
    file = file[type_end + 1..].to_string();

    let ext_pos = file.rfind('.')?;
    if &file[ext_pos..] != dynamic_lib_extension {
        return None;
    }
    file.truncate(ext_pos);

    if !valid_stage_name(&file) {
        return None;
    }

    Some(format!("{plugin_type}s.{file}"))
}

fn valid_stage_name(name: &str) -> bool {
    let mut chars = name.bytes();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_plugin_filenames_like_cpp_contract() {
        let types = ["reader", "writer"];
        let ext = ".dylib";

        assert_eq!(valid_plugin_name("I'm a plugin", &["foo"], ext), None);
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_ac.dylib.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a_b_c.dylib.dylib", &types, ext),
            None
        );
        assert_eq!(valid_plugin_name("reader_a.dylib", &types, ext), None);
        assert_eq!(
            valid_plugin_name("libpdal_plugin_rea_a.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a.dylib", &["foo"], ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_writer.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_writer_.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_1a_b.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a+_b.dylib", &types, ext),
            None
        );

        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_aP.dylib", &types, ext),
            Some("readers.aP".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_Pa.dylib", &types, ext),
            Some("readers.Pa".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a.dylib", &types, ext),
            Some("readers.a".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_foo_foo.dylib", &["foo"], ext),
            Some("foos.foo".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a_b.dylib", &types, ext),
            Some("readers.a_b".into())
        );
    }
}
