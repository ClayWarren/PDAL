//! `readers.obj` -- Wavefront OBJ format (ASCII).
//!
//! Port of `io/ObjReader.cpp`. OBJ is a text format defining vertices, texture
//! coordinates, normals, and faces. The reader emits points for vertices that
//! are used in faces. Multiple faces sharing a vertex/normal/texture triple
//! (Vtn) refer to the same point ID.
//!
//! The reader stores triangulated OBJ faces in the view's mesh, matching the
//! shape existing C++ tests assert.

use crate::source;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Xyzw {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

/// A Vertex-Texture-Normal triple index (1-based in OBJ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Vtn {
    v: i64,
    t: i64,
    n: i64,
}

pub struct ObjReader {
    filename: String,
}

impl ObjReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Reader for ObjReader {
    fn name(&self) -> &str {
        "readers.obj"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "ObjReader requires a filename option.".to_string(),
            ));
        }
        let text = source::read_to_string(&self.filename)
            .map_err(|_| StageError(format!("Couldn't open '{}'.", self.filename)))?;

        let mut vertices: Vec<Xyzw> = Vec::new();
        let mut textures: Vec<Xyzw> = Vec::new();
        let mut normals: Vec<Xyzw> = Vec::new();
        let mut faces: Vec<Vec<Vtn>> = Vec::new();

        for (idx, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let words: Vec<&str> = line.split_whitespace().collect();
            let key = words[0];
            match key {
                "v" => {
                    let v = parse_xyzw(&words, idx + 1, 1.0)?;
                    vertices.push(v);
                }
                "vt" => {
                    let t = parse_xyzw(&words, idx + 1, 0.0)?;
                    textures.push(t);
                }
                "vn" => {
                    let n = parse_xyzw(&words, idx + 1, 0.0)?;
                    normals.push(n);
                }
                "f" => {
                    let mut face = Vec::new();
                    for &word in &words[1..] {
                        face.push(parse_vtn(word, idx + 1)?);
                    }
                    if face.len() < 3 {
                        return Err(StageError(format!(
                            "Not enough vertices in face specification on line {}.",
                            idx + 1
                        )));
                    }
                    faces.push(face);
                }
                _ => {} // Ignore other tags like g, use, mtllib, etc.
            }
        }

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::W, DimType::F64);
        layout.register(DimId::TextureU, DimType::F64);
        layout.register(DimId::TextureV, DimType::F64);
        layout.register(DimId::TextureW, DimType::F64);
        layout.register(DimId::NormalX, DimType::F64);
        layout.register(DimId::NormalY, DimType::F64);
        layout.register(DimId::NormalZ, DimType::F64);

        let mut view = PointView::new(Rc::new(layout));
        let mut points: HashMap<Vtn, u64> = HashMap::new();

        for face in faces {
            // OBJ faces can be polygons; PDAL triangulates them.
            // Triangulation: (v0, v1, v2), (v0, v2, v3), ...
            for i in 1..(face.len() - 1) {
                let tri = [face[0], face[i], face[i + 1]];
                let mut triangle = [0; 3];
                for (next, vtn) in tri.into_iter().enumerate() {
                    let point_id = match points.get(&vtn) {
                        Some(id) => *id,
                        None => {
                            let id = add_point(&mut view, vtn, &vertices, &textures, &normals)?;
                            points.insert(vtn, id);
                            id
                        }
                    };
                    triangle[next] = point_id;
                }
                view.create_mesh()
                    .add(triangle[0], triangle[1], triangle[2]);
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.obj")
    }
}

fn parse_xyzw(words: &[&str], line: usize, default_w: f64) -> Result<Xyzw, StageError> {
    if words.len() < 2 {
        return Err(StageError(format!("Invalid coordinates on line {}.", line)));
    }
    let x = words[1]
        .parse()
        .map_err(|_| StageError(format!("Invalid X on line {}.", line)))?;
    let y = words
        .get(2)
        .map(|w| w.parse())
        .transpose()
        .map_err(|_| StageError(format!("Invalid Y on line {}.", line)))?
        .unwrap_or(0.0);
    let z = words
        .get(3)
        .map(|w| w.parse())
        .transpose()
        .map_err(|_| StageError(format!("Invalid Z on line {}.", line)))?
        .unwrap_or(0.0);
    let w = words
        .get(4)
        .map(|w| w.parse())
        .transpose()
        .map_err(|_| StageError(format!("Invalid W on line {}.", line)))?
        .unwrap_or(default_w);
    Ok(Xyzw { x, y, z, w })
}

fn parse_vtn(word: &str, line: usize) -> Result<Vtn, StageError> {
    let parts: Vec<&str> = word.split('/').collect();
    let v = parts[0]
        .parse()
        .map_err(|_| StageError(format!("Invalid vertex index on line {}.", line)))?;
    let t = if parts.len() > 1 && !parts[1].is_empty() {
        parts[1]
            .parse()
            .map_err(|_| StageError(format!("Invalid texture index on line {}.", line)))?
    } else {
        0
    };
    let n = if parts.len() > 2 && !parts[2].is_empty() {
        parts[2]
            .parse()
            .map_err(|_| StageError(format!("Invalid normal index on line {}.", line)))?
    } else {
        0
    };
    Ok(Vtn { v, t, n })
}

fn add_point(
    view: &mut PointView,
    vtn: Vtn,
    vertices: &[Xyzw],
    textures: &[Xyzw],
    normals: &[Xyzw],
) -> Result<u64, StageError> {
    let id = view.add_point();

    // OBJ indices are 1-based. Negative indices are relative to the end.
    let v_idx = resolve_index(vtn.v, vertices.len())?;
    let v = vertices[v_idx];
    view.set_f64(id, &DimId::X, v.x);
    view.set_f64(id, &DimId::Y, v.y);
    view.set_f64(id, &DimId::Z, v.z);
    view.set_f64(id, &DimId::W, v.w);

    if vtn.t != 0 {
        let t_idx = resolve_index(vtn.t, textures.len())?;
        let t = textures[t_idx];
        view.set_f64(id, &DimId::TextureU, t.x);
        view.set_f64(id, &DimId::TextureV, t.y);
        view.set_f64(id, &DimId::TextureW, t.z);
    }

    if vtn.n != 0 {
        let n_idx = resolve_index(vtn.n, normals.len())?;
        let n = normals[n_idx];
        view.set_f64(id, &DimId::NormalX, n.x);
        view.set_f64(id, &DimId::NormalY, n.y);
        view.set_f64(id, &DimId::NormalZ, n.z);
    }

    Ok(id)
}

fn resolve_index(idx: i64, len: usize) -> Result<usize, StageError> {
    if idx == 0 {
        return Err(StageError("OBJ index cannot be 0.".to_string()));
    }
    let resolved = if idx > 0 {
        (idx - 1) as usize
    } else {
        (len as i64 + idx) as usize
    };
    if resolved >= len {
        return Err(StageError(format!(
            "OBJ index {} out of range (len {}).",
            idx, len
        )));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::DimId;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn reads_simple_vertices_from_faces() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v -1 0 0").unwrap();
        writeln!(file, "v 0 1 0").unwrap();
        writeln!(file, "v 1 0 0").unwrap();
        writeln!(file, "f 1 2 3").unwrap();

        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        let view = &views[0];
        assert_eq!(view.len(), 3);

        assert_eq!(view.get_f64(0, &DimId::X), -1.0);
        assert_eq!(view.get_f64(1, &DimId::X), 0.0);
        assert_eq!(view.get_f64(2, &DimId::X), 1.0);
        let mesh = view.mesh().unwrap();
        assert_eq!(mesh.len(), 1);
        assert_eq!(mesh.triangles()[0].a, 0);
        assert_eq!(mesh.triangles()[0].b, 1);
        assert_eq!(mesh.triangles()[0].c, 2);
    }

    #[test]
    fn de_duplicates_shared_vertices() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 0 0 0").unwrap();
        writeln!(file, "v 1 0 0").unwrap();
        writeln!(file, "v 0 1 0").unwrap();
        writeln!(file, "v 1 1 0").unwrap();
        writeln!(file, "f 1 2 3").unwrap();
        writeln!(file, "f 2 4 3").unwrap(); // Shares 2 and 3

        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        let view = &reader.read().unwrap()[0];
        // 1, 2, 3 from first face; 4 from second face (2 and 3 already exist).
        assert_eq!(view.len(), 4);
        assert_eq!(view.mesh().unwrap().len(), 2);
    }

    #[test]
    fn triangulates_polygon_faces_into_mesh() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 0 0 0").unwrap();
        writeln!(file, "v 1 0 0").unwrap();
        writeln!(file, "v 1 1 0").unwrap();
        writeln!(file, "v 0 1 0").unwrap();
        writeln!(file, "f 1 2 3 4").unwrap();

        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        let view = &reader.read().unwrap()[0];

        let mesh = view.mesh().unwrap();
        assert_eq!(mesh.len(), 2);
        assert_eq!(mesh.triangles()[0].a, 0);
        assert_eq!(mesh.triangles()[0].b, 1);
        assert_eq!(mesh.triangles()[0].c, 2);
        assert_eq!(mesh.triangles()[1].a, 0);
        assert_eq!(mesh.triangles()[1].b, 2);
        assert_eq!(mesh.triangles()[1].c, 3);
    }

    #[test]
    fn handles_normals_and_textures() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 0 0 0").unwrap();
        writeln!(file, "vt 0.5 0.5").unwrap();
        writeln!(file, "vn 0 0 1").unwrap();
        writeln!(file, "f 1/1/1 1/1/1 1/1/1").unwrap(); // Degenerate but valid for test

        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        let view = &reader.read().unwrap()[0];
        assert_eq!(view.len(), 1);
        assert_eq!(view.mesh().unwrap().len(), 1);
        assert_eq!(view.get_f64(0, &DimId::TextureU), 0.5);
        assert_eq!(view.get_f64(0, &DimId::NormalZ), 1.0);
    }

    #[test]
    fn handles_relative_indices() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 10 20 30").unwrap();
        writeln!(file, "v 40 50 60").unwrap();
        writeln!(file, "f -2 -2 -2").unwrap(); // Points to first vertex

        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        let view = &reader.read().unwrap()[0];
        assert_eq!(view.len(), 1);
        assert_eq!(view.get_f64(0, &DimId::X), 10.0);
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = ObjReader::new(&Options::new());
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/file.obj");
        let mut reader = ObjReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_invalid_coordinate() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v notanumber 1 2").unwrap();
        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_face_with_too_few_vertices() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 0 0 0").unwrap();
        writeln!(file, "v 1 0 0").unwrap();
        writeln!(file, "f 1 2").unwrap();
        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_face_index_out_of_range() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 0 0 0").unwrap();
        writeln!(file, "v 1 0 0").unwrap();
        writeln!(file, "v 0 1 0").unwrap();
        writeln!(file, "f 1 2 99").unwrap();
        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_zero_index() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 0 0 0").unwrap();
        writeln!(file, "v 1 0 0").unwrap();
        writeln!(file, "v 0 1 0").unwrap();
        writeln!(file, "f 0 1 2").unwrap();
        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_ignores_comments_and_unknown_tags() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# A comment").unwrap();
        writeln!(file, "g group_name").unwrap();
        writeln!(file, "mtllib unused.mtl").unwrap();
        writeln!(file, "v 0 0 0").unwrap();
        writeln!(file, "v 1 0 0").unwrap();
        writeln!(file, "v 0 1 0").unwrap();
        writeln!(file, "f 1 2 3").unwrap();
        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        let view = &reader.read().unwrap()[0];
        assert_eq!(view.len(), 3);
    }

    #[test]
    fn reader_handles_vertex_with_w_component() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "v 1 2 3 4").unwrap();
        writeln!(file, "v 5 6 7").unwrap();
        writeln!(file, "v 8 9 0").unwrap();
        writeln!(file, "f 1 2 3").unwrap();
        let mut options = Options::new();
        options.add("filename", file.path().to_str().unwrap());
        let mut reader = ObjReader::new(&options);
        let view = &reader.read().unwrap()[0];
        assert_eq!(view.get_f64(0, &DimId::W), 4.0);
    }

    #[test]
    fn reader_name_is_readers_obj() {
        let r = ObjReader::new(&Options::new());
        assert_eq!(r.name(), "readers.obj");
    }
}
