// The depth of the rendered frame as the screen-space passes of AMD-055 REQ-SCN-080 read it, and
// the view-space position it reconstructs to: the WGSL twin of shaders/screen_depth.glsl, shared
// by the occlusion of REQ-SCN-083 and the reflections of REQ-SCN-084 exactly as that file is.
//
// It also carries the full-screen triangle (resolve.vert) and the bind group every screen pass
// takes, because every one of them wants all three.
//
// **What does not differ: the stored depth value.** `zax_clip` maps GL's -1..1 clip depth to
// wgpu's 0..1 as `(z + w) * 0.5` precisely so that the same numbers end up in the depth buffer,
// and `Depth24Plus` matches `DEPTH_COMPONENT24`. So the two conventions the GLSL undoes — the
// Direct3D-style projection to [0, 1] of SPEC-06, then the NDC-to-window mapping — are undone here
// by the same two lines, and `zax_view_depth` is a transcription. Checked against the clip
// mapping rather than assumed, because it is the one place where the obvious worry is unfounded.
//
// **What does differ, and is the real coordinate trap: the uv-to-NDC mapping is mirrored in y.**
// A wgpu framebuffer's origin is top-left while its NDC y points up, so for a full-screen pass
// `ndc = vec2(uv.x * 2 - 1, 1 - uv.y * 2)` — the y negated against the GLSL's `ndc = uv * 2 - 1`
// — and `zax_project_view` negates it back. Sampling the depth and the colour needs no flip at
// all: they were rendered in this same convention, which is what `resolve.wgsl` already relies
// on. A port that kept `ndc = uv * 2 - 1` would not fail obviously — the occlusion is nearly
// invariant under a mirror of view space, because `zax_view_normal` fixes the normal's sign from
// `dot(normal, p)` rather than from the winding — and would then show up in the reflections,
// whose rays march in a direction that has to agree with the eye.
//
// **And the reads are `textureLoad` with the clamp written out**, which is what GL's NEAREST plus
// CLAMP_TO_EDGE does: a `textureLoad` outside the texture hands back 0 here, and 0 is the near
// plane — the one value that would make a sample occlude everything.

// What a screen pass needs of the frame and of the settings. One block and one layout for every
// pass of REQ-SCN-080, the way `screen_depth.glsl` is one chunk for every one of them: the
// alternative is a layout, a bind group and a buffer per pass for no gain, since a pass that does
// not read a field costs nothing for its being there.
struct Screen {
    // xy: the part of the attachments the frame drew into, in texture coordinates — `uDrawnScale`
    // of the GLSL, and (1, 1) on this backend, whose targets are allocated at the frame's exact
    // size. zw: the size in pixels of the target this pass draws into.
    drawn: vec4<f32>,
    // xy: one texel of that target in viewport coordinates (`uTexel`), the step to a neighbour.
    // zw: spare.
    texel: vec4<f32>,
    // The projection as the four numbers these passes need of it (`uProjection`), read straight
    // off the matrix `numerics::Matrix4x4::create_perspective_field_of_view` built:
    //   xy = the two scales (m11, m22), z = `range` = far/(near - far), w = `near`.
    projection: vec4<f32>,
    // REQ-SCN-083: x the radius in scene units, y the darkening ceiling, z the self-occlusion
    // bias; w spare.
    ao: vec4<f32>,
    // REQ-SCN-084: x the intensity, y the longest ray this setting asks for, z the longest any
    // setting can ask for, w the blur as a fraction of the ray.
    ssr: vec4<f32>,
    // REQ-SCN-084: xy the aspect the blur spiral is stretched by; zw spare.
    blur_aspect: vec4<f32>,
    // x the occlusion's sample count, y the march's steps, z its refinements, w the blur's taps.
    counts: vec4<i32>,
}

@group(0) @binding(0) var<uniform> screen: Screen;
// The frame's depth, resolved (REQ-SCN-080's last bullet).
@group(0) @binding(1) var zax_frame_depth: texture_depth_2d;
// The filtered sampler, for the two reads that are an upsample: the occlusion's composite and the
// reflections'. Every other read is a `textureLoad`.
@group(0) @binding(2) var zax_smooth: sampler;
// What this pass reads besides the depth: the occlusion, the frame's colour, the floor mask —
// whichever the pass wants, in the order its own header gives. The slots a pass does not read
// hold a 1 x 1 dummy.
@group(0) @binding(3) var zax_source0: texture_2d<f32>;
@group(0) @binding(4) var zax_source1: texture_2d<f32>;
@group(0) @binding(5) var zax_source2: texture_2d<f32>;

const ZAX_TAU: f32 = 6.283185307;

// The golden angle. Consecutive samples of a spiral land as far apart in azimuth as they can, so
// a kernel of six directions already covers a hemisphere evenly.
const ZAX_GOLDEN_ANGLE: f32 = 2.399963230;

struct ScreenVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// The full-screen triangle of resolve.vert, with the one change this API forces: NDC y points up
// and texture row 0 is at the top, so `uv.y` runs the other way round from the corner's.
@vertex
fn vs_screen(@builtin(vertex_index) index: u32) -> ScreenVertex {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: ScreenVertex;
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    out.position = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

// `gl_FragCoord.xy` of this fragment: the rows run bottom up in GL and top down here, so the
// y is mirrored about the target's height. The only reader is `zax_dither`, whose pattern the
// requirements leave free — but mirroring it means every pixel is rotated by the same angle on
// both backends, and the occlusion of a pixel therefore agrees to the compiler's rounding rather
// than to the kernel's variance.
fn zax_pixel(position: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(position.x, screen.drawn.w - position.y);
}

fn zax_texel() -> vec2<f32> {
    return screen.texel.xy;
}

// The window depth the frame holds at the viewport coordinate `uv`, read as GL's NEAREST with
// CLAMP_TO_EDGE reads it.
fn zax_window_depth(uv: vec2<f32>) -> f32 {
    let size = vec2<f32>(textureDimensions(zax_frame_depth));
    let at = clamp(floor(uv * screen.drawn.xy * size), vec2<f32>(0.0), size - 1.0);
    return textureLoad(zax_frame_depth, vec2<i32>(at), 0);
}

// One texel of `source`, the same way.
fn zax_load0(uv: vec2<f32>) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(zax_source0));
    let at = clamp(floor(uv * screen.drawn.xy * size), vec2<f32>(0.0), size - 1.0);
    return textureLoad(zax_source0, vec2<i32>(at), 0);
}

// The view-space `z` of a window depth, which is **negative** in front of the eye.
//
// The drawn geometry occupies the upper half of the depth buffer, 0.5 at the near plane and 1 at
// the far one, on this backend as on the other (see this file's header). So `z_ndc = 2w - 1`, and
// from `z_ndc = range * (z + near) / (-z)` comes `z = -range * near / (range + z_ndc)`.
fn zax_view_depth(window: f32) -> f32 {
    let ndc = (window * 2.0) - 1.0;
    return -(screen.projection.z * screen.projection.w) / (screen.projection.z + ndc);
}

// The view-space position the frame holds at the viewport coordinate `uv` (0 to 1 over the part
// drawn into), and in `w` the window depth that was read — 1 wherever nothing was drawn.
fn zax_view_sample(uv: vec2<f32>) -> vec4<f32> {
    let window = zax_window_depth(uv);
    let z = zax_view_depth(window);
    // clip.xy = view.xy * scale and clip.w = -view.z, so view.xy = ndc * (-z) / scale.
    let ndc = vec2<f32>((uv.x * 2.0) - 1.0, 1.0 - (uv.y * 2.0));
    return vec4<f32>((ndc * -z) / screen.projection.xy, z, window);
}

// Where a view-space point lands in viewport coordinates, and whether the projection says
// anything about it: `ok` is false at or behind the near plane.
struct ZaxProjected {
    uv: vec2<f32>,
    ok: bool,
}

fn zax_project_view(at: vec3<f32>) -> ZaxProjected {
    var out: ZaxProjected;
    if (at.z > -screen.projection.w) {
        out.uv = vec2<f32>(0.0);
        out.ok = false;
        return out;
    }
    let ndc = (at.xy * screen.projection.xy) / -at.z;
    out.uv = vec2<f32>((ndc.x * 0.5) + 0.5, 0.5 - (ndc.y * 0.5));
    out.ok = true;
    return out;
}

// Interleaved-gradient noise (Jimenez, 2014): a value in [0, 1) that changes from pixel to pixel,
// which is what rotates a kernel per pixel. **No noise texture**: a hash of the pixel coordinate
// does the whole job, one fewer resource to allocate, release, count and carry on a handheld.
fn zax_dither(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

// The distance in depth to a neighbour, or a value no comparison can win when nothing was drawn
// there: a neighbour at the far plane says nothing about this surface.
fn zax_neighbour(neighbour: vec4<f32>, z: f32) -> f32 {
    return select(1.0e30, abs(neighbour.z - z), neighbour.w < 1.0);
}

// The view-space normal of the surface at `uv`, from the depth alone, and whether there was one to
// reconstruct (REQ-SCN-083 reconstructs the normal "à partir de la profondeur de la scène"; there
// is no normal buffer and neither screen-space requirement asks for one).
struct ZaxNormal {
    normal: vec3<f32>,
    ok: bool,
}

// Of the two differences available on each axis the **closer in depth** is taken: across a
// silhouette the far neighbour belongs to another surface, and taking it would tilt the normal
// into exactly the sharp outline REQ-SCN-083 forbids ("sans contour net").
//
// Where an axis has **neither** neighbour on a surface — the sharp corner of a silhouette, a few
// pixels of any scene — there is no plane to speak of, and the caller answers "no effect" rather
// than invent a normal: a pixel it cannot read is then left exactly alone.
fn zax_view_normal(uv: vec2<f32>, p: vec3<f32>) -> ZaxNormal {
    let texel = zax_texel();
    let right = zax_view_sample(uv + vec2<f32>(texel.x, 0.0));
    let left = zax_view_sample(uv - vec2<f32>(texel.x, 0.0));
    let up = zax_view_sample(uv + vec2<f32>(0.0, texel.y));
    let down = zax_view_sample(uv - vec2<f32>(0.0, texel.y));

    var out: ZaxNormal;
    out.normal = vec3<f32>(0.0);
    out.ok = false;
    let along_x = (right.w < 1.0) || (left.w < 1.0);
    let along_y = (up.w < 1.0) || (down.w < 1.0);
    if (!along_x || !along_y) {
        return out;
    }

    let dx = select(p - left.xyz, right.xyz - p, zax_neighbour(right, p.z) < zax_neighbour(left, p.z));
    let dy = select(p - down.xyz, up.xyz - p, zax_neighbour(up, p.z) < zax_neighbour(down, p.z));
    let normal = normalize(cross(dx, dy));
    // The eye is the origin of view space, so the normal of a visible surface points towards it.
    // Fixing the sign here rather than reasoning about the winding keeps this correct whatever the
    // frame's handedness and whatever the mirrored placements of Q-069 did.
    out.normal = select(normal, -normal, dot(normal, p) > 0.0);
    out.ok = true;
    return out;
}
