layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec2 aUv;

// The placement of each instance of a batched draw: the model matrix, the columns of the
// normal (cofactor) matrix and the tile position of the deformation (xyz). The renderer fills
// one block range per draw; 128 instances of 128 bytes fill exactly the 16 KiB every
// implementation grants a uniform block.
struct ZaxInstance
{
    mat4 model;
    vec4 normal[3];
    vec4 deformTile;
};

layout(std140) uniform ZaxInstances
{
    ZaxInstance uInstances[128];
};

uniform float uNormalSign;
uniform mat4 uViewProjection;

// AMD-057 / REQ-SCN-065: how far a lateral wall of a tile box is pushed away from the eye, in
// clip units already scaled by the projection (`geometry::wall_clip_bias`). 0 for a draw whose
// normals are not axis aligned (MARCHING) and for the passes that need no such nudge.
uniform float uWallClipBias;

out vec3 vScene;
out vec3 vNormal;
// ZAX_CENTROID (AMD-058): the texel coordinate of REQ-E3D-506 is interpolated at the centroid of
// the samples this fragment actually covers, instead of at the pixel centre. It only matters when
// the buffer is multisampled: a fragment on the edge of a wall whose centre falls outside the
// triangle gets an **extrapolated** vUv, floor() then lands one texel past the face, and the
// atlas lookup of scene.frag returns a neighbour's texel — the one-pixel light line AMD-058 §1
// measured along the joints. The centroid is always inside the covered area, so the coordinate
// stays within the face. Without the variant this is the plain declaration, exactly as before the
// amendment: the interpolation qualifier has to match between the two stages, so scene.frag
// carries the same guard.
#ifdef ZAX_CENTROID
centroid out vec2 vUv;
#else
out vec2 vUv;
#endif

// ZAX_BEND: the position the curvature of REQ-SCN-067 placed this vertex at, which the shadow
// lookup reads (the shadow map holds the bent geometry, REQ-SCN-068). Without the variant the
// fragment shader reads vScene, exactly as before the amendment.
#ifdef ZAX_BEND
out vec3 vBent;
#endif

// AMD-055 REQ-SCN-081 and REQ-SCN-082: the position and the normal in the **mesh's local frame**
// (REQ-E3D-504 for GREEDY and GREEDY_MULTICOLOR, REQ-E3D-522 for MARCHING), before the animated
// deformation, before the object's transform and before the curvature. That is the frame both
// requirements evaluate in, and it is why the grooves and the shadow squares follow the object's
// scale, its rotation and the curved surface. Nothing of this is emitted without the variants.
#if defined(ZAX_VOXEL_GRID) || defined(ZAX_PIXEL_SHADOW)
out vec3 vLocal;
out vec3 vLocalNormal;
#endif

void main()
{
    // The fields are read from the block one by one: copying the whole instance into a local
    // makes the Adreno 6xx compiler (Android) fail the link ("Unknown named pointer type").
    int index = gl_InstanceID;
    vec3 local = aPosition;
    if (uDeform != 0)
    {
        local += zaxDeformation(uInstances[index].deformTile.xyz, aPosition);
    }

    vec4 scene = uInstances[index].model * vec4(local, 1.0);
    // REQ-SCN-068: the clipping of REQ-SCN-036 is evaluated on the position *before* the
    // curvature, so a tile the margins keep stays visible wherever its bent image lands.
    vScene = scene.xyz;
    // REQ-SCN-028: the normal is the mesh normal, unchanged by the deformation.
    // uNormalSign is -1 for a mirrored placement, whose cofactor matrix points the
    // normal inwards (Q-069).
    mat3 normalMatrix = mat3(uInstances[index].normal[0].xyz, uInstances[index].normal[1].xyz,
        uInstances[index].normal[2].xyz);
    vNormal = uNormalSign * (normalMatrix * aNormal);
    vUv = aUv;

#if defined(ZAX_VOXEL_GRID) || defined(ZAX_PIXEL_SHADOW)
    // The untouched attributes: aPosition is the voxel-grid position of REQ-E3D-504 and aNormal
    // the mesh normal there — the face normal in GREEDY and GREEDY_MULTICOLOR, the triangle's
    // flat normal in MARCHING (REQ-E3D-522), which makes it constant over a triangle and so
    // unchanged by the interpolation.
    vLocal = aPosition;
    vLocalNormal = aNormal;
#endif

#ifdef ZAX_BEND
    // REQ-SCN-067, applied last: after the animated deformation and after the object's own
    // transform, so on the final position in the scene (REQ-SCN-068). REQ-SCN-068 transports
    // the normal with it.
    mat3 bendJacobian;
    vec3 placed = zaxBend(scene.xyz, bendJacobian);
    vNormal = zaxBendNormal(bendJacobian, vNormal);
    vBent = placed;
    gl_Position = uViewProjection * vec4(placed.x, placed.y, -placed.z, 1.0);
#else
    gl_Position = uViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
#endif

    // AMD-057 / REQ-SCN-065. The drawn mesh of a tile is now the whole box of REQ-E3D-501, so
    // two tiles that touch keep a pair of coincident walls at their joint. Left alone those
    // walls tie with the faces that close the surface and win the depth test along the shared
    // edge — the dark grid Q-067 was about. Pushing them a quarter of a scene unit further from
    // the eye settles the tie in favour of the surface.
    //
    // Only the **depth** moves: the shift is added to `gl_Position.z` and divided by `w`, which
    // is exactly the distance `delta` behind the vertex
    // (`z_ndc(d) = (f+n)/(f-n) - 2fn/((f-n)d)`, so `dz_clip = 2fn*delta/(f-n) / w`). `x`, `y`
    // and `w` are untouched, so no silhouette and no texture coordinate moves with it.
    //
    // The face is recognised from the **local** normal, before the normal matrix: a wall of a
    // tile box has `|z| = 0` there and a front or back face `|z| = 1`, whatever the object's
    // rotation. `uWallClipBias` is 0 wherever that reading does not hold.
    if (uWallClipBias != 0.0 && abs(aNormal.z) < 0.5)
    {
        gl_Position.z += uWallClipBias / gl_Position.w;
    }
}
