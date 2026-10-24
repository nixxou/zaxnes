// The textures of the batched draws: every resident texture at once, each draw naming its own
// (modes2.y, 0 for the dummies). A texture of the other kind is a dummy in each array.

@group(2) @binding(0) var index_textures: binding_array<texture_2d<u32>, ZAX_BINDLESS_TEXTURES>;
@group(2) @binding(1) var color_textures: binding_array<texture_2d<f32>, ZAX_BINDLESS_TEXTURES>;

fn zax_index_texel(draw: Draw, texel: vec2<i32>) -> u32 {
    return textureLoad(index_textures[u32(draw.modes2.y)], texel, 0).r;
}

fn zax_color_texel(draw: Draw, texel: vec2<i32>) -> vec4<f32> {
    return textureLoad(color_textures[u32(draw.modes2.y)], texel, 0);
}
