//! The verification harness the acceptance criteria of AMD-043 REQ-SCN-068 and AMD-046
//! REQ-SCN-074 ask for: the scene-space vertices the renderer would place for a frame, computed
//! on the CPU and without a GL context.
//!
//! It walks the same chain the vertex shader applies, in the same order: the placement the frame
//! plan built (the animated deformation of REQ-E3D-568 in the object's local frame, the object's
//! own transform, the sprite nudge of REQ-SCN-037 and the HUD plane of REQ-SCN-045 all
//! included), then the curvature of REQ-SCN-067 on the final scene position — on everything but
//! the HUD, which REQ-SCN-068 keeps flat.
//!
//! It is a verification aid, **not** a rendering path: nothing in the renderer calls it, and the
//! export of a whole frame as a file is out of scope (AMD-040 REQ-LUA-115 makes
//! `BuildExportMeshes` a no-op).

use std::sync::Arc;

use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::{Mat4, Vec3};

use crate::camera::CameraView;
use crate::frame_plan::{FixedMeshes, FramePlan, Residency};
use crate::scene::{SceneFrame, SceneState};

/// One object of an exported frame: where its vertices land in the scene, before and after the
/// curvature of REQ-SCN-068.
#[derive(Debug, Clone, PartialEq)]
// One bool per independent clause of the export — which bucket the object is in, whether the
// curvature reaches it, whether the clipping judges it, whether its instance is a floor. The same
// `allow` `SceneVariant` and `ProgramKey` carry, for the same reason: an enum per clause would say
// less, not more.
#[allow(clippy::struct_excessive_bools)]
pub struct ExportedObject {
    /// Whether the object belongs to the HUD plane of REQ-SCN-045, which stays flat.
    pub hud: bool,
    /// Whether the curvature reaches the object at all (REQ-SCN-068, REQ-SCN-074).
    pub bent: bool,
    /// Whether the visible rectangle of REQ-SCN-036 applies to it.
    pub clipped: bool,
    /// AMD-055 REQ-SCN-084: whether the object's instance passes the **extent** half of the floor
    /// definition — its bounding box in the floor plane, before curvature, measuring at least
    /// `6 × échelle de scène` in both dimensions. False for everything but an opaque object: only
    /// the already-drawn opaque geometry can carry reflections (REQ-SCN-080).
    ///
    /// The other half, the face's normal, is per face and is in [`Self::normals`]
    /// ([`crate::style::is_floor_normal`] judges it); a face reflects when both hold.
    pub floor: bool,
    /// The final scene positions, curvature included.
    pub positions: Vec<Vec3>,
    /// The same vertices before the curvature, which the clipping is judged on (REQ-SCN-068,
    /// order clause).
    pub unbent: Vec<Vec3>,
    /// The shading normals, transported by the curvature (REQ-SCN-028 as REQ-SCN-068 amends it).
    pub normals: Vec<Vec3>,
}

/// The objects of `scene` as the renderer would place them for the displayed frame `frame`, seen
/// through `view`.
///
/// The two meshes the frame plan places itself — the backdrop box of REQ-SCN-048 and the
/// rectangle of the 2D mode — are built here with the subdivision
/// [`crate::interop::subdivision_for`] gives for this curvature, which is the very function the
/// renderer reads: the exported geometry cannot drift from the drawn one.
#[must_use]
pub fn objects(
    scene: &SceneState,
    frame: &SceneFrame<'_>,
    view: &CameraView,
) -> Vec<ExportedObject> {
    // `SceneState::effective_bend` is the one home of the rule: in 2D mode the frame holds the
    // rectangle alone, so the setting of REQ-SCN-074 decides for the whole frame; in 3D mode it
    // has no effect and the HUD alone stays flat.
    let bend = scene.effective_bend();
    let bends = !bend.is_identity();
    let cells = crate::interop::subdivision_for(bends);
    let (Ok(unit_box), Ok(picture_quad)) = (
        crate::interop::unit_box_subdivided(cells),
        crate::interop::picture_quad_subdivided(0, 0, cells),
    ) else {
        // Both build whole triangles by construction; an error here is no geometry to export.
        return Vec::new();
    };
    let (unit_box, picture_quad) = (Arc::new(unit_box), Arc::new(picture_quad));
    let wide = crate::interop::wide_quad_for_subdivided(
        frame.nes_picture,
        frame.nes_picture_height,
        cells,
    )
    .ok()
    .flatten();
    let fixed = FixedMeshes {
        unit_box: &unit_box,
        picture_quad: &picture_quad,
        wide_picture_quad: wide.as_ref().map(|(_, quad)| quad),
        // KIRBY-MERGE: no bands, so a 2D export is the one whole picture quad whether or not the
        // experimental smooth scrolling is shifting the playfield on screen. The export answers
        // the floor and extent criteria of AMD-055 REQ-SCN-084, which a fraction of a pixel of
        // scrolling does not enter, and `collect_picture_quad` takes its ordinary path without
        // them.
        picture_bands: None,
    };

    let mut residency = MeshTable::default();
    let mut plan: FramePlan<usize, usize> = FramePlan::default();
    // The collect of the plan only fails when a residency refuses a mesh, which this one never
    // does; an error would leave the lists as they are, which is an empty export.
    let _ = plan.collect(&mut residency, fixed, scene, frame, view);
    // AMD-055 REQ-SCN-084: the extent half of the floor definition, which the exported objects
    // carry so that the criteria of §2.5 can be read without a GL context. The scene scale of
    // REQ-SCN-002 is 1 outside virtual reality, which Zaxnes has none of — the same reading
    // `SceneConstants::voxel_cell` and the occlusion's radius already take.
    plan.mark_floor_surfaces(scene.camera.local_camera, 1.0);

    let mut exported = Vec::new();
    for (hud, opaque, items) in [
        (false, true, &plan.lists.opaque),
        (false, false, &plan.lists.transparent),
        (true, false, &plan.lists.hud),
    ] {
        for (index, item) in items.iter().enumerate() {
            let Some(mesh) = residency.meshes.get(item.mesh) else {
                continue;
            };
            // REQ-SCN-068: the HUD is the plane attached to the camera and stays flat.
            exported.push(export_one(
                mesh,
                item.model,
                bend,
                hud,
                bends && !hud,
                item.clip,
                opaque && plan.floor_surfaces.get(index).copied().unwrap_or(false),
            ));
        }
    }
    exported
}

/// One object's vertices placed by `model` and then, when `curved`, by `bend`.
#[allow(clippy::fn_params_excessive_bools)] // One flag per clause of the export, each independent.
fn export_one(
    mesh: &Arc<TileMesh>,
    model: Mat4,
    bend: SpaceBend,
    hud: bool,
    curved: bool,
    clipped: bool,
    floor: bool,
) -> ExportedObject {
    let (normals, determinant) = crate::interop::normal_matrix(&model);
    // Q-069: a mirrored placement's cofactor matrix points the normal inwards, and the shader
    // multiplies it back by −1.
    let sign = if determinant < 0.0 { -1.0 } else { 1.0 };
    let count = mesh.vertices().len();
    let mut object = ExportedObject {
        hud,
        bent: curved,
        clipped,
        floor,
        positions: Vec::with_capacity(count),
        unbent: Vec::with_capacity(count),
        normals: Vec::with_capacity(count),
    };

    for vertex in mesh.vertices() {
        let flat = transform_point(&model, vertex.position);
        let normal = scale(transform_normal(&normals, vertex.normal), sign);
        object.unbent.push(flat);
        if curved {
            object.positions.push(bend.apply(flat));
            object
                .normals
                .push(normalise(bend.transform_normal(flat, normal)));
        } else {
            object.positions.push(flat);
            object.normals.push(normalise(normal));
        }
    }
    object
}

fn transform_point(model: &Mat4, point: Vec3) -> Vec3 {
    let row = |index: usize| {
        (model[(index, 0)] * point.x)
            + (model[(index, 1)] * point.y)
            + (model[(index, 2)] * point.z)
            + model[(index, 3)]
    };
    Vec3::new(row(0), row(1), row(2))
}

fn transform_normal(cofactor: &[f32; 9], normal: Vec3) -> Vec3 {
    let row = |index: usize| {
        (cofactor[index * 3] * normal.x)
            + (cofactor[index * 3 + 1] * normal.y)
            + (cofactor[index * 3 + 2] * normal.z)
    };
    Vec3::new(row(0), row(1), row(2))
}

fn scale(vector: Vec3, factor: f32) -> Vec3 {
    Vec3::new(vector.x * factor, vector.y * factor, vector.z * factor)
}

fn normalise(vector: Vec3) -> Vec3 {
    let length = vector.squared_length().sqrt();
    if length <= 1e-12 {
        vector
    } else {
        scale(vector, 1.0 / length)
    }
}

/// A residency that keeps the meshes themselves, indexed, and hands out no texture: the export
/// needs the vertices, not GPU objects.
#[derive(Debug, Default)]
struct MeshTable {
    meshes: Vec<Arc<TileMesh>>,
}

impl Residency for MeshTable {
    type Mesh = usize;
    type Texture = usize;
    type Error = std::convert::Infallible;

    fn mesh(&mut self, mesh: &Arc<TileMesh>) -> Result<Option<usize>, Self::Error> {
        self.meshes.push(Arc::clone(mesh));
        Ok(Some(self.meshes.len() - 1))
    }

    fn texture(&mut self, _texture: &Arc<ObjectTexture>) -> Option<usize> {
        Some(0)
    }

    fn picture(&mut self, _picture: &[u8], _height: usize) -> Result<Option<usize>, Self::Error> {
        Ok(Some(0))
    }

    fn overlay(&mut self, _picture: &[u8], _height: usize) -> Result<Option<usize>, Self::Error> {
        Ok(Some(0))
    }
}
