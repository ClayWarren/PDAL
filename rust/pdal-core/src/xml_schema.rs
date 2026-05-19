//! XML schema compatibility helpers.

pub fn remap_old_dimension_name(input: &str) -> String {
    if input.eq_ignore_ascii_case("Unnamed field 512")
        || input.eq_ignore_ascii_case("Chipper Point ID")
    {
        return "Chipper:PointID".to_string();
    }

    if input.eq_ignore_ascii_case("Unnamed field 513")
        || input.eq_ignore_ascii_case("Chipper Block ID")
    {
        return "Chipper:BlockID".to_string();
    }

    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaps_legacy_chipper_dimension_names() {
        assert_eq!(
            remap_old_dimension_name("Unnamed field 512"),
            "Chipper:PointID"
        );
        assert_eq!(
            remap_old_dimension_name("Chipper Point ID"),
            "Chipper:PointID"
        );
        assert_eq!(
            remap_old_dimension_name("Unnamed field 513"),
            "Chipper:BlockID"
        );
        assert_eq!(
            remap_old_dimension_name("Chipper Block ID"),
            "Chipper:BlockID"
        );
    }

    #[test]
    fn preserves_unknown_dimension_names() {
        assert_eq!(remap_old_dimension_name("Intensity"), "Intensity");
    }
}
