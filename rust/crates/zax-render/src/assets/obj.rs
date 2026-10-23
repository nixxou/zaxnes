//! A reader for the Wavefront OBJ files of `assets-neutral/meshes`, the mesh half of
//! [`SceneAssets`](super::SceneAssets).
//!
//! Only what those files use is read: `v`, `vt`, `vn`, `f` and `g`. Faces are triangulated as a
//! fan, which is exact for the triangles and quads the exported files contain. A missing normal
//! is replaced by `(0, 0, 0)`, a missing texture coordinate by `(0, 0)`.
//!
//! A `g` line starts a new group, which is how an export writes the submeshes of one model.
//! [`parse`] keeps the first group that has faces and drops the rest (Q-023); [`parse_groups`]
//! returns all of them.

use std::collections::HashMap;

use zax_engine3d::geometry::tile_mesh::{MeshVertex, TileMesh, TriangleCountError};
use zax_engine3d::vectors::{Vec2, Vec3};

/// Most corners one face may have; extra corners are ignored.
const MAX_CORNERS: usize = 16;

/// Reads OBJ text, keeping its first non-empty group (Q-023: one material per model, so one
/// group per model).
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: the fan triangulation emits whole triangles.
pub fn parse(text: &str) -> Result<TileMesh, TriangleCountError> {
    Ok(parse_groups(text)?
        .into_iter()
        .next()
        .map_or_else(TileMesh::empty, |(_, mesh)| mesh))
}

/// Reads OBJ text as one mesh per non-empty group, in file order.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: the fan triangulation emits whole triangles.
pub fn parse_groups(text: &str) -> Result<Vec<(String, TileMesh)>, TriangleCountError> {
    let mut parser = Parser::default();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("v ") {
            parser.positions.push(read_vec3(rest));
        } else if let Some(rest) = line.strip_prefix("vt ") {
            let uv = read_vec3(rest);
            parser.uvs.push(Vec2::new(uv.x, uv.y));
        } else if let Some(rest) = line.strip_prefix("vn ") {
            parser.normals.push(read_vec3(rest));
        } else if line == "g" || line.starts_with("g ") {
            parser.flush()?;
            parser.name = line
                .get(2..)
                .map_or_else(String::new, |n| n.trim().to_owned());
        } else if let Some(rest) = line.strip_prefix("f ") {
            parser.add_face(rest);
        }
    }

    parser.flush()?;
    Ok(parser.groups)
}

#[derive(Default)]
struct Parser {
    positions: Vec<Vec3>,
    uvs: Vec<Vec2>,
    normals: Vec<Vec3>,
    groups: Vec<(String, TileMesh)>,
    name: String,
    vertices: Vec<MeshVertex>,
    triangles: Vec<u32>,
    seen: HashMap<(i64, i64, i64), u32>,
}

impl Parser {
    fn flush(&mut self) -> Result<(), TriangleCountError> {
        let vertices = std::mem::take(&mut self.vertices);
        let triangles = std::mem::take(&mut self.triangles);
        self.seen.clear();
        if !triangles.is_empty() {
            let name = std::mem::take(&mut self.name);
            self.groups
                .push((name, TileMesh::new(vertices, triangles)?));
        }

        Ok(())
    }

    fn add_face(&mut self, line: &str) {
        let mut corners = Vec::with_capacity(MAX_CORNERS);
        for token in line.split([' ', '\t']).filter(|t| !t.is_empty()) {
            if corners.len() == MAX_CORNERS {
                continue;
            }

            let (p, t, n) = self.read_corner(token);
            let Some(&position) = usize::try_from(p).ok().and_then(|p| self.positions.get(p))
            else {
                return;
            };

            let index = if let Some(&index) = self.seen.get(&(p, t, n)) {
                index
            } else {
                let normal = usize::try_from(n)
                    .ok()
                    .and_then(|n| self.normals.get(n))
                    .copied()
                    .unwrap_or(Vec3::ZERO);
                let uv = usize::try_from(t)
                    .ok()
                    .and_then(|t| self.uvs.get(t))
                    .copied()
                    .unwrap_or(Vec2::ZERO);
                #[allow(clippy::cast_possible_truncation)]
                // An asset mesh holds far fewer than 2^32 vertices.
                let index = self.vertices.len() as u32;
                self.vertices.push(MeshVertex::new(position, normal, uv));
                self.seen.insert((p, t, n), index);
                index
            };

            corners.push(index);
        }

        for i in 2..corners.len() {
            self.triangles
                .extend([corners[0], corners[i - 1], corners[i]]);
        }
    }

    /// The position, texture and normal indices of one `p/t/n` corner, zero-based; −1 for a
    /// missing field. Negative OBJ indices count back from the end of their list.
    #[allow(clippy::cast_possible_wrap)] // List lengths are far below i64::MAX.
    fn read_corner(&self, token: &str) -> (i64, i64, i64) {
        let counts = [self.positions.len(), self.uvs.len(), self.normals.len()];
        let mut indices = [-1_i64; 3];
        for (field, value) in token.split('/').take(3).enumerate() {
            if let Ok(number) = value.parse::<i64>() {
                indices[field] = if number > 0 {
                    number - 1
                } else {
                    counts[field] as i64 + number
                };
            }
        }

        (indices[0], indices[1], indices[2])
    }
}

/// Up to three floats separated by blanks; an unreadable one is 0, as in the C# reader.
fn read_vec3(line: &str) -> Vec3 {
    let mut values = [0.0_f32; 3];
    for (slot, token) in values
        .iter_mut()
        .zip(line.split([' ', '\t']).filter(|t| !t.is_empty()))
    {
        // The C# reader also reads an unparsable number as 0.
        *slot = token.parse().unwrap_or(0.0);
    }

    Vec3::new(values[0], values[1], values[2])
}
