using System.Numerics;
using Zax.Engine3D;
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
    /// The curvature the aim is to be computed with, as AMD-046 REQ-SCN-074 amends REQ-INP-124:
    /// in 2D mode with « Bend 2D Mode » off, the rectangle the pointer designates is at its flat
    /// position, so REQ-INP-124 does not apply and the aim is the one of REQ-INP-063 — the
    /// identity. Everywhere else, <paramref name="bend"/> is handed back unchanged.
    /// </summary>
    /// <param name="twoDimensionalMode">Whether the view shows the 2D-mode rectangle.</param>
    /// <param name="bend2dMode">The setting <c>scene.bend_2d_mode</c>.</param>
    /// <param name="bend">The curvature of the frame (REQ-SCN-067).</param>
    public static SpaceBend AimBend(bool twoDimensionalMode, bool bend2dMode, SpaceBend bend)
        => twoDimensionalMode && !bend2dMode ? SpaceBend.None : bend;

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
        => TryAim(view, pointer, size, offset, sceneScale, SpaceBend.None, out position);

    /// <summary>
    /// The aim of REQ-INP-124: the same thing on a scene the curvature of AMD-043 has bent.
    /// </summary>
    /// <param name="view">The camera view of the frame.</param>
    /// <param name="pointer">Pointer position in the render target, origin top left.</param>
    /// <param name="size">Size of the render target, in pixels.</param>
    /// <param name="offset">The <c>ZapperOffset</c> of the profile's scripts, in scene units.</param>
    /// <param name="sceneScale">The scene scale <c>e</c>; 1 on desktop.</param>
    /// <param name="bend">The curvature in force; the identity falls back to REQ-INP-063.</param>
    /// <param name="position">The aimed NES pixel.</param>
    /// <returns>False when there is no candidate at all (REQ-INP-064).</returns>
    public static bool TryAim(
        in CameraView view,
        Vector2 pointer,
        Vector2 size,
        Vector3 offset,
        float sceneScale,
        SpaceBend bend,
        out ZapperPosition position)
    {
        if (!TryIntersect(view, pointer, size, offset, bend, sceneScale, out Vector2 point))
        {
            position = default;
            return false;
        }

        position = ToNesPixel(point, offset, sceneScale);
        return true;
    }

    /// <summary>
    /// The point of the plane <c>z = offset.Z</c> the pointer designates on a bent scene
    /// (REQ-INP-124): the candidates are the points whose <b>image</b> <c>B(P)</c> lies on the
    /// aiming ray, in front of the camera; the one taken is the nearest to the ray's origin
    /// among those inside the aiming window, or, failing that, the nearest of all.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <c>B</c> is invertible wherever the depth limit of REQ-SCN-067 does not bite
    /// (AMD-043 §4), so the candidates are read off the ray rather than searched for on the
    /// plane: the ray is walked forward, the preimage of each sample is taken, and a change of
    /// sign of <c>P.z − z₀</c> between two samples brackets a candidate, which a bisection then
    /// locates. Walking <b>forward</b> is what makes the first candidate found the nearest one.
    /// </para>
    /// <para>
    /// The walk is one scene unit at a time, over the sphere of
    /// <see cref="SearchRadius"/> units around the centre of the reference frame: <c>B</c> is
    /// close to an isometry (<c>f</c> is within 12 % of 1 over the whole range of angles), so a
    /// one-unit sample cannot step over a crossing of a plane. Each candidate is then checked
    /// against its own image, which rejects the ones an arc-tangent branch would invent.
    /// </para>
    /// </remarks>
    public static bool TryIntersect(
        in CameraView view,
        Vector2 pointer,
        Vector2 size,
        Vector3 offset,
        SpaceBend bend,
        float sceneScale,
        out Vector2 point)
    {
        if (bend.IsIdentity)
        {
            return TryIntersect(view, pointer, size, offset, out point);
        }

        point = Vector2.Zero;
        (Vector3 origin, Vector3 direction) = Ray(view, pointer, size);
        float length = direction.Length();
        if (length <= 1e-6f || !float.IsFinite(length))
        {
            return false;
        }

        Vector3 step = direction / length;
        if (!Span(origin, step, out float near, out float far))
        {
            return false;
        }

        float scale = sceneScale is > 0f and <= float.MaxValue ? sceneScale : 1f;
        bool hasAny = false;
        Vector2 nearest = Vector2.Zero;

        bool hasPrevious = false;
        float previousT = near;
        float previousDepth = 0f;

        for (float t = near; t <= far; t += MarchStep)
        {
            if (!Preimage(bend, origin, step, t, out Vector3 preimage))
            {
                // The depth limit of REQ-SCN-067 reached: no preimage here, and no bracket
                // across the gap either.
                hasPrevious = false;
                continue;
            }

            float depth = preimage.Z - offset.Z;
            if (hasPrevious
                && (depth == 0f || previousDepth == 0f || (depth < 0f) != (previousDepth < 0f))
                && Refine(bend, origin, step, offset.Z, previousT, t, out Vector2 candidate))
            {
                if (!hasAny)
                {
                    hasAny = true;
                    nearest = candidate;
                }

                if (InAimingWindow(candidate, scale))
                {
                    point = candidate;
                    return true;
                }
            }

            hasPrevious = true;
            previousT = t;
            previousDepth = depth;
        }

        // Only candidates outside the window: the nearest is handed over unbounded, as the
        // off-screen shot of REQ-INP-064 (REQ-INP-124).
        point = nearest;
        return hasAny;
    }

    /// <summary>
    /// The aiming window of REQ-INP-124: the NES image of REQ-SCN-001 widened by 16 scene
    /// units on each of its four edges.
    /// </summary>
    /// <param name="point">A candidate point of the aim plane.</param>
    /// <param name="sceneScale">The scene scale <c>s</c>.</param>
    public static bool InAimingWindow(Vector2 point, float sceneScale = 1f)
        => MathF.Abs(point.X - 128f) <= WindowHalfWidth * sceneScale
        && MathF.Abs(point.Y - 128f) <= WindowHalfHeight * sceneScale;

    /// <summary>Half-width of the aiming window of REQ-INP-124, 144 scene units.</summary>
    public const float WindowHalfWidth = 144f;

    /// <summary>Half-height of the aiming window of REQ-INP-124, 136 scene units.</summary>
    public const float WindowHalfHeight = 136f;

    /// <summary>
    /// Radius of the sphere around <c>C</c> the candidates are looked for in. The bent image of
    /// the aiming window never leaves it: the curvature of REQ-SCN-067 rolls the scene up, it
    /// never spreads it out.
    /// </summary>
    public const float SearchRadius = 1024f;

    /// <summary>Distance between two samples of the ray, in scene units.</summary>
    private const float MarchStep = 1f;

    /// <summary>How far a candidate's image may be from the ray before it is rejected.</summary>
    private const float ImageTolerance = 0.25f;

    private static bool Preimage(
        in SpaceBend bend,
        Vector3 origin,
        Vector3 step,
        float t,
        out Vector3 preimage)
    {
        Vector3 q = origin + (step * t);
        if (!bend.TryInvert(new Vec3(q.X, q.Y, q.Z), out Vec3 found))
        {
            preimage = Vector3.Zero;
            return false;
        }

        preimage = new Vector3(found.X, found.Y, found.Z);
        return float.IsFinite(preimage.X) && float.IsFinite(preimage.Y) && float.IsFinite(preimage.Z);
    }

    /// <summary>
    /// Bisects <c>[low, high]</c>, where the preimage crosses the plane <c>z = plane</c>, and
    /// checks the point found against its own image.
    /// </summary>
    private static bool Refine(
        in SpaceBend bend,
        Vector3 origin,
        Vector3 step,
        float plane,
        float low,
        float high,
        out Vector2 point)
    {
        point = Vector2.Zero;
        if (!Preimage(bend, origin, step, low, out Vector3 start))
        {
            return false;
        }

        bool below = start.Z - plane < 0f;
        Vector3 found = start;
        float middle = low;

        for (int i = 0; i < Bisections; i++)
        {
            middle = 0.5f * (low + high);
            if (!Preimage(bend, origin, step, middle, out found))
            {
                return false;
            }

            if ((found.Z - plane < 0f) == below)
            {
                low = middle;
            }
            else
            {
                high = middle;
            }
        }

        // The arc tangent of the inverse is taken on a principal branch; a candidate whose
        // image is not on the ray comes from the wrong one and is dropped.
        Vector3 image = origin + (step * middle);
        Vec3 bent = bend.Apply(new Vec3(found.X, found.Y, found.Z));
        if (Vector3.Distance(new Vector3(bent.X, bent.Y, bent.Z), image) > ImageTolerance)
        {
            return false;
        }

        point = new Vector2(found.X, found.Y);
        return float.IsFinite(point.X) && float.IsFinite(point.Y);
    }

    /// <summary>How many halvings the bracket is cut by; 32 takes a one-unit span under 10⁻⁹.</summary>
    private const int Bisections = 32;

    /// <summary>
    /// The part of the ray that is both in front of the camera and inside the search sphere.
    /// </summary>
    private static bool Span(Vector3 origin, Vector3 step, out float near, out float far)
    {
        near = 0f;
        far = 0f;

        Vec3 centre = SpaceBend.Center;
        Vector3 toOrigin = origin - new Vector3(centre.X, centre.Y, centre.Z);
        float b = 2f * Vector3.Dot(step, toOrigin);
        float c = Vector3.Dot(toOrigin, toOrigin) - (SearchRadius * SearchRadius);
        float discriminant = (b * b) - (4f * c);
        if (discriminant < 0f)
        {
            return false;
        }

        float root = MathF.Sqrt(discriminant);
        near = MathF.Max((-b - root) * 0.5f, 1e-3f);
        far = (-b + root) * 0.5f;
        return far > near;
    }
}
