//! FerryFilter: Copy data from one dimension to another.

use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct FerryFilter {
    pub dims: Vec<(String, String)>,
}

impl FerryFilter {
    pub fn new(dims: Vec<(String, String)>) -> Self {
        Self { dims }
    }

    pub fn parse_specs(specs: &[String]) -> Result<Vec<(String, String)>, String> {
        if specs.is_empty() {
            return Err(
                "Must specify at least one dimension to ferry using option 'dimensions'.".into(),
            );
        }
        let mut to_names = Vec::new();
        let mut dims = Vec::new();
        for dim in specs {
            let parts: Vec<&str> = dim.split('=').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Invalid dimension specified '{}'.  Need \
                     <from dimension>=><to dimension>.  See documentation for \
                     details.",
                    dim
                ));
            }
            let from = parts[0].trim().to_string();
            let mut to = parts[1].trim().to_string();
            if let Some(stripped) = to.strip_prefix('>') {
                to = stripped.trim().to_string();
            }
            if from == to {
                return Err(format!("Can't ferry dimension '{}' to itself.", from));
            }
            if to_names.iter().any(|name| name == &to) {
                return Err(
                    "Can't ferry two source dimensions to the same destination dimension.".into(),
                );
            }
            to_names.push(to.clone());
            dims.push((from, to));
        }
        Ok(dims)
    }

    pub fn ferry_point(&self, view: &mut PointView, idx: u64) {
        for (from_name, to_name) in &self.dims {
            let from_id = parse_dim_id(from_name);
            let to_id = parse_dim_id(to_name);
            let val = view.get_f64(idx, &from_id);
            view.set_f64(idx, &to_id, val);
        }
    }
}

fn parse_dim_id(name: &str) -> DimId {
    match name {
        "X" => DimId::X,
        "Y" => DimId::Y,
        "Z" => DimId::Z,
        "Intensity" => DimId::Intensity,
        "OffsetTime" => DimId::OffsetTime,
        "Classification" => DimId::Classification,
        other => DimId::Other(other.to_string()),
    }
}

impl Filter for FerryFilter {
    fn name(&self) -> &str {
        "filters.ferry"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();

        for idx in 0..input.len() {
            out.append_point(input, idx);
            let out_idx = out.len() - 1;
            self.ferry_point(&mut out, out_idx);
        }

        Ok(vec![out])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for FerryFilter {
    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        // FerryFilter processes/modifies points inline rather than keeping/dropping,
        // so its streaming behavior is driven via the FFI pdal_stage_ferry_point.
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_specs_empty_is_error() {
        let result = FerryFilter::parse_specs(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one dimension"));
    }

    #[test]
    fn parse_specs_invalid_format_is_error() {
        let result = FerryFilter::parse_specs(&["X=Z".to_string(), "bad".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid dimension"));
    }

    #[test]
    fn parse_specs_self_ferry_is_error() {
        let result = FerryFilter::parse_specs(&["X=X".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("to itself"));
    }

    #[test]
    fn parse_specs_duplicate_destination_is_error() {
        let result = FerryFilter::parse_specs(&["X=Z".to_string(), "Y=Z".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_specs_arrow_syntax() {
        let result = FerryFilter::parse_specs(&["X => Z".to_string()]);
        assert!(result.is_ok());
        let dims = result.unwrap();
        assert_eq!(dims, vec![("X".to_string(), "Z".to_string())]);
    }

    #[test]
    fn parse_specs_valid() {
        let result = FerryFilter::parse_specs(&["X=Z".to_string(), "Y=Intensity".to_string()]);
        assert!(result.is_ok());
        let dims = result.unwrap();
        assert_eq!(dims.len(), 2);
        assert_eq!(dims[0], ("X".to_string(), "Z".to_string()));
        assert_eq!(dims[1], ("Y".to_string(), "Intensity".to_string()));
    }
}
