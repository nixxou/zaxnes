// What the scene and the shadow pass of the wgpu backend share: the per-run block, the
// placements, the sRGB transfer functions of SPEC-06 §2 (transfer.glsl) and the deformation of
// SPEC-04B REQ-E3D-568 (deformation.glsl).
//
// A draw call is one instanced run. Its instances are a range of `instances` (the first
// instance of the call is where the run starts), and `run_of` names the block of `draws` each
// instance belongs to, so that a call needs nothing bound but itself.

struct Draw {
    // xyz the flat colour, w the alpha.
    flat_alpha: vec4<f32>,
    // xy the texture size, zw the texture offset.
    texture: vec4<f32>,
    // xy the texture tilting, z the normal sign (Q-069).
    tilting: vec4<f32>,
    // xyz the light direction of the bucket.
    light: vec4<f32>,
    // Colour mode, palette table, clip, receive shadow.
    modes: vec4<i32>,
    // x: whether the deformation applies.
    modes2: vec4<i32>,
    deform_amplitude: vec4<f32>,
    deform_size: vec4<f32>,
    // Unused: the tile position of the deformation travels with the instance.
    deform_tile: vec4<f32>,
    deform_speed: array<vec4<f32>, 3>,
    deform_phase: array<vec4<f32>, 3>,
    // Unused: the placement travels with the instance.
    model: mat4x4<f32>,
}

// The placement of each instance of a batched draw: the model matrix, the columns of the normal
// (cofactor) matrix and the tile position of the deformation (xyz).
struct Instance {
    model: mat4x4<f32>,
    normal: array<vec4<f32>, 3>,
    deform_tile: vec4<f32>,
}

@group(1) @binding(0) var<storage, read> draws: array<Draw>;
@group(1) @binding(1) var<storage, read> instances: array<Instance>;
@group(1) @binding(2) var<storage, read> run_of: array<u32>;

fn zax_to_linear(c: f32) -> f32 {
    return select(pow((c + 0.055) / 1.055, 2.4), c / 12.92, c <= 0.04045);
}

fn zax_to_srgb(c: f32) -> f32 {
    return select(1.055 * pow(c, 1.0 / 2.4) - 0.055, c * 12.92, c <= 0.0031308);
}

fn zax_to_linear3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(zax_to_linear(c.r), zax_to_linear(c.g), zax_to_linear(c.b));
}

fn zax_to_srgb3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(zax_to_srgb(c.r), zax_to_srgb(c.g), zax_to_srgb(c.b));
}

// GLSL's mod: x - y * floor(x / y), not WGSL's truncated %.
fn zax_mod(x: vec2<f32>, y: vec2<f32>) -> vec2<f32> {
    return x - y * floor(x / y);
}

// The clip position of GL (depth -1..1) as wgpu takes it (depth 0..1): the same depth values
// end up in the depth buffer.
fn zax_clip(p: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(p.x, p.y, (p.z + p.w) * 0.5, p.w);
}

fn zax_deform_row(amplitude: f32, size: f32, n: vec3<f32>, speed: vec4<f32>, phase: vec4<f32>, time: f32) -> f32 {
    return amplitude * size
         * sin(n.x * speed.x + phase.x)
         * sin(n.y * speed.y + phase.y)
         * sin(n.z * speed.z + phase.z)
         * sin(time * speed.w + phase.w);
}

// `tile` is the tile's position relative to the pivot.
fn zax_deformation(draw: Draw, tile: vec3<f32>, v: vec3<f32>, time: f32) -> vec3<f32> {
    let p = tile + v;
    let size = draw.deform_size.xyz;
    let n = vec3<f32>(
        select(p.x / size.x, 0.0, size.x == 0.0),
        select(p.y / size.y, 0.0, size.y == 0.0),
        select(p.z / size.z, 0.0, size.z == 0.0));
    let amplitude = draw.deform_amplitude.xyz;
    return vec3<f32>(
        zax_deform_row(amplitude.x, size.x, n, draw.deform_speed[0], draw.deform_phase[0], time),
        zax_deform_row(amplitude.y, size.y, n, draw.deform_speed[1], draw.deform_phase[1], time),
        zax_deform_row(amplitude.z, size.z, n, draw.deform_speed[2], draw.deform_phase[2], time));
}
