// The voxel grid of AMD-055 REQ-SCN-081, the WGSL twin of shaders/voxel_grid.glsl, prepended to
// the ZAX_VOXEL_GRID variants only.
//
// The grid modulates the **albedo** of a tile mesh's faces and of the backdrop box's faces,
// before any lighting: for a tile in the mesh's local frame of REQ-E3D-504 (so before the
// animated deformation, before the object's transform and before the curvature — the grooves
// therefore follow the object's scale, its rotation and the curved surface), and for the backdrop
// in scene units. The cell side `N = 2^spl.voxel_grid`, times the scene scale for the backdrop,
// arrives in the frame block as `frame.style.x`.
//
// Nothing of the arithmetic differs from the GLSL. Three operators were checked rather than
// assumed, and the third of them had to be changed:
//
// - `round` is round-half-to-even in WGSL and implementation-chosen in GLSL, but the value this
//   shader takes from it is `abs(cells - round(cells))`, which is 0.5 whichever way a tie goes;
// - `smoothstep`, `mix` and `clamp` are component-wise over `vec3` in both languages, with
//   `smoothstep`'s arguments in the same (low, high, x) order;
// - **the derivative is `fwidthFine`, not `fwidth`**, and that is a measurement. The precision of
//   a plain `fwidth` is implementation-chosen in every one of the three languages: naga emits
//   SPIR-V `OpFwidth` for Vulkan and HLSL `fwidth` for DirectX 12, and on the NVIDIA driver of
//   RU-7's machine those are not the same thing — DX12's is quad-coarse. With plain `fwidth` the
//   grid agreed with the OpenGL reference on 99.8675 % of `framing` under DX12 against Vulkan's
//   99.9834 %, and 99.0124 % once the curvature was added, which is below the bar
//   `render_wgpu_tests.rs` holds the flat frame to. With `fwidthFine` **both** backends read
//   99.983 %. So the OpenGL driver's GLSL `fwidth` is the fine derivative, and asking for it by
//   name is what makes the three agree; the smoothing is a free choice of REQ-SCN-081 ("sur
//   environ un pixel"), so naming the variant changes nothing the requirement fixes.

// The albedo factor at the point `p` on a face of unit normal `n`, both in the frame the grid is
// evaluated in, for a cell side of `cell`.
fn zax_voxel_grid(p: vec3<f32>, n: vec3<f32>, cell: f32) -> f32 {
    let an = abs(n);

    // Alignment on an axis: `m = max |n_a|` and `t = clamp((m - 0.8) / 0.1, 0, 1)` give the
    // groove factor `k` and the maximum brightening `e`. A face aligned on an axis (every GREEDY
    // face, REQ-E3D-505) has t = 1, so k = 0.62 and e = 0.10; a face at 45° or more oblique has
    // t = 0, so k = 0.38 and e = 0.20.
    let m = max(an.x, max(an.y, an.z));
    let t = clamp((m - 0.8) / 0.1, 0.0, 1.0);
    let k = 0.38 + 0.24 * t;
    let e = 0.20 - 0.10 * t;

    // Axis weight: an axis perpendicular to the face has weight 0 and cuts no groove at all; the
    // two axes of the face's plane have weight 1.
    let w = clamp(4.0 * (1.0 - an), vec3<f32>(0.0), vec3<f32>(1.0));

    // Distance to the nearest cell edge, as a fraction of a cell: `min_j |p_a - j N| / N`, in
    // [0, 0.5].
    let cells = p / cell;
    let d = abs(cells - round(cells));

    // The groove is the band of 3.5 % of a cell on each side of an edge; the bright edge runs
    // from there to 16 %, where it reaches exactly 1, so the outer transition is continuous
    // without any smoothing. The inner one, at 3.5 %, is smoothed over about one screen pixel —
    // `fwidth` is the screen-space width of `d`, which is what "about one pixel" means here.
    let width = max(fwidthFine(d), vec3<f32>(1e-6));
    let groove = 1.0 - w * (1.0 - k);
    let bright = 1.0 + w * e * max(vec3<f32>(0.0), 0.16 - d) / 0.125;
    let outside = smoothstep(0.035 - width, 0.035 + width, d);
    let f = mix(groove, bright, outside);

    // At the crossing of two edges the two factors multiply (0.62² = 0.384): the corner of a cell
    // is darker than a groove, which is intended (REQ-SCN-081, Notes).
    return f.x * f.y * f.z;
}
