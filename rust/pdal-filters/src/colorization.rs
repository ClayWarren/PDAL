//! `filters.colorization` -- assignments colors from a GDAL-readable datasource.

use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct BandInfo {
    pub name: String,
    pub band: u32,
    pub scale: f64,
}

/// Parse one `name:band:scale` dimension spec, mirroring the C++
/// `ColorizationFilter` `parseDim`. Unsupplied band numbers fall back to
/// `default_band`; the default scale is 1.0.
fn parse_dim(dim: &str, default_band: u32) -> Result<BandInfo, String> {
    let bytes = dim.as_bytes();
    let mut pos = 0usize;
    let skip_spaces = |bytes: &[u8], mut p: usize| {
        while p < bytes.len() && bytes[p] == b' ' {
            p += 1;
        }
        p
    };

    pos = skip_spaces(bytes, pos);

    // extractName: first char alphabetic, then alphanumeric/'_'/' '.
    let name_start = pos;
    if pos >= bytes.len() || !bytes[pos].is_ascii_alphabetic() {
        return Err("No dimension name provided.".to_string());
    }
    pos += 1;
    while pos < bytes.len() {
        let c = bytes[pos];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b' ' {
            pos += 1;
        } else {
            break;
        }
    }
    let name = dim[name_start..pos].trim_end().to_string();

    pos = skip_spaces(bytes, pos);

    let mut band = default_band;
    let mut scale = 1.0;
    if pos < bytes.len() && bytes[pos] == b':' {
        pos += 1;
        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos > start {
            band = dim[start..pos]
                .parse::<u32>()
                .map_err(|_| "Invalid band number.".to_string())?;
        }
        if band == 0 {
            return Err("Invalid band number 0. Bands start at 1.".to_string());
        }

        pos = skip_spaces(bytes, pos);

        if pos < bytes.len() && bytes[pos] == b':' {
            pos += 1;
            let start = pos;
            while pos < bytes.len()
                && (bytes[pos].is_ascii_digit()
                    || matches!(bytes[pos], b'+' | b'-' | b'.' | b'e' | b'E'))
            {
                pos += 1;
            }
            if pos > start {
                scale = dim[start..pos].parse::<f64>().unwrap_or(1.0);
            }
        }
    }

    pos = skip_spaces(bytes, pos);
    if pos != bytes.len() {
        return Err(format!(
            "Invalid character '{}' following dimension specification.",
            bytes[pos] as char
        ));
    }
    Ok(BandInfo { name, band, scale })
}

/// Parse the `dimensions` option into band specs. An empty spec defaults to
/// `Red, Green, Blue`; unsupplied band numbers increment from 1.
pub fn parse_band_spec(spec: &str) -> Result<Vec<BandInfo>, String> {
    let entries: Vec<&str> = if spec.trim().is_empty() {
        vec!["Red", "Green", "Blue"]
    } else {
        spec.split(',').collect()
    };
    let mut bands = Vec::new();
    let mut default_band = 1u32;
    for entry in entries {
        let bi = parse_dim(entry, default_band)?;
        default_band = bi.band + 1;
        bands.push(bi);
    }
    Ok(bands)
}

pub struct ColorizationFilter {
    raster_path: String,
    bands: Vec<BandInfo>,
    raster: Option<Raster>,
}

impl ColorizationFilter {
    pub fn new(raster_path: &str, bands: Vec<BandInfo>) -> Self {
        Self {
            raster_path: raster_path.to_string(),
            bands,
            raster: None,
        }
    }

    fn ensure_raster(&mut self) -> Result<(), StageError> {
        if self.raster.is_none() {
            gdal::register_drivers();
            self.raster = Some(Raster::open(&self.raster_path).map_err(StageError)?);
        }
        Ok(())
    }
}

impl Filter for ColorizationFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.colorization"
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        self.bands
            .iter()
            .map(|b| (DimId::from_name(&b.name), DimType::F64))
            .collect()
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_raster()?;
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for ColorizationFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_raster().is_err() {
            return true;
        }

        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);

        let mut data = vec![0.0; 16];
        if let Some(ref r) = self.raster {
            if r.read_at(x, y, &mut data).is_ok() {
                for band_info in &self.bands {
                    let val = data[(band_info.band - 1) as usize] * band_info.scale;
                    let dim = DimId::from_name(&band_info.name);
                    view.set_f64(idx, &dim, val);
                }
            }
        }
        true
    }

    fn reset(&mut self) {
        self.raster = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_spec_defaults_to_rgb() {
        let bands = parse_band_spec("").unwrap();
        let names: Vec<_> = bands
            .iter()
            .map(|b| (b.name.as_str(), b.band, b.scale))
            .collect();
        assert_eq!(
            names,
            vec![("Red", 1, 1.0), ("Green", 2, 1.0), ("Blue", 3, 1.0)]
        );
    }

    #[test]
    fn parses_incrementing_bands_and_scale() {
        // Mirrors ColorizationFilterTest.test1's "Red, Green,Blue::255".
        let bands = parse_band_spec("Red, Green,Blue::255  ").unwrap();
        let got: Vec<_> = bands
            .iter()
            .map(|b| (b.name.as_str(), b.band, b.scale))
            .collect();
        assert_eq!(
            got,
            vec![("Red", 1, 1.0), ("Green", 2, 1.0), ("Blue", 3, 255.0)]
        );
    }

    #[test]
    fn explicit_bands_and_scale() {
        // ColorizationFilterTest.test3: "Foo:1,Bar_:2,Baz2:3:255".
        let bands = parse_band_spec("Foo:1,Bar_:2,Baz2:3:255").unwrap();
        let got: Vec<_> = bands
            .iter()
            .map(|b| (b.name.as_str(), b.band, b.scale))
            .collect();
        assert_eq!(
            got,
            vec![("Foo", 1, 1.0), ("Bar_", 2, 1.0), ("Baz2", 3, 255.0)]
        );
    }

    #[test]
    fn invalid_trailing_char_errors() {
        // ColorizationFilterTest.test4: "Foo&:1" is rejected.
        assert!(parse_band_spec("Foo&:1,Bar:2,Baz:3:255").is_err());
    }

    #[test]
    fn zero_band_errors() {
        assert!(parse_band_spec("Red:0").is_err());
    }
}
