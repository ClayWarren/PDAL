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

pub(crate) fn stage_options(stage_name: &str) -> Vec<serde_json::Value> {
    use serde_json::json;

    fn filename() -> serde_json::Value {
        json!({"arg": "filename", "description": "Input or output filename."})
    }
    fn option(
        arg: &str,
        description: &str,
        default: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let mut value = json!({"arg": arg, "description": description});
        if let Some(default) = default {
            value["default"] = default;
        }
        value
    }

    match stage_name {
        "readers.faux" => vec![
            option(
                "count",
                "Number of synthetic points to create.",
                Some(json!(10)),
            ),
            option(
                "mode",
                "Synthetic point generation mode.",
                Some(json!("constant")),
            ),
        ],
        "readers.bpf" | "readers.fbi" | "readers.obj" | "readers.optech" | "readers.pcd"
        | "readers.ply" | "readers.pts" | "readers.ptx" | "readers.qfit" | "readers.smrmsg"
        | "readers.terrasolid" | "readers.las" | "readers.laz" => vec![filename()],
        "readers.ilvis2" => vec![
            filename(),
            option(
                "mapping",
                "Point mapping to read: low, high, or all.",
                Some(json!("low")),
            ),
            option("metadata", "Optional ILVIS2 XML metadata sidecar.", None),
        ],
        "readers.sbet" => vec![
            filename(),
            option(
                "angles_as_degrees",
                "Convert stored angular values from radians to degrees.",
                Some(json!(true)),
            ),
        ],
        "filters.decimation" => vec![
            option("step", "Keep every Nth point.", Some(json!(1))),
            option("offset", "Starting point offset.", Some(json!(0))),
            option(
                "limit",
                "Maximum number of points to consider.",
                Some(json!(0)),
            ),
        ],
        "filters.head" | "filters.tail" => vec![
            option("count", "Number of points to keep.", Some(json!(10))),
            option(
                "invert",
                "Invert the selected point range.",
                Some(json!(false)),
            ),
        ],
        "filters.groupby" => vec![option("dimension", "Dimension used to group points.", None)],
        "filters.locate" => vec![
            option("dimension", "Dimension to inspect.", None),
            option("minmax", "Select the min or max point.", Some(json!("max"))),
        ],
        "filters.merge" => Vec::new(),
        "filters.mortonorder" => vec![option(
            "reverse",
            "Sort in reverse Morton order.",
            Some(json!(false)),
        )],
        "filters.randomize" => vec![option("seed", "Optional deterministic shuffle seed.", None)],
        "filters.reprojection" => vec![
            option("out_srs", "Spatial reference for output data.", None),
            option("in_srs", "Override spatial reference for input data.", None),
            option(
                "error_on_failure",
                "Error if a point cannot be reprojected.",
                Some(json!(false)),
            ),
        ],
        "filters.returns" => vec![option(
            "groups",
            "Comma-separated return groups to keep.",
            Some(json!("last")),
        )],
        "filters.sample" => vec![option("radius", "Sample radius.", Some(json!(1.0)))],
        "filters.sort" => vec![
            option("dimensions", "Comma-separated dimensions to sort by.", None),
            option("order", "Sort order: asc or desc.", Some(json!("asc"))),
            option(
                "algorithm",
                "Sort algorithm: normal or stable.",
                Some(json!("normal")),
            ),
        ],
        "filters.stats" => vec![
            option(
                "dimensions",
                "Comma-separated dimensions to summarize.",
                None,
            ),
            option(
                "advanced",
                "Compute advanced statistics.",
                Some(json!(false)),
            ),
        ],
        "filters.voxeldownsize" => vec![option("cell", "Voxel cell size.", Some(json!(1.0)))],
        "writers.null" => Vec::new(),
        "writers.bpf" | "writers.fbi" | "writers.gltf" | "writers.sbet" | "writers.las"
        | "writers.laz" => vec![
            filename(),
            option("compression", "Compress output data.", Some(json!(false))),
            option("point_format", "LAS point format ID.", Some(json!(3))),
        ],
        "writers.text" => vec![
            filename(),
            option("order", "Comma-separated output dimension order.", None),
            option(
                "precision",
                "Floating-point output precision.",
                Some(json!(6)),
            ),
            option("delimiter", "Output delimiter.", Some(json!(","))),
            option(
                "quote_header",
                "Quote output header fields.",
                Some(json!(true)),
            ),
        ],
        "writers.pcd" => vec![
            filename(),
            option("order", "Comma-separated output dimension order.", None),
            option(
                "precision",
                "Floating-point output precision.",
                Some(json!(2)),
            ),
            option("compression", "PCD storage mode.", Some(json!("ascii"))),
        ],
        "writers.ply" => vec![
            filename(),
            option("storage_mode", "PLY storage mode.", Some(json!("ascii"))),
            option("dims", "Comma-separated dimension list.", None),
            option(
                "sized_types",
                "Use sized PLY type names.",
                Some(json!(true)),
            ),
            option("precision", "Floating-point output precision.", None),
            option("faces", "Write triangular mesh faces.", Some(json!(false))),
        ],
        _ => Vec::new(),
    }
}
