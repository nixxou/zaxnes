in vec3 vScene;
in vec3 vNormal;
in vec2 vUv;
out vec4 oColor;

// ZAX_BEND: the bent position of the vertex (REQ-SCN-067). The shadow map holds the bent
// geometry (REQ-SCN-068), so the lookup has to be made at the bent position; the clipping of
// REQ-SCN-036 keeps reading vScene, the position before the curvature.
#ifdef ZAX_BEND
in vec3 vBent;
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
