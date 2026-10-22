using System.Numerics;
using Zax.Nes;
using Zax.Render;

namespace Zax.App.Input;

/// <summary>
/// The Zapper aim of SPEC-08 §3.7: a ray cast from the game camera through the pointer, met with
/// the plane of the Zapper offset, read back as the NES pixel displayed under the pointer
/// (REQ-INP-063, DEC-08-4, DEC-R2-T2).
/// </summary>
/// <remarks>
/// <para>
/// The relation is the inverse relation of the scene frame (SPEC-04B REQ-E3D-410 point 5):
/// <c>px = trunc((x − Ox) / e)</c> and <c>py = trunc(248 − (y − Oy) / e)</c>, with
/// <c>(Ox, Oy)</c> the Zapper offset a profile script may animate and <c>e</c> the scene scale,
/// which is 1 on desktop.
/// </para>
/// <para>
/// DEC-08-4 is an accepted divergence <b>(D)</b> on two points: the scale is taken into account,
/// and the aim is the pixel actually displayed under the pointer instead of the original's point
/// 8 lines above it.
/// </para>
/// </remarks>
public static class ZapperAim
{
    /// <summary>The y-axis flip constant of the scene frame, <c>Hs = 248</c> (REQ-E3D-410).</summary>
    public const float SceneHeight = 248f;

    /// <summary>
    /// The ray the pointer casts through the camera, in scene coordinates.
    /// </summary>
    /// <param name="view">The camera view of the frame.</param>
    /// <param name="pointer">Pointer position in the render target, origin top left.</param>
    /// <param name="size">Size of the render target, in pixels.</param>
    public static (Vector3 Origin, Vector3 Direction) Ray(in CameraView view, Vector2 pointer, Vector2 size)
    {
        float width = MathF.Max(size.X, 1f);
        float height = MathF.Max(size.Y, 1f);

        // Normalised device coordinates: x to the right, y upwards, so the screen y is flipped.
        float ndcX = (2f * pointer.X / width) - 1f;
        float ndcY = 1f - (2f * pointer.Y / height);

        float tanHalfFov = MathF.Tan(view.Fov * MathF.PI / 360f);
        Vector3 direction = view.Forward
            + (view.Right * (ndcX * tanHalfFov * view.Aspect))
            + (view.Up * (ndcY * tanHalfFov));

        return (view.Eye, direction);
    }

    /// <summary>
    /// Where the pointer's ray meets the plane <c>z = offset.Z</c> of the scene frame
    /// (REQ-INP-063).
    /// </summary>
    /// <returns>
    /// False when the ray is parallel to the plane or points away from it, in which case the
    /// caller keeps the last valid position (REQ-INP-064).
    /// </returns>
    public static bool TryIntersect(
        in CameraView view,
        Vector2 pointer,
        Vector2 size,
        Vector3 offset,
        out Vector2 point)
    {
        (Vector3 origin, Vector3 direction) = Ray(view, pointer, size);
        point = Vector2.Zero;

        if (MathF.Abs(direction.Z) < 1e-6f)
        {
            return false;
        }

        float t = (offset.Z - origin.Z) / direction.Z;
        if (t <= 0f || !float.IsFinite(t))
        {
            return false;
        }

        point = new Vector2(origin.X + (t * direction.X), origin.Y + (t * direction.Y));
        return float.IsFinite(point.X) && float.IsFinite(point.Y);
    }

    /// <summary>
    /// The NES pixel a scene point of the aim plane shows (REQ-INP-063, DEC-08-4):
    /// <c>px = trunc((x − Ox) / e)</c>, <c>py = trunc(248 − (y − Oy) / e)</c>.
    /// </summary>
    /// <param name="point">The intersection point, in scene coordinates.</param>
    /// <param name="offset">The <c>ZapperOffset</c> of the profile's scripts, in scene units.</param>
    /// <param name="sceneScale">The scene scale <c>e</c>; 1 on desktop.</param>
    public static ZapperPosition ToNesPixel(Vector2 point, Vector3 offset, float sceneScale = 1f)
    {
        float scale = sceneScale is > 0f and <= float.MaxValue ? sceneScale : 1f;

        // Truncation towards zero, as REQ-INP-063 spells out: a point left of or above the
        // image gives a negative pixel, which is the unclamped off-screen shot of REQ-INP-064.
        float px = (point.X - offset.X) / scale;
        float py = SceneHeight - ((point.Y - offset.Y) / scale);

        return new ZapperPosition((int)MathF.Truncate(px), (int)MathF.Truncate(py));
    }

    /// <summary>
    /// The complete aim of REQ-INP-063: the ray, the plane and the pixel, in one call.
    /// </summary>
    /// <returns>False when the ray misses the plane (REQ-INP-064).</returns>
    public static bool TryAim(
        in CameraView view,
        Vector2 pointer,
        Vector2 size,
        Vector3 offset,
        float sceneScale,
        out ZapperPosition position)
    {
        if (!TryIntersect(view, pointer, size, offset, out Vector2 point))
        {
            position = default;
            return false;
        }

        position = ToNesPixel(point, offset, sceneScale);
        return true;
    }
}
