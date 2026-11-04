// The first pass of AMD-060 REQ-SCN-085: the edges of the frame, found in its **depth** and
// nowhere else. `SMAADepthEdgeDetectionPS` of `third_party/smaa/SMAA.hlsl` (MIT), with the
// threshold of Q-109 §1.
//
// REQ-SCN-085 forbids the luma and the colour detectors, and the reason AMD-032 §2's objection to
// post-processing filters does not reach this one is exactly that: a nearest-neighbour texture on
// a flat face carries no depth contour, so no texel of it can ever be found here. That is a
// property of the detector and not of a tuning, and the test
// `req_scn_085_the_interior_of_a_textured_face_is_untouched` reads it off a capture.
//
// Output: `r` the **west** edge of the pixel and `g` its **north** one, as the reference encodes
// them — "north" meaning +y here, see `smaa.glsl` point 1. A pixel with neither is `discard`ed,
// which is the reference's own early-out and the reason the target is cleared each frame.

in vec2 vUv;
out vec4 oEdges;

// The frame's depth, as a texture (`gl/linear_buffer.rs` allocates it when the frame asks for
// one). The **window** depth, read point by point: this pass reconstructs nothing.
uniform sampler2D uDepth;

// How much of the distance a step has to jump before it is a contour, Q-109 §1. 0,002 is 0,2 % of
// the distance to the surface — a fifth of a voxel at the default reset distance of 190 scene
// units (AMD-045 REQ-SCN-069), where a voxel is one unit (REQ-SCN-002).
//
// It is expressed through `1 − w` rather than in depth units because a fixed threshold on this
// depth buffer is not a threshold at all: the projection of SPEC-06 gives `w ≈ 1 − 0,15/d`
// (`screen_depth.glsl` derives the two conventions behind it), so `dw/dd = 0,15/d²` and the
// reference's own `SMAA_DEPTH_THRESHOLD` of 0,001 would ask for a step of **240 scene units** at
// that same camera distance, where the whole NES picture is 256 × 240. From `w ≈ 1 − 0,15/d` comes
// `0,15/d ≈ 1 − w`, so a jump of `ε · d` in distance is a jump of `ε · (1 − w)` here. The
// reference says the constant "depends on the depth range of the scene"; this is that dependence
// written out.
const float ZAX_SMAA_DEPTH_RELATIVE = 0.002;

// And a floor in depth units, about three quanta of a 24-bit buffer (the quantum is 2⁻²⁴ ≈ 6·10⁻⁸).
// It absorbs the buffer's own quantisation and the shader's own rounding on a flat surface, and it
// keeps the threshold **strictly positive**, which the term above does not: where nothing was drawn
// the window depth is exactly 1 on every neighbour, so `1 − w` and the step are both zero, and
// `step(0, 0)` is **1** — every pixel of the background would be a contour. The reference's own
// constant threshold hides that degenerate case; expressing the threshold relatively exposes it.
const float ZAX_SMAA_DEPTH_FLOOR = 2.0e-7;

void main()
{
    // The reference's `SMAAGatherNeighbours` — the pixel, its west and its north neighbour — and
    // the two the curvature below needs, at the offsets `offset[2]` of the reference's own
    // `SMAAEdgeDetectionVS` already names (it computes them for the luma detector; the depth one
    // does not use them there).
    float here = zaxSmaaFetch(uDepth, vUv).r;
    vec2 near = vec2(
        zaxSmaaFetchOffset(uDepth, vUv, vec2(-1.0, 0.0)).r,
        zaxSmaaFetchOffset(uDepth, vUv, vec2(0.0, -1.0)).r);
    vec2 far = vec2(
        zaxSmaaFetchOffset(uDepth, vUv, vec2(-2.0, 0.0)).r,
        zaxSmaaFetchOffset(uDepth, vUv, vec2(0.0, -2.0)).r);

    // The reference's own first difference — the step between the pixel and its neighbour — and
    // the second one, which is what tells a step from a slope.
    //
    // **The window depth is linear in screen space across a planar surface.** That is a property
    // of the perspective projection and not an approximation (it is why a depth buffer may be
    // interpolated linearly at all), so the second difference is **exactly zero** over a flat
    // face, at any angle, at any resolution and at any distance. It is the acceptance criterion of
    // REQ-SCN-085 — "les pixels intérieurs à la face sont identiques à l'octet" — as an identity
    // rather than as a tolerance.
    vec2 delta = abs(vec2(here) - near);
    vec2 curvature = abs(vec2(here) - (2.0 * near) + far);

    // **Both** have to clear the threshold, and each rejects what the other admits:
    //
    //  - the second difference alone marks the pixel *behind* the contour as well. Along a row
    //    `… bg bg | obj obj …` it is `|obj − 2·bg + bg|` at the first object pixel, which is the
    //    step and right, and `|obj − 2·obj + bg|` at the second, which is the same size and wrong:
    //    the edge would come out two pixels wide and the blending weights of the second pass are
    //    built for one. The first difference is zero there, so the `min` drops it;
    //  - the first difference alone fires on a slope. Measured on the close-up R of AMD-055 §2.5,
    //    whose floor is a plane seen at a shallow angle: its own per-pixel step reaches 3,4·10⁻⁴
    //    while the cube's horizontal silhouette jumps 3,2·10⁻⁴ — the same size. No threshold on
    //    the first difference can separate those two, which is what the first version of this
    //    shader tried and why it found **no north edge at all** on that frame.
    //
    // So `min`, and one `step` on it, which keeps the shape of the reference's own line.
    vec2 threshold = vec2(ZAX_SMAA_DEPTH_FLOOR + (ZAX_SMAA_DEPTH_RELATIVE * max(1.0 - here, 0.0)));
    vec2 edges = step(threshold, min(delta, curvature));

    // The reference's own early-out. Nothing is written where there is no contour, so the target
    // keeps the zero it was cleared to — which is also what makes this pass cheap on a frame whose
    // silhouettes are a small share of the pixels.
    if (dot(edges, vec2(1.0, 1.0)) == 0.0)
    {
        discard;
    }

    oEdges = vec4(edges, 0.0, 1.0);
}
