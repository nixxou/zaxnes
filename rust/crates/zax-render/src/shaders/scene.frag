in vec3 vScene;
in vec3 vNormal;
// ZAX_CENTROID (AMD-058): the qualifier of scene.vert, which the two stages must agree on.
#ifdef ZAX_CENTROID
centroid in vec2 vUv;
#else
in vec2 vUv;
#endif
out vec4 oColor;

// ZAX_BEND: the bent position of the vertex (REQ-SCN-067). The shadow map holds the bent
// geometry (REQ-SCN-068), so the lookup has to be made at the bent position; the clipping of
// REQ-SCN-036 keeps reading vScene, the position before the curvature.
#ifdef ZAX_BEND
in vec3 vBent;
#endif

// AMD-055 REQ-SCN-081, REQ-SCN-082: the position and the normal in the mesh's local frame, the
// declarations of scene.vert.
#if defined(ZAX_VOXEL_GRID) || defined(ZAX_PIXEL_SHADOW)
in vec3 vLocal;
in vec3 vLocalNormal;
#endif

// AMD-055 REQ-SCN-082: the placement of scene.vert, which the two stages must agree on.
#ifdef ZAX_PIXEL_SHADOW
flat in mat3 vModelLinear;
flat in vec3 vDeformTile;
#endif

// 0 = indexed atlas, 1 = direct colour atlas, 2 = flat colour (backdrop).
uniform int uColorMode;
uniform usampler2D uIndexTexture;
uniform sampler2D uColorTexture;
uniform sampler2D uPalette;
// The same palette, linear (gl/linear_palette.rs); ZAX_SRGB_PALETTE linearises uPalette
// per fragment instead.
uniform sampler2D uLinearPalette;
uniform int uPaletteTable;
uniform vec2 uTextureSize;
uniform vec2 uTexOffset;
uniform vec2 uTexTilting;
uniform vec3 uFlatColor;

uniform int uClip;
uniform vec4 uClipRect;

uniform vec3 uLightDirection;
uniform vec3 uLightLinear;
uniform float uAmbient;
uniform float uAlpha;

uniform int uReceiveShadow;
uniform sampler2D uShadowMap;
uniform mat4 uLightViewProjection;
uniform float uShadowBias;
uniform float uNormalOffset;
uniform float uShadowStrength;

void main()
{
    // REQ-SCN-036: a fragment whose scene (x, y) leaves R is dropped, whatever its depth.
    // ZAX_SOLID: the variant for the opaque draws that neither clip nor read a colour texture,
    // the two cases that discard. A shader that may discard keeps a tiled GPU (Mali) from
    // dropping the fragments hidden behind it early.
#ifndef ZAX_SOLID
    if (uClip != 0 && (vScene.x < uClipRect.x || vScene.x > uClipRect.z
                    || vScene.y < uClipRect.y || vScene.y > uClipRect.w))
    {
        discard;
    }
#endif

    // The albedo, linear: the indexed one comes linearised from uLinearPalette.
    vec3 albedoLinear;
    float texelAlpha = 1.0;
    if (uColorMode == 2)
    {
        albedoLinear = zaxToLinear3(uFlatColor);
    }
    else
    {
        // REQ-E3D-563: nearest neighbour, no mipmap, texel coordinates taken modulo
        // the texture size.
        vec2 uv = vec2(
            vUv.x * uTexTilting.x + uTexOffset.x * uTextureSize.x,
            vUv.y * uTexTilting.y + uTexOffset.y * uTextureSize.y);
        ivec2 texel = ivec2(mod(floor(uv), uTextureSize));
        if (uColorMode == 0)
        {
            // REQ-SCN-034: the index selects 4 * P + (t mod 4) in the frame palette.
            uint value = texelFetch(uIndexTexture, texel, 0).r;
            int entry = clamp(uPaletteTable, 0, 15) * 4 + int(value & 3u);
#ifdef ZAX_SRGB_PALETTE
            albedoLinear = zaxToLinear3(texelFetch(uPalette, ivec2(entry, 0), 0).rgb);
#else
            albedoLinear = texelFetch(uLinearPalette, ivec2(entry, 0), 0).rgb;
#endif
        }
        else
        {
            vec4 direct = texelFetch(uColorTexture, texel, 0);
            // A pixel of the 2D picture left out (interop::HIDDEN_PIXEL) is not drawn.
#ifndef ZAX_SOLID
            if (direct.a == 0.0)
            {
                discard;
            }
#endif
            albedoLinear = zaxToLinear3(direct.rgb);
            texelAlpha = direct.a;
        }
    }

#ifdef ZAX_VOXEL_GRID
    // REQ-SCN-081: the grid modulates the **albedo**, before any lighting calculation
    // (REQ-SCN-028). For a tile the position is the one in the mesh's local frame; for the
    // backdrop — colour mode 2, which only reaches this shader when `render.backdrop_lighting`
    // is off, since otherwise backdrop.frag draws it — it is the scene position of REQ-SCN-081's
    // second paragraph. The test is uniform over a whole draw, not a per-fragment switch between
    // having the effect and not having it: that one is the #define.
    albedoLinear *= zaxVoxelGrid(uColorMode == 2 ? vScene : vLocal,
        normalize(vLocalNormal), uVoxelCell);
#endif

    vec3 n = normalize(vNormal);
    vec3 l = normalize(-uLightDirection);
    float ndl = max(0.0, dot(n, l));

    float shadow = 1.0;
    if (uReceiveShadow != 0)
    {
        // REQ-SCN-031: a normal offset and a depth bias of about 0.02 unit.
#ifdef ZAX_BEND
        vec3 biased = vBent + n * uNormalOffset;
#else
        vec3 biased = vScene + n * uNormalOffset;
#endif
#ifdef ZAX_PIXEL_SHADOW
        // REQ-SCN-082: on an opaque **tile**, the shadow is read at the centre of the fragment's
        // voxel instead. The bias and the normal offset of REQ-SCN-031 are applied to that
        // displaced point and not to the fragment, or the shadow acne REQ-SCN-031 avoids comes
        // back (REQ-SCN-082, Notes). Colour mode 2 is the backdrop drawn through this shader —
        // `render.backdrop_lighting` off — which the requirement leaves alone ("sans objet sur le
        // backdrop"); the test is a uniform, constant over the whole draw, as the grid's is. The
        // HUD and the transparent objects carry `uReceiveShadow` = 0, so they are already out.
        if (uColorMode != 2)
        {
            biased = zaxVoxelShadowPoint(vScene, vLocal, normalize(vLocalNormal), vModelLinear,
                vDeformTile) + n * uNormalOffset;
        }
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

    // REQ-SCN-028: albedo_lin * (ambient_lin + max(0, N.L) * light_lin * S).
    // R2-009: written linear, so that the blending of REQ-SCN-040 is linear too.
    vec3 lit = albedoLinear * (vec3(uAmbient) + ndl * uLightLinear * shadow);
    oColor = vec4(min(lit, vec3(1.0)), uAlpha * texelAlpha);
}
