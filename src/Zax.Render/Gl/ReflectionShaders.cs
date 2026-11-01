namespace Zax.Render.Gl;

/// <summary>
/// The GLSL of the screen-space floor reflections of SPEC-06 REQ-SCN-084 (AMD-055): four
/// fragment passes — the ray trace, two separable blur passes, and the linear blend into the
/// scene.
/// </summary>
/// <remarks>
/// <para>
/// REQ-SCN-084 states an <em>intention</em>, normative parameters and verifiable invariants, and
/// leaves the algorithm free; it names, as an orienting and <b>non-normative</b> reference, the
/// screen-space reflected-ray tracing of <b>McGuire and Mara, « Efficient GPU Screen-Space Ray
/// Tracing », JCGT 3(4), 2014</b> — a <b>linear march</b> along the ray followed by a
/// <b>bisection</b> around the first depth crossing. That is the shape followed here. The frame
/// and the progression law of the march, the number and the form of the refinements, the exact
/// shape and width of the two attenuations and the shape of the blur kernel are the free choices
/// the requirement leaves; the parameters and the invariants are what it binds.
/// </para>
/// <para>
/// The march is run in <b>view space</b>, one uniform step of world length at a time, each step
/// projected to the screen — that is the « repère et loi de progression » the amendment leaves
/// open, and it keeps the maximum ray length of the requirement expressed directly in scene
/// units.
/// </para>
/// <para>
/// A ray that crosses no surface produces no reflection. The skybox is in the resolved colour
/// and would be taken like anything else, but it is at infinite distance behind the whole scene
/// (REQ-SCN-080, fourth bullet, REQ-SCN-049), so a ray of finite maximum length never crosses
/// it: in practice the sky is not reflected (<b>Q-093</b>).
/// </para>
/// <para>
/// OpenGL 3.3 has no compute shaders (REQ-SCN-056), so each pass is a full-screen triangle.
/// </para>
/// </remarks>
// Q-093
internal static class ReflectionShaders
{
    /// <summary>Half the number of taps of one separable blur pass.</summary>
    public const int BlurTaps = 4;

    /// <summary>
    /// The trace pass: for every floor pixel, the reflected ray, its hit colour and the
    /// intensity the two attenuations leave, plus the distance travelled, which the blur reads.
    /// </summary>
    public static string TraceFragment { get; } = $$"""
        #version 330 core
        in vec2 vUv;

        // The reflection, premultiplied by its intensity, and that intensity in a: premultiplied
        // so that the blur below can mix a reflecting pixel with a non-reflecting one without
        // dragging a black fringe into it.
        layout(location = 0) out vec4 oReflection;

        // The distance the ray travelled, as a fraction of the maximum length: the blur radius
        // of REQ-SCN-084 grows linearly with it.
        layout(location = 1) out vec4 oTravel;

        {{SceneShaders.ScreenDepth}}

        uniform sampler2D uColor;
        uniform sampler2D uFloor;
        uniform mat4 uProjection;

        // Scene frame to view frame, for the normal the mask carries. The scene frame is
        // left-handed and the clip position mirrors z (REQ-SCN-001), so the mirror is folded in.
        uniform mat3 uViewNormal;

        // 40 x (2 x ssr.length - 1) x scene scale units of scene (REQ-SCN-084).
        uniform float uMaxLength;

        // 0.045 x (2 x ssr.shine - 1): the share of the reflected colour mixed in (v1.5).
        uniform float uIntensity;

        uniform int uSteps;
        uniform int uRefinements;

        // The progression of the march: the first span, in scene units, and the factor the span
        // grows by at every step. Both depend on the quality and the scene scale alone, never on
        // ssr.length — see the note in ReflectionPass.
        uniform float uStepBase;
        uniform float uStepRatio;

        // Projects a view-space point; returns false when it falls behind the eye or off screen.
        bool zaxProject(vec3 at, out vec2 uv)
        {
            vec4 clip = uProjection * vec4(at, 1.0);
            uv = vec2(0.0);
            if (clip.w <= 0.0)
            {
                return false;
            }

            uv = (clip.xy / clip.w) * 0.5 + 0.5;
            return uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
        }

        void main()
        {
            oReflection = vec4(0.0);
            oTravel = vec4(0.0, 0.0, 0.0, 1.0);

            // REQ-SCN-084: only a floor surface carries a reflection. The mask was written by
            // the opaque pass, so the transparent objects and the HUD never appear in it.
            vec4 mask = texture(uFloor, vUv);
            if (mask.a < 0.5)
            {
                return;
            }

            float raw;
            vec3 p = zaxViewPosition(vUv, raw);
            if (zaxIsEmpty(raw))
            {
                return;
            }

            vec3 n = normalize(uViewNormal * (mask.rgb * 2.0 - 1.0));

            // The camera looks down -z in view space, so a face turned towards it has n.p < 0.
            n = dot(n, p) > 0.0 ? -n : n;

            vec3 d = reflect(normalize(p), n);
            if (dot(d, d) < 1e-12)
            {
                return;
            }

            d = normalize(d);

            // The ray starts just off the surface, so the first step does not read the floor
            // it leaves.
            vec3 origin = p + n * (0.05 * uStepBase);

            bool hit = false;
            float lo = 0.0;
            float hi = 0.0;
            float bias = 0.02;
            float previous = 0.0;
            float span = uStepBase;
            float t = 0.0;
            for (int i = 0; i < uSteps; i++)
            {
                t += span;
                span *= uStepRatio;

                // REQ-SCN-084: the ray is at most uMaxLength long. The sample positions
                // themselves do not depend on that length, so lengthening the ray can only add
                // samples at the far end and can never move the ones already taken: a longer
                // ray therefore only ever *extends* the reflected zone.
                if (t > uMaxLength)
                {
                    break;
                }

                // How far behind a drawn surface the ray has to be to count as having crossed
                // it, and how far behind it may be and still be taken as touching the same
                // surface: the free « épaisseur » of a screen-space trace, scaled by the span
                // of the step that crossed, since that is the resolution the march has there.
                float stepSpan = t - previous;
                float stepBias = max(0.02 * stepSpan, 0.02);
                float thickness = (2.0 * stepSpan) + 1.0;

                vec2 uv;
                if (!zaxProject(origin + d * t, uv))
                {
                    break;
                }

                float surfaceRaw;
                float surface = zaxViewPosition(uv, surfaceRaw).z;
                if (!zaxIsEmpty(surfaceRaw))
                {
                    // In view space z grows towards the camera, so a positive difference means
                    // the ray has passed *behind* the surface drawn at that pixel.
                    float delta = surface - (origin + d * t).z;
                    if (delta > stepBias && delta < thickness)
                    {
                        hit = true;
                        bias = stepBias;
                        lo = previous;
                        hi = t;
                        break;
                    }
                }

                previous = t;
            }

            if (!hit)
            {
                return;
            }

            // The refinement of McGuire and Mara: a bisection on the same crossing test.
            for (int r = 0; r < uRefinements; r++)
            {
                float mid = 0.5 * (lo + hi);
                vec2 uv;
                if (!zaxProject(origin + d * mid, uv))
                {
                    hi = mid;
                    continue;
                }

                float surfaceRaw;
                float surface = zaxViewPosition(uv, surfaceRaw).z;
                if (zaxIsEmpty(surfaceRaw) || surface - (origin + d * mid).z <= bias)
                {
                    lo = mid;
                }
                else
                {
                    hi = mid;
                }
            }

            vec2 hitUv;
            if (!zaxProject(origin + d * hi, hitUv))
            {
                return;
            }

            // REQ-SCN-084: the intensity falls continuously to 0 as the ray approaches its
            // maximum length and as its projection approaches the edge of the screen. Neither
            // width nor shape is fixed; what is forbidden is a hard edge.
            //
            // The fall-off spans the *whole* ray, not its last quarter. The Intention of
            // REQ-SCN-084 says the reflection « s'estompe avec la distance parcourue » and its
            // review criterion « se brouille et s'efface avec la distance »: a factor that stays
            // at 1 until three quarters of the maximum length honours the letter of the
            // normative bullet and nothing of either, since at the default ssr.length nothing
            // in a game frame travels 150 units. The shape is a free choice; a smoothstep over
            // [0, 1] keeps the contact sharp (zero slope at the origin) and the far end soft.
            // AMD-055 v1.5 keeps this whole-ray fall-off and answers the rest of Q-099 by
            // halving the intensity constant itself (see SceneState.ReflectionIntensity).
            float fadeLength = 1.0 - smoothstep(0.0, 1.0, hi / uMaxLength);
            vec2 border = min(hitUv, vec2(1.0) - hitUv);
            float fadeEdge = smoothstep(0.0, 0.12, min(border.x, border.y));

            float intensity = uIntensity * fadeLength * fadeEdge;
            vec3 reflected = texture(uColor, hitUv).rgb;

            oReflection = vec4(reflected * intensity, intensity);
            oTravel = vec4(clamp(hi / uMaxLength, 0.0, 1.0), 0.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// One pass of the separable blur: a tent kernel of
    /// <c>2 × <see cref="BlurTaps"/> + 1</c> taps spread over the radius this pixel's travelled
    /// distance calls for. At zero travel every tap falls on the centre, so a pixel that
    /// reflects nothing is left exactly alone.
    /// </summary>
    public static string BlurFragment { get; } = $$"""
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uSource;
        uniform sampler2D uTravel;

        // One texel of this pass along the axis being blurred.
        uniform vec2 uStep;

        // The radius at the maximum ray length, in pixels of this pass:
        // 0.003 x (10 - (2 x ssr.clarity - 1)) of the screen height (REQ-SCN-084).
        uniform float uMaxRadius;

        void main()
        {
            // REQ-SCN-084: the radius grows *linearly* with the distance the ray travelled,
            // from 0 at the point of origin to uMaxRadius at the maximum length.
            float radius = uMaxRadius * texture(uTravel, vUv).r;

            vec4 total = vec4(0.0);
            float weight = 0.0;
            for (int i = -{{BlurTaps}}; i <= {{BlurTaps}}; i++)
            {
                float offset = radius * float(i) / float({{BlurTaps}});
                float w = float({{BlurTaps}} + 1 - abs(i));
                total += texture(uSource, vUv + uStep * offset) * w;
                weight += w;
            }

            oColor = total / weight;
        }
        """;

    /// <summary>
    /// The blend pass: <c>(1 − i) × surface + i × reflection</c> in <b>linear</b> space and
    /// before the sRGB encode of REQ-SCN-028, written by a full-screen triangle whose blend does
    /// the mix.
    /// </summary>
    /// <remarks>
    /// The reflection arrives premultiplied, so the blend is <c>ONE, ONE_MINUS_SRC_ALPHA</c> on
    /// the colour and <c>ZERO, ONE</c> on the alpha: the destination alpha is never a colour
    /// (REQ-SCN-035) and must come out untouched. Mixing the samples of a multisampled target
    /// with one factor per pixel is the same as mixing the resolved colour, the resolve being an
    /// average, so the pass works on resolved values as REQ-SCN-080 asks without giving up the
    /// depth the transparent objects still need.
    /// </remarks>
    public const string ApplyFragment = """
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uReflection;

        void main()
        {
            oColor = texture(uReflection, vUv);
        }
        """;
}
