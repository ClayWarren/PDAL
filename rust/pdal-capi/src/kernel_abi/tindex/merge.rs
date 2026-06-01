use super::{next_value, ParseResult};
use crate::registry::pipeline_from_json;
use pdal_core::bounds::{parse_bounds2d, Bounds2D};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_native::geometry::Geometry;

struct MergeClip {
    bounds: Bounds2D,
    stage_key: &'static str,
    stage_value: String,
}

struct MergeArgs {
    tindex_file: String,
    output_file: String,
    location_field: String,
    target_srs: String,
    clip: Option<MergeClip>,
}

pub(super) fn run_merge(args: &[String]) -> i32 {
    let args = match parse_merge_args(args) {
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
    for (index, feature) in features.iter().enumerate() {
        if let Some(clip) = &args.clip {
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
        if let Some(clip) = &args.clip {
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

fn parse_merge_args(args: &[String]) -> Result<MergeArgs, ParseResult> {
    let mut tindex_file = None;
    let mut output_file = None;
    let mut location_field = "location".to_string();
    let mut target_srs = "EPSG:4326".to_string();
    let mut clip = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => tindex_file = Some(next_value(&mut iter, "--tindex")?.clone()),
            "--filespec" => output_file = Some(next_value(&mut iter, "--filespec")?.clone()),
            "--tindex_name" => location_field = next_value(&mut iter, "--tindex_name")?.clone(),
            "--bounds" => {
                let value = next_value(&mut iter, "--bounds")?;
                clip = Some(parse_merge_bounds(value)?);
            }
            "--polygon" => {
                let value = next_value(&mut iter, "--polygon")?;
                clip = Some(parse_merge_polygon(value)?);
            }
            "--t_srs" => {
                target_srs = next_value(&mut iter, "--t_srs")?.clone();
            }
            "--log" => {
                let _ = next_value(&mut iter, "--log")?;
            }
            "--lyr_name" | "--ogrdriver" | "-f" => {
                let _ = next_value(&mut iter, arg)?;
            }
            _ if let Some(value) = arg.strip_prefix("--bounds=") => {
                clip = Some(parse_merge_bounds(value)?);
            }
            _ if let Some(value) = arg.strip_prefix("--polygon=") => {
                clip = Some(parse_merge_polygon(value)?);
            }
            _ if let Some(value) = arg.strip_prefix("--t_srs=") => {
                target_srs = value.to_string();
            }
            _ if arg.starts_with("--log=") => {}
            _ if arg.starts_with("--") => {
                return Err(ParseResult::Unsupported);
            }
            _ if tindex_file.is_none() => tindex_file = Some(arg.clone()),
            _ if output_file.is_none() => output_file = Some(arg.clone()),
            _ => return Err(ParseResult::Error("too many merge arguments".to_string())),
        }
    }

    let Some(tindex_file) = tindex_file else {
        return Err(ParseResult::Error(
            "merge requires --tindex <index>".to_string(),
        ));
    };
    let Some(output_file) = output_file else {
        return Err(ParseResult::Error(
            "merge requires --filespec <output>".to_string(),
        ));
    };
    Ok(MergeArgs {
        tindex_file,
        output_file,
        location_field,
        target_srs,
        clip,
    })
}

fn parse_merge_bounds(value: &str) -> Result<MergeClip, ParseResult> {
    let bounds = parse_bounds2d(value, 0)
        .map(|parsed| parsed.bounds)
        .map_err(|err| ParseResult::Error(format!("Invalid bounds: {err}")))?;
    Ok(MergeClip {
        bounds,
        stage_key: "bounds",
        stage_value: value.to_string(),
    })
}

fn parse_merge_polygon(value: &str) -> Result<MergeClip, ParseResult> {
    let geometry = Geometry::from_wkt(value)
        .map_err(|err| ParseResult::Error(format!("Invalid polygon: {err}")))?;
    let (minx, maxx, miny, maxy, _, _) = geometry
        .bounds()
        .map_err(|err| ParseResult::Error(format!("Invalid polygon bounds: {err}")))?;
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
