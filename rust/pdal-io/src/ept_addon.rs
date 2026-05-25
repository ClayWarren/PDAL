pub fn validate_ept_addon_input(reader_name: &str) -> Result<(), String> {
    if reader_name == "readers.ept" {
        Ok(())
    } else {
        Err("Cannot use writers.ept_addon without reading using readers.ept".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_ept_reader_input() {
        assert!(validate_ept_addon_input("readers.las").is_err());
        assert!(validate_ept_addon_input("readers.ept").is_ok());
    }
}
