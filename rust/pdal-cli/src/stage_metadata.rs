#[derive(serde::Serialize)]
pub(crate) struct KernelInfo {
    pub(crate) name: &'static str,
    full_name: &'static str,
    description: &'static str,
}

pub(crate) fn kernel_list() -> Vec<KernelInfo> {
    vec![
        KernelInfo {
            name: "chamfer",
            full_name: "kernels.chamfer",
            description: "report the Chamfer distance between two point cloud files",
        },
        KernelInfo {
            name: "delta",
            full_name: "kernels.delta",
            description: "report per-dimension deltas between two point cloud files",
        },
        KernelInfo {
            name: "density",
            full_name: "kernels.density",
            description: "write a hexagonal point-density tessellation as GeoJSON",
        },
        KernelInfo {
            name: "eval",
            full_name: "kernels.eval",
            description: "score predicted classification labels against a truth file",
        },
        KernelInfo {
            name: "fauxplugin",
            full_name: "kernels.fauxplugin",
            description: "Faux Plugin Kernel",
        },
        KernelInfo {
            name: "ground",
            full_name: "kernels.ground",
            description: "classify ground points with the simple morphological filter",
        },
        KernelInfo {
            name: "hausdorff",
            full_name: "kernels.hausdorff",
            description: "report the Hausdorff distance between two point cloud files",
        },
        KernelInfo {
            name: "info",
            full_name: "kernels.info",
            description:
                "report metadata, bounds, and per-dimension summary for a point cloud file",
        },
        KernelInfo {
            name: "lasdump",
            full_name: "tools.lasdump",
            description: "dump LAS header, VLR, and point checksum information",
        },
        KernelInfo {
            name: "nitfwrap",
            full_name: "tools.nitfwrap",
            description: "wrap LAS/LAZ/BPF data in a NITF file or unwrap it",
        },
        KernelInfo {
            name: "merge",
            full_name: "kernels.merge",
            description: "merge several point cloud files into one output",
        },
        KernelInfo {
            name: "pipeline",
            full_name: "kernels.pipeline",
            description: "execute a PDAL pipeline JSON file through the Rust port",
        },
        KernelInfo {
            name: "random",
            full_name: "kernels.random",
            description: "generate a file of uniformly random points",
        },
        KernelInfo {
            name: "sort",
            full_name: "kernels.sort",
            description: "sort the points of a file by one or more dimensions",
        },
        KernelInfo {
            name: "split",
            full_name: "kernels.split",
            description: "split one point cloud file into multiple output files",
        },
        KernelInfo {
            name: "tile",
            full_name: "kernels.tile",
            description: "tile a point cloud into a regular grid of output files",
        },
        KernelInfo {
            name: "tindex",
            full_name: "kernels.tindex",
            description: "create a tile index of point cloud files",
        },
        KernelInfo {
            name: "translate",
            full_name: "kernels.translate",
            description: "convert a point cloud file, optionally applying filters",
        },
    ]
}

#[derive(serde::Serialize)]
pub(crate) struct StageInfo {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    link: &'static str,
}

pub(crate) fn stage_list() -> Vec<StageInfo> {
    pdal_capi::READER_DRIVERS
        .iter()
        .chain(pdal_capi::FILTER_DRIVERS.iter())
        .chain(pdal_capi::WRITER_DRIVERS.iter())
        .map(|name| StageInfo {
            name,
            description: "Rust-backed stage",
            link: "",
        })
        .collect()
}

mod options;

pub(crate) use options::stage_options;

pub(crate) fn all_stage_options() -> serde_json::Map<String, serde_json::Value> {
    stage_list()
        .into_iter()
        .map(|stage| {
            (
                stage.name.to_string(),
                serde_json::Value::Array(stage_options(stage.name)),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_stage_options_includes_every_stage() {
        let all = all_stage_options();

        assert_eq!(all.len(), stage_list().len());
        assert!(all.contains_key("readers.las"));
        assert!(all.contains_key("filters.decimation"));
        assert!(all.contains_key("writers.las"));
    }

    #[test]
    fn rust_backed_stages_have_scoped_option_metadata() {
        let allowed_empty = ["filters.merge", "writers.null"];

        for stage in stage_list() {
            if allowed_empty.contains(&stage.name) {
                continue;
            }
            assert!(
                !stage_options(stage.name).is_empty(),
                "{} has no option metadata",
                stage.name
            );
        }
    }
}
