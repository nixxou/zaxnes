// AMD-049 REQ-SCN-075: the backdrop of REQ-SCN-048 lit with the almost-matt reflection model of
// the amendment instead of the plain diffuse term of REQ-SCN-028. The CPU twin of this file is
// `color::backdrop_linear`, and a test compares the two.
//
// This is the fragment body of a program of its own, chosen for the backdrop run alone
// (`ScenePrograms`), so no tile fragment ever evaluates any of it and there is no branch on a
// uniform. It shares `scene.vert`, which writes more varyings than this stage reads: the
// backdrop carries one flat colour, so the texel coordinate of REQ-E3D-506 is not read here at
// all — and the interpolation variant of AMD-058 therefore does not apply to this program.

in vec3 vScene;
in vec3 vNormal;
out vec4 oColor;

// ZAX_BEND: the bent position, which the shadow lookup reads (REQ-SCN-068).
#ifdef ZAX_BEND
in vec3 vBent;
#endif

// ZAX_VOXEL_GRID (AMD-055 REQ-SCN-081, §3 point 5): the mesh normal of the box face, which is
// also its **scene** normal — the backdrop's placement is a scale and a translation, with no
// rotation, and a non-uniform scale changes an axis-aligned normal's length but not its
// direction. The grid's position here is `vScene`, the scene position before the curvature.
#ifdef ZAX_VOXEL_GRID
in vec3 vLocalNormal;
#endif

// `Backdrop.Color`, sRGB; the albedo `c` of REQ-SCN-075 is this, linearised.
uniform vec3 uFlatColor;
// The eye, in the scene frame: `V` is the unit vector from the fragment towards it. Only this
// program has it — the tile programs need no eye vector and were not given one.
uniform vec3 uEye;

uniform vec3 uLightDirection;
uniform vec3 uLightLinear;
uniform float uAmbient;

uniform int uReceiveShadow;
uniform sampler2D uShadowMap;
uniform mat4 uLightViewProjection;
uniform float uShadowBias;
uniform float uNormalOffset;
uniform float uShadowStrength;

// REQ-SCN-075: perceptual roughness 0.99, and the 0.04 a non-metallic surface reflects at normal
// incidence — the same 4 % the diffuse part gives up.
const float ZAX_BACKDROP_ROUGHNESS = 0.99;
const float ZAX_BACKDROP_REFLECTANCE = 0.04;
const float ZAX_PI = 3.14159265358979323846;

float zaxPow5(float value)
{
    float square = value * value;
    return square * square * value;
}

float zaxBackdropFresnel(float f90, float cosine)
{
    return 1.0 + (f90 - 1.0) * zaxPow5(1.0 - cosine);
}

void main()
{
    // REQ-SCN-048: the backdrop is never clipped by the visible rectangle, and it is opaque, so
    // this stage never rejects a fragment — which is what lets a tiled GPU drop what hides
    // behind it early.
    vec3 n = normalize(vNormal);
    vec3 v = normalize(uEye - vScene);
    vec3 l = normalize(-uLightDirection);
    // REQ-SCN-075: H = (L + V) normalised, and H = N when L + V = 0 (R2's point 49-8).
    vec3 sum = l + v;
    vec3 h = dot(sum, sum) > 0.0 ? normalize(sum) : n;

    float nl = max(dot(n, l), 0.0);
    float nv = abs(dot(n, v));
    float nh = max(dot(n, h), 0.0);
    float lh = max(dot(l, h), 0.0);

    // REQ-SCN-031, as in scene.frag: `s` is 1 out of shadow and uShadowStrength in it, and it
    // scales the specular term too — the specular term is direct light, and REQ-SCN-075 states
    // there is no environment reflection.
    float shadow = 1.0;
    if (uReceiveShadow != 0)
    {
#ifdef ZAX_BEND
        vec3 biased = vBent + n * uNormalOffset;
#else
        vec3 biased = vScene + n * uNormalOffset;
#endif
        vec4 light = uLightViewProjection * vec4(biased.x, biased.y, -biased.z, 1.0);
        vec3 projected = light.xyz / light.w * 0.5 + 0.5;
        if (projected.x >= 0.0 && projected.x <= 1.0
            && projected.y >= 0.0 && projected.y <= 1.0
            && projected.z <= 1.0)
        {
            float stored = texture(uShadowMap, projected.xy).r;
            if (projected.z - uShadowBias > stored)
            {
                shadow = uShadowStrength;
            }
        }
    }

    float alpha = ZAX_BACKDROP_ROUGHNESS * ZAX_BACKDROP_ROUGHNESS;

    // The diffuse term.
    float f90 = 0.5 + 2.0 * lh * lh * ZAX_BACKDROP_ROUGHNESS;
    float fd = zaxBackdropFresnel(f90, nl) * zaxBackdropFresnel(f90, nv);

    // The specular term.
    float vis = 0.5 / (nl * (nv * (1.0 - alpha) + alpha)
                     + nv * (nl * (1.0 - alpha) + alpha) + 1.0e-5);
    float denominator = nh * nh * (alpha * alpha - 1.0) + 1.0;
    float d = alpha * alpha / (ZAX_PI * denominator * denominator);
    float spec = vis * d * ZAX_PI * nl;
    float f = ZAX_BACKDROP_REFLECTANCE
        + (1.0 - ZAX_BACKDROP_REFLECTANCE) * zaxPow5(1.0 - lh);

    // AMD-021 R2-009: written linear, like every other colour these shaders produce; the sRGB
    // encode of REQ-SCN-028 is the resolve pass's job. The backdrop is opaque.
    vec3 albedoLinear = zaxToLinear3(uFlatColor);
#ifdef ZAX_VOXEL_GRID
    // AMD-055 §3 point 5: the grid modulates the albedo `c` **before** the model below, so that a
    // groove does not darken the specular term — which is why this multiplies `albedoLinear` and
    // not `lit`. The edges are at the scene coordinates that are multiples of `N × scene scale`.
    albedoLinear *= zaxVoxelGrid(vScene, normalize(vLocalNormal), uVoxelCell);
#endif
    vec3 lit = albedoLinear * (1.0 - ZAX_BACKDROP_REFLECTANCE)
            * (vec3(uAmbient) + uLightLinear * (shadow * fd * nl))
        + uLightLinear * (shadow * spec * f);
    oColor = vec4(min(lit, vec3(1.0)), 1.0);
}
