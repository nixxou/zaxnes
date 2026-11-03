// The pixel shadows of AMD-055 REQ-SCN-082, the WGSL twin of shaders/pixel_shadow.glsl,
// prepended to the ZAX_PIXEL_SHADOW variants only.
//
// The shadow of an opaque tile is read not at the fragment but at the **centre of the fragment's
// voxel**, so it changes by squares of one voxel, like the pixel grid of the game. The snapping
// happens in the mesh's local frame of REQ-E3D-504 — before the animated deformation, before the
// object's transform and before the curvature — and the displaced point is then transformed
// **like a vertex**, in the order of REQ-E3D-406, before the bias of REQ-SCN-031 is applied to it.
//
// The z offset is the fractional part of `D/2` of the drawn mesh (`style::voxel_z_offset`), which
// is where the voxel edges fall on `z`: 0 for an even construction depth, 0.5 for an odd one. The
// GLSL has it as a uniform of its own, uploaded per run; here it rides per run in the spare
// integer slot of the draw block and is read back with `bitcast`, so the per-run block does not
// grow by sixteen bytes for every draw of every variant.

// The centre of the voxel that holds the local point `p` of a face of local normal `n`.
//
// Two axes move and the third is left alone, which is what keeps the point on the face: the two
// of smallest |n_a|, ties broken by the smaller index in the order x, y, z — so the axis left
// alone is the one of largest |n_a|, ties broken by the larger index. On a GREEDY face
// (REQ-E3D-505) that is exactly the axis the normal points along; on a MARCHING surface, whose
// normal is arbitrary (REQ-E3D-522), it still leaves one axis alone, so there is no plate where
// the effect would vanish.
fn zax_voxel_center(p: vec3<f32>, n: vec3<f32>, z_offset: f32) -> vec3<f32> {
    let an = abs(n);
    var best = an.x;
    var keep = 0;
    if (an.y >= best) {
        best = an.y;
        keep = 1;
    }
    if (an.z >= best) {
        keep = 2;
    }

    // The grid of REQ-E3D-504: whole coordinates on x and y, offset by D/2 on z.
    let snapped = vec3<f32>(
        floor(p.x) + 0.5,
        floor(p.y) + 0.5,
        floor(p.z + z_offset) + 0.5 - z_offset);
    // Branchless restore of the axis that is left alone.
    let untouched = vec3<f32>(
        select(0.0, 1.0, keep == 0),
        select(0.0, 1.0, keep == 1),
        select(0.0, 1.0, keep == 2));
    return mix(snapped, p, untouched);
}

// The linear part of a placement applied to `v`: the `mat3(uInstances[i].model) * v` of the
// GLSL, written as three dot products because the block holds the placement's **rows**
// (`zax_place` in common.wgsl reads them the same way).
//
// The three rows travel from the vertex stage as flat varyings, which is what the GLSL does and
// for the same reason: the placements are a storage buffer the fragment stage cannot see
// (`Layouts::new` binds it `VERTEX` only, and widening that would change the layout of **every**
// variant, Classic included). Nine floats, and only in this variant.
fn zax_place_linear(
    row0: vec3<f32>,
    row1: vec3<f32>,
    row2: vec3<f32>,
    v: vec3<f32>,
) -> vec3<f32> {
    return vec3<f32>(dot(row0, v), dot(row1, v), dot(row2, v));
}

// The scene position of that voxel centre, placed as the vertex stage would have placed it.
//
// `scene` is the fragment's own scene position (before the curvature), `p` and `n` its local
// position and local normal, `row0..row2` the linear part of the instance's placement and `tile`
// the tile position the deformation reads. The placement is affine, so only the **difference**
// between the two points has to be carried through it — which is why the translation of the
// placement is not needed here, and nine varyings suffice rather than twelve.
fn zax_voxel_shadow_point(
    draw: Draw,
    scene: vec3<f32>,
    p: vec3<f32>,
    n: vec3<f32>,
    row0: vec3<f32>,
    row1: vec3<f32>,
    row2: vec3<f32>,
    tile: vec3<f32>,
    time: f32,
) -> vec3<f32> {
    let snapped = zax_voxel_center(p, n, bitcast<f32>(draw.modes2.z));
    var delta = snapped - p;
    // REQ-SCN-082 point 3: the animated deformation first, in the order of REQ-E3D-406. The test
    // reads the run's own block, constant over the whole draw, exactly as it is in the vertex
    // stage.
    if (draw.modes2.x >= 0) {
        let deform = deforms[draw.modes2.x];
        delta += zax_deformation(deform, tile, snapped, time)
               - zax_deformation(deform, tile, p, time);
    }

    let placed = scene + zax_place_linear(row0, row1, row2, delta);
//#ifdef ZAX_BEND
    // And the curvature last (REQ-SCN-067), because the shadow map holds the bent geometry
    // (REQ-SCN-068). The Jacobian is unused: REQ-SCN-082 moves the point the shadow is read at,
    // not the shading normal.
    return zax_bend(placed).scene;
//#else
    return placed;
//#endif
}
