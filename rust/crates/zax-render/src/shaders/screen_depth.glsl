// The depth of the rendered frame, as the screen-space passes of AMD-055 REQ-SCN-080 read it,
// and the view-space position it reconstructs to. Shared by the occlusion of REQ-SCN-083 and the
// reflections of REQ-SCN-084, which both work on "la profondeur et la couleur **rendues**".
//
// The frame's depth, as a texture (`gl/linear_buffer.rs`), and the part of it the frame drew
// into: (1, 1) unless the host reserved room for a size that changes often.
uniform sampler2D uDepth;
uniform vec2 uDrawnScale;

// One texel of the target the pass draws into, in viewport coordinates: the step to a neighbour.
uniform vec2 uTexel;

const float ZAX_TAU = 6.283185307;

// The golden angle. Consecutive samples of a spiral land as far apart in azimuth as they can, so
// a kernel of six directions already covers a hemisphere evenly.
const float ZAX_GOLDEN_ANGLE = 2.399963230;

// The projection, as the four numbers these passes need of it, read straight off the matrix
// `numerics::Matrix4x4::create_perspective_field_of_view` built:
//   xy = the two scales (m11, m22), z = `range` = far/(near − far), w = `near`.
// Four floats rather than the inverse matrix, because that projection is symmetric and the
// renderer has no other: every term below is one multiply.
uniform vec4 uProjection;

/// The view-space `z` of a window depth, which is **negative** in front of the eye.
///
/// Two conventions meet here and both have to be undone. The projection of SPEC-06 maps the clip
/// depth to `[0, 1]` (the Direct3D convention `CreatePerspectiveFieldOfView` has), and OpenGL
/// then maps the NDC `[-1, 1]` to the window `[0, 1]`: the drawn geometry therefore occupies the
/// **upper half** of the depth buffer, `0.5` at the near plane and `1` at the far one. So
/// `z_ndc = 2w − 1`, and from `z_ndc = range·(z + near) / (−z)` comes
/// `z = −range·near / (range + z_ndc)`.
float zaxViewDepth(float window)
{
    float ndc = (window * 2.0) - 1.0;
    return -(uProjection.z * uProjection.w) / (uProjection.z + ndc);
}

/// The view-space position the frame holds at the viewport coordinate `uv` (0 to 1 over the part
/// drawn into), and in `w` the window depth that was read — `1` wherever nothing was drawn.
vec4 zaxViewSample(vec2 uv)
{
    float window = texture(uDepth, uv * uDrawnScale).r;
    float z = zaxViewDepth(window);
    // clip.xy = view.xy · scale and clip.w = −view.z, so view.xy = ndc · (−z) / scale.
    vec2 ndc = (uv * 2.0) - 1.0;
    return vec4((ndc * -z) / uProjection.xy, z, window);
}

/// Where a view-space point lands in viewport coordinates. `false` when it is at or behind the
/// near plane, where the projection says nothing.
bool zaxProjectView(vec3 at, out vec2 uv)
{
    if (at.z > -uProjection.w)
    {
        uv = vec2(0.0);
        return false;
    }

    uv = (((at.xy * uProjection.xy) / -at.z) * 0.5) + 0.5;
    return true;
}

/// Interleaved-gradient noise (Jimenez, 2014): a value in [0, 1) that changes from pixel to
/// pixel, which is what rotates a kernel per pixel — the occlusion's hemisphere and the blur
/// spiral of the reflections both ask for that and both leave the pattern free.
///
/// **No noise texture.** A hash of the pixel coordinate does the whole job: one fewer resource to
/// allocate, release, count and carry on a handheld.
float zaxDither(vec2 pixel)
{
    return fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
}

/// The distance in depth to a neighbour, or a value no comparison can win when nothing was drawn
/// there: a neighbour at the far plane says nothing about this surface.
float zaxNeighbour(vec4 neighbour, float z)
{
    return (neighbour.w < 1.0) ? abs(neighbour.z - z) : 1.0e30;
}

/// The view-space normal of the surface at `uv`, from the depth alone (REQ-SCN-083 reconstructs
/// the normal "à partir de la profondeur de la scène"; there is no normal buffer and neither
/// screen-space requirement asks for one). `false` when there is no normal to reconstruct.
///
/// Of the two differences available on each axis the **closer in depth** is taken: across a
/// silhouette the far neighbour belongs to another surface, and taking it would tilt the normal
/// into exactly the sharp outline REQ-SCN-083 forbids ("sans contour net").
///
/// Where an axis has **neither** neighbour on a surface — the sharp corner of a silhouette, a few
/// pixels of any scene — there is no plane to speak of, and the caller answers "no effect" rather
/// than invent a normal: a pixel it cannot read is then left exactly alone. Three pixels of the
/// plane of the close-up A are in that case, and it is the first invariant of §2.4 — "le rapport
/// vaut 1,00 ± 0,02 en tout pixel du sol" — that found them.
bool zaxViewNormal(vec2 uv, vec3 p, out vec3 normal)
{
    vec4 right = zaxViewSample(uv + vec2(uTexel.x, 0.0));
    vec4 left = zaxViewSample(uv - vec2(uTexel.x, 0.0));
    vec4 up = zaxViewSample(uv + vec2(0.0, uTexel.y));
    vec4 down = zaxViewSample(uv - vec2(0.0, uTexel.y));

    normal = vec3(0.0);
    bool alongX = (right.w < 1.0) || (left.w < 1.0);
    bool alongY = (up.w < 1.0) || (down.w < 1.0);
    if (!alongX || !alongY)
    {
        return false;
    }

    vec3 dx = (zaxNeighbour(right, p.z) < zaxNeighbour(left, p.z))
        ? (right.xyz - p)
        : (p - left.xyz);
    vec3 dy = (zaxNeighbour(up, p.z) < zaxNeighbour(down, p.z)) ? (up.xyz - p) : (p - down.xyz);
    normal = normalize(cross(dx, dy));
    // The eye is the origin of view space, so the normal of a visible surface points towards it.
    // Fixing the sign here rather than reasoning about the winding keeps this correct whatever
    // the frame's handedness and whatever the mirrored placements of Q-069 did.
    normal = (dot(normal, p) > 0.0) ? -normal : normal;
    return true;
}
