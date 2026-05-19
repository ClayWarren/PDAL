//! XML metadata sidecar parser for `readers.ilvis2`.

use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::stage::StageError;
use roxmltree::{Document, Node, ParsingOptions};
use std::fs;
use std::path::Path;

pub fn read_metadata_file(path: &Path) -> Result<MetadataNode, StageError> {
    let text = fs::read_to_string(path).map_err(|_| {
        StageError(format!(
            "Unable to open ILVIS2 metadata file '{}'.",
            path.display()
        ))
    })?;
    read_metadata_str(&text)
}

pub fn read_metadata_str(text: &str) -> Result<MetadataNode, StageError> {
    let document = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
    )
    .map_err(|err| StageError(format!("Invalid ILVIS2 metadata XML: {err}")))?;
    let root = document.root_element();
    let mut metadata = MetadataNode::new("readers.ilvis2");
    parse_granule_metadata_file(root, &mut metadata)?;
    Ok(metadata)
}

fn parse_granule_metadata_file(
    node: Node<'_, '_>,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    assert_element(node, "GranuleMetaDataFile")?;
    let children = element_children(node);
    let dtd = expect_child(&children, 0, "DTDVersion")?;
    add_f64(metadata, "DTDVersion", dtd);
    let data_center = expect_child(&children, 1, "DataCenterId")?;
    add_string(metadata, "DataCenterID", data_center);
    let granule = expect_child(&children, 2, "GranuleURMetaData")?;
    parse_granule_ur_metadata(granule, metadata)?;
    assert_no_extra(&children, 3)
}

fn parse_granule_ur_metadata(
    node: Node<'_, '_>,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    assert_element(node, "GranuleURMetaData")?;
    let children = element_children(node);
    let mut idx = 0;
    add_string(
        metadata,
        "GranuleUR",
        expect_child(&children, idx, "GranuleUR")?,
    );
    idx += 1;
    add_i64(metadata, "DbID", expect_child(&children, idx, "DbID")?);
    idx += 1;
    add_string(
        metadata,
        "InsertTime",
        expect_child(&children, idx, "InsertTime")?,
    );
    idx += 1;
    add_string(
        metadata,
        "LastUpdate",
        expect_child(&children, idx, "LastUpdate")?,
    );
    idx += 1;

    if matches_child(&children, idx, "CollectionMetaData") {
        parse_collection_metadata(children[idx], metadata)?;
        idx += 1;
    }
    if matches_child(&children, idx, "DataFiles") {
        parse_data_files(children[idx], metadata)?;
        idx += 1;
    }
    if matches_child(&children, idx, "ECSDataGranule") {
        parse_ecs_data_granule(children[idx], metadata)?;
        idx += 1;
    }
    if matches_child(&children, idx, "RangeDateTime") {
        parse_range_date_time(children[idx], metadata)?;
        idx += 1;
    }
    if matches_child(&children, idx, "SpatialDomainContainer") {
        parse_spatial_domain_container(children[idx], metadata)?;
        idx += 1;
    }
    while matches_child(&children, idx, "Platform") {
        let mut platform = MetadataNode::new("Platform");
        parse_platform(children[idx], &mut platform)?;
        metadata.add_child(platform);
        idx += 1;
    }
    while matches_child(&children, idx, "Campaign") {
        let campaign = expect_only_child(children[idx], "CampaignShortName")?;
        metadata.add_value("Campaign", string_value(campaign));
        idx += 1;
    }
    if matches_child(&children, idx, "PSAs") {
        parse_psas(children[idx], metadata)?;
        idx += 1;
    }
    for (node_name, prefix) in [
        ("BrowseProduct", "Browse"),
        ("PHProduct", "PH"),
        ("QAProduct", "QA"),
        ("MPProduct", "MP"),
    ] {
        if matches_child(&children, idx, node_name) {
            parse_product(children[idx], prefix, metadata)?;
            idx += 1;
        }
    }
    assert_no_extra(&children, idx)
}

fn parse_collection_metadata(
    node: Node<'_, '_>,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    assert_element(node, "CollectionMetaData")?;
    let children = element_children(node);
    add_string(
        metadata,
        "CollectionShortName",
        expect_child(&children, 0, "ShortName")?,
    );
    add_i64(
        metadata,
        "CollectionVersionID",
        expect_child(&children, 1, "VersionID")?,
    );
    assert_no_extra(&children, 2)
}

fn parse_data_files(node: Node<'_, '_>, metadata: &mut MetadataNode) -> Result<(), StageError> {
    assert_element(node, "DataFiles")?;
    for child in element_children(node) {
        assert_element(child, "DataFileContainer")?;
        let mut file = MetadataNode::new("DataFile");
        parse_data_file_container(child, &mut file)?;
        metadata.add_child(file);
    }
    Ok(())
}

fn parse_data_file_container(
    node: Node<'_, '_>,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    let children = element_children(node);
    let mut idx = 0;
    add_string(
        metadata,
        "DistributedFileName",
        expect_child(&children, idx, "DistributedFileName")?,
    );
    idx += 1;
    add_i64(
        metadata,
        "FileSize",
        expect_child(&children, idx, "FileSize")?,
    );
    idx += 1;
    for name in ["ChecksumType", "Checksum", "ChecksumOrigin"] {
        if matches_child(&children, idx, name) {
            add_string(metadata, name, children[idx]);
            idx += 1;
        }
    }
    assert_no_extra(&children, idx)
}

fn parse_ecs_data_granule(
    node: Node<'_, '_>,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    assert_element(node, "ECSDataGranule")?;
    let children = element_children(node);
    let mut idx = 0;
    if matches_child(&children, idx, "SizeMBECSDataGranule") {
        add_f64(metadata, "SizeMBECSDataGranule", children[idx]);
        idx += 1;
    }
    add_string(
        metadata,
        "LocalGranuleID",
        expect_child(&children, idx, "LocalGranuleID")?,
    );
    idx += 1;
    if matches_child(&children, idx, "ProductionDateTime") {
        add_string(metadata, "ProductionDateTime", children[idx]);
        idx += 1;
    }
    add_string(
        metadata,
        "LocalVersionID",
        expect_child(&children, idx, "LocalVersionID")?,
    );
    idx += 1;
    assert_no_extra(&children, idx)
}

fn parse_range_date_time(
    node: Node<'_, '_>,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    assert_element(node, "RangeDateTime")?;
    let children = element_children(node);
    for (idx, name) in [
        "RangeEndingTime",
        "RangeEndingDate",
        "RangeBeginningTime",
        "RangeBeginningDate",
    ]
    .iter()
    .enumerate()
    {
        add_string(metadata, name, expect_child(&children, idx, name)?);
    }
    assert_no_extra(&children, 4)
}

fn parse_spatial_domain_container(
    node: Node<'_, '_>,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    assert_element(node, "SpatialDomainContainer")?;
    let children = element_children(node);
    if children.is_empty() {
        return Ok(());
    }
    let horizontal = expect_child(&children, 0, "HorizontalSpatialDomainContainer")?;
    let horizontal_children = element_children(horizontal);
    parse_gpolygon(expect_child(&horizontal_children, 0, "GPolygon")?, metadata)?;
    assert_no_extra(&horizontal_children, 1)?;
    assert_no_extra(&children, 1)
}

fn parse_gpolygon(node: Node<'_, '_>, metadata: &mut MetadataNode) -> Result<(), StageError> {
    let mut rings = Vec::new();
    for boundary in element_children(node) {
        assert_element(boundary, "Boundary")?;
        let points = element_children(boundary);
        if points.len() < 3 {
            return Err(StageError(
                "Found a polygon boundary with less than 3 points, invalid for this schema"
                    .to_string(),
            ));
        }
        let mut ring = Vec::new();
        for point in points {
            assert_element(point, "Point")?;
            let point_children = element_children(point);
            let lon = text(expect_child(&point_children, 0, "PointLongitude")?)
                .parse::<f64>()
                .unwrap_or(0.0);
            let lat = text(expect_child(&point_children, 1, "PointLatitude")?)
                .parse::<f64>()
                .unwrap_or(0.0);
            assert_no_extra(&point_children, 2)?;
            ring.push((lon, lat));
        }
        if ring.first() != ring.last() {
            let first = ring[0];
            ring.push(first);
        }
        rings.push(ring);
    }
    if rings.is_empty() {
        return Err(StageError(
            "Expected element 'Boundary', found 'none'".to_string(),
        ));
    }
    let wkt = if rings.len() > 1 {
        format!(
            "MULTIPOLYGON ({})",
            rings
                .iter()
                .map(|ring| format!("(({}))", ring_wkt(ring)))
                .collect::<Vec<_>>()
                .join(",")
        )
    } else {
        format!("POLYGON (({}))", ring_wkt(&rings[0]))
    };
    metadata.add_value("ConvexHull", MetadataValue::String(wkt));
    Ok(())
}

fn parse_platform(node: Node<'_, '_>, metadata: &mut MetadataNode) -> Result<(), StageError> {
    let children = element_children(node);
    let mut idx = 0;
    add_string(
        metadata,
        "PlatformShortName",
        expect_child(&children, idx, "PlatformShortName")?,
    );
    idx += 1;
    while matches_child(&children, idx, "Instrument") {
        let mut instrument = MetadataNode::new("Instrument");
        parse_instrument(children[idx], &mut instrument)?;
        metadata.add_child(instrument);
        idx += 1;
    }
    assert_no_extra(&children, idx)
}

fn parse_instrument(node: Node<'_, '_>, metadata: &mut MetadataNode) -> Result<(), StageError> {
    let children = element_children(node);
    let mut idx = 0;
    add_string(
        metadata,
        "InstrumentShortName",
        expect_child(&children, idx, "InstrumentShortName")?,
    );
    idx += 1;
    while matches_child(&children, idx, "Sensor") {
        let mut sensor = MetadataNode::new("Sensor");
        parse_sensor(children[idx], &mut sensor)?;
        metadata.add_child(sensor);
        idx += 1;
    }
    while matches_child(&children, idx, "OperationMode") {
        metadata.add_value("OperationMode", string_value(children[idx]));
        idx += 1;
    }
    assert_no_extra(&children, idx)
}

fn parse_sensor(node: Node<'_, '_>, metadata: &mut MetadataNode) -> Result<(), StageError> {
    let children = element_children(node);
    let mut idx = 0;
    add_string(
        metadata,
        "SensorShortName",
        expect_child(&children, idx, "SensorShortName")?,
    );
    idx += 1;
    while matches_child(&children, idx, "SensorCharacteristic") {
        let mut characteristic = MetadataNode::new("SensorCharacteristic");
        let characteristic_children = element_children(children[idx]);
        add_string(
            &mut characteristic,
            "CharacteristicName",
            expect_child(&characteristic_children, 0, "SensorCharacteristicName")?,
        );
        add_string(
            &mut characteristic,
            "CharacteristicValue",
            expect_child(&characteristic_children, 1, "SensorCharacteristicValue")?,
        );
        assert_no_extra(&characteristic_children, 2)?;
        metadata.add_child(characteristic);
        idx += 1;
    }
    assert_no_extra(&children, idx)
}

fn parse_psas(node: Node<'_, '_>, metadata: &mut MetadataNode) -> Result<(), StageError> {
    for psa_node in element_children(node) {
        assert_element(psa_node, "PSA")?;
        let mut psa = MetadataNode::new("PSA");
        let children = element_children(psa_node);
        add_string(&mut psa, "PSAName", expect_child(&children, 0, "PSAName")?);
        let mut idx = 1;
        while matches_child(&children, idx, "PSAValue") {
            psa.add_value("PSAValue", string_value(children[idx]));
            idx += 1;
        }
        assert_no_extra(&children, idx)?;
        metadata.add_child(psa);
    }
    Ok(())
}

fn parse_product(
    node: Node<'_, '_>,
    prefix: &str,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    let expected = format!("{prefix}GranuleId");
    let metadata_name = format!("{prefix}ProductGranuleId");
    for child in element_children(node) {
        assert_element(child, &expected)?;
        metadata.add_value(&metadata_name, string_value(child));
    }
    Ok(())
}

fn ring_wkt(ring: &[(f64, f64)]) -> String {
    ring.iter()
        .map(|(lon, lat)| format!("{lon} {lat}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    node.children().filter(Node::is_element).collect()
}

fn expect_child<'a, 'input>(
    children: &[Node<'a, 'input>],
    idx: usize,
    expected: &str,
) -> Result<Node<'a, 'input>, StageError> {
    let node = children.get(idx).copied();
    match node {
        Some(node) if node.has_tag_name(expected) => Ok(node),
        Some(node) => Err(StageError(format!(
            "Expected element '{expected}', found '{}'",
            node.tag_name().name()
        ))),
        None => Err(StageError(format!(
            "Expected element '{expected}', found 'none'"
        ))),
    }
}

fn matches_child(children: &[Node<'_, '_>], idx: usize, expected: &str) -> bool {
    children
        .get(idx)
        .is_some_and(|node| node.has_tag_name(expected))
}

fn assert_element(node: Node<'_, '_>, expected: &str) -> Result<(), StageError> {
    if node.has_tag_name(expected) {
        Ok(())
    } else {
        Err(StageError(format!(
            "Expected element '{expected}', found '{}'",
            node.tag_name().name()
        )))
    }
}

fn assert_no_extra(children: &[Node<'_, '_>], idx: usize) -> Result<(), StageError> {
    if let Some(node) = children.get(idx) {
        Err(StageError(format!(
            "Expected to find no more elements, found '{}'",
            node.tag_name().name()
        )))
    } else {
        Ok(())
    }
}

fn expect_only_child<'a, 'input>(
    node: Node<'a, 'input>,
    expected: &str,
) -> Result<Node<'a, 'input>, StageError> {
    let children = element_children(node);
    let child = expect_child(&children, 0, expected)?;
    assert_no_extra(&children, 1)?;
    Ok(child)
}

fn add_string(metadata: &mut MetadataNode, name: &str, node: Node<'_, '_>) {
    metadata.add_value(name, string_value(node));
}

fn add_i64(metadata: &mut MetadataNode, name: &str, node: Node<'_, '_>) {
    metadata.add_value(name, MetadataValue::I64(text(node).parse().unwrap_or(0)));
}

fn add_f64(metadata: &mut MetadataNode, name: &str, node: Node<'_, '_>) {
    metadata.add_value(name, MetadataValue::F64(text(node).parse().unwrap_or(0.0)));
}

fn string_value(node: Node<'_, '_>) -> MetadataValue {
    MetadataValue::String(text(node).to_string())
}

fn text<'a, 'input>(node: Node<'a, 'input>) -> &'a str {
    node.text().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value<'a>(metadata: &'a MetadataNode, name: &str) -> &'a MetadataValue {
        metadata
            .find_child(name)
            .and_then(MetadataNode::value)
            .unwrap()
    }

    fn children<'a>(metadata: &'a MetadataNode, name: &str) -> Vec<&'a MetadataNode> {
        metadata
            .children()
            .iter()
            .filter(|child| child.name() == name)
            .collect()
    }

    fn node_value(metadata: &MetadataNode) -> &MetadataValue {
        metadata.value().unwrap()
    }

    #[test]
    fn reads_existing_ilvis2_metadata_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ilvis2/ILVIS2_TEST_FILE.TXT.xml");
        let metadata = read_metadata_file(&path).unwrap();

        assert_eq!(
            value(&metadata, "GranuleUR"),
            &MetadataValue::String("SC:ILVIS2.001:51203496".into())
        );
        assert_eq!(value(&metadata, "DbID"), &MetadataValue::I64(51203496));
        let data_files = children(&metadata, "DataFile");
        assert_eq!(data_files.len(), 2);
        assert_eq!(
            value(data_files[1], "ChecksumType"),
            &MetadataValue::String("SHA1".into())
        );
        assert_eq!(children(&metadata, "Campaign").len(), 2);
        let psas = children(&metadata, "PSA");
        assert_eq!(psas.len(), 3);
        assert_eq!(
            value(psas[0], "PSAName"),
            &MetadataValue::String("SIPSMetGenVersion".into())
        );
        assert_eq!(
            value(psas[2], "PSAValue"),
            &MetadataValue::String("N426NA".into())
        );
        assert_eq!(children(&metadata, "BrowseProductGranuleId").len(), 2);
        assert_eq!(
            value(&metadata, "PHProductGranuleId"),
            &MetadataValue::String("PH_ID".into())
        );

        let platform = children(&metadata, "Platform")[0];
        let instrument = children(platform, "Instrument")[0];
        assert_eq!(children(instrument, "OperationMode").len(), 2);
        assert_eq!(
            node_value(children(instrument, "OperationMode")[1]),
            &MetadataValue::String("Safe".into())
        );
        let sensor = children(instrument, "Sensor")[0];
        let sensor_characteristics = children(sensor, "SensorCharacteristic");
        assert_eq!(sensor_characteristics.len(), 2);
        assert_eq!(
            value(sensor_characteristics[0], "CharacteristicName"),
            &MetadataValue::String("CharName1".into())
        );
        assert_eq!(
            value(sensor_characteristics[1], "CharacteristicValue"),
            &MetadataValue::String("MyValue".into())
        );
        assert!(value(&metadata, "ConvexHull")
            .as_string()
            .starts_with("POLYGON"));
    }

    #[test]
    fn missing_expected_element_is_an_error() {
        let err = read_metadata_str("<GranuleMetaDataFile/>").unwrap_err();
        assert!(err.0.contains("Expected element 'DTDVersion'"));
    }
}
