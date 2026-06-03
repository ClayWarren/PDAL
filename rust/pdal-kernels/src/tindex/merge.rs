use super::{
    tindex_next_value, TindexMergeArgs, TindexMergeClip, TindexMergePlan, TindexParseResult,
    TindexResolvedClip,
};
use pdal_core::bounds::{parse_bounds2d, Bounds2D};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};

pub fn parse_tindex_merge_args(args: &[String]) -> Result<TindexMergeArgs, TindexParseResult> {
    let mut tindex_file = None;
    let mut output_file = None;
    let mut location_field = "location".to_string();
    let mut target_srs = "EPSG:4326".to_string();
    let mut clip = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => tindex_file = Some(tindex_next_value(&mut iter, "--tindex")?.clone()),
            "--filespec" => output_file = Some(tindex_next_value(&mut iter, "--filespec")?.clone()),
            "--tindex_name" => {
                location_field = tindex_next_value(&mut iter, "--tindex_name")?.clone()
            }
            "--bounds" => {
                let value = tindex_next_value(&mut iter, "--bounds")?;
                clip = Some(parse_merge_bounds(value)?);
            }
            "--polygon" => {
                clip = Some(TindexMergeClip::Polygon {
                    value: tindex_next_value(&mut iter, "--polygon")?.clone(),
                });
            }
            "--t_srs" => {
                target_srs = tindex_next_value(&mut iter, "--t_srs")?.clone();
            }
            "--log" => {
                let _ = tindex_next_value(&mut iter, "--log")?;
            }
            "--lyr_name" | "--ogrdriver" | "-f" => {
                let _ = tindex_next_value(&mut iter, arg)?;
            }
            _ if let Some(value) = arg.strip_prefix("--bounds=") => {
                clip = Some(parse_merge_bounds(value)?);
            }
            _ if let Some(value) = arg.strip_prefix("--polygon=") => {
                clip = Some(TindexMergeClip::Polygon {
                    value: value.to_string(),
                });
            }
            _ if let Some(value) = arg.strip_prefix("--t_srs=") => {
                target_srs = value.to_string();
            }
            _ if arg.starts_with("--log=") => {}
            _ if arg.starts_with("--") => {
                return Err(TindexParseResult::Error(format!(
                    "unknown tindex merge option '{arg}'"
                )));
            }
            _ if tindex_file.is_none() => tindex_file = Some(arg.clone()),
            _ if output_file.is_none() => output_file = Some(arg.clone()),
            _ => {
                return Err(TindexParseResult::Error(
                    "too many merge arguments".to_string(),
                ))
            }
        }
    }

    let Some(tindex_file) = tindex_file else {
        return Err(TindexParseResult::Error(
            "merge requires --tindex <index>".to_string(),
        ));
    };
    let Some(output_file) = output_file else {
        return Err(TindexParseResult::Error(
            "merge requires --filespec <output>".to_string(),
        ));
    };
    Ok(TindexMergeArgs {
        tindex_file,
        output_file,
        location_field,
        target_srs,
        clip,
    })
}

fn parse_merge_bounds(value: &str) -> Result<TindexMergeClip, TindexParseResult> {
    let bounds = parse_bounds2d(value, 0)
        .map(|parsed| parsed.bounds)
        .map_err(|err| TindexParseResult::Error(format!("Invalid bounds: {err}")))?;
    Ok(TindexMergeClip::Bounds {
        bounds,
        value: value.to_string(),
    })
}

pub fn build_tindex_merge_plan(
    args: &TindexMergeArgs,
    index_json: &str,
    clip: Option<TindexResolvedClip>,
) -> Result<TindexMergePlan, TindexParseResult> {
    let index: serde_json::Value = serde_json::from_str(index_json).map_err(|err| {
        TindexParseResult::Error(format!(
            "Unable to parse GeoJSON tindex '{}': {err}",
            args.tindex_file
        ))
    })?;
    let Some(features) = index["features"].as_array() else {
        return Err(TindexParseResult::Error(
            "tindex merge expects a GeoJSON FeatureCollection.".to_string(),
        ));
    };
    if features.is_empty() {
        return Err(TindexParseResult::Error(
            "tindex contains no features.".to_string(),
        ));
    }

    let mut stages = Vec::new();
    let mut tags = Vec::new();
    let mut file_count = 0;
    for (index, feature) in features.iter().enumerate() {
        if let Some(clip) = &clip {
            let Some(feature_bounds) = feature_bounds_2d(feature) else {
                return Err(TindexParseResult::Error(
                    "Feature has invalid geometry.".to_string(),
                ));
            };
            if !feature_bounds.overlaps(&clip.bounds) {
                continue;
            }
        }
        let Some(location) = feature["properties"][&args.location_field].as_str() else {
            return Err(TindexParseResult::Error(format!(
                "Feature is missing '{}'.",
                args.location_field
            )));
        };
        let Some(reader) = infer_reader_driver(location) else {
            return Err(TindexParseResult::Error(format!(
                "unable to infer reader driver for '{location}'."
            )));
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
    if stages.is_empty() {
        return Err(TindexParseResult::Error(
            "No indexed files matched merge criteria.".to_string(),
        ));
    }
    if stages.len() > 1 {
        stages.push(serde_json::json!({
            "type": "filters.merge",
            "inputs": tags,
        }));
    }
    let Some(writer) = infer_writer_driver(&args.output_file) else {
        return Err(TindexParseResult::Error(format!(
            "Unable to infer writer driver for '{}'.",
            args.output_file
        )));
    };
    stages.push(serde_json::json!({ "type": writer, "filename": args.output_file }));
    Ok(TindexMergePlan {
        file_count,
        pipeline_json: serde_json::Value::Array(stages),
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
