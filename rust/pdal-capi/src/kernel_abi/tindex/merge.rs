use crate::registry::pipeline_from_json;
use pdal_core::bounds::Bounds2D;
use pdal_kernels::{
    build_tindex_merge_plan, parse_tindex_merge_args, TindexMergeClip,
    TindexParseResult as ParseResult, TindexResolvedClip,
};
use pdal_native::geometry::Geometry;

pub(super) fn run_merge(args: &[String]) -> i32 {
    let args = match parse_tindex_merge_args(args) {
        Ok(parsed) => parsed,
        Err(ParseResult::Error(message)) => {
            eprintln!("PDAL: kernels.tindex: {message}");
            return 1;
        }
        Err(ParseResult::Unsupported) => return -1,
    };

    let index_json = match std::fs::read_to_string(&args.tindex_file) {
        Ok(json) => json,
        Err(err) => {
            eprintln!(
                "PDAL: kernels.tindex: Unable to read tindex '{}': {err}",
                args.tindex_file
            );
            return 1;
        }
    };
    let clip = match args.clip.as_ref().map(resolve_merge_clip).transpose() {
        Ok(clip) => clip,
        Err(()) => return 1,
    };

    let plan = match build_tindex_merge_plan(&args, &index_json, clip) {
        Ok(plan) => plan,
        Err(ParseResult::Error(message)) => {
            eprintln!("PDAL: kernels.tindex: {message}");
            return 1;
        }
        Err(ParseResult::Unsupported) => return -1,
    };
    println!("Merge filecount: {}", plan.file_count);
    execute_pipeline(plan.pipeline_json)
}

fn resolve_merge_clip(clip: &TindexMergeClip) -> Result<TindexResolvedClip, ()> {
    match clip {
        TindexMergeClip::Bounds { bounds, value } => Ok(TindexResolvedClip {
            bounds: *bounds,
            stage_key: "bounds",
            stage_value: value.clone(),
        }),
        TindexMergeClip::Polygon { value } => parse_merge_polygon(value),
    }
}

fn parse_merge_polygon(value: &str) -> Result<TindexResolvedClip, ()> {
    let geometry = Geometry::from_wkt(value).map_err(|err| {
        eprintln!("PDAL: kernels.tindex: Invalid polygon: {err}");
    })?;
    let (minx, maxx, miny, maxy, _, _) = geometry.bounds().map_err(|err| {
        eprintln!("PDAL: kernels.tindex: Invalid polygon bounds: {err}");
    })?;
    Ok(TindexResolvedClip {
        bounds: Bounds2D {
            minx,
            maxx,
            miny,
            maxy,
        },
        stage_key: "polygon",
        stage_value: value.to_string(),
    })
}

fn execute_pipeline(pipeline_json: serde_json::Value) -> i32 {
    let mut pipeline = match pipeline_from_json(&pipeline_json.to_string()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            return 1;
        }
    };
    match pipeline.execute_with_result(Vec::new()) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            1
        }
    }
}
