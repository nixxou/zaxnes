// The space bending of SPEC-06 REQ-SCN-067 (AMD-043), the GLSL mirror of
// `zax_engine3d::space_bend`. The per-axis constants k, f and h are computed on the CPU when a
// curvature setting changes and handed over as uniforms, so a vertex only pays for the wrapping
// itself.
//
// This chunk is compiled into the ZAX_BEND variants of the scene and shadow programs only
// (`gl/shaders.rs`). At curvature 0 the renderer runs the programs it ran before the amendment,
// which carry none of this: there is no uniform tested per vertex.
//
// The Jacobian is carried along because REQ-SCN-068 has the shading normal transported by the
// curvature (amending REQ-SCN-028). Its cofactor matrix is built from the cross products of its
// columns, which needs neither `inverse` nor a special case for the singular Jacobian the depth
// bound produces.

uniform vec3 uBendK;
uniform vec3 uBendF;
uniform vec3 uBendH;
// 1 for an axis that bends, 0 for one that does not. The test is uniform over the whole draw, so
// a curvature on one axis costs the other two a comparison and no transcendental.
uniform vec3 uBendOn;
uniform float uBendPivot;

const vec3 zaxBendCenter = vec3(128.0, 120.0, 0.0);
const float zaxBendDepthLimit = 0.95;

// E(k, f, h) of REQ-SCN-067 step 2, with its four partial derivatives in
// d = (du/du, du/dw, dw/du, dw/dw).
vec2 zaxWrap(float k, float f, float h, vec2 p, out vec4 d)
{
    // Neither direction of curvature goes past 95 % of the radius.
    bool held = p.y * k > zaxBendDepthLimit;
    float depth = held ? zaxBendDepthLimit / k : p.y;
    float slope = held ? 0.0 : 1.0;

    float a = p.x * k;
    float s = sin(a);
    float c = cos(a);
    float radial = 1.0 - depth * k;

    d = vec4(f * radial * c, -f * slope * s, f * radial * s, f * slope * c);
    return vec2(f * radial * s / k, f * ((1.0 - c) / k + depth * c - h));
}

// B(P) of REQ-SCN-067, with the Jacobian REQ-SCN-068 transports the normal by.
vec3 zaxBend(vec3 scene, out mat3 jacobian)
{
    vec3 q = scene - zaxBendCenter;
    jacobian = mat3(1.0);
    vec4 d;

    // Step 3, in this order: left-right on (u, w), top-bottom on (v, w), the arc on (u, p - v).
    if (uBendOn.x != 0.0)
    {
        vec2 r = zaxWrap(uBendK.x, uBendF.x, uBendH.x, vec2(q.x, q.z), d);
        q.x = r.x;
        q.z = r.y;
        jacobian = mat3(d.x, 0.0, d.z, 0.0, 1.0, 0.0, d.y, 0.0, d.w) * jacobian;
    }

    if (uBendOn.y != 0.0)
    {
        vec2 r = zaxWrap(uBendK.y, uBendF.y, uBendH.y, vec2(q.y, q.z), d);
        q.y = r.x;
        q.z = r.y;
        jacobian = mat3(1.0, 0.0, 0.0, 0.0, d.x, d.z, 0.0, d.y, d.w) * jacobian;
    }

    if (uBendOn.z != 0.0)
    {
        // The arc wraps (u, p - v) and reads the second component back as p - s.
        vec2 r = zaxWrap(uBendK.z, uBendF.z, uBendH.z, vec2(q.x, uBendPivot - q.y), d);
        q.x = r.x;
        q.y = uBendPivot - r.y;
        jacobian = mat3(d.x, -d.z, 0.0, -d.y, d.w, 0.0, 0.0, 0.0, 1.0) * jacobian;
    }

    return q + zaxBendCenter;
}

// The normal transported by the cofactor matrix of the Jacobian (REQ-SCN-068, REQ-SCN-028
// amended). The shader normalises it anyway, so the cofactor need not be scaled.
vec3 zaxBendNormal(mat3 jacobian, vec3 n)
{
    vec3 k0 = cross(jacobian[1], jacobian[2]);
    vec3 k1 = cross(jacobian[2], jacobian[0]);
    vec3 k2 = cross(jacobian[0], jacobian[1]);
    vec3 t = k0 * n.x + k1 * n.y + k2 * n.z;
    return dot(t, t) > 1e-12 ? t : n;
}
