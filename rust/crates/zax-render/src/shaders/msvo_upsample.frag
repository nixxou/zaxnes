// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// The **bilateral upsample** of `AoBlurAndUpsampleCS.hlsli`, the step that makes the estimate
// multi-scale (AMD-055 REQ-SCN-083 *Mode MSVO*: "l'occultation est estimée à plusieurs résolutions
// de la profondeur puis combinée"): the coarse level is carried into the finer one with weights
// taken from how well the two depths agree, then **multiplied** by the finer level's own occlusion.
// Run from the coarsest pair down to the finest, each step folding everything coarser into the
// level below it.
//
// The **last** step is also where the access becomes the occlusion `O` the composite reads, and
// where the ceiling of REQ-SCN-083 is applied — the same convention `ao.frag` follows for the Depth
// estimator, so that `ao_composite.frag` stays a bare `1 − O` shared by both modes. Scaling an
// intermediate level instead would compound the gain across levels.

in vec2 vUv;
out vec4 oValue;

uniform sampler2D uFineDepth;
uniform sampler2D uCoarseDepth;
uniform sampler2D uCoarseAo;
uniform sampler2D uFineAo;

// The part of each of the two levels the frame drew into, and one texel of the coarse one in
// viewport coordinates.
uniform vec2 uFineScale;
uniform vec2 uCoarseScale;
uniform vec2 uCoarseTexel;

// Whether this is the last step, which writes `O` rather than the access the earlier steps carry.
uniform int uFinal;

// `style::MSVO_SHARE`, the share of the hemisphere the estimator has to find blocked for the
// darkening to reach the ceiling. One uniform rather than a constant repeated here, so that the
// calibration of Q-095 and Q-097 lives in exactly one place.
uniform float uShare;

// `style::ao_ceiling`, the ceiling `O ≤ min(0,95 ; 0,3 × ao.strength)` of REQ-SCN-083.
uniform float uCeiling;

// The distance a pixel with no geometry carries — the far plane, which the clear and the skybox
// leave behind. REQ-SCN-083's scope bullet keeps the skybox and the cleared background out of the
// effect, and the sphere test cannot honour it on its own: a sphere centred on an empty pixel
// beside a near object penetrates that object's silhouette and comes out darkened. A hair under
// the far plane is the test, so that the interpolation of the pyramid cannot drag a background
// pixel back in.
uniform float uFarDistance;

// `style::MSVO_ACCESS_HEADROOM`: the access of every level is stored divided by this, so that a
// value above 1 survives the `R8` targets (see the end of `msvo_occlude.frag`). This pass therefore
// decodes what it reads and re-encodes what the next step will read.
uniform float uHeadroom;

void main()
{
    float fine = texture(uFineDepth, vUv * uFineScale).r;

    // The tolerance of the original, expressed on a depth kept in scene units: a tap whose depth
    // is within a hundredth of this pixel's distance is a good match.
    float tolerance = (0.01 * fine) + 1e-3;

    float total = 0.0;
    float weight = 0.0;
    for (int y = -1; y <= 1; ++y)
    {
        for (int x = -1; x <= 1; ++x)
        {
            vec2 uv = vUv + (vec2(float(x), float(y)) * uCoarseTexel);
            float depth = texture(uCoarseDepth, uv * uCoarseScale).r;
            float tent = ((x == 0) ? 2.0 : 1.0) * ((y == 0) ? 2.0 : 1.0);
            float w = tent / (abs(fine - depth) + tolerance);
            total += texture(uCoarseAo, uv * uCoarseScale).r * w;
            weight += w;
        }
    }

    float coarse = ((weight > 0.0) ? (total / weight) : (1.0 / uHeadroom)) * uHeadroom;
    float own = texture(uFineAo, vUv * uFineScale).r * uHeadroom;
    float access = own * coarse;

    // Only the last step turns the access into the occlusion `O`, and only there is it bounded:
    // an access above 1 is "less than half occluded", which the subtraction reads as no
    // darkening, and the ceiling of REQ-SCN-083 is applied in the same expression.
    float occlusion = (fine >= uFarDistance)
        ? 0.0
        : clamp((1.0 - access) / max(uShare, 1e-4), 0.0, 1.0);
    oValue = vec4(
        (uFinal != 0) ? (uCeiling * occlusion) : (clamp(access, 0.0, uHeadroom) / uHeadroom),
        0.0,
        0.0,
        1.0);
}
