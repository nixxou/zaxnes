// The floor reflections of AMD-055 REQ-SCN-084, the WGSL twin of shaders/ssr.frag: a screen-space
// reflected ray, marched along the projection of the ray and then refined by bisection around the
// first depth crossing — the public family the amendment names (McGuire and Mara, "Efficient GPU
// Screen-Space Ray Tracing", JCGT 3(4), 2014).
//
// The amendment is explicit that this reference "oriente sans être normative": the frame and the
// progression of the march, the number and shape of the refinements, the exact width and shape of
// the two attenuations and the shape of the blur kernel are **free implementation choices**, and
// what makes faith are its parameters and its invariants. What is normative and lives here is the
// maximum length (`style::ssr_max_length`), the intensity (`style::ssr_intensity`), the blur that
// grows linearly with the distance travelled up to `style::ssr_blur_fraction` of the screen
// height, and the two attenuations that must reach 0 "sans bord franc".
//
// The target is `Rgba16Float` at half of each dimension except at `ssr.quality` Ultra, and holds
// the reflection **premultiplied** by its own weight: `rgb = w * c`, `a = w`. Premultiplied
// because the composite samples this target with a linear filter to come back up to the frame's
// size, and only a premultiplied value filters correctly across the edge of the reflected area.
//
// Q-093 (b), answered: a ray that crosses no surface produces **no** reflection. The skybox leaves
// the far plane behind it, so no ray of finite length can cross it and the sky is never reflected
// — which is also what keeps a swept floor from being tinted uniformly.
//
// **This is the pass the y mirror of screen_depth.wgsl actually matters to**, and step 4b measured
// why: the occlusion is nearly invariant under a mirror of view space because it fixes the
// normal's sign from `dot(normal, p)`, so the wrong mapping cost it 0.08 of a percentage point.
// Here the ray is `reflect(normalize(p), normal)` and marched by projecting view-space points back
// to the screen, so a mirrored reconstruction sends every ray the wrong way across the frame.
//
// `zax_source0` is the frame's linear colour, resolved — the only thing that may be reflected is
// the scene already drawn (REQ-SCN-084) — and `zax_source1` is the floor mask, both read texel by
// texel as the OpenGL backend reads them.

// How far along the reflected ray the surface is left behind before a crossing counts, in view
// units. Without it the floor finds itself at the first step, through the depth buffer's own
// quantisation, and reflects its own colour.
const ZAX_SSR_SURFACE_BIAS: f32 = 0.05;

// How far, in view units, the refined hit may still be from the surface it crossed and count as a
// hit. Checked **after** the bisection and not during the march, because during the march the step
// is coarse and the first positive crossing can be far behind the surface — rejecting it there is
// what made a cube more than a dozen units off the floor stop reflecting at all. After the
// refinement the distance is the real one, and a hit that is still far from any surface is a ray
// that passed behind a thin object and came out the other side.
const ZAX_SSR_THICKNESS: f32 = 2.0;

// Where the distance attenuation starts, as a fraction of the maximum length. It reaches 0 exactly
// at the end of the ray, with a `smoothstep` either side of it: the requirement forbids a visible
// edge and leaves the width free.
const ZAX_SSR_FADE_FROM: f32 = 0.45;

// How near the border of the drawn rectangle, in viewport coordinates, the reflection has faded to
// nothing. Same clause, the other attenuation.
const ZAX_SSR_EDGE: f32 = 0.06;

// The colour of the frame at `uv`, averaged over a spiral of `counts.w` taps of radius `radius`,
// rotated per pixel. A radius under half an output texel takes a single tap, which is what keeps
// the reflection sharp "près du contact" where the radius is 0 by definition.
fn zax_blurred_colour(uv: vec2<f32>, radius: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    if (max(radius.x, radius.y) < (zax_texel().y * 0.5)) {
        return zax_load0(uv).rgb;
    }

    let taps = screen.counts.w;
    let angle = zax_dither(pixel) * ZAX_TAU;
    var sum = vec3<f32>(0.0);
    for (var tap = 0; tap < taps; tap++) {
        let fraction = (f32(tap) + 0.5) / f32(taps);
        let theta = angle + (f32(tap) * ZAX_GOLDEN_ANGLE);
        // sqrt of the fraction spreads the taps evenly over the **disc**, not over the radius.
        let at = uv + (vec2<f32>(cos(theta), sin(theta)) * sqrt(fraction) * radius);
        sum += zax_load0(clamp(at, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
    }

    return sum / f32(taps);
}

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    // The extent half of the floor definition decided which runs were drawn into the mask, and the
    // normal half decided which of their fragments were marked. Everything else — the skybox, the
    // HUD, the transparent objects, the walls, the 2D-mode rectangle — is 0 here and leaves this
    // pass at the first line.
    if (zax_load1(in.uv).r < 0.5) {
        return vec4<f32>(0.0);
    }

    let here = zax_view_sample(in.uv);
    let p = here.xyz;
    let found_normal = zax_view_normal(in.uv, p);
    if (here.w >= 1.0 || !found_normal.ok) {
        return vec4<f32>(0.0);
    }
    let normal = found_normal.normal;

    let direction = reflect(normalize(p), normal);
    let origin = p + (normal * ZAX_SSR_SURFACE_BIAS);

    let steps = screen.counts.y;
    let max_length = screen.ssr.y;
    let reference_length = screen.ssr.z;

    // 1. The linear march, on positions fitted to the longest ray and truncated at this one's.
    var previous = 0.0;
    var crossed = -1.0;
    for (var step = 1; step <= steps; step++) {
        let fraction = f32(step) / f32(steps);
        // Quadratic rather than uniform: the first steps are fractions of a unit, which is what
        // resolves a reflection at the contact, and the last are tens of units, where the blur has
        // widened past them anyway.
        let travel = reference_length * fraction * fraction;
        if (travel > max_length) {
            break;
        }

        let at = origin + (direction * travel);
        let projected = zax_project_view(at);
        if (!projected.ok
            || any(projected.uv < vec2<f32>(0.0))
            || any(projected.uv > vec2<f32>(1.0))) {
            break;
        }

        let found = zax_view_sample(projected.uv);
        if (found.w < 1.0) {
            // `z` is negative forward, so a surface in **front** of the ray's point has the
            // greater z: that is the crossing.
            if ((found.z - at.z) > ZAX_SSR_SURFACE_BIAS) {
                crossed = travel;
                break;
            }
        }

        previous = travel;
    }

    if (crossed < 0.0) {
        // Nothing was crossed: no reflection at all (Q-093 (b)).
        return vec4<f32>(0.0);
    }

    // 2. The bisection, which brings the hit back to where the ray actually met the surface.
    var low = previous;
    var high = crossed;
    let refinements = screen.counts.z;
    for (var refinement = 0; refinement < refinements; refinement++) {
        let middle = 0.5 * (low + high);
        let at = origin + (direction * middle);
        let projected = zax_project_view(at);
        if (!projected.ok) {
            high = middle;
            continue;
        }

        let found = zax_view_sample(projected.uv);
        if (found.w < 1.0 && (found.z - at.z) > 0.0) {
            high = middle;
        } else {
            low = middle;
        }
    }

    let at = origin + (direction * high);
    let projected = zax_project_view(at);
    if (!projected.ok) {
        return vec4<f32>(0.0);
    }

    // The thickness test, on the refined hit: a ray that ends up well behind every surface never
    // met one — it passed behind a thin object — and must bring nothing back.
    let hit = zax_view_sample(projected.uv);
    if (hit.w >= 1.0 || abs(hit.z - at.z) > ZAX_SSR_THICKNESS) {
        return vec4<f32>(0.0);
    }

    // 3. The blur, whose radius grows linearly with the distance travelled, from 0 at the origin to
    // `ssr.w` of the screen height at the maximum length (REQ-SCN-084).
    let fraction = clamp(high / max(max_length, 1e-4), 0.0, 1.0);
    let colour = zax_blurred_colour(
        projected.uv,
        screen.ssr.w * fraction * screen.blur_aspect.xy,
        zax_pixel(in.position.xy));

    // 4. The two attenuations, both continuous and neither with an edge: towards the end of the ray
    // and towards the border of the drawn rectangle.
    let distant = 1.0 - smoothstep(ZAX_SSR_FADE_FROM, 1.0, fraction);
    let border = min(projected.uv, vec2<f32>(1.0) - projected.uv);
    let edge = smoothstep(0.0, ZAX_SSR_EDGE, min(border.x, border.y));

    let weight = screen.ssr.x * distant * edge;
    return vec4<f32>(colour * weight, weight);
}
