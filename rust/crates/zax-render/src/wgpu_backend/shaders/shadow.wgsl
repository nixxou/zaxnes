// The shadow pass of the wgpu backend (shadow.vert): the same placement as the scene pass,
// depth only, the casters of a run drawn as instances.

struct ShadowFrame {
    light_view_projection: mat4x4<f32>,
    // x time.
    params: vec4<f32>,
}

@group(0) @binding(0) var<uniform> shadow_frame: ShadowFrame;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
) -> @builtin(position) vec4<f32> {
    let instance = instances[instance_index];
    let draw = draws[run_of[instance_index]];
    var local = position;
    if (draw.modes2.x >= 0) {
        local += zax_deformation(deforms[draw.modes2.x], instance.deform_tile.xyz, position, shadow_frame.params.x);
    }

    let scene = zax_place(instance, local);
    return zax_clip(shadow_frame.light_view_projection * vec4<f32>(scene.x, scene.y, -scene.z, 1.0));
}
