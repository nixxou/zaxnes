// The resolve pass of the wgpu backend (resolve.vert, resolve.frag): one full-screen triangle
// and the sRGB encode of SPEC-06 §2, applied once, after the blending (AMD-021 R2-009). The
// linear buffer has the size of the target, so each pixel reads its own texel.

@group(0) @binding(0) var linear: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let value = textureLoad(linear, vec2<i32>(position.xy), 0);
    // REQ-SCN-035: the alpha channel is never converted.
    return vec4<f32>(zax_to_srgb3(clamp(value.rgb, vec3<f32>(0.0), vec3<f32>(1.0))), value.a);
}
