// The pixel shadows of AMD-055 REQ-SCN-082, compiled only into the ZAX_PIXEL_SHADOW variants.
//
// The shadow of an opaque tile is read not at the fragment but at the **centre of the fragment's
// voxel**, so it changes by squares of one voxel, like the pixel grid of the game. The snapping
// happens in the mesh's local frame of REQ-E3D-504 — before the animated deformation, before the
// object's transform and before the curvature — and the displaced point is then transformed
// **like a vertex**, in the order of REQ-E3D-406, before the bias of REQ-SCN-031 is applied to it.
//
// `uVoxelZOffset` is the fractional part of `D/2` of the drawn mesh (`style::voxel_z_offset`),
// which is where the voxel edges fall on `z`: 0 for an even construction depth, 0.5 for an odd
// one. It is uploaded per run, and only while this variant is in use.

uniform float uVoxelZOffset;

// The centre of the voxel that holds the local point `p` of a face of local normal `n`.
//
// Two axes move and the third is left alone, which is what keeps the point on the face: the two
// of smallest |n_a|, ties broken by the smaller index in the order x, y, z — so the axis left
// alone is the one of largest |n_a|, ties broken by the larger index. On a GREEDY face
// (REQ-E3D-505) that is exactly the axis the normal points along; on a MARCHING surface, whose
// normal is arbitrary (REQ-E3D-522), it still leaves one axis alone, so there is no plate where
// the effect would vanish.
vec3 zaxVoxelCenter(vec3 p, vec3 n, float zOffset)
{
    vec3 an = abs(n);
    float best = an.x;
    int keep = 0;
    if (an.y >= best)
    {
        best = an.y;
        keep = 1;
    }
    if (an.z >= best)
    {
        keep = 2;
    }

    // The grid of REQ-E3D-504: whole coordinates on x and y, offset by D/2 on z.
    vec3 snapped = vec3(
        floor(p.x) + 0.5,
        floor(p.y) + 0.5,
        floor(p.z + zOffset) + 0.5 - zOffset);
    // Branchless restore of the axis that is left alone.
    vec3 untouched = vec3(float(keep == 0), float(keep == 1), float(keep == 2));
    return mix(snapped, p, untouched);
}

// The scene position of that voxel centre, placed as the vertex stage would have placed it.
//
// `scene` is the fragment's own scene position (before the curvature), `p` and `n` its local
// position and local normal, `modelLinear` the linear part of the instance's model matrix and
// `tile` the tile position the deformation reads. The placement is affine, so only the
// **difference** between the two points has to be carried through it — which is why the
// translation of the model matrix is not needed here and three varyings suffice.
vec3 zaxVoxelShadowPoint(vec3 scene, vec3 p, vec3 n, mat3 modelLinear, vec3 tile)
{
    vec3 snapped = zaxVoxelCenter(p, n, uVoxelZOffset);
    vec3 delta = snapped - p;
    // REQ-SCN-082 point 3: the animated deformation first, in the order of REQ-E3D-406. The test
    // is a uniform, constant over the whole draw, exactly as it is in the vertex stage.
    if (uDeform != 0)
    {
        delta += zaxDeformation(tile, snapped) - zaxDeformation(tile, p);
    }

    vec3 placed = scene + modelLinear * delta;
#ifdef ZAX_BEND
    // And the curvature last (REQ-SCN-067), because the shadow map holds the bent geometry
    // (REQ-SCN-068). The Jacobian is unused: REQ-SCN-082 moves the point the shadow is read at,
    // not the shading normal.
    mat3 jacobian;
    placed = zaxBend(placed, jacobian);
#endif
    return placed;
}
