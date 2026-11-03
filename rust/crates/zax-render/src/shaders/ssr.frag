// The floor reflections of AMD-055 REQ-SCN-084: a screen-space reflected ray, marched along the
// projection of the ray and then refined by bisection around the first depth crossing — the
// public family the amendment names (McGuire and Mara, "Efficient GPU Screen-Space Ray Tracing",
// JCGT 3(4), 2014).
//
// The amendment is explicit that this reference "oriente sans être normative": the frame and the
// progression of the march, the number and shape of the refinements, the exact width and shape of
// the two attenuations and the shape of the blur kernel are **free implementation choices**, and
// what makes faith are its parameters and its invariants. What is normative and lives here is the
// maximum length (`style::ssr_max_length`), the intensity (`style::ssr_intensity`), the blur that
// grows linearly with the distance travelled up to `style::ssr_blur_fraction` of the screen
// height, and the two attenuations that must reach 0 "sans bord franc".
//
// The target is `RGBA16F` at half of each dimension except at `ssr.quality` Ultra, and holds the
// reflection **premultiplied** by its own weight: `rgb = w · c`, `a = w`. Premultiplied because
// the composite samples this target with a linear filter to come back up to the frame's size, and
// only a premultiplied value filters correctly across the edge of the reflected area.
//
// Q-093 (b), answered: a ray that crosses no surface produces **no** reflection. The skybox
// leaves the far plane behind it, so no ray of finite length can cross it and the sky is never
// reflected — which is also what keeps a swept floor from being tinted uniformly.

in vec2 vUv;
out vec4 oReflection;

// The frame's colour, linear and resolved: the only thing that may be reflected is the scene
// already drawn (REQ-SCN-084), which in this pass is exactly what this texture holds.
uniform sampler2D uColor;
// The floor mask of `floor_mask.frag`, at the frame's own size.
uniform sampler2D uMask;

// `style::ssr_intensity`, the share of the reflected colour mixed into the surface.
uniform float uIntensity;
// `style::ssr_max_length`, in scene units — which *are* view-space units, because the view matrix
// of SPEC-06 §3.2 is built from scene coordinates and scales nothing.
uniform float uMaxLength;
// The **longest** ray any setting of `ssr.length` can ask for, `40 × 9 × échelle de scène`. The
// march's sample positions are fitted on it and truncated at `uMaxLength`, so raising
// `ssr.length` only ever adds positions at the far end: the invariant "`ssr.length` croissant
// allonge la zone modifiée" then holds by construction instead of by luck, which it would not if
// the same step count were spread over each setting's own length.
uniform float uReferenceLength;
// `style::ssr_blur_fraction`: the blur radius at `uMaxLength`, as a fraction of the screen height.
uniform float uBlur;
// (height / width, 1): turns a radius given as a fraction of the screen height into one in
// viewport coordinates, which are normalised per axis.
uniform vec2 uBlurAspect;
// `style::ssr_steps` and `style::ssr_refinements`. Uniforms rather than defines, so `ssr.quality`
// costs no shader variant; neither is observable (REQ-SCN-084).
uniform int uSteps;
uniform int uRefinements;
// Taps of the blur spiral, by quality. Not observable either.
uniform int uBlurTaps;

// How far along the reflected ray the surface is left behind before a crossing counts, in view
// units. Without it the floor finds itself at the first step, through the depth buffer's own
// quantisation, and reflects its own colour.
const float ZAX_SSR_SURFACE_BIAS = 0.05;

// How far, in view units, the refined hit may still be from the surface it crossed and count as a
// hit. Checked **after** the bisection and not during the march, because during the march the step
// is coarse and the first positive crossing can be far behind the surface — rejecting it there is
// what made a cube more than a dozen units off the floor stop reflecting at all. After the
// refinement the distance is the real one, and a hit that is still far from any surface is a ray
// that passed behind a thin object and came out the other side.
const float ZAX_SSR_THICKNESS = 2.0;

// Where the distance attenuation starts, as a fraction of `uMaxLength`. It reaches 0 exactly at
// the end of the ray, with a `smoothstep` either side of it: the requirement forbids a visible
// edge and leaves the width free.
const float ZAX_SSR_FADE_FROM = 0.45;

// How near the border of the drawn rectangle, in viewport coordinates, the reflection has faded
// to nothing. Same clause, the other attenuation.
const float ZAX_SSR_EDGE = 0.06;

/// The colour of the frame at `uv`, averaged over a spiral of `uBlurTaps` taps of radius
/// `radius`, rotated per pixel. A radius under half an output texel takes a single tap, which is
/// what keeps the reflection sharp "près du contact" where the radius is 0 by definition.
vec3 zaxBlurredColour(vec2 uv, vec2 radius)
{
    if (max(radius.x, radius.y) < (uTexel.y * 0.5))
    {
        return texture(uColor, uv * uDrawnScale).rgb;
    }

    float angle = zaxDither(gl_FragCoord.xy) * ZAX_TAU;
    vec3 sum = vec3(0.0);
    for (int tap = 0; tap < uBlurTaps; ++tap)
    {
        float fraction = (float(tap) + 0.5) / float(uBlurTaps);
        float theta = angle + (float(tap) * ZAX_GOLDEN_ANGLE);
        // sqrt of the fraction spreads the taps evenly over the **disc**, not over the radius.
        vec2 at = uv + (vec2(cos(theta), sin(theta)) * sqrt(fraction) * radius);
        sum += texture(uColor, clamp(at, vec2(0.0), vec2(1.0)) * uDrawnScale).rgb;
    }

    return sum / float(uBlurTaps);
}

void main()
{
    // The extent half of the floor definition decided which runs were drawn into the mask, and
    // the normal half decided which of their fragments were marked. Everything else — the
    // skybox, the HUD, the transparent objects, the walls, the 2D-mode rectangle — is 0 here and
    // leaves this pass at the first line.
    if (texture(uMask, vUv * uDrawnScale).r < 0.5)
    {
        oReflection = vec4(0.0);
        return;
    }

    vec4 here = zaxViewSample(vUv);
    vec3 p = here.xyz;
    vec3 normal;
    if (here.w >= 1.0 || !zaxViewNormal(vUv, p, normal))
    {
        oReflection = vec4(0.0);
        return;
    }

    vec3 direction = reflect(normalize(p), normal);
    vec3 origin = p + (normal * ZAX_SSR_SURFACE_BIAS);

    // 1. The linear march, on positions fitted to the longest ray and truncated at this one's.
    float previous = 0.0;
    float crossed = -1.0;
    for (int step = 1; step <= uSteps; ++step)
    {
        float fraction = float(step) / float(uSteps);
        // Quadratic rather than uniform: the first steps are fractions of a unit, which is what
        // resolves a reflection at the contact, and the last are tens of units, where the blur
        // has widened past them anyway.
        float travel = uReferenceLength * fraction * fraction;
        if (travel > uMaxLength)
        {
            break;
        }

        vec2 uv;
        vec3 at = origin + (direction * travel);
        if (!zaxProjectView(at, uv) || any(lessThan(uv, vec2(0.0)))
            || any(greaterThan(uv, vec2(1.0))))
        {
            break;
        }

        vec4 found = zaxViewSample(uv);
        if (found.w < 1.0)
        {
            // `z` is negative forward, so a surface in **front** of the ray's point has the
            // greater z: that is the crossing.
            if ((found.z - at.z) > ZAX_SSR_SURFACE_BIAS)
            {
                crossed = travel;
                break;
            }
        }

        previous = travel;
    }

    if (crossed < 0.0)
    {
        // Nothing was crossed: no reflection at all (Q-093 (b)).
        oReflection = vec4(0.0);
        return;
    }

    // 2. The bisection, which brings the hit back to where the ray actually met the surface.
    float low = previous;
    float high = crossed;
    for (int refinement = 0; refinement < uRefinements; ++refinement)
    {
        float middle = 0.5 * (low + high);
        vec2 uv;
        vec3 at = origin + (direction * middle);
        if (!zaxProjectView(at, uv))
        {
            high = middle;
            continue;
        }

        vec4 found = zaxViewSample(uv);
        if (found.w < 1.0 && (found.z - at.z) > 0.0)
        {
            high = middle;
        }
        else
        {
            low = middle;
        }
    }

    vec2 uv;
    vec3 at = origin + (direction * high);
    if (!zaxProjectView(at, uv))
    {
        oReflection = vec4(0.0);
        return;
    }

    // The thickness test, on the refined hit: a ray that ends up well behind every surface never
    // met one — it passed behind a thin object — and must bring nothing back.
    vec4 hit = zaxViewSample(uv);
    if (hit.w >= 1.0 || abs(hit.z - at.z) > ZAX_SSR_THICKNESS)
    {
        oReflection = vec4(0.0);
        return;
    }

    // 3. The blur, whose radius grows linearly with the distance travelled, from 0 at the origin
    // to `uBlur` of the screen height at `uMaxLength` (REQ-SCN-084).
    float fraction = clamp(high / max(uMaxLength, 1e-4), 0.0, 1.0);
    vec3 colour = zaxBlurredColour(uv, uBlur * fraction * uBlurAspect);

    // 4. The two attenuations, both continuous and neither with an edge: towards the end of the
    // ray and towards the border of the drawn rectangle.
    float distant = 1.0 - smoothstep(ZAX_SSR_FADE_FROM, 1.0, fraction);
    vec2 border = min(uv, vec2(1.0) - uv);
    float edge = smoothstep(0.0, ZAX_SSR_EDGE, min(border.x, border.y));

    float weight = uIntensity * distant * edge;
    oReflection = vec4(colour * weight, weight);
}
