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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn skip_ulem_data_no_magic_does_nothing() {
        let mut buf = Cursor::new(vec![0u8; 8]);
        let pos_before = buf.position();
        skip_ulem_data(&mut buf).unwrap();
        assert_eq!(buf.position(), pos_before);
    }

    #[test]
    fn skip_ulem_data_with_magic() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"ULEM");
        buf.extend_from_slice(&0u32.to_le_bytes()); // 0 frames
        buf.resize(64, 0u8);
        let mut cursor = Cursor::new(buf);
        skip_ulem_data(&mut cursor).unwrap();
    }

    #[test]
    fn skip_polar_data_no_magic_does_nothing() {
        let mut buf = Cursor::new(vec![0u8; 8]);
        let pos_before = buf.position();
        skip_polar_data(&mut buf).unwrap();
        assert_eq!(buf.position(), pos_before);
    }

    #[test]
    fn skip_polar_data_with_magic() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"POL$");
        buf.extend_from_slice(&0i16.to_le_bytes()); // record size
        buf.extend_from_slice(&0u32.to_le_bytes()); // 0 frames
        buf.extend_from_slice(&[0u8; 2]); // padding
        buf.extend_from_slice(&0u32.to_le_bytes()); // 0 xmit
        buf.extend_from_slice(&0u32.to_le_bytes()); // 0 rcv
        buf.resize(64, 0u8);
        let mut cursor = Cursor::new(buf);
        skip_polar_data(&mut cursor).unwrap();
    }

    #[test]
    fn read_bundled_file_metadata_no_magic_stops() {
        let mut buf = Cursor::new(vec![0u8; 8]);
        let mut meta = MetadataNode::new("root");
        read_bundled_file_metadata(&mut buf, &mut meta).unwrap();
        assert!(meta.children().is_empty());
    }

    #[test]
    fn read_bundled_file_metadata_with_file() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"FILE");
        buf.extend_from_slice(&4u32.to_le_bytes()); // len = 4
        let mut name = [0u8; 32];
        name[..3].copy_from_slice(b"abc");
        buf.extend_from_slice(&name);
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(b"XXXX"); // non-FILE -> stop
        let mut cursor = Cursor::new(buf);
        let mut meta = MetadataNode::new("root");
        read_bundled_file_metadata(&mut cursor, &mut meta).unwrap();
        assert!(!meta.children().is_empty());
    }
}
