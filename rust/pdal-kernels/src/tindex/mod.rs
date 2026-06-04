use pdal_core::bounds::Bounds2D;

pub const INVALID_TINDEX_FILTER_STAGE_MESSAGE: &str = "Argument references invalid/unused stage";

#[derive(Clone, Debug)]
pub struct BoundaryOptions {
    pub density: i32,
    pub edge_length: f64,
    pub sample_size: u32,
    pub smooth: bool,
    pub fast_boundary: bool,
    pub where_expr: Option<String>,
}

impl Default for BoundaryOptions {
    fn default() -> Self {
        Self {
            density: 15,
            edge_length: 0.0,
            sample_size: 5000,
            smooth: true,
            fast_boundary: false,
            where_expr: None,
        }
    }
}

impl BoundaryOptions {
    pub fn exact(&self) -> bool {
        !self.fast_boundary
    }
}

#[derive(Clone, Debug)]
pub struct TindexCreateArgs {
    pub tindex_file: String,
    pub files: Vec<String>,
    pub driver_name: String,
    pub target_srs: String,
    pub assign_srs: String,
    pub override_source_srs: bool,
    pub path_prefix: Option<String>,
    pub write_absolute_path: bool,
    pub layer_name: String,
    pub location_field: String,
    pub lco_options: Vec<String>,
    pub lco_description: Option<String>,
    pub rich_boundary_options: bool,
    pub boundary: BoundaryOptions,
    stdin_requested: bool,
    input_methods: u8,
    filelists: Vec<String>,
    pub skip_different_srs: bool,
}

impl Default for TindexCreateArgs {
    fn default() -> Self {
        Self {
            tindex_file: String::new(),
            files: Vec::new(),
            driver_name: "ESRI Shapefile".to_string(),
            target_srs: "EPSG:4326".to_string(),
            assign_srs: "EPSG:4326".to_string(),
            override_source_srs: false,
            path_prefix: None,
            write_absolute_path: false,
            layer_name: "pdal".to_string(),
            location_field: "location".to_string(),
            lco_options: Vec::new(),
            lco_description: None,
            rich_boundary_options: false,
            boundary: BoundaryOptions::default(),
            stdin_requested: false,
            input_methods: 0,
            filelists: Vec::new(),
            skip_different_srs: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TindexParseResult {
    Error(String),
}

#[derive(Clone, Debug)]
pub struct TindexMergeArgs {
    pub tindex_file: String,
    pub output_file: String,
    pub layer_name: String,
    pub location_field: String,
    pub target_srs: String,
    pub clip: Option<TindexMergeClip>,
}

#[derive(Clone, Debug)]
pub enum TindexMergeClip {
    Bounds { bounds: Bounds2D, value: String },
    Polygon { value: String },
}

#[derive(Clone, Debug)]
pub struct TindexResolvedClip {
    pub bounds: Bounds2D,
    pub stage_key: &'static str,
    pub stage_value: String,
}

#[derive(Clone, Debug)]
pub struct TindexMergePlan {
    pub file_count: usize,
    pub pipeline_json: serde_json::Value,
}

pub fn print_tindex_usage() {
    println!("Usage:");
    println!("  pdal tindex create --tindex <output> <files...> [-f GeoJSON]");
    println!("  pdal tindex create --tindex <output> --filelist <path> [-f GeoJSON]");
    println!("  pdal tindex create --tindex <output> --glob <pattern> [-f GeoJSON]");
    println!("  pdal tindex merge --tindex <index> --filespec <output>");
}

pub fn tindex_next_value<'a, I>(iter: &mut I, arg: &str) -> Result<&'a String, TindexParseResult>
where
    I: Iterator<Item = &'a String>,
{
    iter.next()
        .ok_or_else(|| TindexParseResult::Error(format!("{arg} requires a value")))
}

mod create;
mod merge;

pub use create::parse_tindex_create_args;
pub use merge::{build_tindex_merge_plan, parse_tindex_merge_args};

#[cfg(test)]
mod tests;
