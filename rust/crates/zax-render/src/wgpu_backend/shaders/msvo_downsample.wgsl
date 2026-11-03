// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// One level of the depth pyramid of AMD-055 REQ-SCN-083 *Mode MSVO*, the WGSL twin of
// shaders/msvo_downsample.frag: one texel of each two-by-two block of the level above, which is
// what `AoPrepareDepthBuffers2CS.hlsl` stores in its `DS2x` buffers. A *point* decimation and not
// an average on purpose — the original does the same, and averaging two depths across a silhouette
// would invent a surface that is in neither.
//
// **Which of the four texels, and why it is not the obvious one.** The OpenGL twin reads
// `texelFetch(uSource, ivec2(gl_FragCoord.xy) * 2, 0)`, and in OpenGL both `gl_FragCoord.y` and a
// `texelFetch` row index count from the **bottom**. Here both count from the top. Transcribing the
// expression would therefore keep a *different* texel of the block — geometrically the one above
// the one OpenGL keeps — and the whole pyramid would be built on a half-texel shift. Either choice
// is a correct point decimation (the block's four texels are equally admissible, which is why the
// requirement says nothing about it), but this lot's rule is that the OpenGL backend is the
// reference, so the row is mirrored into its convention: destination row `j` from the top reads
// source row `2j + 1` from the top, which is the same geometric texel OpenGL's `2j` from the bottom
// is.
//
// The clamp is the odd-size case: a level of one texel has a source of one texel, where `2j + 1`
// would be out of range and `textureLoad` would hand back 0 — a depth of zero, right in front of
// the eye.

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(zax_source0));
    let at = vec2<i32>(in.position.xy);
    // `from` is a reserved word in WGSL.
    let source = vec2<i32>(min(at.x * 2, size.x - 1), min(at.y * 2 + 1, size.y - 1));
    return vec4<f32>(textureLoad(zax_source0, source, 0).r, 0.0, 0.0, 1.0);
}
