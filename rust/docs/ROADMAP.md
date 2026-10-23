# Rust port — ideas for later

Ideas discussed with the owner and deliberately postponed. Nothing here is scheduled.

## Rendering performance (draw-call batching)

Today every object is one draw call with about twelve uniforms set one by one and two texture
binds (`zax-render/src/renderer.rs`, per-item loop). Synthetic scenes issue 160 to 3 852 draw
calls per frame (`drawCalls` in `goldens/render-synthetic/render.json`). On the H700 Mali-G31
a draw call costs about 17 us of CPU (see `HANDHELD-RESEARCH.md`), so this matters most on the
handheld.

1. Measure first: how many objects share mesh and texture (`render --explain` on the synthetic
   scenes and on real games).
2. Per-object parameters in one uniform buffer filled once per frame (GL 3.3 / GLES 3.0 UBO),
   draws pointing at their slice. Low risk, about 2-3 h.
3. Sort opaque items by program, texture and mesh; blended items keep the REQ-SCN-040 order.
   About 1 h.
4. Instancing of items sharing mesh and texture. Biggest gain, medium risk (order, clipping,
   deformation, shadows), about 3-5 h.

Validation: synthetic scenes pixel-identical to the current GL output, `drawCalls` and
`render --bench` before/after, owner's C#-vs-Rust capture campaign.

## Vulkan / DirectX 12 backend

Second render backend through `wgpu` (Vulkan, DX12, Metal), OpenGL kept as reference and for
GLES handhelds; `--renderer auto|gl|vulkan|dx12`.

- Steps: backend interface in `zax-render`; `wgpu` backend (shaders via `naga` or WGSL); ImGui
  renderer on `wgpu`; SDL2 window surface; validation with lavapipe here and on the owner's GPU.
- Do it on top of the batching above, with per-object data in one buffer and dynamic offsets,
  not as a line-by-line port of the GL calls.
- Gains: ReShade works best on DX12/Vulkan (depth detection, add-ons), vkBasalt on Linux.
  Costs: +3-5 MB, pixel parity with C# only within tolerance, two backends to maintain.
- Pipeline cache on disk to avoid first-use stutter.

## ReShade depth on OpenGL

Superseded by the Vulkan/DX12 backend if that is done. Otherwise: an off-by-default setting that
requests a 24-bit depth buffer on the window and blits the resolved scene depth into it before
the UI pass (standard depth, not reversed, not logarithmic).

## Visual effects inspired by public 3dSen v2 previews (ideas only)

No 3dSen program, files or decompiled code may be used; each effect would be specified as a
Zaxnes amendment first.

- World curvature in the vertex shader (per profile or Lua-driven). Cheap.
- Bloom post-process pass. Cheap to moderate; off on handhelds.
- Soft shadows (PCF filtering of the existing shadow map). Cheap.
- Planar or screen-space reflections. Moderate GPU cost.
- Ambient occlusion, depth of field (post-process from depth), or left to ReShade.
- Smoothed or replacement models per pattern. Large; profile format change (SPEC-03/04).
- Check 3dSen v2 profile compatibility with the AMD-030 importer once v2 is released, using an
  installation the owner owns.

## Other display modes

Stereoscopic side-by-side / top-bottom, anaglyph, orthographic or isometric camera, built-in CRT
or scanline filters, toon shading, native VR (OpenXR, large).
