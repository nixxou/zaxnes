using System.Numerics;
using Zax.Engine3D;

namespace Zax.Render;

/// <summary>
/// The occlusion transparency of SPEC-06 REQ-SCN-022: in FPV with a followed shape, every tile
/// of <b>another</b> shape whose geometry crosses the segment from the eye to the target is
/// drawn transparent for that frame.
/// </summary>
/// <remarks>
/// The test is done on the tile's axis-aligned bounding box in scene coordinates, which is the
/// cheapest conservative test: a tile that is near the line of sight but does not quite hide the
/// character may fade too, never the other way round.
/// </remarks>
public readonly struct Occlusion
{
    private readonly Vector3 _eye;
    private readonly Vector3 _target;
    private readonly FrameShape? _followed;

    private Occlusion(Vector3 eye, Vector3 target, FrameShape? followed)
    {
        _eye = eye;
        _target = target;
        _followed = followed;
        Enabled = followed is not null;
    }

    /// <summary>Whether any tile can become an occluder this frame.</summary>
    public bool Enabled { get; }

    /// <summary>The disabled test.</summary>
    public static Occlusion None => default;

    /// <summary>
    /// The test for <paramref name="scene"/>: enabled only in FPV with a followed shape
    /// (REQ-SCN-021, REQ-SCN-022).
    /// </summary>
    public static Occlusion For(SceneState scene, in CameraView view)
    {
        ArgumentNullException.ThrowIfNull(scene);
        return scene.Camera.Mode == CameraMode.Fpv && scene.FollowedShape is { } shape
            ? new Occlusion(view.Eye, view.Target, shape)
            : None;
    }

    /// <summary>Whether <paramref name="tile"/> hides the followed shape this frame.</summary>
    /// <param name="tile">The tile of the draw list.</param>
    /// <param name="mesh">Its mesh, for the local bounding box.</param>
    /// <param name="model">Its placement, the sprite nudge included.</param>
    public bool Hides(in DrawTile tile, TileMesh mesh, Mat4 model)
    {
        ArgumentNullException.ThrowIfNull(mesh);
        if (!Enabled || ReferenceEquals(tile.Instance.Shape, _followed))
        {
            return false;
        }

        (Vector3 min, Vector3 max) = WorldBounds(mesh, model);
        return SegmentHitsBox(_eye, _target, min, max);
    }

    /// <summary>The scene-space bounding box of <paramref name="mesh"/> placed by <paramref name="model"/>.</summary>
    public static (Vector3 Min, Vector3 Max) WorldBounds(TileMesh mesh, Mat4 model)
    {
        ArgumentNullException.ThrowIfNull(mesh);
        Vec3 lo = mesh.BoundsMin;
        Vec3 hi = mesh.BoundsMax;

        Vector3 min = new(float.MaxValue);
        Vector3 max = new(float.MinValue);
        for (int corner = 0; corner < 8; corner++)
        {
            Vec3 point = new(
                (corner & 1) == 0 ? lo.X : hi.X,
                (corner & 2) == 0 ? lo.Y : hi.Y,
                (corner & 4) == 0 ? lo.Z : hi.Z);
            Vec3 placed = model.TransformPoint(point);
            Vector3 v = new(placed.X, placed.Y, placed.Z);
            min = Vector3.Min(min, v);
            max = Vector3.Max(max, v);
        }

        return (min, max);
    }

    /// <summary>
    /// Whether the segment <c>[a, b]</c> meets the axis-aligned box, by the usual slab test.
    /// </summary>
    public static bool SegmentHitsBox(Vector3 a, Vector3 b, Vector3 min, Vector3 max)
    {
        Vector3 direction = b - a;
        float enter = 0f;
        float exit = 1f;

        for (int axis = 0; axis < 3; axis++)
        {
            float origin = Component(a, axis);
            float delta = Component(direction, axis);
            float low = Component(min, axis);
            float high = Component(max, axis);

            if (MathF.Abs(delta) < 1e-6f)
            {
                if (origin < low || origin > high)
                {
                    return false;
                }

                continue;
            }

            float t0 = (low - origin) / delta;
            float t1 = (high - origin) / delta;
            if (t0 > t1)
            {
                (t0, t1) = (t1, t0);
            }

            enter = MathF.Max(enter, t0);
            exit = MathF.Min(exit, t1);
            if (enter > exit)
            {
                return false;
            }
        }

        return true;
    }

    private static float Component(Vector3 v, int axis) => axis switch
    {
        0 => v.X,
        1 => v.Y,
        _ => v.Z,
    };
}
