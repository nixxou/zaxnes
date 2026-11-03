// The composite of AMD-055 REQ-SCN-084: "le mélange se fait en espace linéaire, avant l'encodage
// sRGB de REQ-SCN-028", with `(1 - i) * surface + i * reflet`. The WGSL twin of
// shaders/ssr_composite.frag.
//
// The trace holds its result premultiplied (`rgb = w * c`, `a = w`), so with the OpenGL path's
// `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)` — here
// `BlendComponent { src_factor: One, dst_factor: OneMinusSrcAlpha, operation: Add }` — the
// destination becomes `w * c + (1 - w) * dst`, exactly the blend the requirement writes, in the
// linear buffer, in place, with no second colour target and no copy of the scene.
//
// **The mask is applied again here, at full resolution**, and that is not a belt: the trace runs
// at half of each dimension, so sampling it back up with a linear filter would spread the
// reflection half an output pixel past the floor's silhouette. Multiplying by the mask the frame
// itself rasterised makes "aucun pixel qui n'est pas une surface de sol n'est modifié" — the
// invariants of the variants R-m, R-s and R-h — true pixel by pixel rather than nearly.
//
// `zax_source0` is the traced reflection, read through the **filtered** sampler (that upsample is
// the whole reason the trace writes a premultiplied value); `zax_source1` is the mask, read texel
// by texel, as the OpenGL backend's `NEAREST` reads it.

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let reflection = textureSampleLevel(zax_source0, zax_smooth, in.uv * screen.drawn.xy, 0.0);
    let mask = zax_load1(in.uv).r;
    return reflection * mask;
}
