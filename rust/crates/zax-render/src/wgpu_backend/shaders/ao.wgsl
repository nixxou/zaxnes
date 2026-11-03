// The ambient occlusion of AMD-055 REQ-SCN-083, mode `spl.ao` = 2 and the declared fallback (D)
// of the MSVO mode: the WGSL twin of shaders/ao.frag, hemisphere screen-space occlusion
// reconstructed from the frame's depth alone — a kernel drawn in the normal's hemisphere and
// tightened near the point, rotated per pixel, a range check that drops samples too far away in
// depth, and then the blur of ao_blur.wgsl.
//
// The amendment is explicit that its reference (the SSAO tutorial of John Chapman, 2013)
// "oriente sans être normative": the weighting, the distribution and the per-pixel variation of
// the samples are free choices, and what makes faith are its parameters and its invariants. What
// is normative and lives here is the **radius** in scene units (`style::ao_radius_units`) and the
// darkening ceiling `O <= min(0,95 ; 0,3 x ao.strength)` (`style::ao_ceiling`), which this pass
// applies as `O = ceiling x occlusion` with `occlusion` in [0, 1].
//
// Nothing of the estimator is API-dependent: the depth, the reconstructed normal, the per-pixel
// dither and the two constants of the spiral are in screen_depth.wgsl, which is where the two
// conventions this backend differs by are undone once. So this file is a transcription of the
// GLSL, and the places WGSL says the same thing differently are the `select` of a ternary, the
// struct a function with an `out` parameter returns, and `@builtin(position)` for `gl_FragCoord`.

// One direction of the kernel, in the tangent frame: a cosine-distributed unit vector of the
// hemisphere, scaled so that the samples cluster near the point considered ("noyau ...
// **resserré près du point**").
fn zax_kernel(index: i32, count: f32, angle: f32) -> vec3<f32> {
    let fraction = (f32(index) + 0.5) / count;
    let theta = angle + (f32(index) * ZAX_GOLDEN_ANGLE);
    // planar^2 + lift^2 = fraction + (1 - fraction) = 1: a unit vector, denser near the normal.
    let planar = sqrt(fraction);
    let lift = sqrt(1.0 - fraction);
    return vec3<f32>(cos(theta) * planar, sin(theta) * planar, lift)
        * mix(0.1, 1.0, fraction * fraction);
}

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let here = zax_view_sample(in.uv);
    // REQ-SCN-080, 4th bullet: the skybox is at infinite distance, behind the whole scene, so it
    // is never within 2r of anything and never occludes — and nothing of it is darkened either.
    // The cleared background of REQ-SCN-046 is at the far plane too, so one depth test excludes
    // both, with no mask of any kind. The HUD and the transparent objects are drawn *after* this
    // pass, so they need nothing at all.
    if (here.w >= 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let p = here.xyz;
    let found_normal = zax_view_normal(in.uv, p);
    if (!found_normal.ok) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let normal = found_normal.normal;

    // Any tangent will do: the kernel is turned by `angle` on top of it.
    let guide = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(normal.z) < 0.9);
    let tangent = normalize(cross(normal, guide));
    let bitangent = cross(normal, tangent);

    let samples = screen.counts.x;
    let radius = screen.ao.x;
    let bias = screen.ao.z;
    let count = f32(samples);
    let angle = zax_dither(zax_pixel(in.position.xy)) * ZAX_TAU;
    var occlusion = 0.0;
    for (var index = 0; index < samples; index++) {
        let offset = zax_kernel(index, count, angle);
        let at = p
            + (((tangent * offset.x) + (bitangent * offset.y) + (normal * offset.z)) * radius);

        // A sample that falls outside the drawn rectangle tells nothing about what is there, so
        // it does not occlude. That is also what keeps the floor of the close-up A unoccluded out
        // to the edge of the image.
        let projected = zax_project_view(at);
        if (!projected.ok
            || any(projected.uv < vec2<f32>(0.0))
            || any(projected.uv > vec2<f32>(1.0))) {
            continue;
        }

        let found = zax_view_sample(projected.uv);
        if (found.w >= 1.0) {
            continue;
        }

        // `z` is negative forward, so a surface in **front** of the sample has the greater z.
        if ((found.z - at.z) <= bias) {
            continue;
        }

        // The range check of REQ-SCN-083: a surface further away in depth than the radius is
        // another part of the scene, not an occluder of this point — which is also what makes
        // `O` null "là où aucune géométrie occultante ne se trouve à moins de 2r".
        occlusion += smoothstep(0.0, 1.0, radius / max(abs(p.z - found.z), 1e-4));
    }

    return vec4<f32>(screen.ao.y * (occlusion / count), 0.0, 0.0, 1.0);
}
