//! XML schema compatibility helpers.

use crate::point::{DimType, PointView};

pub fn remap_old_dimension_name(input: &str) -> String {
    if input.eq_ignore_ascii_case("Unnamed field 512")
        || input.eq_ignore_ascii_case("Chipper Point ID")
    {
        return "Chipper:PointID".to_string();
    }

    if input.eq_ignore_ascii_case("Unnamed field 513")
        || input.eq_ignore_ascii_case("Chipper Block ID")
    {
        return "Chipper:BlockID".to_string();
    }

    input.to_string()
}

pub fn point_cloud_schema_xml(views: &[PointView]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str(
        "<pc:PointCloudSchema xmlns:pc=\"http://pointcloud.org/schemas/PC/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\n",
    );

    let Some(view) = views.first() else {
        out.push_str(" <pc:orientation>point</pc:orientation>\n");
        out.push_str(" <pc:version>1.3</pc:version>\n");
        out.push_str("</pc:PointCloudSchema>\n");
        return out;
    };

    for idx in 0..view.layout().dim_count() {
        let Some((dim, dim_type)) = view.layout().dim_at(idx) else {
            continue;
        };
        out.push_str(" <pc:dimension>\n");
        out.push_str(&format!("  <pc:position>{}</pc:position>\n", idx + 1));
        out.push_str(&format!("  <pc:size>{}</pc:size>\n", dim_type.size()));
        out.push_str(&format!(
            "  <pc:name>{}</pc:name>\n",
            xml_escape(dim.name())
        ));
        out.push_str(&format!(
            "  <pc:interpretation>{}</pc:interpretation>\n",
            interpretation_name(dim_type)
        ));
        out.push_str("  <pc:active>true</pc:active>\n");
        out.push_str(" </pc:dimension>\n");
    }

    out.push_str(" <pc:orientation>point</pc:orientation>\n");
    out.push_str(" <pc:version>1.3</pc:version>\n");
    out.push_str("</pc:PointCloudSchema>\n");
    out
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn interpretation_name(dim_type: DimType) -> &'static str {
    match dim_type {
        DimType::U8 => "uint8_t",
        DimType::U16 => "uint16_t",
        DimType::U32 => "uint32_t",
        DimType::U64 => "uint64_t",
        DimType::I8 => "int8_t",
        DimType::I16 => "int16_t",
        DimType::I32 => "int32_t",
        DimType::I64 => "int64_t",
        DimType::F32 => "float",
        DimType::F64 => "double",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaps_legacy_chipper_dimension_names() {
        assert_eq!(
            remap_old_dimension_name("Unnamed field 512"),
            "Chipper:PointID"
        );
        assert_eq!(
            remap_old_dimension_name("Chipper Point ID"),
            "Chipper:PointID"
        );
        assert_eq!(
            remap_old_dimension_name("Unnamed field 513"),
            "Chipper:BlockID"
        );
        assert_eq!(
            remap_old_dimension_name("Chipper Block ID"),
            "Chipper:BlockID"
        );
    }

    #[test]
    fn preserves_unknown_dimension_names() {
        assert_eq!(remap_old_dimension_name("Intensity"), "Intensity");
    }

    #[test]
    fn writes_point_cloud_schema_for_view_layout() {
        use crate::point::{DimId, DimType, PointLayout, PointView};
        use std::rc::Rc;

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Intensity, DimType::U16);
        let view = PointView::new(Rc::new(layout));

        let xml = point_cloud_schema_xml(&[view]);

        assert!(xml.contains("<pc:PointCloudSchema"));
        assert!(xml.contains("<pc:name>X</pc:name>"));
        assert!(xml.contains("<pc:size>8</pc:size>"));
        assert!(xml.contains("<pc:interpretation>double</pc:interpretation>"));
        assert!(xml.contains("<pc:name>Intensity</pc:name>"));
        assert!(xml.contains("<pc:orientation>point</pc:orientation>"));
    }
}
