// The composite of AMD-055 REQ-SCN-083: "elle multiplie la couleur de chaque pixel de la scène
// par un facteur `1 - O`, en espace linéaire, avant l'encodage sRGB de REQ-SCN-028". The WGSL twin
// of shaders/ao_composite.frag.
//
// **No colour target and no copy of the scene.** A pass cannot sample the attachment it draws
// into, and it does not have to: the GL path's `glBlendFunc(GL_ZERO, GL_SRC_COLOR)` makes the
// destination `dst x src`, and its wgpu spelling is
// `BlendComponent { src_factor: Zero, dst_factor: Src, operation: Add }` — so one full-screen
// triangle that outputs `1 - O` multiplies the linear buffer in place here too. The alpha is
// multiplied by 1, which leaves it alone: it is never a colour (REQ-SCN-035).
//
// Where nothing was drawn the occlusion is 0 and the factor exactly 1, so the skybox and the
// cleared background come through bit for bit, which is the invariant of the A-h variant.
//
// `zax_source0` is the blurred occlusion, read through the **filtered** sampler: the pass runs at
// half of each dimension of the render target except at `ao.quality` High, and that filter is what
// smooths the half-resolution occlusion back up to the frame's size.

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let occlusion = textureSampleLevel(zax_source0, zax_smooth, in.uv * screen.drawn.xy, 0.0).r;
    return vec4<f32>(vec3<f32>(1.0 - occlusion), 1.0);
}
