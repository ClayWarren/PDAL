use super::{filename, option};
use serde_json::json;

pub(super) fn options(stage_name: &str) -> Vec<serde_json::Value> {
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
        | "readers.spz" | "readers.terrasolid" => vec![filename()],
        "readers.las" | "readers.laz" => vec![
            filename(),
            option(
                "start",
                "Point at which reading should start.",
                Some(json!(0)),
            ),
            option("count", "Maximum number of points to read.", None),
            option(
                "nosrs",
                "Skip reading file spatial reference.",
                Some(json!(false)),
            ),
        ],
        "readers.nitf" => vec![
            filename(),
            option("count", "Maximum number of points to read.", None),
            option(
                "spatialreference",
                "Override the inferred spatial reference.",
                None,
            ),
        ],
        "readers.text" => vec![
            filename(),
            option(
                "separator",
                "Separator character overriding header-line inference.",
                Some(json!(" ")),
            ),
            option("header", "Use this string as the header line.", None),
            option(
                "skip",
                "Lines to skip before reading the header line.",
                Some(json!(0)),
            ),
        ],
        "readers.gdal" => vec![
            filename(),
            option(
                "header",
                "Comma-separated dimension names for raster bands.",
                None,
            ),
            option("gdalopts", "GDAL open options.", None),
        ],
        "readers.copc" => vec![
            filename(),
            option(
                "bounds",
                "Optional 2D or 3D bounds used to filter returned points.",
                None,
            ),
        ],
        "readers.ept" => vec![
            filename(),
            option(
                "bounds",
                "Optional 2D or 3D bounds used to filter returned points.",
                None,
            ),
            option(
                "resolution",
                "Optional EPT hierarchy resolution limit.",
                None,
            ),
            option("origin", "Optional EPT source origin id or name.", None),
            option(
                "ignore_unreadable",
                "Skip unreadable EPT tiles instead of failing.",
                Some(json!(false)),
            ),
        ],
        "readers.tindex" => vec![
            filename(),
            option(
                "tindex_name",
                "Tile index field containing source filenames.",
                Some(json!("location")),
            ),
        ],
        "readers.stac" => vec![
            filename(),
            option(
                "asset_names",
                "STAC asset names to read.",
                Some(json!("data")),
            ),
        ],
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
        _ => Vec::new(),
    }
}
