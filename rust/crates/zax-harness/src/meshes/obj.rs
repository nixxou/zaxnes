//! Writes the abstract meshes of SPEC-04B REQ-E3D-640 as Wavefront OBJ, for review and for the
//! round-1 verification (REQ-ARCH-031). Port of `ObjWriter.cs`.
//!
//! The scene frame of REQ-E3D-400 is left handed. In right-handed mode the writer negates z and
//! swaps two indices of every triangle, which is the conversion the note of REQ-E3D-400
//! describes; the file then opens the right way round in any viewer.

use std::fmt::{self, Write as _};

use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::instance::InstanceRef;
use zax_engine3d::vectors::{Mat4, Vec2, Vec3};

use super::decimals;

/// The tiles to draw of one frame, each placed in the scene by its own transform
/// (REQ-E3D-406, REQ-E3D-641). The sprite depth nudge is not applied: that is the renderer's
/// (REQ-SCN-037, DEC-R2-T4).
///
/// # Errors
/// [`fmt::Error`] when the text cannot be formatted.
pub fn scene(list: &FrameDrawList, right_handed: bool) -> Result<String, fmt::Error> {
    let mut text = String::new();
    header(&mut text, right_handed);
    line(
        &mut text,
        format_args!(
            "# frame {}, time {:.4} s",
            list.frame_counter(),
            list.time()
        ),
    )?;
    line(
        &mut text,
        format_args!("# {} tiles to draw", list.tiles().len()),
    )?;

    let mut base = 1;
    for (n, tile) in list.tiles().iter().enumerate() {
        let Some(mesh) = tile.mesh.as_ref().filter(|mesh| !mesh.is_empty()) else {
            continue;
        };

        let kind = if tile.sprite { "sprite" } else { "bg" };
        line(
            &mut text,
            format_args!("g tile{n:05}_{:?}_{kind}", tile.material),
        )?;
        for vertex in mesh.vertices() {
            let position = tile.transform.transform_point(vertex.position);
            vertex_line(&mut text, "v", position, right_handed)?;
            vertex_line(
                &mut text,
                "vn",
                direction(&tile.transform, vertex.normal),
                right_handed,
            )?;
            texture_line(&mut text, vertex.uv, tile.texture.as_deref())?;
        }

        faces(&mut text, mesh, base, right_handed)?;
        base += mesh.vertex_count();
    }

    Ok(text)
}

/// Every 3D object of the registry, its tile meshes in their local frame (REQ-E3D-504,
/// REQ-E3D-522), one group per tile image.
///
/// # Errors
/// [`fmt::Error`] when the text cannot be formatted.
pub fn objects(engine: &ConversionEngine, right_handed: bool) -> Result<String, fmt::Error> {
    let mut text = String::new();
    header(&mut text, right_handed);
    let Some(builder) = engine.geometry_builder() else {
        return Ok(text);
    };

    let mut base = 1;
    for (number, &id) in engine.patterns().permanent().iter().enumerate() {
        let Some(pattern) = engine.patterns().get(id) else {
            continue;
        };

        for (index, instance) in pattern.instances().iter().enumerate() {
            let Some(built) = builder.of(InstanceRef { pattern: id, index }) else {
                continue;
            };

            for (image, mesh) in built.meshes().iter().enumerate() {
                if mesh.is_empty() {
                    continue;
                }

                line(
                    &mut text,
                    format_args!("g p{number:05}_o{index}_t{image}_{:?}", instance.geometry),
                )?;
                for vertex in mesh.vertices() {
                    vertex_line(&mut text, "v", vertex.position, right_handed)?;
                    vertex_line(&mut text, "vn", vertex.normal, right_handed)?;
                    texture_line(&mut text, vertex.uv, Some(built.texture()))?;
                }

                faces(&mut text, mesh, base, right_handed)?;
                base += mesh.vertex_count();
            }
        }
    }

    Ok(text)
}

fn line(text: &mut String, arguments: fmt::Arguments<'_>) -> fmt::Result {
    writeln!(text, "{arguments}")
}

fn header(text: &mut String, right_handed: bool) {
    text.push_str("# Zaxnes harness mesh export (SPEC-04B REQ-E3D-640)\n");
    text.push_str(if right_handed {
        "# right-handed: z negated and winding swapped from the scene frame\n"
    } else {
        "# left-handed scene frame of REQ-E3D-400, written as is\n"
    });
    text.push_str("# one scene unit = one NES pixel\n");
}

fn vertex_line(text: &mut String, tag: &str, value: Vec3, right_handed: bool) -> fmt::Result {
    let z = if right_handed { -value.z } else { value.z };
    line(
        text,
        format_args!(
            "{tag} {} {} {}",
            decimals(value.x, 4),
            decimals(value.y, 4),
            decimals(z, 4)
        ),
    )
}

/// Texture coordinates, normalised into `[0, 1]` by the texture size so that a viewer can show
/// the texture; the engine's own unit is the texel (REQ-E3D-506).
#[allow(clippy::cast_precision_loss)] // Texture sizes are at most 512 texels.
fn texture_line(text: &mut String, uv: Vec2, texture: Option<&ObjectTexture>) -> fmt::Result {
    let (width, height) = texture.map_or((1.0, 1.0), |texture| {
        (texture.width() as f32, texture.height() as f32)
    });
    line(
        text,
        format_args!(
            "vt {} {}",
            decimals(uv.x / width, 6),
            decimals(1.0 - (uv.y / height), 6)
        ),
    )
}

fn faces(text: &mut String, mesh: &TileMesh, base: usize, right_handed: bool) -> fmt::Result {
    for triangle in mesh.triangles().chunks_exact(3) {
        let [a, mut b, mut c] =
            [triangle[0], triangle[1], triangle[2]].map(|index| base + index as usize);
        if right_handed {
            std::mem::swap(&mut b, &mut c);
        }

        line(text, format_args!("f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}"))?;
    }

    Ok(())
}

/// Transforms a direction: the transform without its translation.
fn direction(transform: &Mat4, direction: Vec3) -> Vec3 {
    let origin = transform.transform_point(Vec3::ZERO);
    let moved = transform.transform_point(direction) - origin;
    let length = moved.squared_length().sqrt();
    if length > 0.0 {
        moved * (1.0 / length)
    } else {
        direction
    }
}
