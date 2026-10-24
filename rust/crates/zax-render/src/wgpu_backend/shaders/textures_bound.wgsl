// The textures of a draw bound one draw call at a time: its own on the binding its colour mode
// reads, a dummy on the other.

@group(2) @binding(0) var index_texture: texture_2d<u32>;
@group(2) @binding(1) var color_texture: texture_2d<f32>;

fn zax_index_texel(draw: Draw, texel: vec2<i32>) -> u32 {
    return textureLoad(index_texture, texel, 0).r;
}

fn zax_color_texel(draw: Draw, texel: vec2<i32>) -> vec4<f32> {
    return textureLoad(color_texture, texel, 0);
}
