// The first pass of AMD-060 REQ-SCN-085 on this backend: the edges of the frame, found in its
// **depth** and nowhere else. The WGSL twin of shaders/smaa_edges.frag, line for line, with the
// threshold of Q-109 §1 — `SMAADepthEdgeDetectionPS` of `third_party/smaa/SMAA.hlsl` (MIT).
//
// Output: `r` the **west** edge of the pixel and `g` its **north** one. A pixel with neither is
// `discard`ed, which is the reference's own early-out and the reason the target is cleared every
// frame (here by the pass's own `LoadOp::Clear`, where the OpenGL path calls `glClear`).

// How much of the distance a step has to jump before it is a contour, Q-109 §1. The constants and
// the whole of the argument are `smaa_edges.frag`'s: a fixed threshold on this depth buffer is not
// a threshold at all, because the projection of SPEC-06 gives `w ≈ 1 − 0,15/d`, so the reference's
// own `SMAA_DEPTH_THRESHOLD` of 0,001 would ask for a step of 240 scene units at the default reset
// distance where a voxel is one unit. Expressed through `1 − w`, the threshold is free of the
// scale and of the distance.
const ZAX_SMAA_DEPTH_RELATIVE: f32 = 0.002;

// And a floor in depth units, about three quanta of a 24-bit buffer. It absorbs the buffer's
// quantisation and the shader's rounding on a flat surface, and it keeps the threshold **strictly
// positive** — where nothing was drawn every sample is exactly 1, so the step is zero and
// `step(0, 0)` is 1: every pixel of the background would be a contour.
const ZAX_SMAA_DEPTH_FLOOR: f32 = 2.0e-7;

@fragment
fn fs_main(in: SmaaVertex) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // The reference's `SMAAGatherNeighbours` — the pixel, its west and its north neighbour — and
    // the two the second difference needs, at the offsets `offset[2]` of the reference's own
    // `SMAAEdgeDetectionVS` already names (it computes them for the luma detector; the depth one
    // does not use them there).
    let here = zax_smaa_depth(uv);
    let near = vec2<f32>(
        zax_smaa_depth_offset(uv, vec2<f32>(-1.0, 0.0)),
        zax_smaa_depth_offset(uv, vec2<f32>(0.0, -1.0)),
    );
    let far = vec2<f32>(
        zax_smaa_depth_offset(uv, vec2<f32>(-2.0, 0.0)),
        zax_smaa_depth_offset(uv, vec2<f32>(0.0, -2.0)),
    );

    // The reference's own first difference, and the second one, which is what tells a step from a
    // slope. **The window depth is linear in screen space across a planar surface** — a property
    // of the perspective projection and not an approximation — so the second difference is exactly
    // zero over a flat face, at any angle, any resolution and any distance. That is REQ-SCN-085's
    // second acceptance criterion as an identity rather than as a tolerance.
    let delta = abs(vec2<f32>(here) - near);
    let curvature = abs(vec2<f32>(here) - (2.0 * near) + far);

    // **Both** have to clear the threshold, and each rejects what the other admits: the second
    // difference alone marks the pixel *behind* the contour as well, and the first alone fires on
    // a slope (measured, Q-109 §1). So `min`, and one `step` on it, which keeps the shape of the
    // reference's own line.
    let threshold = vec2<f32>(
        ZAX_SMAA_DEPTH_FLOOR + (ZAX_SMAA_DEPTH_RELATIVE * max(1.0 - here, 0.0)),
    );
    let edges = step(threshold, min(delta, curvature));

    // The reference's own early-out. Nothing is written where there is no contour, so the target
    // keeps the zero the pass cleared it to.
    if (dot(edges, vec2<f32>(1.0, 1.0)) == 0.0) {
        discard;
    }
    return vec4<f32>(edges, 0.0, 1.0);
}
