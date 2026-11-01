// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`).
// The MIT licence is compatible with the GPL-3.0 this reimplementation is distributed under.

namespace Zax.Render.Gl;

/// <summary>
/// The GLSL of the <b>multi-scale</b> screen-space ambient occlusion of SPEC-06 REQ-SCN-083
/// (AMD-055), the <c>spl.ao</c> = 1 « MSVO » mode: the occlusion is estimated at several
/// resolutions of the depth and combined, which gives both a fine contact and a wide darkening
/// at a moderate cost.
/// </summary>
/// <remarks>
/// <para>
/// Ported from the public MIT-licensed multi-scale occlusion of Microsoft's MiniEngine, as
/// REQ-SCN-083 asks, and <b>adapted to OpenGL 3.3 fragment passes</b>: the original is Direct3D
/// 12 compute shaders that cache a tile of the depth in group-shared memory and gather from it,
/// and OpenGL 3.3 has neither compute shaders nor group-shared memory (REQ-SCN-056). Each pass is
/// therefore a full-screen triangle that samples the depth textures directly. What is kept is the
/// substance of the effect:
/// </para>
/// <list type="bullet">
/// <item>the pyramid of <b>linear</b> depths, each level half the previous one;</item>
/// <item>the occlusion estimated per level by the <b>sphere penetration</b> test of the original
/// — <c>TestSamplePair</c> and <c>TestSamples</c> — with its table of per-offset sphere
/// thicknesses, its per-offset weights and its rejection fade-off;</item>
/// <item>the same checkerboard of sample offsets, and the same axial / diagonal / L-shaped
/// grouping of the pairs;</item>
/// <item>the <b>bilateral upsample</b> that carries each level into the next finer one and
/// multiplies the two, from the coarsest to the finest.</item>
/// </list>
/// <para>
/// What is adapted: the original's sampling sphere has a fixed radius of five <em>pixels</em> per
/// level, while REQ-SCN-083 binds the radius <c>r</c> in <b>scene units</b>, so the offsets of
/// the pattern are scaled per pixel by <c>r × focal / depth</c> and each level takes a quarter,
/// a half and the whole of <c>r</c>; the depth is kept in view units rather than normalised by
/// the far plane, which makes the sphere test exactly
/// <c>0.5 + Δz / (2 r T)</c>; the accentuation of the original is left at 0, since
/// REQ-SCN-083's first invariant wants a lone plane untouched; and the blur of the upsample is a
/// three by three depth-weighted tent rather than the original's group-shared
/// <c>SmartBlur</c>.
/// </para>
/// </remarks>
internal static class MsvoShaders
{
    /// <summary>Offsets of the sample pattern, in fifths of the radius.</summary>
    public const int PatternSize = 7;

    /// <summary>
    /// The linear depth of a pixel, in scene units, from the scene's window-space depth: the
    /// <c>LinearizeZ</c> of the original, which stores the view distance the sphere test
    /// compares. A pixel with no geometry keeps the far distance, so it never occludes.
    /// </summary>
    public static string LinearizeFragment { get; } = $$"""
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        {{SceneShaders.ScreenDepth}}

        void main()
        {
            float raw;
            vec3 view = zaxViewPosition(vUv, raw);

            // The camera looks down −z in view space, so the distance is −z.
            oColor = vec4(-view.z, 0.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// One level of the depth pyramid: the top-left texel of each two by two block of the level
    /// above, which is what <c>AoPrepareDepthBuffers</c> stores in its <c>DS2x</c> buffers.
    /// </summary>
    public const string DownsampleFragment = """
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uSource;

        void main()
        {
            ivec2 at = ivec2(gl_FragCoord.xy) * 2;
            oColor = vec4(texelFetch(uSource, at, 0).r, 0.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// The occlusion of one level: the sphere-penetration test of <c>AoRenderCS.hlsli</c>, whose
    /// result is an <b>access</b> value — 1 where nothing occludes, and 1 as well on a lone
    /// plane, which is what REQ-SCN-083's first invariant asks for.
    /// </summary>
    public static string OcclusionFragment { get; } = $$"""
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uDepth;
        uniform vec2 uTexel;

        // The world radius of this level's sphere, in scene units (REQ-SCN-083: a quarter, a
        // half and the whole of r from the finest level to the coarsest).
        uniform float uRadius;

        // Focal length in pixels of this level, so the world radius becomes a pixel radius.
        uniform float uFocal;

        // −1 / RejectionFalloff of the original: the assumed thickness of the occluders, which
        // REQ-SCN-083 binds to ao.thickness in this mode and in this mode only.
        uniform float uFalloff;

        // x, y of the offset in fifths of the radius; the weight of the entry; the thickness of
        // the sphere at that offset. Built on the CPU, exactly the two tables of SSAO.cpp.
        uniform vec4 uPattern[{{PatternSize}}];
        uniform int uPatternSize;

        // One pair of opposite samples: « disocclusion » measures how far the depth sample
        // penetrates the sphere. Below 0 the sample fell in front of it (full occlusion), above
        // 1 it fell behind (none). The pseudo-disocclusions fade off the samples that occlude by
        // so much as to be unreliable — a thin screen rather than a solid volume.
        float zaxTestPair(vec2 offset, vec2 step, float center, float scale)
        {
            float s1 = texture(uDepth, vUv + offset * step).r;
            float s2 = texture(uDepth, vUv - offset * step).r;

            float d1 = 0.5 + (s1 - center) * scale;
            float d2 = 0.5 + (s2 - center) * scale;

            float p1 = clamp(uFalloff * d1, 0.0, 1.0);
            float p2 = clamp(uFalloff * d2, 0.0, 1.0);

            return clamp(d1, p2, 1.0) + clamp(d2, p1, 1.0) - p1 * p2;
        }

        void main()
        {
            float center = texture(uDepth, vUv).r;

            // The pattern spans five of its own units, and the radius of the sphere at this
            // pixel covers uRadius scene units: that is uRadius × focal / depth pixels.
            float pixels = uRadius * uFocal / max(center, 1e-3);
            vec2 step = (pixels * 0.2) * uTexel;

            // V2-25, defect B: the pattern is turned by a per-pixel angle. The original samples
            // a *deinterleaved* depth — sixteen sub-grids of one buffer, each gathered with the
            // same offsets — so neighbouring output pixels probe different directions and the
            // blur that follows averages them, which is how a seven-entry checkerboard stands in
            // for a dense neighbourhood. A fragment pass has no deinterleaving (REQ-SCN-056), and
            // sampling the *same* seven azimuths at every pixel left the estimate blind between
            // them: an occluder a radius away was registered by one sub-pair or none, and the
            // mode's mid-field came out a third of what the hemisphere estimator of the Depth
            // mode finds at the same distance. REQ-SCN-083 allows exactly this — « Le motif
            // d'échantillonnage PEUT varier par pixel, à condition qu'aucun motif fixe ne reste
            // visible après lissage » — and the smoothing that makes good on the condition is
            // the SmartBlur of the original, now run over every level. Measured, it changes the
            // profile of the darkening by under a per cent — the mid-field of this mode is not
            // limited by its azimuths (Q-100) — and it is kept for the fixed-pattern clause of
            // the review criterion and for the faithfulness of the port.
            // Q-100
            //
            // The angle is a hash of the pixel, so it changes from pixel to pixel but never from
            // frame to frame: a still camera gives a still image (REQ-SCN-083, review), the same
            // choice the Depth estimator makes for its kernel. A rotation leaves the sphere
            // thickness of an entry — and therefore its weight and its depth window — untouched,
            // since both depend on the length of the offset alone; a lone plane still reads
            // exactly 1, which is the first invariant of §2.4.
            vec2 at = floor(vUv / uTexel);
            float hash = fract(sin(dot(at, vec2(12.9898, 78.233))) * 43758.5453);
            float angle = hash * 6.2831853;
            float c = cos(angle);
            float s = sin(angle);
            mat2 spin = mat2(c, s, -s, c);

            float access = 0.0;
            for (int i = 0; i < uPatternSize; i++)
            {
                float x = uPattern[i].x;
                float y = uPattern[i].y;
                float weight = uPattern[i].z;

                // Δz = ±r·T is the back and the front of the sphere at this offset.
                float scale = 1.0 / (2.0 * uRadius * uPattern[i].w);

                float value;
                if (y == 0.0)
                {
                    // Axial.
                    value = 0.5 * (
                        zaxTestPair(spin * vec2(x, 0.0), step, center, scale) +
                        zaxTestPair(spin * vec2(0.0, x), step, center, scale));
                }
                else if (x == y)
                {
                    // Diagonal.
                    value = 0.5 * (
                        zaxTestPair(spin * vec2(x, -x), step, center, scale) +
                        zaxTestPair(spin * vec2(x, x), step, center, scale));
                }
                else
                {
                    // L-shaped.
                    value = 0.25 * (
                        zaxTestPair(spin * vec2(x, y), step, center, scale) +
                        zaxTestPair(spin * vec2(x, -y), step, center, scale) +
                        zaxTestPair(spin * vec2(y, x), step, center, scale) +
                        zaxTestPair(spin * vec2(y, -x), step, center, scale));
                }

                access += weight * value;
            }

            // The weights are normalised on the CPU, so a pair that is not occluded at all
            // contributes 2 and a lone plane contributes 1: the original's « anything that's
            // 50 % occluded or less is fully unoccluded ».
            oColor = vec4(clamp(access, 0.0, 1.0), 0.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// The <c>SmartBlur</c> of <c>AoBlurAndUpsampleCS.hlsli</c>: one separable pass of the
    /// five-tap depth-respecting blur the original runs over <b>every</b> level of its chain
    /// before that level is combined with the next.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>V2-25, defect B.</b> This pass is what the port was missing, and it is why its
    /// darkening came out « concentrated in bands along edges/contacts » where the reference's
    /// MSVO darkens broadly. In the original, every <c>BlurAndUpsample</c> step blurs its
    /// low-resolution input over ±2 texels (<c>BlurHorizontally</c> then <c>BlurVertically</c>,
    /// the kernel <c>((a + e) / 2 + b + c + d) / 4</c>) and the chain ends with one more step at
    /// full resolution, so <em>no</em> level ever reaches the output unblurred. This port
    /// multiplied each level's own occlusion in raw, which left the finest level — the one that
    /// carries the contact — completely unsmoothed and gave the estimate a single sharp scale.
    /// </para>
    /// <para>
    /// What is adapted: the original caches a tile of <c>1 / depth</c> in group-shared memory and
    /// decides tap by tap, with <c>CompareDeltas</c>, whether two consecutive reciprocal-depth
    /// deltas agree enough to blur across; OpenGL 3.3 has neither compute shaders nor that cache
    /// (REQ-SCN-056), so the boolean becomes a continuous weight on the <em>relative</em> depth
    /// difference. REQ-SCN-083 wants « un lissage qui respecte les discontinuités de profondeur »
    /// and leaves « le rayon et le seuil du lissage » free, which is exactly what the two differ
    /// by.
    /// </para>
    /// </remarks>
    public const string BlurFragment = """
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uAo;
        uniform sampler2D uDepth;

        // One texel of this level along the axis being blurred.
        uniform vec2 uStep;

        // How fast a tap loses its weight as its depth parts from this pixel's, as a multiple of
        // the relative difference: the « seuil du lissage » REQ-SCN-083 leaves free. At 32 a tap
        // 3 % away in depth — a receding floor, one texel over — keeps two thirds of its weight
        // while a silhouette step of 10 % keeps under a quarter.
        uniform float uSharpness;

        void main()
        {
            float center = texture(uDepth, vUv).r;
            float scale = uSharpness / max(center, 1e-3);

            float total = 0.0;
            float weight = 0.0;
            for (int i = -2; i <= 2; i++)
            {
                vec2 uv = vUv + (uStep * float(i));
                float depth = texture(uDepth, uv).r;

                // The kernel of SmartBlur: the two outer taps weigh half of the three inner.
                float tent = abs(i) == 2 ? 0.5 : 1.0;
                float w = tent / (1.0 + (abs(center - depth) * scale));
                total += texture(uAo, uv).r * w;
                weight += w;
            }

            oColor = vec4(weight > 0.0 ? total / weight : texture(uAo, vUv).r, 0.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// The bilateral upsample of <c>AoBlurAndUpsampleCS.hlsli</c>: the coarse level is blurred
    /// and carried into the finer one with weights taken from how well the two depths agree, then
    /// <b>multiplied</b> by the finer level's own occlusion.
    /// </summary>
    public const string UpsampleFragment = """
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uFineDepth;
        uniform sampler2D uCoarseDepth;
        uniform sampler2D uCoarseAo;
        uniform sampler2D uFineAo;
        uniform vec2 uCoarseTexel;

        // Whether this level has an occlusion of its own to multiply in; the coarsest step has
        // none (the original's BLEND_WITH_HIGHER_RESOLUTION).
        uniform int uHasFineAo;

        // Whether this is the last step, which writes the occlusion O = 1 − access the apply
        // pass expects rather than the access the intermediate steps carry.
        uniform int uFinal;

        // The distance a pixel with no geometry carries — the far plane, which the clear and the
        // skybox leave behind. REQ-SCN-083's scope bullet keeps the skybox and the cleared
        // background out of the effect, and the Depth estimator honours it by dropping those
        // pixels outright (zaxIsEmpty). This port had no such test: its sphere penetrates the
        // silhouette of a nearby object, so an empty pixel beside one came out darkened. Under
        // the gain of V2-22 that leak stayed inside the ±1 sRGB level the scope invariant
        // tolerates; calibrating the magnitude brought it into view, and it is a scope breach
        // either way.
        uniform float uFarDistance;

        // Q-095, Q-097: the port's equivalent of the Depth mode's zaxCornerShare — the share of
        // the hemisphere the MiniEngine estimator has to find blocked for the darkening to reach
        // the ceiling « O <= min(0,95 ; 0,3 x ao.strength) » of REQ-SCN-083. The original writes
        // an accessibility it is free to scale, the amendment binds only the ceiling and the
        // proportionality to ao.strength, and the ceiling itself is enforced by the shared apply
        // pass.
        //
        // Calibrated, like the Depth constant, on the figures of the Q-095 answer under the
        // ceiling of AMD-055 v1.4 (Q-097). The anchor is the ratio the reference shows *between
        // the two modes*, which no framing factor can disturb: MSVO stronger than Depth on the
        // ring, 7.1 against 5.2 in mean darkening, a factor of 1.37. Swept over 0.12 to 1.00
        // (see docs/progress/V2-24.md), this value measures **1.355** times the Depth mode's
        // mean darkening on « Punch-Out!! » — 9.22 against 6.81 — and overshoots the
        // framing-adjusted absolute target by the same 6 % the Depth mode does, so the two modes
        // are calibrated consistently rather than each to its own best fit. The darkest 1 % of
        // the frame reads 0.11 against the reference's 0.16.
        //
        // Note that this is near the accessibility the port writes unscaled: V2-23 had to divide
        // by 0.12 to get any magnitude at all under the old ceiling of 0.3, and under the ceiling
        // v1.4 states the MiniEngine estimator very nearly carries the reference's magnitude on
        // its own. What that lot read as a missing gain was the ceiling clipping the port.
        const float zaxMsvoShare = 0.75;

        void main()
        {
            float fine = texture(uFineDepth, vUv).r;

            // The tolerance of the original, expressed on a depth kept in scene units: a tap
            // whose depth is within a hundredth of this pixel's distance is a good match.
            float tolerance = (0.01 * fine) + 1e-3;

            float total = 0.0;
            float weight = 0.0;
            for (int y = -1; y <= 1; y++)
            {
                for (int x = -1; x <= 1; x++)
                {
                    vec2 uv = vUv + (vec2(float(x), float(y)) * uCoarseTexel);
                    float depth = texture(uCoarseDepth, uv).r;
                    float tent = (x == 0 ? 2.0 : 1.0) * (y == 0 ? 2.0 : 1.0);
                    float w = tent / (abs(fine - depth) + tolerance);
                    total += texture(uCoarseAo, uv).r * w;
                    weight += w;
                }
            }

            float coarse = weight > 0.0 ? total / weight : 1.0;
            float own = uHasFineAo != 0 ? texture(uFineAo, vUv).r : 1.0;
            float access = clamp(own * coarse, 0.0, 1.0);

            // The intermediate steps carry the accessibility the next one multiplies into; only
            // the last one turns it into the occlusion the apply pass reads, and that is where
            // the gain belongs — scaling an intermediate level would compound across levels.
            // The buffer is R8, so anything over 1 clamps at the ceiling.
            float occlusion = fine >= uFarDistance ? 0.0 : (1.0 - access) / zaxMsvoShare;
            oColor = vec4(uFinal != 0 ? occlusion : access, 0.0, 0.0, 1.0);
        }
        """;
}
