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
    /// Reconstruction of the view-space geometry from the depth texture, shared with the
    /// reflections of REQ-SCN-084. The pixels <c>zaxIsEmpty</c> rejects — the far plane the
    /// clear and the skybox leave behind — are never occluded.
    /// </summary>
    private const string Reconstruct = SceneShaders.ScreenDepth;

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

        // Q-095: the share of the kernel this estimator has to find blocked for the darkening to
        // reach the ceiling « O <= 0.1 x ao.strength » of REQ-SCN-083. That is the law the
        // requirement leaves free: O is the blocked share of the hemisphere, scaled so that an
        // ordinary deep corner approaches the ceiling. Lot V2-22 set it at the share measured at
        // a right-angle corner, 0.35; the answer to Q-095 measured the reference and found the
        // result still about three times too weak in mean darkening at comparable area, so the
        // constant is calibrated on those figures instead — 0.06, which is nearly six times the
        // gain, and brings the contact of close-up A to the ceiling.
        //
        // What bounds it is the second invariant of §2.4: the contact pixel of close-up A must
        // keep a ratio « supérieur ou égal à 0,70 » to its unoccluded value — and 0.70 *is* the
        // ceiling at the default ao.strength = 3, so a contact that reaches the ceiling measures
        // 0.69850 once the capture has been through the 8-bit sRGB the amendment's own method
        // prescribes. Read without a tolerance, that invariant forbids ever reaching the ceiling.
        // It cannot be meant that way: the MSVO mode the amendment mandates in v1.2 saturates
        // that same contact at *every* gain, its own included, so the literal reading would make
        // the mandated port non-conformant on arrival. The tolerance of one sRGB level — the one
        // the MSVO invariants have carried since V2-20 — is therefore the reading taken, and it
        // is taken in both modes alike. Q-097 asks R1 to confirm it.
        // Q-097
        const float zaxCornerShare = 0.06;

        // How far above the tangent plane of the point an occluder must rise before it counts at
        // all, as the sine of the angle it makes with that plane — about 6 degrees. It leaves a
        // surface unable to darken itself whatever the distance to the occluder.
        const float zaxTangentSine = 0.1;

        // ... and how much further it must rise to count in full. Q-095: a *hard* threshold on
        // that sine is what turned the residual error of the rebuilt normal into the regular
        // horizontal bands lot V2-23 measured on the ring mat. The normal of a flat surface is
        // not exact — rebuilt from a depth texture it scatters by a couple of degrees — so a
        // point of that very surface clears a 0.1 threshold here and not there, and a hard step
        // prints that as an edge. Over a ramp the same scatter costs a few thousandths of
        // occlusion instead of all or nothing, while a true corner or edge, whose occluders rise
        // at sines approaching 1, still counts in full. REQ-SCN-083 leaves « la loi de
        // pondération et d'atténuation de l'occultation » free; this is part of that choice.
        const float zaxRiseRamp = 0.25;

        // One step of the window-space depth, the depth texture being 24 bits. The view position
        // rebuilt from it is a staircase, and the normal rebuilt from two such positions carries
        // a tilt of about one step over the baseline: the test above has to leave room for it,
        // or a flat surface darkens itself wherever that tilt crosses zaxTangentSine — the
        // regular horizontal bands lot V2-23 measured on the ring mat (Q-095).
        const float zaxDepthStep = 1.0 / 16777216.0;

        // How many of those tilts to allow for: one for the point's own position and one for the
        // neighbour's, the difference of the two carrying both.
        const float zaxTiltSlack = 2.0;

        // The view-space position the depth texture would give at uv for a window depth of raw.
        vec3 zaxViewAt(vec2 uv, float raw)
        {
            vec4 clip = vec4((uv * 2.0 - 1.0), raw * 2.0 - 1.0, 1.0);
            vec4 view = uInverseProjection * clip;
            return view.xyz / view.w;
        }

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
            // The baseline is two texels rather than one: the view position rebuilt from a
            // 24-bit depth carries a quantisation error that does not shrink with the step, so
            // the shorter the step the more it tilts the normal. Two texels halve that tilt and
            // still follow the geometry closely enough at the half resolution the pass runs at.
            vec2 baseline = uDepthTexel * 2.0;
            vec3 right = zaxViewPosition(vUv + vec2(baseline.x, 0.0));
            vec3 left = zaxViewPosition(vUv - vec2(baseline.x, 0.0));
            vec3 up = zaxViewPosition(vUv + vec2(0.0, baseline.y));
            vec3 down = zaxViewPosition(vUv - vec2(0.0, baseline.y));
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

            // Q-095: how far this normal can be tilted by the quantisation of the depth texture.
            // One step of window depth moves the rebuilt position by « quantum » view units
            // along the view ray; a difference taken over a baseline of length L therefore
            // carries a tilt of up to zaxTiltSlack × quantum / L, and a point of the very same
            // surface then appears to rise above the tangent plane by that much. Adding it to
            // the threshold makes the test scale-free: it costs nothing where the baseline is
            // long compared with the quantum — which is everywhere the surface is resolved —
            // and it is exactly what stops a flat surface from banding where it is not.
            float quantum = length(zaxViewAt(vUv, raw + zaxDepthStep) - p);
            float baseline3d = min(length(dx), length(dy));
            float slack = baseline3d > 1e-6 ? (zaxTiltSlack * quantum) / baseline3d : 1.0;
            float riseSine = min(zaxTangentSine + slack, 0.95);

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
                vec3 occluder = zaxViewPosition(uv, sampleRaw);
                if (zaxIsEmpty(sampleRaw))
                {
                    continue;
                }

                vec3 toOccluder = occluder - p;
                float span = length(toOccluder);
                if (span < 1e-5)
                {
                    continue;
                }

                // The occluder is in front of the sample: in view space z grows towards the
                // camera, so "in front" is a larger z.
                float inFront = step(at.z, occluder.z);

                // ... and it rises above the tangent plane of the point, by more than that plane
                // is itself uncertain. Another point of the same surface lies *in* that plane
                // and so never darkens it, however grazing the view of that surface is: this is
                // what keeps a lone plane untouched, the first invariant of §2.4, without a
                // depth bias that would also eat the darkening at a contact.
                float above = smoothstep(riseSine, riseSine + zaxRiseRamp, dot(toOccluder, n) / span);

                // The range check of Chapman 2013, measured on the distance rather than on the
                // depth alone: an occluder farther than the radius is not part of this point's
                // neighbourhood, and past 2 r it weighs nothing at all, which is what makes O
                // nil wherever no occluding geometry is within 2 r (REQ-SCN-083). The transition
                // is smooth, the effect having no sharp outline.
                float range = 1.0 - smoothstep(uRadius, 2.0 * uRadius, span);
                occlusion += inFront * above * range;
            }

            // Q-095: the raw share of blocked samples, brought to the ceiling at a right-angle
            // contact. The buffer is R8, so values above 1 clamp there; the apply pass scales
            // what is left by « 0.1 x ao.strength » (REQ-SCN-083).
            oColor = vec4(occlusion / (float(uSamples) * zaxCornerShare), 0.0, 0.0, 1.0);
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
