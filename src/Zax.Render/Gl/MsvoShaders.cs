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
                        zaxTestPair(vec2(x, 0.0), step, center, scale) +
                        zaxTestPair(vec2(0.0, x), step, center, scale));
                }
                else if (x == y)
                {
                    // Diagonal.
                    value = 0.5 * (
                        zaxTestPair(vec2(x, -x), step, center, scale) +
                        zaxTestPair(vec2(x, x), step, center, scale));
                }
                else
                {
                    // L-shaped.
                    value = 0.25 * (
                        zaxTestPair(vec2(x, y), step, center, scale) +
                        zaxTestPair(vec2(x, -y), step, center, scale) +
                        zaxTestPair(vec2(y, x), step, center, scale) +
                        zaxTestPair(vec2(y, -x), step, center, scale));
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

            oColor = vec4(uFinal != 0 ? 1.0 - access : access, 0.0, 0.0, 1.0);
        }
        """;
}
