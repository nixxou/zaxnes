namespace Zax.Render.Gl;

/// <summary>
/// The GLSL of the screen-space ambient occlusion of SPEC-06 REQ-SCN-083 (AMD-055): three
/// fragment passes — occlusion, depth-aware blur, and the linear multiply into the scene.
/// </summary>
/// <remarks>
/// <para>
/// REQ-SCN-083 states an <em>intention</em>, normative parameters and verifiable invariants, and
/// leaves the algorithm free; it names, as an orienting and <b>non-normative</b> reference, the
/// hemisphere screen-space ambient occlusion described publicly by <b>John Chapman (2013)</b>.
/// That is the shape followed here: a kernel of samples drawn in the hemisphere of the normal
/// and packed towards the point, a per-pixel rotation of the kernel, a range check that drops
/// samples too far away in depth, then a blur. The weighting law, the sample distribution and
/// the blur radius are the free choices the requirement leaves; the parameters and the
/// invariants are what it binds.
/// </para>
/// <para>
/// OpenGL 3.3 has no compute shaders (REQ-SCN-056), so each pass is a full-screen triangle. The
/// occlusion reads the <b>resolved</b> depth of the scene (REQ-SCN-080, fourth bullet) and
/// reconstructs the view-space position and a flat normal from it: there is no normal buffer to
/// read, the scene pass writing colour alone.
/// </para>
/// </remarks>
internal static class OcclusionShaders
{
    /// <summary>The highest number of kernel samples a quality level asks for.</summary>
    public const int MaxSamples = 16;

    /// <summary>
    /// Reconstruction of the view-space geometry from the depth texture, shared by the
    /// occlusion and the blur.
    /// </summary>
    private const string Reconstruct = """
        uniform sampler2D uDepth;
        uniform mat4 uInverseProjection;
        uniform vec2 uDepthTexel;

        // The view-space position of the pixel at uv, from its window-space depth.
        vec3 zaxViewPosition(vec2 uv, out float raw)
        {
            raw = texture(uDepth, uv).r;
            vec4 clip = vec4((uv * 2.0 - 1.0), raw * 2.0 - 1.0, 1.0);
            vec4 view = uInverseProjection * clip;
            return view.xyz / view.w;
        }

        vec3 zaxViewPosition(vec2 uv)
        {
            float raw;
            return zaxViewPosition(uv, raw);
        }

        // Whether the pixel holds no geometry at all: the far plane, which the clear and the
        // skybox leave behind (REQ-SCN-049). Those pixels are never occluded.
        bool zaxIsEmpty(float raw)
        {
            return raw >= 0.999999;
        }
        """;

    /// <summary>
    /// The occlusion pass: <c>O</c> of REQ-SCN-083, written to the red channel of a buffer that
    /// is half the size of the scene except at the High quality.
    /// </summary>
    public static string OcclusionFragment { get; } = $$"""
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        {{Reconstruct}}

        // The kernel of REQ-SCN-083, built on the CPU once: unit-hemisphere directions about
        // +Z, already packed towards the origin, so that the samples near the point weigh more
        // than the ones at the rim (Chapman 2013).
        uniform vec4 uKernel[{{MaxSamples}}];
        uniform int uSamples;

        // The radius r of REQ-SCN-083, in scene units — which are view units, the view matrix
        // of REQ-SCN-007 being a rigid transform.
        uniform float uRadius;
        uniform mat4 uProjection;

        void main()
        {
            float raw;
            vec3 p = zaxViewPosition(vUv, raw);
            if (zaxIsEmpty(raw))
            {
                // No geometry here: O = 0, so the skybox and the cleared background keep their
                // colour (REQ-SCN-083, scope).
                oColor = vec4(0.0);
                return;
            }

            // The flat normal of the pixel, from two neighbours of the depth texture. Taken on
            // the side where the depth is continuous, so that a silhouette does not tilt it.
            vec3 right = zaxViewPosition(vUv + vec2(uDepthTexel.x, 0.0));
            vec3 left = zaxViewPosition(vUv - vec2(uDepthTexel.x, 0.0));
            vec3 up = zaxViewPosition(vUv + vec2(0.0, uDepthTexel.y));
            vec3 down = zaxViewPosition(vUv - vec2(0.0, uDepthTexel.y));
            vec3 dx = abs(right.z - p.z) < abs(left.z - p.z) ? right - p : p - left;
            vec3 dy = abs(up.z - p.z) < abs(down.z - p.z) ? up - p : p - down;

            vec3 n = cross(dx, dy);
            if (dot(n, n) < 1e-12)
            {
                oColor = vec4(0.0);
                return;
            }

            n = normalize(n);

            // The camera looks down −z in view space, so a face turned towards it has n·p < 0.
            n = dot(n, p) > 0.0 ? -n : n;

            // The per-pixel rotation of the kernel: a direction of the tangent plane taken from
            // a hash of the pixel, so the pattern changes from pixel to pixel but never from
            // frame to frame — a still camera gives a still image (REQ-SCN-083, review).
            vec2 pixel = floor(vUv / uDepthTexel);
            float hash = fract(sin(dot(pixel, vec2(12.9898, 78.233))) * 43758.5453);
            float angle = hash * 6.2831853;
            vec3 random = vec3(cos(angle), sin(angle), 0.0);

            vec3 tangent = random - n * dot(random, n);
            if (dot(tangent, tangent) < 1e-6)
            {
                tangent = abs(n.x) < 0.9 ? vec3(1.0, 0.0, 0.0) : vec3(0.0, 1.0, 0.0);
                tangent = tangent - n * dot(tangent, n);
            }

            tangent = normalize(tangent);
            mat3 basis = mat3(tangent, cross(n, tangent), n);

            // A bias along the normal, a fraction of the radius: without it a flat surface
            // occludes itself, and the first invariant of §2.4 wants a lone plane untouched.
            float bias = 0.04 * uRadius;

            float occlusion = 0.0;
            for (int i = 0; i < uSamples; i++)
            {
                vec3 at = p + (basis * uKernel[i].xyz) * uRadius;

                vec4 clip = uProjection * vec4(at, 1.0);
                if (clip.w <= 0.0)
                {
                    continue;
                }

                vec2 uv = (clip.xy / clip.w) * 0.5 + 0.5;
                if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0)
                {
                    continue;
                }

                float sampleRaw;
                float sampled = zaxViewPosition(uv, sampleRaw).z;
                if (zaxIsEmpty(sampleRaw))
                {
                    continue;
                }

                // The occluder is in front of the sample: in view space z grows towards the
                // camera, so "in front" is a larger z.
                float inFront = step(at.z + bias, sampled);

                // The range check of Chapman 2013: an occluder farther than the radius in depth
                // is not part of this point's neighbourhood. It is what makes O nil wherever no
                // occluding geometry is within 2 r (REQ-SCN-083).
                float range = smoothstep(0.0, 1.0, uRadius / max(1e-4, abs(p.z - sampled)));
                occlusion += inFront * range;
            }

            oColor = vec4(occlusion / float(uSamples), 0.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// The blur pass: a box that rejects the taps whose depth is too far from the centre's, so
    /// the occlusion is smoothed without crossing a silhouette (REQ-SCN-083: « un lissage qui
    /// respecte les discontinuités de profondeur »).
    /// </summary>
    public static string BlurFragment { get; } = $$"""
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        {{Reconstruct}}

        uniform sampler2D uOcclusion;
        uniform vec2 uTexel;
        uniform int uRadiusPixels;

        void main()
        {
            float raw;
            vec3 p = zaxViewPosition(vUv, raw);
            if (zaxIsEmpty(raw))
            {
                oColor = vec4(0.0);
                return;
            }

            // A tap is kept while its depth is within a twentieth of the centre's distance:
            // enough to smooth the noise of the kernel, little enough to stop at a silhouette.
            float tolerance = max(0.5, 0.05 * abs(p.z));
            float total = 0.0;
            float weight = 0.0;
            for (int y = -uRadiusPixels; y <= uRadiusPixels; y++)
            {
                for (int x = -uRadiusPixels; x <= uRadiusPixels; x++)
                {
                    vec2 uv = vUv + vec2(float(x), float(y)) * uTexel;
                    float tapRaw;
                    vec3 tap = zaxViewPosition(uv, tapRaw);
                    if (zaxIsEmpty(tapRaw) || abs(tap.z - p.z) > tolerance)
                    {
                        continue;
                    }

                    total += texture(uOcclusion, uv).r;
                    weight += 1.0;
                }
            }

            oColor = vec4(weight > 0.0 ? total / weight : texture(uOcclusion, vUv).r, 0.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// The apply pass: the scene colour is multiplied by <c>1 − O</c>, in <b>linear</b> space
    /// and before the sRGB encode of REQ-SCN-028, by a full-screen triangle blended
    /// <c>dst × src</c>.
    /// </summary>
    /// <remarks>
    /// Multiplying the samples of a multisampled target by one factor per pixel is the same as
    /// multiplying the resolved colour, the resolve being an average: the pass therefore works
    /// on the resolved colour as REQ-SCN-080 asks, without having to resolve it in the middle of
    /// the frame and give up the depth test the transparent objects still need.
    /// </remarks>
    public static string ApplyFragment { get; } = """
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uOcclusion;

        // The darkening ceiling of REQ-SCN-083: O <= 0.1 x ao.strength in every pixel.
        uniform float uCeiling;

        void main()
        {
            float occlusion = clamp(texture(uOcclusion, vUv).r, 0.0, 1.0) * uCeiling;

            // The alpha of the target is never a colour (REQ-SCN-035), so it is multiplied by 1.
            oColor = vec4(vec3(1.0 - occlusion), 1.0);
        }
        """;
}
