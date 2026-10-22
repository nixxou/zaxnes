using System.Numerics;
using Zax.Engine3D;

namespace Zax.Render.Gl;

/// <summary>
/// Converts between the matrix conventions of the 3D engine and of <see cref="Matrix4x4"/>.
/// </summary>
/// <remarks>
/// <para>
/// <see cref="Mat4"/> holds its cells row by row and is applied to column vectors
/// (<c>P = M · (v, 1)</c>, SPEC-04B REQ-E3D-406). <see cref="Matrix4x4"/> holds <c>M11..M14</c>
/// as its first row and is applied to row vectors, so the same transform is the transpose.
/// </para>
/// <para>
/// GLSL reads a <c>mat4</c> uniform column by column. Uploading the fields of a
/// <see cref="Matrix4x4"/> in field order therefore hands GLSL its transpose, that is the
/// column-vector matrix the shaders use — which is why the conversion below transposes.
/// </para>
/// </remarks>
internal static class MatrixInterop
{
    /// <summary>The <see cref="Matrix4x4"/> that uploads <paramref name="matrix"/> unchanged.</summary>
    public static Matrix4x4 ToNumerics(Mat4 matrix)
    {
        Span<float> cells = stackalloc float[16];
        matrix.CopyTo(cells);
        return new Matrix4x4(
            cells[0], cells[4], cells[8], cells[12],
            cells[1], cells[5], cells[9], cells[13],
            cells[2], cells[6], cells[10], cells[14],
            cells[3], cells[7], cells[11], cells[15]);
    }

    /// <summary>
    /// The cofactor matrix of the upper-left 3 × 3 block, row by row. It is
    /// <c>det(M) · M⁻ᵗ</c>, so once the shader normalises the result it is the normal matrix of
    /// REQ-SCN-028 for every transform of positive determinant. REQ-SCN-028 does not say how
    /// the mesh normal crosses an anisotropic object scale; the inverse transpose is the
    /// provisional choice of Q-021.
    /// </summary>
    /// <returns>
    /// The determinant of that block, which the first row of the cofactors already holds: it is
    /// negative exactly when the placement mirrors the object, the case of Q-069.
    /// </returns>
    public static float NormalMatrix(Mat4 matrix, Span<float> rowMajor)
    {
        float m00 = matrix[0, 0];
        float m01 = matrix[0, 1];
        float m02 = matrix[0, 2];
        float m10 = matrix[1, 0];
        float m11 = matrix[1, 1];
        float m12 = matrix[1, 2];
        float m20 = matrix[2, 0];
        float m21 = matrix[2, 1];
        float m22 = matrix[2, 2];

        rowMajor[0] = (m11 * m22) - (m12 * m21);
        rowMajor[1] = (m12 * m20) - (m10 * m22);
        rowMajor[2] = (m10 * m21) - (m11 * m20);
        rowMajor[3] = (m02 * m21) - (m01 * m22);
        rowMajor[4] = (m00 * m22) - (m02 * m20);
        rowMajor[5] = (m01 * m20) - (m00 * m21);
        rowMajor[6] = (m01 * m12) - (m02 * m11);
        rowMajor[7] = (m02 * m10) - (m00 * m12);
        rowMajor[8] = (m00 * m11) - (m01 * m10);

        return (m00 * rowMajor[0]) + (m01 * rowMajor[1]) + (m02 * rowMajor[2]);
    }

    /// <summary>A <see cref="Vec3"/> as a <see cref="Vector3"/>.</summary>
    public static Vector3 ToVector(Vec3 v) => new(v.X, v.Y, v.Z);

    /// <summary>A <see cref="Vector3"/> as a <see cref="Vec3"/>.</summary>
    public static Vec3 ToVec3(Vector3 v) => new(v.X, v.Y, v.Z);

    /// <summary>
    /// The affine map of REQ-SCN-045 that takes a point of the reference frame to the plane
    /// attached to the camera, as a <see cref="Mat4"/> so it composes with the engine's
    /// transforms.
    /// </summary>
    public static Mat4 HudMatrix(in CameraView view)
    {
        Vector3 right = view.Right * SceneConstants.HudScale;
        Vector3 up = view.Up * SceneConstants.HudScale;
        Vector3 forward = view.Forward * SceneConstants.HudScale;
        Vector3 origin = view.Eye
            + (view.Forward * view.HudDistance)
            - (right * SceneConstants.Center.X)
            - (up * SceneConstants.Center.Y)
            - (forward * SceneConstants.Center.Z);

        Mat4 m = Mat4.Identity;
        m[0, 0] = right.X;
        m[1, 0] = right.Y;
        m[2, 0] = right.Z;
        m[0, 1] = up.X;
        m[1, 1] = up.Y;
        m[2, 1] = up.Z;
        m[0, 2] = forward.X;
        m[1, 2] = forward.Y;
        m[2, 2] = forward.Z;
        m[0, 3] = origin.X;
        m[1, 3] = origin.Y;
        m[2, 3] = origin.Z;
        return m;
    }
}
