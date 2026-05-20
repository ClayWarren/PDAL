use super::bpf_base64::encode_base64;
use super::{fixed_label, io_error, BpfHeader};
use byteorder::{LittleEndian, ReadBytesExt};
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::stage::StageError;
use std::io::{Read, Seek, SeekFrom};

pub(super) fn reader_metadata<R: Read + Seek>(
    reader: &mut R,
    header: &BpfHeader,
) -> Result<MetadataNode, StageError> {
    let mut metadata = MetadataNode::new("readers.bpf");
    skip_ulem_data(reader)?;
    read_bundled_file_metadata(reader, &mut metadata)?;
    skip_polar_data(reader)?;

    let pos = reader.stream_position().map_err(io_error)?;
    if pos > header.len as u64 {
        return Err(StageError(
            "BPF Header length exceeded that reported by file.".to_string(),
        ));
    }
    if pos < header.len as u64 {
        let mut bytes = vec![0; header.len as usize - pos as usize];
        reader.read_exact(&mut bytes).map_err(io_error)?;
        let mut child = MetadataNode::new("header_data");
        child.set_value(MetadataValue::String(encode_base64(&bytes)));
        child.set_type_name("base64Binary");
        metadata.add_child(child);
    }
    metadata.add_value("count", MetadataValue::U64(header.num_pts as u64));
    Ok(metadata)
}

fn skip_ulem_data<R: Read + Seek>(reader: &mut R) -> Result<(), StageError> {
    let start = reader.stream_position().map_err(io_error)?;
    let mut magic = [0u8; 4];
    if reader.read_exact(&mut magic).is_err() {
        reader.seek(SeekFrom::Start(start)).map_err(io_error)?;
        return Ok(());
    }
    if &magic != b"ULEM" {
        reader.seek(SeekFrom::Start(start)).map_err(io_error)?;
        return Ok(());
    }

    let frames = reader.read_u32::<LittleEndian>().map_err(io_error)? as u64;
    reader.seek(SeekFrom::Current(54)).map_err(io_error)?;
    reader
        .seek(SeekFrom::Current((frames * 160) as i64))
        .map_err(io_error)?;
    Ok(())
}

fn read_bundled_file_metadata<R: Read + Seek>(
    reader: &mut R,
    metadata: &mut MetadataNode,
) -> Result<(), StageError> {
    loop {
        let start = reader.stream_position().map_err(io_error)?;
        let mut magic = [0u8; 4];
        if reader.read_exact(&mut magic).is_err() {
            reader.seek(SeekFrom::Start(start)).map_err(io_error)?;
            return Ok(());
        }
        if &magic != b"FILE" {
            reader.seek(SeekFrom::Start(start)).map_err(io_error)?;
            return Ok(());
        }

        let len = reader.read_u32::<LittleEndian>().map_err(io_error)? as usize;
        let mut name = [0u8; 32];
        reader.read_exact(&mut name).map_err(io_error)?;
        let mut data = vec![0; len];
        reader.read_exact(&mut data).map_err(io_error)?;

        let filename = fixed_label(&name);
        let mut bundle = MetadataNode::new("bundled_file");
        let mut child = MetadataNode::new(filename);
        child.set_value(MetadataValue::String(encode_base64(&data)));
        child.set_type_name("base64Binary");
        bundle.add_child(child);
        metadata.add_child(bundle);
    }
}

fn skip_polar_data<R: Read + Seek>(reader: &mut R) -> Result<(), StageError> {
    let start = reader.stream_position().map_err(io_error)?;
    let mut magic = [0u8; 4];
    if reader.read_exact(&mut magic).is_err() {
        reader.seek(SeekFrom::Start(start)).map_err(io_error)?;
        return Ok(());
    }
    if &magic != b"POL$" {
        reader.seek(SeekFrom::Start(start)).map_err(io_error)?;
        return Ok(());
    }

    let _record_size = reader.read_i16::<LittleEndian>().map_err(io_error)?;
    let frames = reader.read_u32::<LittleEndian>().map_err(io_error)? as u64;
    reader.seek(SeekFrom::Current(2)).map_err(io_error)?;
    let xmit = reader.read_u32::<LittleEndian>().map_err(io_error)? as u64;
    let rcv = reader.read_u32::<LittleEndian>().map_err(io_error)? as u64;
    let header_bytes = xmit * 16 + rcv * 128;
    let frame_bytes = frames * 168;
    reader
        .seek(SeekFrom::Current((header_bytes + frame_bytes) as i64))
        .map_err(io_error)?;
    Ok(())
}
