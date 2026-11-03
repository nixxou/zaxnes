// The space bending of SPEC-06 REQ-SCN-067 (AMD-043), the WGSL twin of shaders/bend.glsl and,
// like it, a mirror of `zax_engine3d::space_bend`. The per-axis constants k, f and h are computed
// on the CPU when a curvature setting changes and reach the shader in the frame block, so a
// vertex only pays for the wrapping itself.
//
// This chunk is prepended to the ZAX_BEND variants of the scene and shadow modules only
// (`wgpu_backend/renderer.rs`). At curvature 0 the renderer builds the modules it built before
// RU-7, which carry none of this: no frame field is tested per vertex and the bent position is
// not a varying.
//
// The Jacobian is carried along because REQ-SCN-068 has the shading normal transported by the
// curvature (amending REQ-SCN-028). Its cofactor matrix is built from the cross products of its
// columns, which needs neither an inverse nor a special case for the singular Jacobian the depth
// bound produces.
//
// The frame block supplies k, f, h and `on` as `vec4`, the arc pivot riding in `bend_k.w`
// (`frame_bytes`); the GLSL has them as four `vec3` and a `float` uniform. Nothing else differs:
// the wrapping is the same arithmetic in the same order, so the two backends place a vertex at
// the same point up to the order the two compilers evaluate it in.

const ZAX_BEND_CENTER = vec3<f32>(128.0, 120.0, 0.0);
const ZAX_BEND_DEPTH_LIMIT = 0.95;

// What one wrapping returns: the two wrapped components, and the four partial derivatives in
// d = (du/du, du/dw, dw/du, dw/dw). WGSL has no `out` parameter, so the pair travels in a struct.
struct ZaxWrap {
    p: vec2<f32>,
    d: vec4<f32>,
}

// E(k, f, h) of REQ-SCN-067 step 2, with its four partial derivatives.
fn zax_wrap(k: f32, f: f32, h: f32, p: vec2<f32>) -> ZaxWrap {
    // Neither direction of curvature goes past 95 % of the radius.
    let held = p.y * k > ZAX_BEND_DEPTH_LIMIT;
    let depth = select(p.y, ZAX_BEND_DEPTH_LIMIT / k, held);
    let slope = select(1.0, 0.0, held);

    let a = p.x * k;
    let s = sin(a);
    let c = cos(a);
    let radial = 1.0 - depth * k;

    var out: ZaxWrap;
    out.d = vec4<f32>(f * radial * c, -f * slope * s, f * radial * s, f * slope * c);
    out.p = vec2<f32>(f * radial * s / k, f * ((1.0 - c) / k + depth * c - h));
    return out;
}

// What the bending returns: the placed position and the Jacobian REQ-SCN-068 transports the
// normal by.
struct ZaxBend {
    scene: vec3<f32>,
    jacobian: mat3x3<f32>,
}

// B(P) of REQ-SCN-067.
//
// `mat3x3<f32>(a, b, c)` takes three **columns** in WGSL, and `mat3(...)` takes three columns in
// GLSL too, so each of the three factors below is written with the same nine numbers in the same
// nine places as `bend.glsl` has them.
fn zax_bend(scene: vec3<f32>) -> ZaxBend {
    var q = scene - ZAX_BEND_CENTER;
    var jacobian = mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0));
    let k = frame_bend_k().xyz;
    let f = frame_bend_f().xyz;
    let h = frame_bend_h().xyz;
    let on = frame_bend_on().xyz;
    let pivot = frame_bend_k().w;

    // Step 3, in this order: left-right on (u, w), top-bottom on (v, w), the arc on (u, p - v).
    if (on.x != 0.0) {
        let r = zax_wrap(k.x, f.x, h.x, vec2<f32>(q.x, q.z));
        q.x = r.p.x;
        q.z = r.p.y;
        jacobian = mat3x3<f32>(
            vec3<f32>(r.d.x, 0.0, r.d.z),
            vec3<f32>(0.0, 1.0, 0.0),
            vec3<f32>(r.d.y, 0.0, r.d.w)) * jacobian;
    }

    if (on.y != 0.0) {
        let r = zax_wrap(k.y, f.y, h.y, vec2<f32>(q.y, q.z));
        q.y = r.p.x;
        q.z = r.p.y;
        jacobian = mat3x3<f32>(
            vec3<f32>(1.0, 0.0, 0.0),
            vec3<f32>(0.0, r.d.x, r.d.z),
            vec3<f32>(0.0, r.d.y, r.d.w)) * jacobian;
    }

    if (on.z != 0.0) {
        // The arc wraps (u, p - v) and reads the second component back as p - s.
        let r = zax_wrap(k.z, f.z, h.z, vec2<f32>(q.x, pivot - q.y));
        q.x = r.p.x;
        q.y = pivot - r.p.y;
        jacobian = mat3x3<f32>(
            vec3<f32>(r.d.x, -r.d.z, 0.0),
            vec3<f32>(-r.d.y, r.d.w, 0.0),
            vec3<f32>(0.0, 0.0, 1.0)) * jacobian;
    }

    var out: ZaxBend;
    out.scene = q + ZAX_BEND_CENTER;
    out.jacobian = jacobian;
    return out;
}

// The normal transported by the cofactor matrix of the Jacobian (REQ-SCN-068, REQ-SCN-028
// amended). The shader normalises it anyway, so the cofactor need not be scaled.
fn zax_bend_normal(jacobian: mat3x3<f32>, n: vec3<f32>) -> vec3<f32> {
    let k0 = cross(jacobian[1], jacobian[2]);
    let k1 = cross(jacobian[2], jacobian[0]);
    let k2 = cross(jacobian[0], jacobian[1]);
    let t = k0 * n.x + k1 * n.y + k2 * n.z;
    return select(n, t, dot(t, t) > 1e-12);
}
