// The depth of the rendered frame, as the screen-space passes of AMD-055 REQ-SCN-080 read it,
// and the view-space position it reconstructs to. Shared by the occlusion of REQ-SCN-083 and the
// reflections of REQ-SCN-084, which both work on "la profondeur et la couleur **rendues**".
//
// The frame's depth, as a texture (`gl/linear_buffer.rs`), and the part of it the frame drew
// into: (1, 1) unless the host reserved room for a size that changes often.
uniform sampler2D uDepth;
uniform vec2 uDrawnScale;

// The projection, as the four numbers these passes need of it, read straight off the matrix
// `numerics::Matrix4x4::create_perspective_field_of_view` built:
//   xy = the two scales (m11, m22), z = `range` = far/(near − far), w = `near`.
// Four floats rather than the inverse matrix, because that projection is symmetric and the
// renderer has no other: every term below is one multiply.
uniform vec4 uProjection;

/// The view-space `z` of a window depth, which is **negative** in front of the eye.
///
/// Two conventions meet here and both have to be undone. The projection of SPEC-06 maps the clip
/// depth to `[0, 1]` (the Direct3D convention `CreatePerspectiveFieldOfView` has), and OpenGL
/// then maps the NDC `[-1, 1]` to the window `[0, 1]`: the drawn geometry therefore occupies the
/// **upper half** of the depth buffer, `0.5` at the near plane and `1` at the far one. So
/// `z_ndc = 2w − 1`, and from `z_ndc = range·(z + near) / (−z)` comes
/// `z = −range·near / (range + z_ndc)`.
float zaxViewDepth(float window)
{
    float ndc = (window * 2.0) - 1.0;
    return -(uProjection.z * uProjection.w) / (uProjection.z + ndc);
}

/// The view-space position the frame holds at the viewport coordinate `uv` (0 to 1 over the part
/// drawn into), and in `w` the window depth that was read — `1` wherever nothing was drawn.
vec4 zaxViewSample(vec2 uv)
{
    float window = texture(uDepth, uv * uDrawnScale).r;
    float z = zaxViewDepth(window);
    // clip.xy = view.xy · scale and clip.w = −view.z, so view.xy = ndc · (−z) / scale.
    vec2 ndc = (uv * 2.0) - 1.0;
    return vec4((ndc * -z) / uProjection.xy, z, window);
}

/// Where a view-space point lands in viewport coordinates. `false` when it is at or behind the
/// near plane, where the projection says nothing.
bool zaxProjectView(vec3 at, out vec2 uv)
{
    if (at.z > -uProjection.w)
    {
        uv = vec2(0.0);
        return false;
    }

    uv = (((at.xy * uProjection.xy) / -at.z) * 0.5) + 0.5;
    return true;
}
