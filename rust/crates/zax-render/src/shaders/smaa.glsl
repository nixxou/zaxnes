// What the three passes of the depth-detecting SMAA of AMD-060 REQ-SCN-085 share: the render
// target's metrics, and the two reads every pass makes.
//
// Translated from `third_party/smaa/SMAA.hlsl` (iryoku/smaa, MIT; notice in
// THIRD-PARTY-NOTICES.md) by reading what it computes. Two things about the translation are worth
// stating here, because neither is recoverable from reading the two files side by side.
//
// 1. **`uRtMetrics.y` is NEGATIVE in OpenGL.** The reference is written for Direct3D, where a
//    texture coordinate's y grows *downward*, and an OpenGL framebuffer's grows upward. The
//    algorithm is **not** symmetric under a y flip: `SMAAArea(dist, e1, e2, offset)` splits a
//    trapezoid's coverage between the two pixels on either side of the edge, so swapping "up" and
//    "down" mirrors the areas and displaces the result along every vertical edge — wrongly, and
//    nearly invisibly. Every place the reference crosses the screen in y does it as
//    `± offset · SMAA_RT_METRICS.y`, so handing it a negative y makes "north" mean +y, which in an
//    OpenGL texture *is* up on screen, and the whole of its arithmetic then runs unchanged. The
//    `wgpu` port passes +1/h for the same reason, and that one number is the only difference
//    between the two backends' sources.
//
//    What follows from it: an integer texel offset (`textureLodOffset`) is in the texture's own
//    space and would **not** pick up the sign, so every offset read below is written in the
//    arithmetic form the reference's own `SMAA_HLSL_3` path uses — `coord + offset · metrics.xy` —
//    and never as `textureLodOffset`.
//
// 2. **The offsets are computed per fragment, not interpolated.** The reference precomputes
//    `offset[3]` in a vertex shader (`SMAAEdgeDetectionVS`, `SMAABlendingWeightCalculationVS`) as
//    an interpolation optimisation. Here every screen pass shares one attribute-less vertex stage
//    (`resolve.vert`, as the passes of AMD-055 REQ-SCN-080 do), so the same `mad` happens in the
//    fragment. Identical arithmetic, one stage instead of four.

// (1/width, ySign/height, width, height) of the part of the attachments the frame **drew into**.
// `SMAA_RT_METRICS`, with the sign of point 1 above.
uniform vec4 uRtMetrics;

// The part of the attachments the frame drew into, in texture coordinates: (1, 1) unless the host
// reserved room for a size that changes often. Every target of this filter is allocated at the
// attachments' capacity and drawn into at the frame's size, exactly as the occlusion's are, so
// every read of one scales by this and every offset above does not.
uniform vec2 uDrawnScale;

// One read of a full-resolution source at a viewport coordinate (0 to 1 over the part drawn into).
vec4 zaxSmaaFetch(sampler2D source, vec2 uv)
{
    return texture(source, uv * uDrawnScale);
}

// The same, `texels` texels away: the reference's `SMAASampleLevelZeroOffset`, written as the
// multiply-add of point 1 so that the y sign carries.
vec4 zaxSmaaFetchOffset(sampler2D source, vec2 uv, vec2 texels)
{
    return texture(source, (uv + (texels * uRtMetrics.xy)) * uDrawnScale);
}
