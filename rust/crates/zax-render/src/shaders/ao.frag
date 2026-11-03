// The ambient occlusion of AMD-055 REQ-SCN-083: hemisphere screen-space occlusion reconstructed
// from the frame's depth, the public technique the amendment names (the SSAO tutorial of John
// Chapman, 2013) — a kernel drawn in the normal's hemisphere and tightened near the point, rotated
// per pixel, a range check that drops samples too far away in depth, and then the blur of
// `ao_blur.frag`.
//
// The amendment is explicit that this reference "oriente sans être normative": the weighting, the
// distribution and the per-pixel variation of the samples are free choices, and what makes faith
// are its parameters and its invariants. What is normative and lives here is the **radius** (in
// scene units, from `style::ao_radius_units`) and the **darkening ceiling**
// `O ≤ min(0,95 ; 0,3 × ao.strength)` (`style::ao_ceiling`), which this pass applies as
// `O = ceiling × occlusion` with `occlusion` in [0, 1]: the ceiling is then reached only where
// every sample is occluded, `O` grows with `ao.strength` exactly as the invariants of §2.4 require
// (strictly from rung 1 to 4, and flat from 4 to 5 where the ceiling saturates), and `O ≤ ceiling`
// holds in every pixel by construction rather than by a clamp.
//
// The target is a one-channel `R8` at half of each dimension except at `ao.quality` High.

in vec2 vUv;
out vec4 oOcclusion;

// One texel of this target, in viewport coordinates: the step to a neighbour.
uniform vec2 uTexel;
// `style::ao_radius_units`, in scene units — which *are* view-space units, because the view
// matrix of SPEC-06 §3.2 is built from scene coordinates and scales nothing.
uniform float uRadius;
// `style::ao_ceiling`.
uniform float uCeiling;
// The depth, in view units, a sample has to be in front of the point before it counts: it keeps a
// sample that lies almost in the surface — which the flattest of the kernel's directions do — from
// occluding the surface itself.
uniform float uBias;
// `style::ao_samples`, the order of magnitude REQ-SCN-083 names (6, 10 and 16). Not observable,
// and a uniform rather than a define so that `ao.quality` costs no shader variant.
uniform int uSamples;

const float ZAX_TAU = 6.283185307;

// The golden angle. Consecutive samples of the spiral land as far apart in azimuth as they can,
// so a kernel of six directions already covers the hemisphere evenly.
const float ZAX_GOLDEN_ANGLE = 2.399963230;

/// Interleaved-gradient noise (Jimenez, 2014): a value in [0, 1) that changes from pixel to
/// pixel, which is what rotates the kernel per pixel ("rotation du noyau par pixel").
///
/// **No noise texture.** REQ-SCN-083 leaves the pattern free and asks only that none of it survive
/// the blur, so a hash of the pixel coordinate does the whole job — one fewer resource to
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
/// the normal "à partir de la profondeur de la scène"; there is no normal buffer and the
/// requirement asks for none). `false` when there is no normal to reconstruct.
///
/// Of the two differences available on each axis the **closer in depth** is taken: across a
/// silhouette the far neighbour belongs to another surface, and taking it would tilt the normal
/// into exactly the sharp outline REQ-SCN-083 forbids ("sans contour net").
///
/// Where an axis has **neither** neighbour on a surface — the sharp corner of a silhouette, a few
/// pixels of any scene — there is no plane to speak of, and this pass answers "no occlusion"
/// rather than invent a normal: a pixel it cannot read is then left exactly alone, never darkened
/// wrongly. Three pixels of the plane of the close-up A are in that case, and it is the first
/// invariant of §2.4 — "le rapport vaut 1,00 ± 0,02 en tout pixel du sol" — that found them.
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

/// One direction of the kernel, in the tangent frame: a cosine-distributed unit vector of the
/// hemisphere, scaled so that the samples cluster near the point considered ("noyau …
/// **resserré près du point**").
vec3 zaxKernel(int index, float count, float angle)
{
    float fraction = (float(index) + 0.5) / count;
    float theta = angle + (float(index) * ZAX_GOLDEN_ANGLE);
    // planar² + lift² = fraction + (1 − fraction) = 1: a unit vector, denser near the normal.
    float planar = sqrt(fraction);
    float lift = sqrt(1.0 - fraction);
    return vec3(cos(theta) * planar, sin(theta) * planar, lift) * mix(0.1, 1.0, fraction * fraction);
}

void main()
{
    vec4 here = zaxViewSample(vUv);
    // REQ-SCN-080, 4th bullet: the skybox is at infinite distance, behind the whole scene, so it
    // is never within 2r of anything and never occludes — and nothing of it is darkened either.
    // The cleared background of REQ-SCN-046 is at the far plane too, so one depth test excludes
    // both, with no mask of any kind. The HUD and the transparent objects are drawn *after* this
    // pass, so they need nothing at all.
    if (here.w >= 1.0)
    {
        oOcclusion = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    vec3 p = here.xyz;
    vec3 normal;
    if (!zaxViewNormal(vUv, p, normal))
    {
        oOcclusion = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    // Any tangent will do: the kernel is turned by `angle` on top of it.
    vec3 guide = (abs(normal.z) < 0.9) ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
    vec3 tangent = normalize(cross(normal, guide));
    vec3 bitangent = cross(normal, tangent);

    float count = float(uSamples);
    float angle = zaxDither(gl_FragCoord.xy) * ZAX_TAU;
    float occlusion = 0.0;
    for (int index = 0; index < uSamples; ++index)
    {
        vec3 offset = zaxKernel(index, count, angle);
        vec3 at = p
            + (((tangent * offset.x) + (bitangent * offset.y) + (normal * offset.z)) * uRadius);

        vec2 uv;
        // A sample that falls outside the drawn rectangle tells nothing about what is there, so
        // it does not occlude. That is also what keeps the floor of the close-up A unoccluded out
        // to the edge of the image.
        if (!zaxProjectView(at, uv)
            || any(lessThan(uv, vec2(0.0)))
            || any(greaterThan(uv, vec2(1.0))))
        {
            continue;
        }

        vec4 found = zaxViewSample(uv);
        if (found.w >= 1.0)
        {
            continue;
        }

        // `z` is negative forward, so a surface in **front** of the sample has the greater z.
        if ((found.z - at.z) <= uBias)
        {
            continue;
        }

        // The range check of REQ-SCN-083: a surface further away in depth than the radius is
        // another part of the scene, not an occluder of this point — which is also what makes
        // `O` null "là où aucune géométrie occultante ne se trouve à moins de 2r".
        occlusion += smoothstep(0.0, 1.0, uRadius / max(abs(p.z - found.z), 1e-4));
    }

    oOcclusion = vec4(uCeiling * (occlusion / count), 0.0, 0.0, 1.0);
}
