//! Native dependency adapters for the PDAL Rust port.
//!
//! Keep direct GDAL/OGR, GEOS, PROJ, LASzip/laz-perf, and similar native
//! bindings behind this layer or another explicit adapter crate. Higher-level
//! crates should expose PDAL behavior, not vendor-specific types.

pub mod gdal;
pub mod geometry;
pub mod nitf;
pub mod srs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCapability {
    Gdal,
    Geos,
    Nitro,
    Proj,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeDependency {
    pub capability: NativeCapability,
    pub name: &'static str,
    pub version: String,
}

pub fn built_capabilities() -> &'static [NativeCapability] {
    &[
        NativeCapability::Gdal,
        NativeCapability::Geos,
        NativeCapability::Nitro,
        NativeCapability::Proj,
    ]
}

pub fn built_dependencies() -> Vec<NativeDependency> {
    vec![
        NativeDependency {
            capability: NativeCapability::Gdal,
            name: "GDAL",
            version: gdal::version(),
        },
        NativeDependency {
            capability: NativeCapability::Geos,
            name: "GEOS",
            version: geometry::version(),
        },
        NativeDependency {
            capability: NativeCapability::Nitro,
            name: "NITRO",
            version: "linked".to_string(),
        },
        NativeDependency {
            capability: NativeCapability::Proj,
            name: "PROJ",
            version: srs::version(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_dependencies_report_each_native_capability() {
        let dependencies = built_dependencies();

        assert_eq!(dependencies.len(), built_capabilities().len());
        for capability in built_capabilities() {
            let dependency = dependencies
                .iter()
                .find(|dependency| dependency.capability == *capability)
                .expect("dependency is listed for every capability");
            assert!(!dependency.name.is_empty());
            assert!(!dependency.version.is_empty());
        }
    }
}
