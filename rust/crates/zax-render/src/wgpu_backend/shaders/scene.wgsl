// The scene pass of the wgpu backend, the WGSL twin of scene.vert and scene.frag: albedo of
// REQ-SCN-034, visible-rectangle clipping of REQ-SCN-036, hard shadow of REQ-SCN-030/031 and
// the Lambert shading of REQ-SCN-028, written linear (AMD-021 R2-009). The transfer functions
// and the deformation come from common.wgsl, prepended by the renderer.

struct Frame {
    view_projection: mat4x4<f32>,
    light_view_projection: mat4x4<f32>,
    clip_rect: vec4<f32>,
    // xyz the linear light colour, w the ambient term.
    light_ambient: vec4<f32>,
    // x time, y shadow strength, z shadow bias, w normal offset.
    params: vec4<f32>,
    // The light direction of the scene buckets, then the HUD's.
    light: array<vec4<f32>, 2>,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var palette: texture_2d<f32>;
@group(0) @binding(2) var shadow_map: texture_depth_2d;

// The textures of group 2 and zax_index_texel / zax_color_texel come from textures_bound.wgsl
// or textures_bindless.wgsl, appended by the renderer.

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) scene: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) run: u32,
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
) -> VertexOut {
    let instance = instances[instance_index];
    let run = run_of[instance_index];
    let draw = draws[run];
    var local = position;
    if (draw.modes2.x >= 0) {
        local += zax_deformation(deforms[draw.modes2.x], instance.deform_tile.xyz, position, frame.params.x);
    }

    let scene = zax_place(instance, local);
    var out: VertexOut;
    out.scene = scene.xyz;
    // REQ-SCN-028: the normal is the mesh normal, unchanged by the deformation. The sign is -1
    // for a mirrored placement, whose cofactor matrix points the normal inwards (Q-069).
    out.normal = draw.tilting.z * zax_place_normal(instance, normal);
    out.uv = uv;
    out.run = run;
    out.position = zax_clip(frame.view_projection * vec4<f32>(scene.x, scene.y, -scene.z, 1.0));
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let draw = draws[in.run];
    // REQ-SCN-036: a fragment whose scene (x, y) leaves R is dropped, whatever its depth.
    let rect = frame.clip_rect;
    if (draw.modes.z != 0 && (in.scene.x < rect.x || in.scene.x > rect.z
                           || in.scene.y < rect.y || in.scene.y > rect.w)) {
        discard;
    }

    var albedo: vec3<f32>;
    var texel_alpha = 1.0;
    let color_mode = draw.modes.x;
    if (color_mode == 2) {
        albedo = draw.flat_alpha.xyz;
    } else {
        // REQ-E3D-563: nearest neighbour, no mipmap, texel coordinates taken modulo the
        // texture size.
        let size = draw.texture.xy;
        let uv = vec2<f32>(
            in.uv.x * draw.tilting.x + draw.texture.z * size.x,
            in.uv.y * draw.tilting.y + draw.texture.w * size.y);
        let texel = vec2<i32>(zax_mod(floor(uv), size));
        if (color_mode == 0) {
            // REQ-SCN-034: the index selects 4 * P + (t mod 4) in the frame palette.
            let value = zax_index_texel(draw, texel);
            let entry = clamp(draw.modes.y, 0, 15) * 4 + i32(value & 3u);
            albedo = textureLoad(palette, vec2<i32>(entry, 0), 0).rgb;
        } else {
            let direct = zax_color_texel(draw, texel);
            albedo = direct.rgb;
            texel_alpha = direct.a;
        }
    }

    let n = normalize(in.normal);
    let l = normalize(-frame.light[i32(draw.tilting.w)].xyz);
    let ndl = max(0.0, dot(n, l));

    var shadow = 1.0;
    if (draw.modes.w != 0) {
        // REQ-SCN-031: a normal offset and a depth bias of about 0.02 unit.
        let biased = in.scene + n * frame.params.w;
        let light = frame.light_view_projection * vec4<f32>(biased.x, biased.y, -biased.z, 1.0);
        let projected = light.xyz / light.w * 0.5 + 0.5;
        if (projected.x >= 0.0 && projected.x <= 1.0
            && projected.y >= 0.0 && projected.y <= 1.0
            && projected.z <= 1.0) {
            // The map's rows run top down here, bottom up in GL: the same texel is 1 - y.
            let side = vec2<f32>(textureDimensions(shadow_map));
            let at = vec2<i32>(min(floor(vec2<f32>(projected.x, 1.0 - projected.y) * side), side - 1.0));
            let stored = textureLoad(shadow_map, at, 0);
            if (projected.z - frame.params.z > stored) {
                shadow = frame.params.y;
            }
        }
    }

    // REQ-SCN-028: albedo_lin * (ambient_lin + max(0, N.L) * light_lin * S).
    let lit = zax_to_linear3(albedo) * (vec3<f32>(frame.light_ambient.w) + ndl * frame.light_ambient.xyz * shadow);
    return vec4<f32>(min(lit, vec3<f32>(1.0)), draw.flat_alpha.w * texel_alpha);
}
