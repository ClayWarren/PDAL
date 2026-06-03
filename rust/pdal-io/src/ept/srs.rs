use pdal_core::stage::StageError;
use serde_json::Value;

/// Build the SRS WKT/user-input string from an EPT info `srs` object, matching
/// the C++ `EptInfo::initialize()` rules. Returns `Ok(None)` when no usable
/// `srs` is present (missing, null, or an empty object), otherwise the string
/// that should be handed to `SpatialReference::set`.
pub fn ept_srs_wkt(info: &Value) -> Result<Option<String>, StageError> {
    let srs = match info.get("srs") {
        Some(srs) => srs,
        None => return Ok(None),
    };
    // C++ treats a null or empty srs as "no srs" (`iSrs->size()` is falsy).
    let is_empty = srs.is_null()
        || srs.as_object().map(|o| o.is_empty()).unwrap_or(false)
        || srs.as_array().map(|a| a.is_empty()).unwrap_or(false);
    if is_empty {
        return Ok(None);
    }

    if let Some(wkt) = srs.get("wkt") {
        let wkt = wkt.as_str().ok_or_else(|| {
            StageError(format!(
                "srs.wkt must be specified as a string. Found '{}'.",
                json_dump(wkt)
            ))
        })?;
        return Ok(Some(wkt.to_string()));
    }

    let authority = srs.get("authority");
    let horizontal = srs.get("horizontal");
    if authority.is_none() || horizontal.is_none() {
        return Err(StageError(
            "srs must be defined with at least one of \
             wkt or both authority and horizontal specifications."
                .to_string(),
        ));
    }
    let authority = authority.expect("checked above");
    let horizontal = horizontal.expect("checked above");

    let mut wkt = authority
        .as_str()
        .ok_or_else(|| {
            StageError(format!(
                "srs.authority must be specified as a string.  Found '{}'.",
                json_dump(authority)
            ))
        })?
        .to_string();

    let horiz = json_unsigned_or_string(horizontal).ok_or_else(|| {
        StageError(format!(
            "srs.horizontal must be specified as a non-negative integer or \
             equivalent string. Found '{}'.",
            json_dump(horizontal)
        ))
    })?;
    wkt.push(':');
    wkt.push_str(&horiz);

    if let Some(vertical) = srs.get("vertical") {
        let vert = json_unsigned_or_string(vertical).ok_or_else(|| {
            StageError(format!(
                "srs.vertical must be specified as a non-negative integer or \
                 equivalent string. Found '{}'.",
                json_dump(vertical)
            ))
        })?;
        wkt.push('+');
        wkt.push_str(&vert);
    }

    Ok(Some(wkt))
}

/// Accept a non-negative integer (rendered as its decimal string) or an
/// already-string value, mirroring the C++ `is_number_unsigned()`/`is_string()`
/// branches.
fn json_unsigned_or_string(value: &Value) -> Option<String> {
    if let Some(n) = value.as_u64() {
        Some(n.to_string())
    } else {
        value.as_str().map(|s| s.to_string())
    }
}

/// Compact JSON rendering used in error messages, matching nlohmann `dump()`
/// for the scalar/compound cases EPT srs validation reports.
fn json_dump(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srs_wkt_uses_explicit_wkt() {
        let info = serde_json::json!({ "srs": { "wkt": "EPSG:26915 wkt text" } });
        assert_eq!(
            ept_srs_wkt(&info).unwrap(),
            Some("EPSG:26915 wkt text".to_string())
        );
    }

    #[test]
    fn srs_wkt_builds_from_authority_and_horizontal() {
        let info = serde_json::json!({
            "srs": { "authority": "EPSG", "horizontal": 26915 }
        });
        assert_eq!(ept_srs_wkt(&info).unwrap(), Some("EPSG:26915".to_string()));

        let info = serde_json::json!({
            "srs": { "authority": "EPSG", "horizontal": "26915" }
        });
        assert_eq!(ept_srs_wkt(&info).unwrap(), Some("EPSG:26915".to_string()));
    }

    #[test]
    fn srs_wkt_appends_vertical() {
        let info = serde_json::json!({
            "srs": { "authority": "EPSG", "horizontal": 26915, "vertical": 5703 }
        });
        assert_eq!(
            ept_srs_wkt(&info).unwrap(),
            Some("EPSG:26915+5703".to_string())
        );
    }

    #[test]
    fn srs_wkt_absent_or_empty_is_none() {
        assert_eq!(ept_srs_wkt(&serde_json::json!({})).unwrap(), None);
        assert_eq!(
            ept_srs_wkt(&serde_json::json!({ "srs": {} })).unwrap(),
            None
        );
        assert_eq!(
            ept_srs_wkt(&serde_json::json!({ "srs": null })).unwrap(),
            None
        );
    }

    #[test]
    fn srs_wkt_validation_errors_match_cpp() {
        let err = ept_srs_wkt(&serde_json::json!({ "srs": { "wkt": 5 } }))
            .err()
            .unwrap();
        assert!(err.0.contains("srs.wkt must be specified as a string"));

        let err = ept_srs_wkt(&serde_json::json!({ "srs": { "authority": "EPSG" } }))
            .err()
            .unwrap();
        assert!(err.0.contains("at least one of"));

        let err = ept_srs_wkt(&serde_json::json!({
            "srs": { "authority": "EPSG", "horizontal": -1 }
        }))
        .err()
        .unwrap();
        assert!(err.0.contains("srs.horizontal must be specified"));

        let err = ept_srs_wkt(&serde_json::json!({
            "srs": { "authority": "EPSG", "horizontal": 26915, "vertical": 1.5 }
        }))
        .err()
        .unwrap();
        assert!(err.0.contains("srs.vertical must be specified"));
    }
}
