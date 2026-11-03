// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// The finest level of the depth pyramid of AMD-055 REQ-SCN-083 *Mode MSVO*, the WGSL twin of
// shaders/msvo_linearize.frag: the `LinearizeZ` of `AoPrepareDepthBuffers1CS.hlsl`, which turns the
// frame's window depth into the **view distance** the sphere test of `msvo_occlude.wgsl` compares.
//
// The distance is kept in **scene units** rather than normalised by the far plane, which is what
// makes the sphere test exactly `0,5 + Δz / (2 r T)` — the radius `r` of REQ-SCN-083 is given in
// scene units, so the estimator and the requirement speak the same units with no conversion.
//
// A pixel with no geometry keeps the far distance: the skybox and the cleared background of
// REQ-SCN-046 are both there, and `msvo_upsample.wgsl` drops them outright, which is REQ-SCN-083's
// scope bullet.

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    // The camera looks down -z in view space, so the distance in front of the eye is -z.
    return vec4<f32>(-zax_view_sample(in.uv).z, 0.0, 0.0, 1.0);
}
