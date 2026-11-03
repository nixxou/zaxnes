// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// The occlusion of **one level** of the pyramid of AMD-055 REQ-SCN-083 *Mode MSVO*, the WGSL twin
// of shaders/msvo_occlude.frag: the sphere-penetration test of `AoRenderCS.hlsli`
// (`TestSamplePair` and `TestSamples`), whose result is an **access** value — 1 where nothing
// occludes, and 1 as well on a lone plane, which is what §2.4's first invariant asks for.
//
// The original caches a 32 × 32 tile of the depth in group-shared memory and gathers from it by
// integer offsets; a fragment pass has no such cache, so each pair is two point reads of this
// level's depth. The offsets, their sphere thicknesses, their weights and the axial / diagonal /
// L-shaped grouping are the original's, unchanged.
//
// What is adapted: the original's sphere is a fixed five *pixels* per level, while REQ-SCN-083
// binds the radius `r` in **scene units** — so the pattern is scaled per pixel by
// `r × focal / depth`, which is what makes the effect's reach a property of the scene and not of
// the resolution.
//
// `zax_source0` is this level's linear depth. Read with `textureLoad`, which is the `NEAREST` the
// OpenGL level carries and which `gl/msvo.rs` explains at length: a level holds a *perspective*
// depth, interpolating one between texel centres adds a ripple that peaks mid-texel, and the two
// samples of an opposite pair would pick up different amounts of it — which is exactly the
// cancellation the sphere test rests on.

const ZAX_MSVO_TAU: f32 = 6.283185307;

// The checkerboard of `AoRenderCS.hlsli`: `xy` the offset in fifths of the radius, `z` the
// half-thickness of the sphere at that offset, `w` how many samples the entry stands for. The
// table is `style::MSVO_PATTERN`, written once there and unit-tested against `sqrt(1 - u² - v²)`;
// here it is a constant array so that no uniform array has to be uploaded per frame.
const ZAX_MSVO_PATTERN = array<vec4<f32>, 7>(
    vec4<f32>(2.0, 0.0, 0.9165151, 4.0),
    vec4<f32>(1.0, 1.0, 0.9591663, 4.0),
    vec4<f32>(2.0, 2.0, 0.8246211, 4.0),
    vec4<f32>(1.0, 3.0, 0.7745967, 8.0),
    vec4<f32>(4.0, 0.0, 0.6000000, 4.0),
    vec4<f32>(3.0, 3.0, 0.5291503, 4.0),
    vec4<f32>(2.0, 4.0, 0.4472136, 8.0),
);

// The "disocclusion" of one sample: how far its depth penetrates the sphere, `0,5 + Δz / (2 r T)`
// — the original's `depth · invThickness − frontDepth` with its
// `invThickness = 1 / (sphere diameter × T)` written out for a depth kept in scene units. Below 0
// the sample fell in front of the sphere (full occlusion), above 1 it fell behind it (none).
//
// A sample that falls **outside the drawn rectangle** says nothing about what is there, so it must
// not occlude, and 1 is what the test reads as "no occlusion". The original has no such arm
// because it gathers through a *border* sampler; clamping to the edge would repeat the edge texel
// instead, and a repeated texel breaks the cancellation the opposite pairs rest on.
fn zax_msvo_disocclusion(uv: vec2<f32>, center: f32, scale: f32) -> f32 {
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return 1.0;
    }
    return 0.5 + ((zax_load0(uv).r - center) * scale);
}

// One pair of **opposite** samples. The pseudo-disocclusions fade off a sample that occludes by so
// much as to be unreliable — a thin screen rather than a solid volume, which is what
// `ao.thickness` sets.
//
// The construction by opposite pairs is what leaves a lone plane at exactly 1: on an inclined
// surface the two samples' depth differences are equal and opposite, so they cancel.
fn zax_msvo_pair(
    uv: vec2<f32>,
    offset: vec2<f32>,
    stride: vec2<f32>,
    center: f32,
    scale: f32,
) -> f32 {
    let d1 = zax_msvo_disocclusion(uv + (offset * stride), center, scale);
    let d2 = zax_msvo_disocclusion(uv - (offset * stride), center, scale);

    let falloff = screen.msvo_a.z;
    let p1 = clamp(falloff * d1, 0.0, 1.0);
    let p2 = clamp(falloff * d2, 0.0, 1.0);

    return clamp(d1, p2, 1.0) + clamp(d2, p1, 1.0) - (p1 * p2);
}

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let radius = screen.msvo_a.x;
    let focal = screen.msvo_a.y;
    let weight_scale = screen.msvo_a.w;
    let headroom = screen.msvo_b.x;
    let entries = screen.msvo_counts.x;

    let center = zax_load0(in.uv).r;

    // The pattern spans five of its own units, and the sphere at this pixel covers `radius` scene
    // units: that is `radius × focal / depth` pixels.
    let pixels = (radius * focal) / max(center, 1e-3);
    let stride = (pixels * 0.2) * zax_texel();

    // The pattern is turned by a per-pixel angle. The original samples a **deinterleaved** depth
    // — sixteen sub-grids of one buffer, each gathered with the same offsets — so neighbouring
    // output pixels probe different directions and the blur that follows averages them, which is
    // how a seven-entry checkerboard stands in for a dense neighbourhood. A fragment pass has no
    // deinterleaving, and sampling the *same* seven azimuths at every pixel leaves the estimate
    // blind between them. REQ-SCN-083 allows exactly this — "Le motif d'échantillonnage PEUT
    // varier par pixel, à condition qu'aucun motif fixe ne reste visible après lissage" — and the
    // smoothing that makes good on the condition is the `SmartBlur` of `msvo_blur.wgsl`, run over
    // every level.
    //
    // The angle is a hash of the pixel, so it changes from pixel to pixel but never from frame to
    // frame: a still camera gives a still image (REQ-SCN-083, review criterion).
    //
    // **The pixel coordinate is mirrored into OpenGL's row order** (`zax_pixel`). The GLSL reads
    // `floor(vUv / uTexel)`, which on that side counts rows from the bottom; the hash is wildly
    // discontinuous, so keeping the top-down coordinate would give every pixel a *different*
    // azimuth from the one OpenGL gives it. The pattern would still be admissible — the
    // requirement leaves it free — but the two backends' noise would no longer cancel pixel by
    // pixel, and the parity figures would read the estimator's variance instead of the port's
    // fidelity.
    // Q-100
    let at = floor(zax_pixel(in.position.xy));
    let hash = fract(sin(dot(at, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let angle = hash * ZAX_MSVO_TAU;
    let c = cos(angle);
    let s = sin(angle);
    // `mat2x2<f32>(a, b)` takes two columns, as GLSL's `mat2(c, s, -s, c)` does: the same four
    // numbers in the same four places.
    let spin = mat2x2<f32>(vec2<f32>(c, s), vec2<f32>(-s, c));

    var access = 0.0;
    for (var index = 0; index < entries; index++) {
        let point = ZAX_MSVO_PATTERN[index];
        let x = point.x;
        let y = point.y;

        // ±`r·T` is the back and the front of the sphere at this offset.
        let scale = 1.0 / (2.0 * radius * point.z);
        let weight = (point.w * point.z) * weight_scale;

        var value: f32;
        if (y == 0.0) {
            // Axial.
            value = 0.5
                * (zax_msvo_pair(in.uv, spin * vec2<f32>(x, 0.0), stride, center, scale)
                    + zax_msvo_pair(in.uv, spin * vec2<f32>(0.0, x), stride, center, scale));
        } else if (x == y) {
            // Diagonal.
            value = 0.5
                * (zax_msvo_pair(in.uv, spin * vec2<f32>(x, -x), stride, center, scale)
                    + zax_msvo_pair(in.uv, spin * vec2<f32>(x, x), stride, center, scale));
        } else {
            // L-shaped.
            value = 0.25
                * (zax_msvo_pair(in.uv, spin * vec2<f32>(x, y), stride, center, scale)
                    + zax_msvo_pair(in.uv, spin * vec2<f32>(x, -y), stride, center, scale)
                    + zax_msvo_pair(in.uv, spin * vec2<f32>(y, x), stride, center, scale)
                    + zax_msvo_pair(in.uv, spin * vec2<f32>(y, -x), stride, center, scale));
        }

        access += weight * value;
    }

    // The weights are normalised on the CPU, so a pair that nothing occludes contributes 2 and a
    // lone plane contributes 1: the original's "anything that's 50 % occluded or less is fully
    // unoccluded".
    //
    // **The access is stored with headroom, and that is the first invariant of §2.4.** A value
    // above 1 means "less than half occluded", which the original is free to write because its
    // destination is a float; clamping it at 1 before the smoothing instead turns the estimator's
    // own **zero-mean** noise into a one-sided bias — only the darker half survives — and four
    // levels of that bias multiply.
    return vec4<f32>(clamp(access, 0.0, headroom) / headroom, 0.0, 0.0, 1.0);
}
