// What the three passes of the depth-detecting SMAA of AMD-060 REQ-SCN-085 share on this backend:
// the render target's metrics, the bindings, the full-screen triangle and the reads every pass
// makes. The WGSL twin of shaders/smaa.glsl, and the same translation of
// `third_party/smaa/SMAA.hlsl` (iryoku/smaa, MIT; notice in THIRD-PARTY-NOTICES.md) — read, not
// macro-substituted.
//
// **The one number that differs from the OpenGL side is the sign of `metrics.y`, and it is
// positive here.** The reference is written for Direct3D, where a texture coordinate's y grows
// *downward*; a wgpu framebuffer's origin is top-left, so y grows downward here too and the
// reference's own arithmetic — every crossing of the screen written as `± offset · RT_METRICS.y`
// — means exactly what it means there. The GLSL passes `−1/h` to reach the same screen
// orientation in an OpenGL texture, and `gl/smaa.rs::rt_metrics` carries the argument. Nothing
// else about the two ports differs: the two tables are indexed in their own space, and a data
// texture's row 0 sits at v = 0 under both APIs (it is a *framebuffer*, never an uploaded table,
// whose rows mirror between them).
//
// What does differ from `screen_depth.wgsl`, and on purpose: this chunk declares a group of its
// own. The passes of AMD-055 REQ-SCN-080 reconstruct a view-space position and want the
// projection; this filter reads the window depth point by point and reconstructs nothing, so the
// twelve `vec4` of that block would be numbers it never looks at — which is the same reason
// `SMAA_EDGES` does not take `SCREEN_DEPTH` on the OpenGL side.

// The filter's own uniform block.
struct Smaa {
    // xy: the part of the attachments the frame drew into, in texture coordinates — `uDrawnScale`
    // of the GLSL, and (1, 1) on this backend, whose targets are allocated at the frame's exact
    // size. zw: that size in pixels, which is `SMAA_RT_METRICS.zw`.
    drawn: vec4<f32>,
    // `SMAA_RT_METRICS` = (1/w, +1/h, w, h). See this file's header for the sign.
    metrics: vec4<f32>,
}

@group(0) @binding(0) var<uniform> smaa: Smaa;
// The frame's depth, single-sample: what the first pass detects contours in, and the only texture
// it reads (REQ-SCN-085 forbids the luma and the colour detectors).
@group(0) @binding(1) var zax_smaa_depth_texture: texture_depth_2d;
// The **filtered** sampler of every read but the depth's and the frame colour's. Not a
// convenience: the searches of the second pass fetch four edges at a time through the filter (the
// reference's `@PSEUDO_GATHER4`) and `SMAAArea` interpolates between the areas of two distances,
// so a point sampler anywhere here would give a plausible, wrong picture. It is the reference's
// own `LinearSampler`.
@group(0) @binding(2) var zax_smaa_linear: sampler;
// What each pass reads besides the depth, in the order its own header gives: the edges, the area
// table and the search table for the second pass; the frame's colour and the weights for the
// third. A slot a pass does not read holds a 1 x 1 dummy.
@group(0) @binding(3) var zax_smaa_source0: texture_2d<f32>;
@group(0) @binding(4) var zax_smaa_source1: texture_2d<f32>;
@group(0) @binding(5) var zax_smaa_source2: texture_2d<f32>;

struct SmaaVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// The full-screen triangle of resolve.vert, with the one change this API forces — NDC y points up
// while texture row 0 is at the top, so `uv.y` runs the other way round from the corner's. The
// same triangle `screen_depth.wgsl` draws, written out because this chunk is its own group.
@vertex
fn vs_smaa(@builtin(vertex_index) index: u32) -> SmaaVertex {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: SmaaVertex;
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    out.position = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

// The window depth at a viewport coordinate (0 to 1 over the part drawn into), read as GL's
// NEAREST with CLAMP_TO_EDGE reads it — which is what the GLSL's `zaxSmaaFetch(uDepth, ...)` is,
// the depth texture being the one source the reference samples with its `PointSampler`.
fn zax_smaa_depth(uv: vec2<f32>) -> f32 {
    let size = vec2<f32>(textureDimensions(zax_smaa_depth_texture));
    let at = clamp(floor(uv * smaa.drawn.xy * size), vec2<f32>(0.0), size - 1.0);
    return textureLoad(zax_smaa_depth_texture, vec2<i32>(at), 0);
}

// The same, `texels` texels away: the reference's `SMAASampleLevelZeroOffset`, written as the
// multiply-add its own `SMAA_HLSL_3` path uses so that the sign of `metrics.y` carries. **Never**
// an integer texel offset, which is in the texture's own space and would not pick it up.
fn zax_smaa_depth_offset(uv: vec2<f32>, texels: vec2<f32>) -> f32 {
    return zax_smaa_depth(uv + (texels * smaa.metrics.xy));
}

// The edges of the first pass, `r` west and `g` north, through the filtered sampler.
fn zax_smaa_edges(uv: vec2<f32>) -> vec2<f32> {
    return textureSampleLevel(zax_smaa_source0, zax_smaa_linear, uv * smaa.drawn.xy, 0.0).rg;
}

fn zax_smaa_edges_offset(uv: vec2<f32>, texels: vec2<f32>) -> vec2<f32> {
    return zax_smaa_edges(uv + (texels * smaa.metrics.xy));
}

// The area table, indexed in its **own** coordinate space — never through `metrics`, and never
// through the drawn scale either.
fn zax_smaa_table_area(coord: vec2<f32>) -> vec2<f32> {
    return textureSampleLevel(zax_smaa_source1, zax_smaa_linear, coord, 0.0).rg;
}

// And the search table, the same way.
fn zax_smaa_table_search(coord: vec2<f32>) -> f32 {
    return textureSampleLevel(zax_smaa_source2, zax_smaa_linear, coord, 0.0).r;
}

// The blending weights of the second pass, as the third reads them: `rg` the horizontal pair,
// `ba` the vertical one, through the filter.
fn zax_smaa_weights(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(zax_smaa_source1, zax_smaa_linear, uv * smaa.drawn.xy, 0.0);
}

fn zax_smaa_weights_offset(uv: vec2<f32>, texels: vec2<f32>) -> vec4<f32> {
    return zax_smaa_weights(uv + (texels * smaa.metrics.xy));
}

// The frame's linear colour, read texel by texel — the `NEAREST` the OpenGL backend has given
// that texture since AMD-021 R2-009, which is why the third pass writes its two taps out rather
// than leaning on the filter (`smaa_blend.wgsl` carries that).
fn zax_smaa_color(uv: vec2<f32>) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(zax_smaa_source0));
    let at = clamp(floor(uv * smaa.drawn.xy * size), vec2<f32>(0.0), size - 1.0);
    return textureLoad(zax_smaa_source0, vec2<i32>(at), 0);
}

fn zax_smaa_color_offset(uv: vec2<f32>, texels: vec2<f32>) -> vec4<f32> {
    return zax_smaa_color(uv + (texels * smaa.metrics.xy));
}
