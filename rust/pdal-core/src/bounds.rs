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

pub fn bounds2d_equal(a: &Bounds2D, b: &Bounds2D) -> bool {
    a == b
}

pub fn bounds3d_equal(a: &Bounds3D, b: &Bounds3D) -> bool {
    a == b
}

pub fn default_bounds2d() -> Bounds2D {
    Bounds2D {
        minx: -f64::MAX,
        miny: -f64::MAX,
        maxx: f64::MAX,
        maxy: f64::MAX,
    }
}

pub fn default_bounds3d() -> Bounds3D {
    Bounds3D {
        minx: -f64::MAX,
        miny: -f64::MAX,
        minz: -f64::MAX,
        maxx: f64::MAX,
        maxy: f64::MAX,
        maxz: f64::MAX,
    }
}

fn format_fixed(value: f64, precision: u32) -> String {
    format!("{value:.prec$}", prec = precision as usize)
}

fn format_stream(value: f64) -> String {
    format!("{value}")
}

pub fn format_bounds2d(bounds: &Bounds2D, _precision: u32) -> String {
    if bounds.is_empty() {
        return "()".to_string();
    }
    format!(
        "([{}, {}], [{}, {}])",
        format_stream(bounds.minx),
        format_stream(bounds.maxx),
        format_stream(bounds.miny),
        format_stream(bounds.maxy)
    )
}

pub fn format_bounds3d(bounds: &Bounds3D, _precision: u32) -> String {
    if bounds.is_empty() {
        return "()".to_string();
    }
    format!(
        "([{}, {}], [{}, {}], [{}, {}])",
        format_stream(bounds.minx),
        format_stream(bounds.maxx),
        format_stream(bounds.miny),
        format_stream(bounds.maxy),
        format_stream(bounds.minz),
        format_stream(bounds.maxz)
    )
}

pub fn bounds2d_to_wkt(bounds: &Bounds2D, precision: u32) -> String {
    if bounds.is_empty() {
        return String::new();
    }
    format!(
        "POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))",
        format_fixed(bounds.minx, precision),
        format_fixed(bounds.miny, precision),
        format_fixed(bounds.minx, precision),
        format_fixed(bounds.maxy, precision),
        format_fixed(bounds.maxx, precision),
        format_fixed(bounds.maxy, precision),
        format_fixed(bounds.maxx, precision),
        format_fixed(bounds.miny, precision),
        format_fixed(bounds.minx, precision),
        format_fixed(bounds.miny, precision)
    )
}

pub fn bounds3d_to_wkt(bounds: &Bounds3D, precision: u32) -> String {
    if bounds.is_empty() {
        return String::new();
    }

    let face = |points: [(&str, &str, &str); 5]| {
        let mut out = String::from("((");
        for (idx, (x, y, z)) in points.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push_str(x);
            out.push(' ');
            out.push_str(y);
            out.push(' ');
            out.push_str(z);
        }
        out.push_str(", ))");
        out
    };

    let minx = format_fixed(bounds.minx, precision);
    let maxx = format_fixed(bounds.maxx, precision);
    let miny = format_fixed(bounds.miny, precision);
    let maxy = format_fixed(bounds.maxy, precision);
    let minz = format_fixed(bounds.minz, precision);
    let maxz = format_fixed(bounds.maxz, precision);

    format!(
        "POLYHEDRON Z ( {}, {}, {}, {}, {}, {} )",
        face([
            (&minx, &miny, &minz),
            (&maxx, &miny, &minz),
            (&maxx, &maxy, &minz),
            (&minx, &maxy, &minz),
            (&minx, &miny, &minz),
        ]),
        face([
            (&minx, &miny, &minz),
            (&maxx, &miny, &minz),
            (&maxx, &miny, &maxz),
            (&minx, &miny, &maxz),
            (&minx, &miny, &minz),
        ]),
        face([
            (&maxx, &miny, &minz),
            (&maxx, &maxy, &minz),
            (&maxx, &maxy, &maxz),
            (&maxx, &miny, &maxz),
            (&maxx, &miny, &minz),
        ]),
        face([
            (&maxx, &maxy, &minz),
            (&minx, &maxy, &minz),
            (&minx, &maxy, &maxz),
            (&maxx, &maxy, &maxz),
            (&maxx, &maxy, &minz),
        ]),
        face([
            (&minx, &maxy, &minz),
            (&minx, &miny, &minz),
            (&minx, &miny, &maxz),
            (&minx, &maxy, &maxz),
            (&minx, &maxy, &minz),
        ]),
        face([
            (&minx, &miny, &maxz),
            (&maxx, &miny, &maxz),
            (&maxx, &maxy, &maxz),
            (&minx, &maxy, &maxz),
            (&minx, &miny, &maxz),
        ])
    )
}

pub fn bounds2d_to_geojson(bounds: &Bounds2D, precision: u32) -> String {
    if bounds.is_empty() {
        return String::new();
    }
    format!(
        "{{\"bbox\":[{}, {}, {},{}]}}",
        format_fixed(bounds.minx, precision),
        format_fixed(bounds.miny, precision),
        format_fixed(bounds.maxx, precision),
        format_fixed(bounds.maxy, precision)
    )
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
#[cfg(test)]
#[path = "bounds/tests.rs"]
mod tests;
