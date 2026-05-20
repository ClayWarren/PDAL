//! Bounding-box helpers for PDAL core.

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds2D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
    pub minz: f64,
    pub maxz: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedBounds2D {
    pub bounds: Bounds2D,
    pub wkt: String,
    pub pos: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedBounds3D {
    pub bounds: Bounds3D,
    pub wkt: String,
    pub pos: usize,
}

impl Bounds2D {
    pub fn empty() -> Self {
        Self {
            minx: f64::MAX,
            maxx: f64::MIN,
            miny: f64::MAX,
            maxy: f64::MIN,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::empty()
    }

    pub fn grow_point(&mut self, x: f64, y: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
    }

    pub fn grow_distance(&mut self, dist: f64) {
        self.minx -= dist;
        self.maxx += dist;
        self.miny -= dist;
        self.maxy += dist;
    }

    pub fn grow_bounds(&mut self, other: &Self) {
        self.minx = self.minx.min(other.minx);
        self.maxx = self.maxx.max(other.maxx);
        self.miny = self.miny.min(other.miny);
        self.maxy = self.maxy.max(other.maxy);
    }

    pub fn clip(&mut self, other: &Self) {
        self.minx = self.minx.max(other.minx);
        self.maxx = self.maxx.min(other.maxx);
        self.miny = self.miny.max(other.miny);
        self.maxy = self.maxy.min(other.maxy);
    }

    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        self.minx <= x && x <= self.maxx && self.miny <= y && y <= self.maxy
    }

    pub fn contains_bounds(&self, other: &Self) -> bool {
        self.minx <= other.minx
            && self.maxx >= other.maxx
            && self.miny <= other.miny
            && self.maxy >= other.maxy
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.minx <= other.maxx
            && self.maxx >= other.minx
            && self.miny <= other.maxy
            && self.maxy >= other.miny
    }
}

impl Bounds3D {
    pub fn empty() -> Self {
        Self {
            minx: f64::MAX,
            maxx: f64::MIN,
            miny: f64::MAX,
            maxy: f64::MIN,
            minz: f64::MAX,
            maxz: f64::MIN,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::empty()
    }

    pub fn grow_point(&mut self, x: f64, y: f64, z: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
        self.minz = self.minz.min(z);
        self.maxz = self.maxz.max(z);
    }

    pub fn grow_bounds(&mut self, other: &Self) {
        self.minx = self.minx.min(other.minx);
        self.maxx = self.maxx.max(other.maxx);
        self.miny = self.miny.min(other.miny);
        self.maxy = self.maxy.max(other.maxy);
        self.minz = self.minz.min(other.minz);
        self.maxz = self.maxz.max(other.maxz);
    }

    pub fn grow_distance(&mut self, dist: f64) {
        self.minx -= dist;
        self.maxx += dist;
        self.miny -= dist;
        self.maxy += dist;
        self.minz -= dist;
        self.maxz += dist;
    }

    pub fn clip(&mut self, other: &Self) {
        self.minx = self.minx.max(other.minx);
        self.maxx = self.maxx.min(other.maxx);
        self.miny = self.miny.max(other.miny);
        self.maxy = self.maxy.min(other.maxy);
        if other.minz > self.minz && other.minz < self.maxz {
            self.minz = other.minz;
        }
        if other.maxz < self.maxz && other.maxz > self.minz {
            self.maxz = other.maxz;
        }
    }

    pub fn contains_point(&self, x: f64, y: f64, z: f64) -> bool {
        self.minx <= x
            && x <= self.maxx
            && self.miny <= y
            && y <= self.maxy
            && self.minz <= z
            && z <= self.maxz
    }

    pub fn contains_bounds(&self, other: &Self) -> bool {
        self.minx <= other.minx
            && self.maxx >= other.maxx
            && self.miny <= other.miny
            && self.maxy >= other.maxy
            && self.minz <= other.minz
            && other.maxz <= self.maxz
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.minx <= other.maxx
            && self.maxx >= other.minx
            && self.miny <= other.maxy
            && self.maxy >= other.miny
            && self.minz <= other.maxz
            && self.maxz >= other.minz
    }
}

pub fn parse_bounds2d(input: &str, pos: usize) -> Result<ParsedBounds2D, String> {
    if pos == 0 {
        if let Some(parsed) = parse_json_bounds2d(input)? {
            return Ok(parsed);
        }
    }

    let mut parser = BoundsParser::new(input, pos);
    parser.discard_spaces_before('(')?;
    let (minx, maxx) = parser.parse_pair::<Bounds2D>()?;
    parser.discard_spaces_before(',')?;
    let (miny, maxy) = parser.parse_pair::<Bounds2D>()?;
    parser.discard_spaces_before(')')?;
    parser.eat_whitespace();

    Ok(ParsedBounds2D {
        bounds: Bounds2D {
            minx,
            maxx,
            miny,
            maxy,
        },
        wkt: String::new(),
        pos: parser.pos,
    })
}

pub fn parse_bounds3d(input: &str, pos: usize) -> Result<ParsedBounds3D, String> {
    if pos == 0 {
        if let Some(parsed) = parse_json_bounds3d(input)? {
            return Ok(parsed);
        }
    }

    let mut parser = BoundsParser::new(input, pos);
    parser.discard_spaces_before('(')?;
    let (minx, maxx) = parser.parse_pair::<Bounds3D>()?;
    parser.discard_spaces_before(',')?;
    let (miny, maxy) = parser.parse_pair::<Bounds3D>()?;
    parser.discard_spaces_before(',')?;
    let (minz, maxz) = parser.parse_pair::<Bounds3D>()?;
    parser.discard_spaces_before(')')?;
    parser.eat_whitespace();

    Ok(ParsedBounds3D {
        bounds: Bounds3D {
            minx,
            maxx,
            miny,
            maxy,
            minz,
            maxz,
        },
        wkt: String::new(),
        pos: parser.pos,
    })
}

fn parse_json_bounds2d(input: &str) -> Result<Option<ParsedBounds2D>, String> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Ok(None);
    };

    match value {
        Value::Array(values) => {
            if values.len() != 4 {
                return Err(format!(
                    "GeoJSON array size must be 4 for BOX2d. It was {}",
                    values.len()
                ));
            }
            Ok(Some(ParsedBounds2D {
                bounds: Bounds2D {
                    minx: json_number(&values[0])?,
                    miny: json_number(&values[1])?,
                    maxx: json_number(&values[2])?,
                    maxy: json_number(&values[3])?,
                },
                wkt: String::new(),
                pos: input.len(),
            }))
        }
        Value::Object(object) => Ok(Some(ParsedBounds2D {
            bounds: Bounds2D {
                minx: json_field_number(&object, "minx")?,
                miny: json_field_number(&object, "miny")?,
                maxx: json_field_number(&object, "maxx")?,
                maxy: json_field_number(&object, "maxy")?,
            },
            wkt: json_srs(&object),
            pos: input.len(),
        })),
        _ => Ok(None),
    }
}

fn parse_json_bounds3d(input: &str) -> Result<Option<ParsedBounds3D>, String> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Ok(None);
    };

    match value {
        Value::Array(values) => {
            if values.len() != 6 {
                return Err(format!("GeoJSON array must be 6. It was {}", values.len()));
            }
            Ok(Some(ParsedBounds3D {
                bounds: Bounds3D {
                    minx: json_number(&values[0])?,
                    miny: json_number(&values[1])?,
                    minz: json_number(&values[2])?,
                    maxx: json_number(&values[3])?,
                    maxy: json_number(&values[4])?,
                    maxz: json_number(&values[5])?,
                },
                wkt: String::new(),
                pos: input.len(),
            }))
        }
        Value::Object(object) => {
            let minx = json_field_number(&object, "minx")?;
            let miny = json_field_number(&object, "miny")?;
            let maxx = json_field_number(&object, "maxx")?;
            let maxy = json_field_number(&object, "maxy")?;
            let minz = object.get("minz").map(json_number).transpose()?;
            let maxz = object.get("maxz").map(json_number).transpose()?;
            Ok(Some(ParsedBounds3D {
                bounds: Bounds3D {
                    minx,
                    maxx,
                    miny,
                    maxy,
                    minz: minz.unwrap_or_else(|| Bounds3D::empty().minz),
                    maxz: maxz.unwrap_or_else(|| Bounds3D::empty().maxz),
                },
                wkt: json_srs(&object),
                pos: input.len(),
            }))
        }
        _ => Ok(None),
    }
}

fn json_number(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| "Bounds JSON field must be numeric.".to_string())
}

fn json_field_number(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<f64, String> {
    object
        .get(field)
        .ok_or_else(|| format!("Object must contain '{field}'"))
        .and_then(json_number)
}

fn json_srs(object: &serde_json::Map<String, Value>) -> String {
    object
        .get("srs")
        .or_else(|| object.get("crs"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

struct BoundsParser<'a> {
    input: &'a str,
    pos: usize,
}

trait BoundsError {
    fn no_opening_range() -> &'static str;
    fn no_minimum() -> &'static str;
    fn no_separator() -> &'static str;
    fn no_maximum() -> &'static str;
    fn no_closing_range() -> &'static str;
}

impl BoundsError for Bounds2D {
    fn no_opening_range() -> &'static str {
        "No opening '[' in range."
    }

    fn no_minimum() -> &'static str {
        "No valid minimum value for range."
    }

    fn no_separator() -> &'static str {
        "No ',' separating minimum/maximum values."
    }

    fn no_maximum() -> &'static str {
        "No valid maximum value for range."
    }

    fn no_closing_range() -> &'static str {
        "No closing ']' in range."
    }
}

impl BoundsError for Bounds3D {
    fn no_opening_range() -> &'static str {
        "No opening '[' in range."
    }

    fn no_minimum() -> &'static str {
        "No valid minimum value for range."
    }

    fn no_separator() -> &'static str {
        "No ',' separating minimum/maximum values."
    }

    fn no_maximum() -> &'static str {
        "No valid maximum value for range."
    }

    fn no_closing_range() -> &'static str {
        "No closing ']' in range."
    }
}

impl<'a> BoundsParser<'a> {
    fn new(input: &'a str, pos: usize) -> Self {
        Self {
            input,
            pos: pos.min(input.len()),
        }
    }

    fn eat_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.pos += ch.len_utf8();
        }
    }

    fn discard_spaces_before(&mut self, next: char) -> Result<(), String> {
        self.eat_whitespace();
        if self.peek_char() == Some(next) {
            self.pos += next.len_utf8();
            return Ok(());
        }
        match next {
            '(' => Err("No opening '('.".to_string()),
            ')' => Err("No closing ')'.".to_string()),
            ',' => Err("No comma separating dimensions.".to_string()),
            '[' => Err("No opening '[' in range.".to_string()),
            ']' => Err("No closing ']' in range.".to_string()),
            _ => Err(format!("Expected '{next}'.")),
        }
    }

    fn parse_pair<T: BoundsError>(&mut self) -> Result<(f64, f64), String> {
        self.discard_spaces_before('[')
            .map_err(|_| T::no_opening_range().to_string())?;
        let low = self
            .parse_number()
            .ok_or_else(|| T::no_minimum().to_string())?;
        self.discard_spaces_before(',')
            .map_err(|_| T::no_separator().to_string())?;
        let high = self
            .parse_number()
            .ok_or_else(|| T::no_maximum().to_string())?;
        self.discard_spaces_before(']')
            .map_err(|_| T::no_closing_range().to_string())?;
        Ok((low, high))
    }

    fn parse_number(&mut self) -> Option<f64> {
        self.eat_whitespace();
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.' | 'e' | 'E') {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        self.input[start..self.pos].parse::<f64>().ok()
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds2d_empty_and_grow_match_cpp_contract() {
        let mut bounds = Bounds2D::empty();
        assert!(bounds.is_empty());

        bounds.grow_point(0.0, 201.0);
        assert_eq!(
            bounds,
            Bounds2D {
                minx: 0.0,
                maxx: 0.0,
                miny: 201.0,
                maxy: 201.0,
            }
        );

        bounds.grow_distance(2.0);
        assert_eq!(
            bounds,
            Bounds2D {
                minx: -2.0,
                maxx: 2.0,
                miny: 199.0,
                maxy: 203.0,
            }
        );

        let other = Bounds2D {
            minx: -1.0,
            maxx: 10.0,
            miny: 200.0,
            maxy: 204.0,
        };
        assert!(bounds.contains_point(0.0, 201.0));
        assert!(bounds.overlaps(&other));
        bounds.grow_bounds(&other);
        assert_eq!(bounds.maxx, 10.0);
        bounds.clip(&other);
        assert_eq!(bounds, other);
        assert!(bounds.contains_bounds(&other));
    }

    #[test]
    fn bounds3d_empty_and_grow_match_cpp_contract() {
        let mut bounds = Bounds3D::empty();
        assert!(bounds.is_empty());

        bounds.grow_point(0.0, 201.0, 202.0);
        assert_eq!(
            bounds,
            Bounds3D {
                minx: 0.0,
                maxx: 0.0,
                miny: 201.0,
                maxy: 201.0,
                minz: 202.0,
                maxz: 202.0,
            }
        );

        bounds.grow_distance(2.0);
        assert!(bounds.contains_point(0.0, 201.0, 202.0));
        let other = Bounds3D {
            minx: -1.0,
            maxx: 1.0,
            miny: 200.0,
            maxy: 202.0,
            minz: 201.0,
            maxz: 203.0,
        };
        assert!(bounds.overlaps(&other));
        assert!(bounds.contains_bounds(&other));
        bounds.clip(&other);
        assert_eq!(bounds, other);
    }

    #[test]
    fn parses_tuple_and_json_bounds() {
        let parsed = parse_bounds3d("([1,101],[2,102],[3,103])", 0).unwrap();
        assert_eq!(parsed.bounds.minx, 1.0);
        assert_eq!(parsed.bounds.maxz, 103.0);
        assert_eq!(parsed.pos, 25);

        let parsed = parse_bounds2d("[1, 2, 101, 102]", 0).unwrap();
        assert_eq!(parsed.bounds.maxx, 101.0);
        assert_eq!(parsed.bounds.maxy, 102.0);

        let parsed = parse_bounds2d(
            r#"{"minx": 1,"miny": 2,"maxx": 101,"maxy": 102,"crs":"EPSG:2596"}"#,
            0,
        )
        .unwrap();
        assert_eq!(parsed.wkt, "EPSG:2596");
    }
}
