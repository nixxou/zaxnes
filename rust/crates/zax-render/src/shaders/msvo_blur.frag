// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// The `SmartBlur` of `AoBlurAndUpsampleCS.hlsli`: one separable pass of the five-tap
// depth-respecting blur the original runs over **every** level of its chain before that level is
// combined with the next.
//
// **This pass is what a port without it is missing** (Q-100 correction 1). In the original, every
// `BlurAndUpsample` step blurs its low-resolution input over ±2 texels (`BlurHorizontally` then
// `BlurVertically`, the kernel `((a + e) / 2 + b + c + d) / 4`) and the chain ends with one more
// step at full resolution, so *no* level ever reaches the output unblurred. A port that multiplies
// each level's own occlusion in raw leaves the finest level — the one that carries the contact —
// completely unsmoothed, and gives the estimate a single sharp scale. It is also the "lissage qui
// respecte les discontinuités de profondeur" REQ-SCN-083 asks for.
//
// What is adapted: the original caches a tile of `1 / depth` in group-shared memory and decides
// tap by tap, with `CompareDeltas`, whether two consecutive reciprocal-depth deltas agree enough
// to blur across; OpenGL 3.3 has neither compute shaders nor that cache (REQ-SCN-056), so the
// boolean becomes a continuous weight on the **relative** depth difference. REQ-SCN-083 leaves
// "le rayon et le seuil du lissage" free, which is exactly what the two differ by.

in vec2 vUv;
out vec4 oOcclusion;

uniform sampler2D uAo;
uniform sampler2D uDepth;

// The part of this level the frame drew into, and one texel of it along the axis being blurred.
uniform vec2 uScale;
uniform vec2 uStep;

// `style::MSVO_BLUR_SHARPNESS`, in multiples of the relative depth difference.
uniform float uSharpness;

void main()
{
    float center = texture(uDepth, vUv * uScale).r;
    float falloff = uSharpness / max(center, 1e-3);

    float total = 0.0;
    float weight = 0.0;
    for (int tap = -2; tap <= 2; ++tap)
    {
        vec2 uv = vUv + (uStep * float(tap));
        float depth = texture(uDepth, uv * uScale).r;

        // The kernel of `SmartBlur`: the two outer taps weigh half of the three inner.
        float tent = (abs(tap) == 2) ? 0.5 : 1.0;
        float w = tent / (1.0 + (abs(center - depth) * falloff));
        total += texture(uAo, uv * uScale).r * w;
        weight += w;
    }

    oOcclusion = vec4(
        (weight > 0.0) ? (total / weight) : texture(uAo, vUv * uScale).r,
        0.0,
        0.0,
        1.0);
}
