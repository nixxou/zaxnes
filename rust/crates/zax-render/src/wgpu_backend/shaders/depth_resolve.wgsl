// The one pass of AMD-055 REQ-SCN-080 that has no OpenGL counterpart at all.
//
// REQ-SCN-080's last bullet has the screen-space passes read the **resolved** colour and depth.
// On OpenGL that is one call — `glBlitFramebuffer` with `COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT`
// (`LinearColorBuffer::resolve_samples`). On wgpu the colour side is free, because the scene pass
// already names the single-sample target as its `resolve_target`, but **there is no depth
// resolve**: `RenderPassDepthStencilAttachment` has no `resolve_target` field, and there is no
// texture-to-texture blit for a depth format. So a multisampled frame that runs a screen pass
// needs this: one full-screen triangle that reads the multisampled depth and writes a
// single-sample one.
//
// **Sample 0 is the faithful choice**, not an approximation. It is what the OpenGL blit gives,
// and `gl/screen_effects.rs` already says so where it matters — "the depth the pass reads is a
// resolved multisampled depth — one sample's, not the fragment centre's" — which is also why the
// floor mask of REQ-SCN-084 carries its polygon-offset bias.
//
// With `video.anti_alias` = 0, the handheld default and the one the goldens are taken at, this
// pass is **not submitted**: the frame's depth texture is already single-sample and the screen
// passes read it directly.

@group(0) @binding(0) var zax_multisampled_depth: texture_depth_multisampled_2d;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @builtin(frag_depth) f32 {
    return textureLoad(zax_multisampled_depth, vec2<i32>(position.xy), 0);
}
