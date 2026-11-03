// The floor mask of AMD-055 REQ-SCN-084: where the reflections of §2.5 may act at all.
//
// **Why there is a pass at all.** The requirement's definition of a floor surface is the
// conjunction of two tests, and the invariants of §2.5 forbid either from standing alone:
//
//   - the instance's bounding box in the floor plane, before curvature, at least `6 × échelle de
//     scène` in both dimensions — the variant R-s puts the same block at `scale = (1 ; 1 ; 0,5)`
//     beside itself, and the two show the same pixels, at the same depth, with the same normal,
//     so no reconstruction from the depth buffer can tell them apart. That half is decided on the
//     CPU (`frame_plan::FramePlan::mark_floor_surfaces`) and reaches this pass as *which runs are
//     drawn*;
//   - the face's own normal, `+Y ≥ 0,9` in the scene frame before curvature — the variant R-m is
//     the same shape without rotation, so its `xz` box measures 64 × 8 and **passes** the extent
//     test, and not one pixel of its front face may be modified. That half is this file.
//
// The vertex stage is `scene.vert` itself with `ZAX_FLOOR_MASK` defined, so the mask is laid down
// by exactly the geometry, the animated deformation, the placement, the curvature and the wall
// bias of AMD-057 that drew the frame — it cannot drift from the depth it is tested against. The
// target is one `R8` channel at the frame's own size, cleared to 0, with the frame's depth
// attached and depth writes off: a floor hidden behind something is not marked.

in vec3 vSceneNormal;
out vec4 oMask;

// `style::floor_vertical`: `+Y`, or the image plane's normal `(0, 0, −1)` under the local camera
// of REQ-SCN-009 (AMD-055 v1.4, Q-096). One uniform, decided once a frame.
uniform vec3 uFloorUp;

// `style::FLOOR_NORMAL_THRESHOLD`, the "+Y ≥ 0,9" of REQ-SCN-084.
const float ZAX_FLOOR_NORMAL_THRESHOLD = 0.9;

void main()
{
    float up = dot(normalize(vSceneNormal), uFloorUp);
    oMask = vec4(step(ZAX_FLOOR_NORMAL_THRESHOLD, up), 0.0, 0.0, 1.0);
}
