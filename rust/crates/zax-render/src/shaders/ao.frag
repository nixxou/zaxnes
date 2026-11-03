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
//
// The depth, the reconstructed normal, the per-pixel dither and the two constants of the spiral
// are in `screen_depth.glsl`: the reflections of REQ-SCN-084 read the same frame the same way, so
// they are written once and shared by both passes.

in vec2 vUv;
out vec4 oOcclusion;

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
