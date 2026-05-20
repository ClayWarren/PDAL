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
        "filters.approximatecoplanar" => vec![
            option(
                "knn",
                "Number of nearest neighbors to inspect.",
                Some(json!(8)),
            ),
            option("thresh1", "First coplanarity threshold.", Some(json!(25.0))),
            option("thresh2", "Second coplanarity threshold.", Some(json!(6.0))),
        ],
        "filters.chipper" => vec![option(
            "capacity",
            "Maximum number of points per chip.",
            Some(json!(5000)),
        )],
        "filters.cluster" => vec![
            option(
                "min_points",
                "Minimum number of points in a cluster.",
                Some(json!(1)),
            ),
            option("max_points", "Maximum number of points in a cluster.", None),
            option(
                "tolerance",
                "Cluster neighbor distance tolerance.",
                Some(json!(1.0)),
            ),
            option("is3d", "Use X/Y/Z instead of X/Y.", Some(json!(true))),
        ],
        "filters.dbscan" => vec![
            option(
                "min_points",
                "Minimum points required to form a dense region.",
                Some(json!(6)),
            ),
            option("eps", "Neighborhood search radius.", Some(json!(1.0))),
            option(
                "dimensions",
                "Comma-separated dimensions used for distance checks.",
                Some(json!("X,Y,Z")),
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
        "filters.eigenvalues" => vec![
            option(
                "knn",
                "Number of nearest neighbors to inspect.",
                Some(json!(8)),
            ),
            option(
                "normalize",
                "Normalize eigenvalue output.",
                Some(json!(false)),
            ),
            option("stride", "Point stride for calculations.", Some(json!(1))),
            option("radius", "Optional radius search distance.", None),
            option(
                "min_k",
                "Minimum neighbors required for radius mode.",
                Some(json!(3)),
            ),
        ],
        "filters.elm" => vec![
            option(
                "cell",
                "Cell size for low point detection.",
                Some(json!(10.0)),
            ),
            option(
                "class",
                "Classification value for low points.",
                Some(json!(7)),
            ),
            option(
                "threshold",
                "Minimum vertical separation for low point detection.",
                Some(json!(1.0)),
            ),
        ],
        "filters.estimaterank" => vec![
            option(
                "knn",
                "Number of nearest neighbors to inspect.",
                Some(json!(8)),
            ),
            option("threshold", "Rank threshold.", Some(json!(0.01))),
        ],
        "filters.gpstimeconvert" => vec![
            option(
                "conversion",
                "GPS time conversion in {input}2{output} form.",
                None,
            ),
            option("in_time", "Input GPS time representation.", None),
            option("out_time", "Output GPS time representation.", None),
            option("start_date", "Start date for GWS/GDS conversions.", None),
            option(
                "wrap",
                "Wrap converted week/day seconds.",
                Some(json!(false)),
            ),
            option(
                "wrapped",
                "Treat source week/day seconds as wrapped.",
                Some(json!(false)),
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
        "filters.hag_nn" => vec![
            option(
                "count",
                "Neighbor count for ground interpolation.",
                Some(json!(1)),
            ),
            option(
                "max_distance",
                "Maximum ground neighbor distance.",
                Some(json!(0.0)),
            ),
            option(
                "allow_extrapolation",
                "Allow extrapolation without enough neighbors.",
                Some(json!(false)),
            ),
            option("class", "Ground classification value.", Some(json!(2))),
        ],
        "filters.iqr" => vec![
            option(
                "multiplier",
                "Interquartile range multiplier.",
                Some(json!(1.5)),
            ),
            option("dimension", "Dimension to classify.", Some(json!("Z"))),
        ],
        "filters.label_duplicates" => vec![option(
            "dimensions",
            "Comma-separated dimensions used to identify adjacent duplicates.",
            Some(json!("X,Y,Z")),
        )],
        "filters.locate" => vec![
            option("dimension", "Dimension to inspect.", None),
            option("minmax", "Select the min or max point.", Some(json!("max"))),
        ],
        "filters.lof" => vec![option(
            "minpts",
            "Number of neighbors used for local outlier factor.",
            Some(json!(10)),
        )],
        "filters.mad" => vec![
            option("multiplier", "MAD scale multiplier.", Some(json!(1.4826))),
            option("dimension", "Dimension to classify.", Some(json!("Z"))),
            option(
                "mad_multiplier",
                "Outlier threshold multiplier.",
                Some(json!(2.0)),
            ),
        ],
        "filters.merge" => Vec::new(),
        "filters.mortonorder" => vec![option(
            "reverse",
            "Sort in reverse Morton order.",
            Some(json!(false)),
        )],
        "filters.nndistance" => vec![
            option("knn", "Neighbor rank or count.", Some(json!(8))),
            option("mode", "Distance mode: kth or avg.", Some(json!("kth"))),
        ],
        "filters.optimalneighborhood" => vec![
            option("min_k", "Minimum neighborhood size.", Some(json!(3))),
            option("max_k", "Maximum neighborhood size.", Some(json!(8))),
        ],
        "filters.outlier" => vec![
            option(
                "method",
                "Outlier method: statistical or radius.",
                Some(json!("statistical")),
            ),
            option(
                "min_k",
                "Minimum neighbors for radius mode.",
                Some(json!(2)),
            ),
            option("radius", "Radius for radius mode.", Some(json!(1.0))),
            option(
                "mean_k",
                "Neighbor count for statistical mode.",
                Some(json!(8)),
            ),
            option(
                "multiplier",
                "Standard deviation multiplier for statistical mode.",
                Some(json!(2.0)),
            ),
            option(
                "class",
                "Classification value for outliers.",
                Some(json!(7)),
            ),
        ],
        "filters.radialdensity" => {
            vec![option("radius", "Density search radius.", Some(json!(1.0)))]
        }
        "filters.planefit" => vec![option(
            "knn",
            "Number of nearest neighbors to fit.",
            Some(json!(8)),
        )],
        "filters.randomize" => vec![option("seed", "Optional deterministic shuffle seed.", None)],
        "filters.reciprocity" => vec![option(
            "knn",
            "Number of nearest neighbors to inspect.",
            Some(json!(8)),
        )],
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
        "filters.separatescanline" => vec![option(
            "groupby",
            "Number of scan lines to group into each output view.",
            Some(json!(1)),
        )],
        "filters.splitter" => vec![
            option(
                "length",
                "Edge length for splitter cells.",
                Some(json!(1000.0)),
            ),
            option("origin_x", "X origin for splitter cells.", None),
            option("origin_y", "Y origin for splitter cells.", None),
            option(
                "buffer",
                "Buffer distance around splitter cells.",
                Some(json!(0.0)),
            ),
        ],
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
        "filters.voxelcenternearestneighbor" | "filters.voxelcentroidnearestneighbor" => {
            vec![option("cell", "Voxel cell size.", Some(json!(1.0)))]
        }
        "filters.voxeldownsize" => vec![option("cell", "Voxel cell size.", Some(json!(1.0)))],
        "filters.zsmooth" => vec![
            option("radius", "2D neighbor search radius.", Some(json!(1.0))),
            option(
                "position",
                "Sorted-neighbor percentile position.",
                Some(json!(0.5)),
            ),
            option(
                "dimension",
                "Output dimension for smoothed Z values.",
                Some(json!("Z")),
            ),
        ],
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
}
