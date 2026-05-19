//! `writers.gltf` -- binary glTF (`.glb`) writer for mesh-backed views.
//!
//! This is the deterministic local GLB slice used by PDAL's existing unit
//! tests. It writes embedded JSON plus binary index/vertex buffers for views
//! that already carry a triangular mesh.

use byteorder::{LittleEndian, WriteBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;
use serde_json::json;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

const HEADER_SIZE: u64 = 12;
const JSON_CHUNK_DATA_SIZE: usize = 5000;
const CHUNK_HEADER_SIZE: u64 = 8;
const JSON_CHUNK_TYPE: u32 = 0x4E4F534A;
const BIN_CHUNK_TYPE: u32 = 0x004E4942;

pub struct GltfWriter {
    filename: String,
    metallic: f64,
    roughness: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    double_sided: bool,
    colors: bool,
    normals: bool,
}

#[derive(Clone, Debug)]
struct ViewData {
    index_offset: usize,
    index_byte_length: usize,
    index_count: usize,
    vertex_offset: usize,
    vertex_byte_length: usize,
    vertex_count: usize,
    bounds: Bounds,
}

#[derive(Clone, Debug)]
struct Bounds {
    min: [f32; 3],
    max: [f32; 3],
}

impl GltfWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            metallic: options.get_f64("metallic", 0.0),
            roughness: options.get_f64("roughness", 0.0),
            red: options.get_f64("red", 0.0),
            green: options.get_f64("green", 0.0),
            blue: options.get_f64("blue", 0.0),
            alpha: options.get_f64("alpha", 1.0),
            double_sided: options.get_bool("double_sided", false),
            colors: options.get_bool("colors", false),
            normals: options.get_bool("normals", false),
        }
    }
}

impl Writer for GltfWriter {
    fn name(&self) -> &str {
        "writers.gltf"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "GltfWriter requires a filename option.".to_string(),
            ));
        }
        let file = File::create(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}' for writing.", self.filename)))?;
        let mut writer = BufWriter::new(file);
        writer
            .seek(SeekFrom::Start(
                HEADER_SIZE + JSON_CHUNK_DATA_SIZE as u64 + (2 * CHUNK_HEADER_SIZE),
            ))
            .map_err(io_error)?;

        let mut view_data = Vec::new();
        let mut bin_size = 0usize;
        let write_normals = self.normals
            && mesh_views_have_dims(views, &[DimId::NormalX, DimId::NormalY, DimId::NormalZ]);
        let write_colors =
            self.colors && mesh_views_have_dims(views, &[DimId::Red, DimId::Green, DimId::Blue]);

        for view in views {
            let Some(mesh) = view.mesh() else {
                continue;
            };
            if mesh.is_empty() {
                continue;
            }
            let index_count = mesh.len() * 3;
            let index_byte_length = index_count * std::mem::size_of::<u32>();
            let mut vertex_byte_length = view.len() as usize * std::mem::size_of::<f32>() * 3;
            if write_normals {
                vertex_byte_length += view.len() as usize * std::mem::size_of::<f32>() * 3;
            }
            if write_colors {
                vertex_byte_length += view.len() as usize * std::mem::size_of::<f32>() * 3;
            }
            let index_offset = bin_size;
            let vertex_offset = index_offset + index_byte_length;
            bin_size += index_byte_length + vertex_byte_length;

            for triangle in mesh.triangles() {
                writer
                    .write_u32::<LittleEndian>(triangle.a as u32)
                    .map_err(io_error)?;
                writer
                    .write_u32::<LittleEndian>(triangle.b as u32)
                    .map_err(io_error)?;
                writer
                    .write_u32::<LittleEndian>(triangle.c as u32)
                    .map_err(io_error)?;
            }

            let mut bounds = Bounds::new();
            for idx in 0..view.len() {
                let xyz = [
                    view.get_f64(idx, &DimId::X) as f32,
                    view.get_f64(idx, &DimId::Y) as f32,
                    view.get_f64(idx, &DimId::Z) as f32,
                ];
                bounds.grow(xyz);
                for value in xyz {
                    writer.write_f32::<LittleEndian>(value).map_err(io_error)?;
                }
                if write_normals {
                    for dim in [DimId::NormalX, DimId::NormalY, DimId::NormalZ] {
                        writer
                            .write_f32::<LittleEndian>(view.get_f64(idx, &dim) as f32)
                            .map_err(io_error)?;
                    }
                }
                if write_colors {
                    for dim in [DimId::Red, DimId::Green, DimId::Blue] {
                        let value = view.get_f64(idx, &dim) / f64::from(u16::MAX);
                        writer
                            .write_f32::<LittleEndian>(value as f32)
                            .map_err(io_error)?;
                    }
                }
            }

            view_data.push(ViewData {
                index_offset,
                index_byte_length,
                index_count,
                vertex_offset,
                vertex_byte_length,
                vertex_count: view.len() as usize,
                bounds,
            });
        }

        let total_size = HEADER_SIZE as usize
            + JSON_CHUNK_DATA_SIZE
            + (2 * CHUNK_HEADER_SIZE as usize)
            + bin_size;
        writer.seek(SeekFrom::Start(0)).map_err(io_error)?;
        write_glb_header(&mut writer, total_size)?;
        write_json_chunk(
            &mut writer,
            &view_data,
            bin_size,
            write_normals,
            write_colors,
            self,
        )?;
        write_bin_header(&mut writer, bin_size)?;
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.gltf")
    }
}

fn write_glb_header<W: Write>(writer: &mut W, total_size: usize) -> Result<(), StageError> {
    writer.write_all(b"glTF").map_err(io_error)?;
    writer.write_u32::<LittleEndian>(2).map_err(io_error)?;
    writer
        .write_u32::<LittleEndian>(total_size as u32)
        .map_err(io_error)
}

fn write_json_chunk<W: Write>(
    writer: &mut W,
    view_data: &[ViewData],
    bin_size: usize,
    write_normals: bool,
    write_colors: bool,
    options: &GltfWriter,
) -> Result<(), StageError> {
    let json = build_json(view_data, bin_size, write_normals, write_colors, options);
    let mut bytes = serde_json::to_vec(&json).map_err(|err| StageError(err.to_string()))?;
    if bytes.len() > JSON_CHUNK_DATA_SIZE {
        return Err(StageError(
            "GLB JSON chunk exceeded reserved size.".to_string(),
        ));
    }
    bytes.resize(JSON_CHUNK_DATA_SIZE, b' ');
    writer
        .write_u32::<LittleEndian>(JSON_CHUNK_DATA_SIZE as u32)
        .map_err(io_error)?;
    writer
        .write_u32::<LittleEndian>(JSON_CHUNK_TYPE)
        .map_err(io_error)?;
    writer.write_all(&bytes).map_err(io_error)
}

fn write_bin_header<W: Write>(writer: &mut W, bin_size: usize) -> Result<(), StageError> {
    writer
        .write_u32::<LittleEndian>(bin_size as u32)
        .map_err(io_error)?;
    writer
        .write_u32::<LittleEndian>(BIN_CHUNK_TYPE)
        .map_err(io_error)
}

fn build_json(
    view_data: &[ViewData],
    bin_size: usize,
    write_normals: bool,
    write_colors: bool,
    options: &GltfWriter,
) -> serde_json::Value {
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    let mut next_accessor = 0usize;
    let mut next_buffer_view = 0usize;
    let element_size = 12 + usize::from(write_normals) * 12 + usize::from(write_colors) * 12;

    for data in view_data {
        let face_view = next_buffer_view;
        next_buffer_view += 1;
        buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": data.index_offset,
            "byteLength": data.index_byte_length,
            "target": 34963
        }));
        let vertex_view = next_buffer_view;
        next_buffer_view += 1;
        buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": data.vertex_offset,
            "byteLength": data.vertex_byte_length,
            "byteStride": element_size,
            "target": 34962
        }));

        let index_accessor = next_accessor;
        next_accessor += 1;
        accessors.push(json!({
            "bufferView": face_view,
            "byteOffset": 0,
            "componentType": 5125,
            "count": data.index_count,
            "type": "SCALAR"
        }));
        let position_accessor = next_accessor;
        next_accessor += 1;
        accessors.push(json!({
            "bufferView": vertex_view,
            "byteOffset": 0,
            "componentType": 5126,
            "count": data.vertex_count,
            "type": "VEC3",
            "min": data.bounds.min,
            "max": data.bounds.max
        }));
        let mut attrs = json!({ "POSITION": position_accessor });
        if write_normals {
            attrs["NORMAL"] = json!(next_accessor);
            accessors.push(json!({
                "bufferView": vertex_view,
                "byteOffset": 12,
                "componentType": 5126,
                "count": data.vertex_count,
                "type": "VEC3"
            }));
            next_accessor += 1;
        }
        if write_colors {
            attrs["COLOR_0"] = json!(next_accessor);
            let color_offset = 12 + usize::from(write_normals) * 12;
            accessors.push(json!({
                "bufferView": vertex_view,
                "byteOffset": color_offset,
                "componentType": 5126,
                "count": data.vertex_count,
                "type": "VEC3"
            }));
            next_accessor += 1;
        }
        meshes.push(json!({
            "primitives": [{
                "attributes": attrs,
                "indices": index_accessor,
                "material": 0
            }]
        }));
    }

    json!({
        "asset": {"version": "2.0"},
        "buffers": [{"byteLength": bin_size}],
        "bufferViews": buffer_views,
        "accessors": accessors,
        "materials": [{
            "doubleSided": options.double_sided,
            "pbrMetallicRoughness": {
                "baseColorFactor": [options.red, options.green, options.blue, options.alpha],
                "metallicFactor": options.metallic,
                "roughnessFactor": options.roughness
            }
        }],
        "meshes": meshes,
        "nodes": (0..meshes.len()).map(|idx| json!({"mesh": idx})).collect::<Vec<_>>(),
        "scenes": [{"nodes": (0..meshes.len()).collect::<Vec<_>>()}],
        "scene": 0
    })
}

fn mesh_views_have_dims(views: &[PointView], dims: &[DimId]) -> bool {
    views
        .iter()
        .filter(|view| view.mesh().is_some_and(|mesh| !mesh.is_empty()))
        .all(|view| dims.iter().all(|dim| view.layout().dim(dim).is_some()))
}

impl Bounds {
    fn new() -> Self {
        Self {
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
        }
    }

    fn grow(&mut self, xyz: [f32; 3]) {
        for (idx, value) in xyz.into_iter().enumerate() {
            self.min[idx] = self.min[idx].min(value);
            self.max[idx] = self.max[idx].max(value);
        }
    }
}

fn io_error(error: std::io::Error) -> StageError {
    StageError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("pdal-rust-gltf-{}-{name}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    fn test_view(write_normals: bool, write_colors: bool) -> PointView {
        let mut layout = PointLayout::new();
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            layout.register(dim, DimType::F64);
        }
        if write_normals {
            for dim in [DimId::NormalX, DimId::NormalY, DimId::NormalZ] {
                layout.register(dim, DimType::F64);
            }
        }
        if write_colors {
            for dim in [DimId::Red, DimId::Blue, DimId::Green] {
                layout.register(dim, DimType::F64);
            }
        }
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in [
            (1.0, 1.0, 0.0),
            (2.0, 1.0, 0.0),
            (1.0, 2.0, 0.0),
            (2.0, 2.0, 2.0),
        ] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, x);
            view.set_f64(point, &DimId::Y, y);
            view.set_f64(point, &DimId::Z, z);
        }
        if write_normals {
            for idx in 0..4 {
                view.set_f64(idx, &DimId::NormalX, if idx == 3 { 1.0 } else { 0.0 });
                view.set_f64(idx, &DimId::NormalY, if idx == 3 { 0.0 } else { 1.0 });
                view.set_f64(idx, &DimId::NormalZ, 0.0);
            }
        }
        if write_colors {
            for idx in 0..4 {
                view.set_f64(
                    idx,
                    &DimId::Red,
                    if idx == 1 || idx == 2 {
                        0.0
                    } else {
                        255.0 * 256.0
                    },
                );
                view.set_f64(
                    idx,
                    &DimId::Green,
                    if idx == 2 { 255.0 * 256.0 } else { 0.0 },
                );
                view.set_f64(
                    idx,
                    &DimId::Blue,
                    if idx == 1 { 255.0 * 256.0 } else { 0.0 },
                );
            }
        }
        let mesh = view.create_mesh();
        mesh.add(0, 1, 2);
        mesh.add(3, 2, 1);
        view
    }

    fn write_size(write_normals: bool, write_colors: bool, expected: u64) {
        let path = temp_path(&format!("{write_normals}-{write_colors}.glb"));
        let mut options = Options::new();
        options
            .add("filename", &path)
            .add("normals", write_normals)
            .add("colors", write_colors);
        let mut writer = GltfWriter::new(&options);
        writer
            .write(&[test_view(write_normals, write_colors)])
            .unwrap();

        assert_eq!(std::fs::metadata(&path).unwrap().len(), expected);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn writes_size_matching_existing_basic_case() {
        write_size(false, false, 5100);
    }

    #[test]
    fn writes_size_matching_existing_normals_case() {
        write_size(true, false, 5148);
    }

    #[test]
    fn writes_size_matching_existing_colors_case() {
        write_size(false, true, 5148);
    }

    #[test]
    fn writes_size_matching_existing_normals_and_colors_case() {
        write_size(true, true, 5196);
    }
}
