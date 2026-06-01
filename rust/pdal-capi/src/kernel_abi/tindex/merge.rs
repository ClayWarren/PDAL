use crate::registry::pipeline_from_json;
use pdal_core::bounds::Bounds2D;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_kernels::{parse_tindex_merge_args, TindexMergeClip, TindexParseResult as ParseResult};
use pdal_native::geometry::Geometry;

struct MergeClip {
    bounds: Bounds2D,
    stage_key: &'static str,
    stage_value: String,
}

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
    let index: serde_json::Value = match serde_json::from_str(&index_json) {
        Ok(index) => index,
        Err(err) => {
            eprintln!(
                "PDAL: kernels.tindex: Unable to parse GeoJSON tindex '{}': {err}",
                args.tindex_file
            );
            return 1;
        }
    };
    let Some(features) = index["features"].as_array() else {
        eprintln!("PDAL: kernels.tindex: tindex merge expects a GeoJSON FeatureCollection.");
        return 1;
    };
    if features.is_empty() {
        eprintln!("PDAL: kernels.tindex: tindex contains no features.");
        return 1;
    }

    let mut stages = Vec::new();
    let mut tags = Vec::new();
    let mut file_count = 0;
    let clip = match args.clip.as_ref().map(resolve_merge_clip).transpose() {
        Ok(clip) => clip,
        Err(()) => return 1,
    };
    for (index, feature) in features.iter().enumerate() {
        if let Some(clip) = &clip {
            let Some(feature_bounds) = feature_bounds_2d(feature) else {
                eprintln!("PDAL: kernels.tindex: Feature has invalid geometry.");
                return 1;
            };
            if !feature_bounds.overlaps(&clip.bounds) {
                continue;
            }
        }
        let Some(location) = feature["properties"][&args.location_field].as_str() else {
            eprintln!(
                "PDAL: kernels.tindex: Feature is missing '{}'.",
                args.location_field
            );
            return 1;
        };
        let Some(reader) = infer_reader_driver(location) else {
            eprintln!("PDAL: kernels.tindex: unable to infer reader driver for '{location}'.");
            return 1;
        };
        let tag = format!("tindex_input_{index}");
        stages.push(serde_json::json!({
            "type": reader,
            "filename": location,
            "tag": tag.clone(),
        }));
        let mut input_tag = tag;
        let feature_srs = feature["properties"]["srs"].as_str().unwrap_or("");
        if !feature_srs.is_empty() && feature_srs != args.target_srs {
            let reprojection_tag = format!("tindex_reprojection_{index}");
            stages.push(serde_json::json!({
                "type": "filters.reprojection",
                "in_srs": feature_srs,
                "out_srs": &args.target_srs,
                "inputs": [input_tag],
                "tag": reprojection_tag,
            }));
            input_tag = reprojection_tag;
        }
        if let Some(clip) = &clip {
            let crop_tag = format!("tindex_crop_{index}");
            stages.push(serde_json::json!({
                "type": "filters.crop",
                (clip.stage_key): clip.stage_value,
                "inputs": [input_tag],
                "tag": crop_tag,
            }));
            tags.push(crop_tag);
        } else {
            tags.push(input_tag);
        }
        file_count += 1;
    }
    println!("Merge filecount: {file_count}");
    if stages.is_empty() {
        eprintln!("PDAL: kernels.tindex: No indexed files matched merge criteria.");
        return 1;
    }
    if stages.len() > 1 {
        stages.push(serde_json::json!({
            "type": "filters.merge",
            "inputs": tags,
        }));
    }
    let Some(writer) = infer_writer_driver(&args.output_file) else {
        eprintln!(
            "PDAL: kernels.tindex: Unable to infer writer driver for '{}'.",
            args.output_file
        );
        return 1;
    };
    stages.push(serde_json::json!({ "type": writer, "filename": args.output_file }));
    execute_pipeline(serde_json::Value::Array(stages))
}

fn resolve_merge_clip(clip: &TindexMergeClip) -> Result<MergeClip, ()> {
    match clip {
        TindexMergeClip::Bounds { bounds, value } => Ok(MergeClip {
            bounds: *bounds,
            stage_key: "bounds",
            stage_value: value.clone(),
        }),
        TindexMergeClip::Polygon { value } => parse_merge_polygon(value),
    }
}

fn parse_merge_polygon(value: &str) -> Result<MergeClip, ()> {
    let geometry = Geometry::from_wkt(value).map_err(|err| {
        eprintln!("PDAL: kernels.tindex: Invalid polygon: {err}");
    })?;
    let (minx, maxx, miny, maxy, _, _) = geometry.bounds().map_err(|err| {
        eprintln!("PDAL: kernels.tindex: Invalid polygon bounds: {err}");
    })?;
    Ok(MergeClip {
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

fn feature_bounds_2d(feature: &serde_json::Value) -> Option<Bounds2D> {
    let geometry = feature.get("geometry")?;
    match geometry.get("type")?.as_str()? {
        "Polygon" => bounds_from_positions(geometry.get("coordinates")?.get(0)?.as_array()?),
        "MultiPolygon" => {
            let polygons = geometry.get("coordinates")?.as_array()?;
            let mut output: Option<Bounds2D> = None;
            for polygon in polygons {
                let ring = polygon.get(0)?.as_array()?;
                let bounds = bounds_from_positions(ring)?;
                if let Some(out) = output.as_mut() {
                    out.grow_bounds(&bounds);
                } else {
                    output = Some(bounds);
                }
            }
            output
        }
        _ => None,
    }
}

fn bounds_from_positions(positions: &[serde_json::Value]) -> Option<Bounds2D> {
    let mut iter = positions.iter();
    let first = iter.next()?.as_array()?;
    let mut bounds = Bounds2D::empty();
    bounds.grow_point(first.first()?.as_f64()?, first.get(1)?.as_f64()?);
    for position in iter {
        let coords = position.as_array()?;
        bounds.grow_point(coords.first()?.as_f64()?, coords.get(1)?.as_f64()?);
    }
    Some(bounds)
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
