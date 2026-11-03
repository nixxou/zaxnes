// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// The occlusion of **one level** of the pyramid of AMD-055 REQ-SCN-083 *Mode MSVO*: the
// sphere-penetration test of `AoRenderCS.hlsli` (`TestSamplePair` and `TestSamples`), whose result
// is an **access** value — 1 where nothing occludes, and 1 as well on a lone plane, which is what
// §2.4's first invariant asks for.
//
// The original caches a 32 × 32 tile of the depth in group-shared memory and gathers from it by
// integer offsets; a fragment pass has no such cache (REQ-SCN-056), so each pair is two
// `texture` reads of this level's depth. The offsets, their sphere thicknesses, their weights and
// the axial / diagonal / L-shaped grouping are the original's, unchanged.
//
// What is adapted: the original's sphere is a fixed five *pixels* per level, while REQ-SCN-083
// binds the radius `r` in **scene units** — so the pattern is scaled per pixel by
// `r × focal / depth`, which is what makes the effect's reach a property of the scene and not of
// the resolution.

in vec2 vUv;
out vec4 oAccess;

// This level's linear depth, in scene units, and the part of it the frame drew into.
uniform sampler2D uDepth;
uniform vec2 uScale;

// One texel of this level, in viewport coordinates.
uniform vec2 uTexel;

// The world radius of this level's sphere, in scene units: a sixteenth, an eighth, a quarter and
// the whole of `1,75 r` from the finest level to the coarsest (`style::msvo_level_radius`).
uniform float uRadius;

// Focal length of this level in pixels, so the world radius becomes a pixel radius.
uniform float uFocal;

// `−1 / RejectionFalloff` of the original: the assumed thickness of the occluders, which
// REQ-SCN-083 binds to `ao.thickness` in this mode and in this mode only
// (`style::msvo_falloff`).
uniform float uFalloff;

// How many entries of the pattern this quality takes, and `1 / Σ weights` over them
// (`style::msvo_pattern_entries`, `style::msvo_weight_scale`).
uniform int uPatternSize;
uniform float uWeightScale;

// `style::MSVO_ACCESS_HEADROOM`: the access is stored divided by this, so that a value **above**
// 1 survives the `R8` target. See the note at the end of `main`.
uniform float uHeadroom;

const float ZAX_MSVO_TAU = 6.283185307;

// The checkerboard of `AoRenderCS.hlsli`: `xy` the offset in fifths of the radius, `z` the
// half-thickness of the sphere at that offset, `w` how many samples the entry stands for. The
// table is `style::MSVO_PATTERN`, written once there and unit-tested against `√(1 − u² − v²)`;
// here it is a constant array so that no uniform array has to be uploaded per frame.
const vec4 ZAX_MSVO_PATTERN[7] = vec4[7](
    vec4(2.0, 0.0, 0.9165151, 4.0),
    vec4(1.0, 1.0, 0.9591663, 4.0),
    vec4(2.0, 2.0, 0.8246211, 4.0),
    vec4(1.0, 3.0, 0.7745967, 8.0),
    vec4(4.0, 0.0, 0.6000000, 4.0),
    vec4(3.0, 3.0, 0.5291503, 4.0),
    vec4(2.0, 4.0, 0.4472136, 8.0)
);

/// The "disocclusion" of one sample: how far its depth penetrates the sphere, `0,5 + Δz / (2 r T)`
/// — the original's `depth · invThickness − frontDepth` with its
/// `invThickness = 1 / (sphere diameter × T)` written out for a depth kept in scene units. Below 0
/// the sample fell in front of the sphere (full occlusion), above 1 it fell behind it (none).
///
/// A sample that falls **outside the drawn rectangle** says nothing about what is there, so it must
/// not occlude, and 1 is what the test reads as "no occlusion". The original has no such arm
/// because it gathers through a *border* sampler, which a port that keeps the drawn rectangle of a
/// reserved attachment (`uScale`) cannot: `CLAMP_TO_EDGE` would repeat the edge texel instead, and
/// a repeated texel breaks the cancellation the opposite pairs rest on. Measured on the lone plane
/// of §2.4's first invariant, that is a 5 % darkening at a quarter of the image from the edge —
/// where the coarsest sphere reaches — against the 2 % the invariant allows. It is the same arm
/// `ao.frag` carries for the hemisphere estimator, and for the same reason.
float zaxMsvoDisocclusion(vec2 uv, float center, float scale)
{
    if (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0))))
    {
        return 1.0;
    }

    return 0.5 + ((texture(uDepth, uv * uScale).r - center) * scale);
}

/// One pair of **opposite** samples. The pseudo-disocclusions fade off a sample that occludes by so
/// much as to be unreliable — a thin screen rather than a solid volume, which is what
/// `ao.thickness` sets.
///
/// The construction by opposite pairs is what leaves a lone plane at exactly 1: on an inclined
/// surface the two samples' depth differences are equal and opposite, so they cancel.
float zaxMsvoPair(vec2 offset, vec2 stride, float center, float scale)
{
    float d1 = zaxMsvoDisocclusion(vUv + (offset * stride), center, scale);
    float d2 = zaxMsvoDisocclusion(vUv - (offset * stride), center, scale);

    float p1 = clamp(uFalloff * d1, 0.0, 1.0);
    float p2 = clamp(uFalloff * d2, 0.0, 1.0);

    return clamp(d1, p2, 1.0) + clamp(d2, p1, 1.0) - (p1 * p2);
}

void main()
{
    float center = texture(uDepth, vUv * uScale).r;

    // The pattern spans five of its own units, and the sphere at this pixel covers `uRadius`
    // scene units: that is `uRadius × focal / depth` pixels.
    float pixels = (uRadius * uFocal) / max(center, 1e-3);
    vec2 stride = (pixels * 0.2) * uTexel;

    // The pattern is turned by a per-pixel angle. The original samples a **deinterleaved** depth
    // — sixteen sub-grids of one buffer, each gathered with the same offsets — so neighbouring
    // output pixels probe different directions and the blur that follows averages them, which is
    // how a seven-entry checkerboard stands in for a dense neighbourhood. A fragment pass has no
    // deinterleaving, and sampling the *same* seven azimuths at every pixel leaves the estimate
    // blind between them. REQ-SCN-083 allows exactly this — "Le motif d'échantillonnage PEUT
    // varier par pixel, à condition qu'aucun motif fixe ne reste visible après lissage" — and the
    // smoothing that makes good on the condition is the `SmartBlur` of `msvo_blur.frag`, run over
    // every level.
    //
    // The angle is a hash of the pixel, so it changes from pixel to pixel but never from frame to
    // frame: a still camera gives a still image (REQ-SCN-083, review criterion), the same choice
    // `zaxDither` makes for the Depth estimator's kernel. A rotation leaves an entry's sphere
    // thickness — and therefore its weight and its depth window — untouched, since both depend on
    // the length of the offset alone, so a lone plane still reads exactly 1.
    // Q-100
    vec2 at = floor(vUv / uTexel);
    float hash = fract(sin(dot(at, vec2(12.9898, 78.233))) * 43758.5453);
    float angle = hash * ZAX_MSVO_TAU;
    float c = cos(angle);
    float s = sin(angle);
    mat2 spin = mat2(c, s, -s, c);

    float access = 0.0;
    for (int index = 0; index < uPatternSize; ++index)
    {
        vec4 point = ZAX_MSVO_PATTERN[index];
        float x = point.x;
        float y = point.y;

        // ±`r·T` is the back and the front of the sphere at this offset.
        float scale = 1.0 / (2.0 * uRadius * point.z);
        float weight = (point.w * point.z) * uWeightScale;

        float value;
        if (y == 0.0)
        {
            // Axial.
            value = 0.5 * (
                zaxMsvoPair(spin * vec2(x, 0.0), stride, center, scale) +
                zaxMsvoPair(spin * vec2(0.0, x), stride, center, scale));
        }
        else if (x == y)
        {
            // Diagonal.
            value = 0.5 * (
                zaxMsvoPair(spin * vec2(x, -x), stride, center, scale) +
                zaxMsvoPair(spin * vec2(x, x), stride, center, scale));
        }
        else
        {
            // L-shaped.
            value = 0.25 * (
                zaxMsvoPair(spin * vec2(x, y), stride, center, scale) +
                zaxMsvoPair(spin * vec2(x, -y), stride, center, scale) +
                zaxMsvoPair(spin * vec2(y, x), stride, center, scale) +
                zaxMsvoPair(spin * vec2(y, -x), stride, center, scale));
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
    // levels of that bias multiply. Measured on the lone plane: a worst ratio of 0,949 against the
    // 1,00 ± 0,02 the invariant allows, where each level on its own was within 2 %. Dividing by
    // `uHeadroom` keeps the sign of the noise in an `R8` target and lets `msvo_blur.frag` average
    // it away, at the cost of a quantisation step of `headroom / 255` — two thousandths, an order
    // of magnitude under what it buys. The noise itself is irreducible: the pyramid samples a
    // perspective depth on a grid coarser than the frame, so two opposite samples of a pair do not
    // land at exactly opposite depths.
    oAccess = vec4(clamp(access, 0.0, uHeadroom) / uHeadroom, 0.0, 0.0, 1.0);
}
