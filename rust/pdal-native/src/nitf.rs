#[cfg(feature = "nitf")]
#[path = "nitf_enabled.rs"]
mod enabled;

#[cfg(feature = "nitf")]
pub use enabled::*;

#[cfg(not(feature = "nitf"))]
mod disabled {
    use std::collections::BTreeMap;

    #[derive(Clone, Debug, Default)]
    pub struct NitfWriteOptions {
        pub file_title: Option<String>,
        pub complexity_level: Option<String>,
        pub system_type: Option<String>,
        pub origin_station_id: Option<String>,
        pub file_class: Option<String>,
        pub origin_name: Option<String>,
        pub origin_phone: Option<String>,
        pub fsclsy: Option<String>,
        pub fsctlh: Option<String>,
        pub fscltx: Option<String>,
        pub image_security_class: Option<String>,
        pub image_date_time: Option<String>,
        pub image_id2: Option<String>,
        pub aimidb: Vec<String>,
        pub acftb: Vec<String>,
        pub minx: f64,
        pub miny: f64,
        pub maxx: f64,
        pub maxy: f64,
    }

    pub fn lidar_segment(_path: &str) -> Result<(u64, u64), String> {
        Err(unavailable())
    }

    pub fn read_metadata(_path: &str) -> Result<BTreeMap<String, String>, String> {
        Err(unavailable())
    }

    pub fn write(_input: &str, _output: &str, _opts: &NitfWriteOptions) -> Result<(), String> {
        Err(unavailable())
    }

    pub fn wrap(
        _input: &str,
        _output: &str,
        _title: &str,
        _bounds: [f64; 4],
    ) -> Result<(), String> {
        Err(unavailable())
    }

    fn unavailable() -> String {
        "NITF support is not available in this build.".to_string()
    }
}

#[cfg(not(feature = "nitf"))]
pub use disabled::*;
