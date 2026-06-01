use super::option;
use serde_json::json;

pub(super) fn options(stage_name: &str) -> Vec<serde_json::Value> {
    match stage_name {
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
        "filters.assign" => vec![
            option(
                "assignment",
                "Dimension assignment of the form 'Dim[range]=value'.",
                None,
            ),
            option(
                "condition",
                "Only assign where this 'Dim[range]' condition holds.",
                None,
            ),
            option(
                "value",
                "Assignment expression(s): 'Dim = expression WHERE condition'.",
                None,
            ),
        ],
        "filters.colorinterp" => vec![
            option(
                "dimension",
                "Dimension to interpolate colors from.",
                Some(json!("Z")),
            ),
            option(
                "minimum",
                "Minimum value for scaling (auto if unset).",
                None,
            ),
            option(
                "maximum",
                "Maximum value for scaling (auto if unset).",
                None,
            ),
            option(
                "clamp",
                "Clamp values outside [minimum, maximum] to the range.",
                Some(json!(false)),
            ),
            option(
                "ramp",
                "Named built-in ramp or GDAL-readable color ramp image.",
                Some(json!("pestel_shades")),
            ),
            option("invert", "Invert the ramp direction.", Some(json!(false))),
            option(
                "mad",
                "Use Median Absolute Deviation with 'k' to compute bounds.",
                Some(json!(false)),
            ),
            option(
                "mad_multiplier",
                "MAD threshold multiplier.",
                Some(json!(1.4862)),
            ),
            option(
                "k",
                "Number of deviations for the computed minimum/maximum.",
                Some(json!(0.0)),
            ),
        ],
        "filters.colorization" => vec![
            option(
                "raster",
                "Raster filename to sample band values from.",
                None,
            ),
            option(
                "dimensions",
                "Comma-separated 'dimension:band:scale' mappings to populate.",
                None,
            ),
        ],
        "filters.dem" => vec![
            option("raster", "Raster filename to compare points against.", None),
            option(
                "limits",
                "Dimension range the raster comparison applies to.",
                None,
            ),
            option("band", "Raster band to sample.", Some(json!(1))),
        ],
        "filters.divider" => vec![
            option(
                "mode",
                "Partitioning mode ('partition' or 'round_robin').",
                Some(json!("partition")),
            ),
            option(
                "capacity",
                "Maximum number of points per output view.",
                None,
            ),
            option("count", "Number of output views to create.", Some(json!(1))),
        ],
        "filters.expression" => vec![
            option(
                "expression",
                "Keep points where this expression is true.",
                None,
            ),
            option(
                "limits",
                "Legacy dimension-range form of the keep condition.",
                None,
            ),
        ],
        "filters.expressionstats" => vec![
            option("dimension", "Dimension to summarize.", None),
            option(
                "expressions",
                "Expressions whose matching points are counted.",
                None,
            ),
        ],
        "filters.farthestpointsampling" => vec![option(
            "count",
            "Number of points to retain.",
            Some(json!(1000)),
        )],
        "filters.ferry" => vec![option(
            "dimensions",
            "Comma-separated 'from=>to' dimension copy mappings.",
            None,
        )],
        "filters.geomdistance" => vec![
            option(
                "geometry",
                "WKT/GeoJSON polygon to measure distance from.",
                None,
            ),
            option(
                "ogr",
                "OGR datasource JSON to load the geometry from.",
                None,
            ),
            option(
                "dimension",
                "Dimension to store the computed distance in.",
                Some(json!("distance")),
            ),
            option(
                "ring",
                "Measure distance to the ring instead of the filled polygon.",
                Some(json!(false)),
            ),
        ],
        "filters.hag_dem" => vec![
            option("raster", "Ground-surface raster filename.", None),
            option("band", "Raster band to sample.", Some(json!(1))),
            option(
                "zero_ground",
                "Set HeightAboveGround to 0 for ground-classified points.",
                Some(json!(true)),
            ),
            option("min_clamp", "Minimum HeightAboveGround clamp.", None),
            option("max_clamp", "Maximum HeightAboveGround clamp.", None),
            option(
                "nodata_hag",
                "HeightAboveGround assigned where the raster has no data.",
                Some(json!(0.0)),
            ),
            option(
                "class",
                "Classification value treated as ground.",
                Some(json!(2)),
            ),
        ],
        "filters.mongo" => vec![option(
            "expression",
            "MongoDB-style JSON filter expression.",
            None,
        )],
        "filters.neighborclassifier" => vec![
            option(
                "candidate",
                "Candidate filename providing reference classifications.",
                None,
            ),
            option("k", "Number of nearest neighbors to vote.", Some(json!(8))),
            option(
                "dimension",
                "Dimension to assign by majority vote.",
                Some(json!("Classification")),
            ),
        ],
        "filters.overlay" => vec![
            option("datasource", "OGR datasource to read polygons from.", None),
            option(
                "column",
                "Attribute column providing the value to assign.",
                None,
            ),
            option(
                "dimension",
                "Dimension to assign the polygon value to.",
                None,
            ),
        ],
        "filters.projpipeline" => vec![
            option("coord_op", "PROJ coordinate operation pipeline.", None),
            option("out_srs", "Output spatial reference.", None),
            option(
                "reverse_transfo",
                "Apply the coordinate operation in reverse.",
                Some(json!(false)),
            ),
        ],
        "filters.radiusassign" => vec![
            option(
                "radius",
                "Search radius for the assignment.",
                Some(json!(0.0)),
            ),
            option(
                "update_expression",
                "Assignment expression(s) applied within the radius.",
                None,
            ),
        ],
        "filters.skewnessbalancing" => vec![
            option(
                "ground_class",
                "Classification assigned to ground points.",
                Some(json!(2)),
            ),
            option(
                "other_class",
                "Classification assigned to non-ground points.",
                Some(json!(1)),
            ),
            option(
                "only_ground",
                "Only set the ground class, leaving others unchanged.",
                Some(json!(false)),
            ),
        ],
        "filters.sparsesurface" => vec![
            option(
                "ground_class",
                "Classification value treated as ground.",
                Some(json!(2)),
            ),
            option(
                "low_point_class",
                "Classification assigned to suppressed points.",
                Some(json!(7)),
            ),
            option(
                "radius",
                "Suppression radius around retained points.",
                Some(json!(1.0)),
            ),
        ],
        "filters.transformation" => vec![
            option(
                "matrix",
                "Row-major 4x4 affine transformation matrix.",
                None,
            ),
            option(
                "invert",
                "Invert the matrix before applying it.",
                Some(json!(false)),
            ),
        ],
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
        "filters.covariancefeatures" => vec![
            option(
                "knn",
                "Number of nearest neighbors to inspect.",
                Some(json!(10)),
            ),
            option(
                "stride",
                "Point stride for neighbor selection.",
                Some(json!(1)),
            ),
            option("radius", "Optional radius search distance.", None),
            option(
                "min_k",
                "Minimum neighbors required for radius mode.",
                Some(json!(3)),
            ),
            option(
                "mode",
                "Eigenvalue scaling mode: raw, sqrt, or normalized.",
                Some(json!("sqrt")),
            ),
            option(
                "optimized",
                "Use per-point optimal neighborhood dimensions.",
                Some(json!(false)),
            ),
            option(
                "feature_set",
                "Comma-separated covariance feature set.",
                Some(json!("dimensionality")),
            ),
        ],
        "filters.crop" => vec![
            option("bounds", "Point box for cropped points.", None),
            option(
                "polygon",
                "Bounding polygon(s) for cropped points (WKT or GeoJSON).",
                None,
            ),
            option("point", "Center point for distance-based cropping.", None),
            option(
                "distance",
                "Distance from 2D or 3D 'point' for cropping.",
                None,
            ),
            option(
                "outside",
                "Invert cropping: keep points outside the region.",
                Some(json!(false)),
            ),
            option("a_srs", "Spatial reference for bounding region.", None),
            option("ogr", "OGR datasource for filter geometries.", None),
        ],
        "filters.csf" => vec![
            option("smooth", "Apply slope postprocessing.", Some(json!(true))),
            option("step", "Time step.", Some(json!(0.65))),
            option("threshold", "Classification threshold.", Some(json!(0.5))),
            option("hdiff", "Height difference threshold.", Some(json!(0.3))),
            option("resolution", "Cloth resolution.", Some(json!(1.0))),
            option("rigidness", "Rigidness.", Some(json!(3))),
            option("iterations", "Max iterations.", Some(json!(500))),
            option("ignore", "Ignore values.", None),
            option(
                "returns",
                "Comma-separated return kinds to include.",
                Some(json!("last,only")),
            ),
            option(
                "debug",
                "Enable debug output to the 'dir' directory.",
                Some(json!(false)),
            ),
            option("dir", "Optional output directory for debugging.", None),
            option(
                "ground_class",
                "Classification value for ground points.",
                Some(json!(2)),
            ),
            option(
                "other_class",
                "Classification value for non-ground points.",
                Some(json!(1)),
            ),
            option(
                "only_ground",
                "Only emit ground-classified points.",
                Some(json!(false)),
            ),
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
        "filters.faceraster" => vec![
            option("resolution", "Raster cell edge length.", Some(json!(1.0))),
            option("origin_x", "Fixed raster X origin.", None),
            option("origin_y", "Fixed raster Y origin.", None),
            option("width", "Fixed raster width in cells.", None),
            option("height", "Fixed raster height in cells.", None),
            option("nodata", "No-data value.", None),
            option("mesh", "Mesh name.", None),
            option(
                "max_triangle_edge_length",
                "Maximum triangle edge length to rasterize.",
                None,
            ),
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
        "filters.h3" => vec![option(
            "resolution",
            "H3 resolution (0-15) for the computed index.",
            None,
        )],
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
        "filters.hag_delaunay" => vec![
            option(
                "count",
                "Neighbor count for ground interpolation.",
                Some(json!(10)),
            ),
            option(
                "allow_extrapolation",
                "Allow interpolation outside the ground point bounds.",
                Some(json!(true)),
            ),
            option("class", "Ground classification value.", Some(json!(2))),
        ],
        "filters.hexbin" => vec![
            option(
                "sample_size",
                "Maximum sample size for auto-edge length calculation.",
                Some(json!(5000)),
            ),
            option("threshold", "Required cell density.", Some(json!(15))),
            option("edge_size", "Deprecated synonym for edge_length.", None),
            option("edge_length", "Length of each hex edge.", None),
            option("density", "Density tessellation output filename.", None),
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
        "filters.litree" => vec![
            option(
                "min_points",
                "Minimum point count for a tree.",
                Some(json!(10)),
            ),
            option(
                "min_height",
                "Minimum height above ground.",
                Some(json!(3.0)),
            ),
            option(
                "radius",
                "Search radius for dummy points.",
                Some(json!(100.0)),
            ),
        ],
        "filters.lloydkmeans" => vec![
            option("k", "Number of clusters to segment.", Some(json!(10))),
            option(
                "dimensions",
                "Comma-separated dimensions used for clustering.",
                Some(json!("X,Y,Z")),
            ),
            option(
                "maxiters",
                "Maximum number of Lloyd iterations.",
                Some(json!(10)),
            ),
        ],
        "filters.m3c2" => vec![
            option(
                "normal_radius",
                "Radius used to estimate normals.",
                Some(json!(2.0)),
            ),
            option(
                "cyl_radius",
                "Cylinder radius for point comparisons.",
                Some(json!(2.0)),
            ),
            option(
                "cyl_halflen",
                "Cylinder half length for point comparisons.",
                Some(json!(5.0)),
            ),
            option("reg_error", "Registration error term.", Some(json!(0.0))),
            option(
                "orientation",
                "Normal orientation policy.",
                Some(json!("up")),
            ),
            option(
                "min_points",
                "Minimum points required per cloud.",
                Some(json!(1)),
            ),
        ],
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
        "filters.normal" => vec![
            option(
                "knn",
                "Number of nearest neighbors used to estimate normals.",
                Some(json!(8)),
            ),
            option("radius", "Optional radius search distance.", None),
            option("viewpoint", "WKT viewpoint used to orient normals.", None),
            option(
                "always_up",
                "Orient normals toward positive Z when no viewpoint is set.",
                Some(json!(true)),
            ),
            option(
                "refine",
                "Refine normals using minimum-spanning-tree propagation.",
                Some(json!(false)),
            ),
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
        "filters.miniball" => vec![option(
            "knn",
            "Number of nearest neighbors to inspect.",
            Some(json!(8)),
        )],
        "filters.pmf" => vec![
            option("cell_size", "Cell size.", Some(json!(1.0))),
            option(
                "exponential",
                "Use exponentially increasing window sizes.",
                Some(json!(true)),
            ),
            option(
                "initial_distance",
                "Initial elevation distance threshold.",
                Some(json!(0.15)),
            ),
            option(
                "returns",
                "Comma-separated return groups to include.",
                Some(json!("last,only")),
            ),
            option(
                "max_distance",
                "Maximum elevation distance threshold.",
                Some(json!(2.5)),
            ),
            option(
                "max_window_size",
                "Maximum morphological window size.",
                Some(json!(33.0)),
            ),
            option("slope", "Terrain slope factor.", Some(json!(1.0))),
            option(
                "ground_class",
                "Classification value for ground points.",
                Some(json!(2)),
            ),
            option(
                "other_class",
                "Classification value for non-ground points.",
                Some(json!(1)),
            ),
            option(
                "only_ground",
                "Only modify classification for detected ground points.",
                Some(json!(false)),
            ),
        ],
        "filters.randomize" => vec![option("seed", "Optional deterministic shuffle seed.", None)],
        "filters.range" => vec![option(
            "limits",
            "Dimension range expression used to keep points.",
            None,
        )],
        "filters.reciprocity" => vec![option(
            "knn",
            "Number of nearest neighbors to inspect.",
            Some(json!(8)),
        )],
        "filters.relaxationdartthrowing" => vec![
            option(
                "decay",
                "Radius decay rate after each pass.",
                Some(json!(0.9)),
            ),
            option("radius", "Initial exclusion radius.", Some(json!(1.0))),
            option(
                "terminal_radius",
                "Smallest exclusion radius before termination.",
                Some(json!(0.001)),
            ),
            option(
                "count",
                "Target number of output points.",
                Some(json!(1000)),
            ),
            option(
                "shuffle",
                "Shuffle points before sampling.",
                Some(json!(true)),
            ),
            option("seed", "Optional deterministic shuffle seed.", None),
        ],
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
        "filters.smrf" => vec![
            option("cell", "Cell size.", Some(json!(1.0))),
            option("slope", "Percent slope.", Some(json!(0.15))),
            option("scalar", "Elevation scalar.", Some(json!(1.25))),
            option("threshold", "Elevation threshold.", Some(json!(0.5))),
            option("window", "Maximum window size.", None),
            option(
                "returns",
                "Comma-separated return groups to include.",
                Some(json!("last,only")),
            ),
            option(
                "ground_class",
                "Classification value for ground points.",
                Some(json!(2)),
            ),
            option(
                "other_class",
                "Classification value for non-ground points.",
                Some(json!(1)),
            ),
            option(
                "only_ground",
                "Only modify classification for detected ground points.",
                Some(json!(true)),
            ),
        ],
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
        "filters.straighten" => vec![
            option(
                "polyline",
                "Track polyline as LINESTRING ZM, with M as roll in radians.",
                None,
            ),
            option(
                "reverse",
                "Map from straightened coordinates back to world coordinates.",
                Some(json!(false)),
            ),
            option(
                "offset",
                "Global offset for the straightened X coordinate.",
                Some(json!(0.0)),
            ),
        ],
        "filters.supervoxel" => vec![
            option(
                "knn",
                "Neighbor count for local graph construction.",
                Some(json!(32)),
            ),
            option("resolution", "Voxel resolution.", Some(json!(1.0))),
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
        _ => Vec::new(),
    }
}
