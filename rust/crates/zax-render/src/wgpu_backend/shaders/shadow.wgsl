// The shadow pass of the wgpu backend (shadow.vert): the same placement as the scene pass,
// depth only, the casters of a run drawn as instances.

struct ShadowFrame {
    light_view_projection: mat4x4<f32>,
    // x time.
    params: vec4<f32>,
    // AMD-043 REQ-SCN-067, as in the scene block: k, f, h, the arc pivot in bend_k.w, and the
    // per-axis switch. Written whatever the variant, read only by the ZAX_BEND one.
    bend_k: vec4<f32>,
    bend_f: vec4<f32>,
    bend_h: vec4<f32>,
    bend_on: vec4<f32>,
}

@group(0) @binding(0) var<uniform> shadow_frame: ShadowFrame;

//#ifdef ZAX_BEND
// The same four accessors the scene module defines, over this pass's own frame block.
fn frame_bend_k() -> vec4<f32> { return shadow_frame.bend_k; }
fn frame_bend_f() -> vec4<f32> { return shadow_frame.bend_f; }
fn frame_bend_h() -> vec4<f32> { return shadow_frame.bend_h; }
fn frame_bend_on() -> vec4<f32> { return shadow_frame.bend_on; }
//#endif

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
//#ifdef ZAX_BEND
    // REQ-SCN-068: the shadow pass is bent too, so the shadows follow the curved surface instead
    // of sliding off it. Only the position is wanted here — the pass writes depth alone.
    let placed = zax_bend(scene).scene;
    return zax_clip(shadow_frame.light_view_projection
        * vec4<f32>(placed.x, placed.y, -placed.z, 1.0));
//#else
    return zax_clip(shadow_frame.light_view_projection * vec4<f32>(scene.x, scene.y, -scene.z, 1.0));
//#endif
}
