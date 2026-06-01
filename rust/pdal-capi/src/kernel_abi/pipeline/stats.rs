use pdal_core::point::{DimId, DimType, PointView};

pub(super) struct InfoDimensionStats {
    pub(super) name: String,
    pub(super) count: u64,
    pub(super) minimum: f64,
    pub(super) maximum: f64,
    pub(super) average: f64,
    pub(super) variance: f64,
    pub(super) stddev: f64,
    pub(super) values: Option<Vec<f64>>,
}

pub(super) fn dimension_stats(
    views: &[PointView],
    dimensions: Option<&[DimId]>,
    enumerate: Option<&[DimId]>,
) -> Vec<InfoDimensionStats> {
    let Some(first) = views.first() else {
        return Vec::new();
    };
    let layout = first.layout();
    let mut output = Vec::new();
    let mut selected = Vec::new();
    if let Some(dimensions) = dimensions {
        selected.extend(dimensions.iter().cloned());
    } else {
        for idx in 0..layout.dim_count() {
            if let Some((dim, _)) = layout.dim_at(idx) {
                selected.push(dim.clone());
            }
        }
    }

    for dim in &selected {
        let mut values = Vec::new();
        for view in views {
            if view.layout().dim(dim).is_none() {
                continue;
            }
            for point_idx in 0..view.len() {
                values.push(view.get_f64(point_idx, dim));
            }
        }
        if values.is_empty() {
            continue;
        }
        let count = values.len() as u64;
        let sum = values.iter().sum::<f64>();
        let average = sum / count as f64;
        let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let variance = if count > 1 {
            values
                .iter()
                .map(|value| {
                    let diff = value - average;
                    diff * diff
                })
                .sum::<f64>()
                / (count - 1) as f64
        } else {
            0.0
        };
        output.push(InfoDimensionStats {
            name: dim.name().to_string(),
            count,
            minimum,
            maximum,
            average,
            variance,
            stddev: variance.sqrt(),
            values: enumerate
                .is_some_and(|dims| dims.contains(dim))
                .then(|| unique_sorted_values(&values)),
        });
    }
    output
}

pub(super) fn unique_sorted_values(values: &[f64]) -> Vec<f64> {
    let mut unique = values.to_vec();
    unique.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    unique.dedup_by(|a, b| a == b);
    unique
}

pub(super) fn breakout_body(dim: &DimId, list_pad: &str, item_pad: &str) -> String {
    let value_pad = format!("{item_pad}  ");
    let expressions = [
        "(Withheld==1)",
        "(Keypoint==1)",
        "(Overlap==1)",
        "(Synthetic==1)",
    ];
    let mut output = format!(
        "{list_pad}\"breakout\":\n{list_pad}{{\n{item_pad}\"dimension\": \"{}\",\n{item_pad}\"statistic\":\n{item_pad}[\n",
        dim.name()
    );
    for (idx, expression) in expressions.iter().enumerate() {
        if idx > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "{value_pad}{{\n{value_pad}  \"expression\": \"{expression}\",\n{value_pad}  \"position\": {idx}\n{value_pad}}}"
        ));
    }
    output.push_str(&format!("\n{item_pad}]\n{list_pad}}}"));
    output
}

pub(super) fn dim_type_name(ty: DimType) -> &'static str {
    match ty {
        DimType::U8 | DimType::U16 | DimType::U32 | DimType::U64 => "unsigned",
        DimType::I8 | DimType::I16 | DimType::I32 | DimType::I64 => "signed",
        DimType::F32 | DimType::F64 => "floating",
    }
}

pub(super) fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let digits_before_decimal = value.abs().log10().floor().max(0.0) as i32 + 1;
    let decimals = (10 - digits_before_decimal).max(0) as usize;
    let mut text = format!("{value:.decimals$}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

pub(super) fn format_point_value(value: f64) -> String {
    format_number(value)
}
