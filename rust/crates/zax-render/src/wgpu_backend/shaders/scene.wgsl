// The scene pass of the wgpu backend, the WGSL twin of scene.vert and scene.frag: albedo of
// REQ-SCN-034, visible-rectangle clipping of REQ-SCN-036, hard shadow of REQ-SCN-030/031 and
// the Lambert shading of REQ-SCN-028, written linear (AMD-021 R2-009). The transfer functions
// and the deformation come from common.wgsl, prepended by the renderer.

struct Frame {
    view_projection: mat4x4<f32>,
    light_view_projection: mat4x4<f32>,
    clip_rect: vec4<f32>,
    // xyz the linear light colour, w the ambient term.
    light_ambient: vec4<f32>,
    // x time, y shadow strength, z shadow bias, w normal offset.
    params: vec4<f32>,
    // The light direction of the scene buckets, then the HUD's.
    light: array<vec4<f32>, 2>,
    // AMD-043 REQ-SCN-067: the per-axis curvature constants, xyz each, the arc pivot in bend_k.w
    // and "this axis bends" as 1 or 0 in bend_on. Written by `frame_bytes` whatever the variant,
    // because the frame block has one layout; read only by the ZAX_BEND variant below.
    bend_k: vec4<f32>,
    bend_f: vec4<f32>,
    bend_h: vec4<f32>,
    bend_on: vec4<f32>,
    // AMD-055: x the voxel cell side of REQ-SCN-081 (`SceneConstants::voxel_cell`), yzw the
    // vertical the floor definition of REQ-SCN-084 is written against (`style::floor_vertical`).
    // Both written whatever the variant, read only by ZAX_VOXEL_GRID and ZAX_FLOOR_MASK: the floor
    // vertical rides in the three spare slots of a `vec4` the block already had, so the frame
    // block does **not** grow for the reflections and a Classic frame copies no more bytes than it
    // copied after step 2.
    style: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var palette: texture_2d<f32>;
@group(0) @binding(2) var shadow_map: texture_depth_2d;

//#ifdef ZAX_BEND
// What bend.wgsl reads its constants through: the chunk is shared with the shadow module, whose
// frame block is a different struct under a different name.
fn frame_bend_k() -> vec4<f32> { return frame.bend_k; }
fn frame_bend_f() -> vec4<f32> { return frame.bend_f; }
fn frame_bend_h() -> vec4<f32> { return frame.bend_h; }
fn frame_bend_on() -> vec4<f32> { return frame.bend_on; }
//#endif

// The textures of group 2 and zax_index_texel / zax_color_texel come from textures_bound.wgsl
// or textures_bindless.wgsl, appended by the renderer.

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) scene: vec3<f32>,
    @location(1) normal: vec3<f32>,
//#ifdef ZAX_CENTROID
    // AMD-058 REQ-SCN-065: interpolated at the centroid of the samples the fragment covers,
    // which is always inside them. A fragment on the edge of a wall whose pixel centre falls
    // *outside* the triangle otherwise gets an extrapolated `uv`, the `floor` below then lands
    // one texel past the face, and the atlas returns a neighbour's texel — a light line one pixel
    // wide along the joint. Only ever asked for when the buffer is multisampled; with
    // `video.anti_alias` = 0 the centroid **is** the pixel centre, so the variant would change
    // nothing and is not built (the handheld default).
    @location(2) @interpolate(perspective, centroid) uv: vec2<f32>,
//#else
    @location(2) uv: vec2<f32>,
//#endif
    @location(3) @interpolate(flat) run: u32,
//#ifdef ZAX_BEND
    // The position the curvature of REQ-SCN-067 placed this vertex at, which the shadow lookup
    // reads (the shadow map holds the bent geometry, REQ-SCN-068). Without the variant the
    // fragment stage reads `scene`, exactly as before the amendment, and this varying does not
    // exist — which is the whole point of its being a variant (`preprocess.rs`).
    @location(4) bent: vec3<f32>,
//#endif
//#if ZAX_VOXEL_GRID || ZAX_PIXEL_SHADOW
    // AMD-055 REQ-SCN-081 and REQ-SCN-082: the position and the normal in the **mesh's local
    // frame** (REQ-E3D-504 for GREEDY and GREEDY_MULTICOLOR, REQ-E3D-522 for MARCHING), before
    // the animated deformation, before the object's transform and before the curvature. That is
    // the frame both requirements evaluate in, and it is why the grooves and the shadow squares
    // follow the object's scale, its rotation and the curved surface.
    @location(5) local: vec3<f32>,
    @location(6) local_normal: vec3<f32>,
//#endif
//#ifdef ZAX_PIXEL_SHADOW
    // AMD-055 REQ-SCN-082: the placement the fragment stage needs to transform a voxel centre
    // like a vertex. The snapping varies **across** a face, so it cannot be done in the vertex
    // stage; and the placements are a storage buffer the fragment stage cannot see, so the linear
    // part of this instance's travels. Only the **linear part**, nine floats rather than sixteen:
    // the placement is affine, so the translation cancels in the difference between the snapped
    // point and the fragment's own. `flat` because a placement is one value per instance.
    @location(7) @interpolate(flat) model_row0: vec3<f32>,
    @location(8) @interpolate(flat) model_row1: vec3<f32>,
    @location(9) @interpolate(flat) model_row2: vec3<f32>,
    @location(10) @interpolate(flat) deform_tile: vec3<f32>,
//#endif
//#ifdef ZAX_FLOOR_MASK
    // AMD-055 REQ-SCN-084: the normal in the scene frame **before the curvature**, which is the
    // frame the floor definition is written in ("la normale, exprimée dans le repère de scène
    // avant courbure"). The curvature overwrites `normal` below (REQ-SCN-068 transports it), so
    // the mask pass takes its own copy of the value that requirement judges. Emitted by no other
    // variant.
    @location(11) scene_normal: vec3<f32>,
//#endif
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
) -> VertexOut {
    let instance = instances[instance_index];
    let run = run_of[instance_index];
    let draw = draws[run];
    var local = position;
    if (draw.modes2.x >= 0) {
        local += zax_deformation(deforms[draw.modes2.x], instance.deform_tile.xyz, position, frame.params.x);
    }

    let scene = zax_place(instance, local);
    var out: VertexOut;
    out.scene = scene.xyz;
    // REQ-SCN-028: the normal is the mesh normal, unchanged by the deformation. The sign is -1
    // for a mirrored placement, whose cofactor matrix points the normal inwards (Q-069).
    out.normal = draw.tilting.z * zax_place_normal(instance, normal);
    out.uv = uv;
    out.run = run;
//#if ZAX_VOXEL_GRID || ZAX_PIXEL_SHADOW
    // The untouched attributes: `position` is the voxel-grid position of REQ-E3D-504 and `normal`
    // the mesh normal there — the face normal in GREEDY and GREEDY_MULTICOLOR, the triangle's
    // flat normal in MARCHING (REQ-E3D-522), which makes it constant over a triangle and so
    // unchanged by the interpolation.
    out.local = position;
    out.local_normal = normal;
//#endif
//#ifdef ZAX_PIXEL_SHADOW
    out.model_row0 = instance.rows[0].xyz;
    out.model_row1 = instance.rows[1].xyz;
    out.model_row2 = instance.rows[2].xyz;
    out.deform_tile = instance.deform_tile.xyz;
//#endif
//#ifdef ZAX_FLOOR_MASK
    // Before the curvature, and after the normal matrix: the scene normal of REQ-SCN-084.
    out.scene_normal = out.normal;
//#endif
//#ifdef ZAX_BEND
    // REQ-SCN-067, applied last: after the animated deformation and after the object's own
    // transform, so on the final position in the scene (REQ-SCN-068). REQ-SCN-068 transports the
    // normal with it. `out.scene` keeps the position **before** the curvature, because
    // REQ-SCN-068 has the clipping of REQ-SCN-036 evaluated there — a tile the margins keep stays
    // visible wherever its bent image lands.
    let placed = zax_bend(scene);
    out.normal = zax_bend_normal(placed.jacobian, out.normal);
    out.bent = placed.scene;
    out.position = zax_clip(frame.view_projection
        * vec4<f32>(placed.scene.x, placed.scene.y, -placed.scene.z, 1.0));
//#else
    out.position = zax_clip(frame.view_projection * vec4<f32>(scene.x, scene.y, -scene.z, 1.0));
//#endif
    return out;
}

//#ifdef ZAX_FLOOR_MASK
// The floor mask of AMD-055 REQ-SCN-084 (shaders/floor_mask.frag): where the reflections of §2.5
// may act at all.
//
// **Why there is a pass at all.** The requirement's definition of a floor surface is the
// conjunction of two tests, and the invariants of §2.5 forbid either from standing alone:
//
//   - the instance's bounding box in the floor plane, before curvature, at least `6 × échelle de
//     scène` in both dimensions — the variant R-s puts the same block at `scale = (1 ; 1 ; 0,5)`
//     beside itself, and the two show the same pixels, at the same depth, with the same normal, so
//     no reconstruction from the depth buffer can tell them apart. That half is decided on the CPU
//     (`frame_plan::FramePlan::mark_floor_surfaces`) and reaches this pass as *which runs are
//     drawn*;
//   - the face's own normal, `+Y ≥ 0,9` in the scene frame before curvature — the variant R-m is
//     the same shape without rotation, so its `xz` box measures 64 × 8 and **passes** the extent
//     test, and not one pixel of its front face may be modified. That half is this entry point.
//
// The vertex stage is the scene's own with `ZAX_FLOOR_MASK` defined, so the mask is laid down by
// exactly the geometry, the animated deformation, the placement and the curvature that drew the
// frame — it cannot drift from the depth it is tested against.

// `style::FLOOR_NORMAL_THRESHOLD`, the "+Y ≥ 0,9" of REQ-SCN-084.
const ZAX_FLOOR_NORMAL_THRESHOLD: f32 = 0.9;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // `style::floor_vertical`: `+Y`, or the image plane's normal (0, 0, -1) under the local camera
    // of REQ-SCN-009 (AMD-055 v1.4, Q-096). Decided once a frame, on the CPU.
    let up = dot(normalize(in.scene_normal), frame.style.yzw);
    return vec4<f32>(step(ZAX_FLOOR_NORMAL_THRESHOLD, up), 0.0, 0.0, 1.0);
}
//#else

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let draw = draws[in.run];
    // REQ-SCN-036: a fragment whose scene (x, y) leaves R is dropped, whatever its depth.
    let rect = frame.clip_rect;
    if (draw.modes.z != 0 && (in.scene.x < rect.x || in.scene.x > rect.z
                           || in.scene.y < rect.y || in.scene.y > rect.w)) {
        discard;
    }

    var albedo: vec3<f32>;
    var texel_alpha = 1.0;
    let color_mode = draw.modes.x;
    if (color_mode == 2) {
        albedo = draw.flat_alpha.xyz;
    } else {
        // REQ-E3D-563: nearest neighbour, no mipmap, texel coordinates taken modulo the
        // texture size.
        let size = draw.texture.xy;
        let uv = vec2<f32>(
            in.uv.x * draw.tilting.x + draw.texture.z * size.x,
            in.uv.y * draw.tilting.y + draw.texture.w * size.y);
        let texel = vec2<i32>(zax_mod(floor(uv), size));
        if (color_mode == 0) {
            // REQ-SCN-034: the index selects 4 * P + (t mod 4) in the frame palette.
            let value = zax_index_texel(draw, texel);
            let entry = clamp(draw.modes.y, 0, 15) * 4 + i32(value & 3u);
            albedo = textureLoad(palette, vec2<i32>(entry, 0), 0).rgb;
        } else {
            let direct = zax_color_texel(draw, texel);
            // A pixel of the 2D picture left out (interop::HIDDEN_PIXEL) is not drawn.
            if (direct.a == 0.0) {
                discard;
            }
            albedo = direct.rgb;
            texel_alpha = direct.a;
        }
    }

    // The albedo, linear. The OpenGL path reads the indexed albedo from an already-linear palette
    // and this one linearises per fragment (its `ZAX_SRGB_PALETTE` variant), which is why the
    // conversion is here rather than at the three places the albedo is read.
    var albedo_linear = zax_to_linear3(albedo);
//#ifdef ZAX_VOXEL_GRID
    // REQ-SCN-081: the grid modulates the **albedo**, before any lighting calculation
    // (REQ-SCN-028) — so on the linear albedo, as `scene.frag` does it. For a tile the position
    // is the one in the mesh's local frame; for the backdrop (colour mode 2) it is the scene
    // position of REQ-SCN-081's second paragraph. The test is uniform over a whole draw, not a
    // per-fragment switch between having the effect and not having it: that one is the variant.
    albedo_linear *= zax_voxel_grid(
        select(in.local, in.scene, color_mode == 2),
        normalize(in.local_normal),
        frame.style.x);
//#endif

    let n = normalize(in.normal);
    let l = normalize(-frame.light[i32(draw.tilting.w)].xyz);
    let ndl = max(0.0, dot(n, l));

    var shadow = 1.0;
    if (draw.modes.w != 0) {
        // REQ-SCN-031: a normal offset and a depth bias of about 0.02 unit.
        // REQ-SCN-068: the shadow map holds the **bent** geometry, so the lookup starts from the
        // bent position; without the variant there is only the one position.
//#ifdef ZAX_BEND
        var biased = in.bent + n * frame.params.w;
//#else
        var biased = in.scene + n * frame.params.w;
//#endif
//#ifdef ZAX_PIXEL_SHADOW
        // REQ-SCN-082: on an opaque **tile**, the shadow is read at the centre of the fragment's
        // voxel instead. The bias and the normal offset of REQ-SCN-031 are applied to that
        // displaced point and not to the fragment, or the shadow acne REQ-SCN-031 avoids comes
        // back (REQ-SCN-082, Notes). Colour mode 2 is the backdrop, which the requirement leaves
        // alone ("sans objet sur le backdrop"); the test reads the run's own block, constant over
        // the whole draw, as the grid's is. The HUD and the transparent objects carry
        // `modes.w` = 0, so they are already out.
        if (color_mode != 2) {
            biased = zax_voxel_shadow_point(
                draw,
                in.scene,
                in.local,
                normalize(in.local_normal),
                in.model_row0,
                in.model_row1,
                in.model_row2,
                in.deform_tile,
                frame.params.x) + n * frame.params.w;
        }
//#endif
        let light = frame.light_view_projection * vec4<f32>(biased.x, biased.y, -biased.z, 1.0);
        let projected = light.xyz / light.w * 0.5 + 0.5;
        if (projected.x >= 0.0 && projected.x <= 1.0
            && projected.y >= 0.0 && projected.y <= 1.0
            && projected.z <= 1.0) {
            // The map's rows run top down here, bottom up in GL: the same texel is 1 - y.
            let side = vec2<f32>(textureDimensions(shadow_map));
            let at = vec2<i32>(min(floor(vec2<f32>(projected.x, 1.0 - projected.y) * side), side - 1.0));
            let stored = textureLoad(shadow_map, at, 0);
            if (projected.z - frame.params.z > stored) {
                shadow = frame.params.y;
            }
        }
    }

    // REQ-SCN-028: albedo_lin * (ambient_lin + max(0, N.L) * light_lin * S).
    let lit = albedo_linear * (vec3<f32>(frame.light_ambient.w) + ndl * frame.light_ambient.xyz * shadow);
    return vec4<f32>(min(lit, vec3<f32>(1.0)), draw.flat_alpha.w * texel_alpha);
}
//#endif
