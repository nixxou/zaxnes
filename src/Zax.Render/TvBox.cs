using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Render;

/// <summary>An axis-aligned box of the TV tunnel, in scene coordinates (REQ-SCN-053).</summary>
/// <param name="Min">Lowest corner.</param>
/// <param name="Max">Highest corner.</param>
public readonly record struct TunnelBox(Vector3 Min, Vector3 Max)
{
    /// <summary>Centre of the box.</summary>
    public Vector3 Center => (Min + Max) * 0.5f;

    /// <summary>Side lengths of the box.</summary>
    public Vector3 Size => Max - Min;

    /// <summary>
    /// The box with its x and y clamped to <paramref name="footprint"/>, which is how the
    /// tunnel is kept inside the silhouette of the television in front of it (AMD-025).
    /// </summary>
    /// <remarks>
    /// REQ-SCN-053 pads the tunnel past the reference frame, out to <c>[−8, 264] × [−4, 244]</c>.
    /// The cabinet of a television whose opening is the visible rectangle covers that padding
    /// for the usual clip margins, but not for very large ones — the front faces of the tunnel
    /// would then show as a frame around the set. Clipping the boxes to what the cabinet hides
    /// changes nothing the camera can see of the tunnel itself: its inner faces, the ones that
    /// give the opening its depth, are at the visible rectangle and never touched.
    /// </remarks>
    public TunnelBox ClampTo(VisibleRect footprint) => new(
        new Vector3(MathF.Max(Min.X, footprint.MinX), MathF.Max(Min.Y, footprint.MinY), Min.Z),
        new Vector3(MathF.Min(Max.X, footprint.MaxX), MathF.Min(Max.Y, footprint.MaxY), Max.Z));
}

/// <summary>
/// The decorative TV box of SPEC-06 §3.15: the five-box tunnel of REQ-SCN-053 and the frame of
/// REQ-SCN-054 the television model is placed in.
/// </summary>
/// <remarks>
/// Everything here is pure geometry in scene coordinates, recomputed from the clip margins at
/// every rendered frame (REQ-SCN-053). The model itself is <see cref="TvModel"/>: AMD-025
/// replaced the three models of REQ-SCN-054 — and the three procedural ones of AMD-024
/// REQ-PUB-004 — by the single CC BY file the release ships.
/// </remarks>
public static class TvBox
{
    /// <summary>Number of boxes in the tunnel.</summary>
    public const int TunnelBoxCount = 5;

    /// <summary>The five tunnel boxes of REQ-SCN-053, in scene coordinates.</summary>
    /// <param name="clipping">The four clip margins <c>(c0, c1, c2, c3)</c>.</param>
    public static TunnelBox[] Tunnel(Vec4F clipping)
    {
        float c0 = clipping.X;
        float c1 = clipping.Y;
        float c2 = clipping.Z;
        float c3 = clipping.W;
        const float w = SceneConstants.FrameWidth;
        const float h = SceneConstants.FrameHeight;

        return
        [
            // left
            new TunnelBox(new Vector3(-8f, c1 - 4f, -16f), new Vector3(c0, 244f - c3, 112f)),
            // right
            new TunnelBox(new Vector3(w - c2, c1 - 4f, -16f), new Vector3(264f, 244f - c3, 112f)),
            // bottom
            new TunnelBox(new Vector3(-8f, -4f, -16f), new Vector3(264f, c1, 112f)),
            // top
            new TunnelBox(new Vector3(-8f, h - c3, -16f), new Vector3(264f, 244f, 112f)),
            // back
            new TunnelBox(new Vector3(0f, 0f, 112f), new Vector3(w, h, 120f)),
        ];
    }

    /// <summary>
    /// The origin of the TV frame of REQ-SCN-054:
    /// <c>C + ((c0 − c2) / 256, (c1 − c3) / 240, −16)</c>. The divisor of the first component
    /// really is 256, not 2.
    /// </summary>
    public static Vector3 FrameOrigin(Vec4F clipping) => SceneConstants.Center + new Vector3(
        (clipping.X - clipping.Z) / SceneConstants.FrameWidth,
        (clipping.Y - clipping.W) / SceneConstants.FrameHeight,
        -16f);

    /// <summary>
    /// The scale factors of the TV frame of REQ-SCN-054:
    /// <c>((256 − c0 − c2) / 256, (240 − c1 − c3) / 240, 1)</c>.
    /// </summary>
    public static Vector3 FrameScale(Vec4F clipping) => new(
        (SceneConstants.FrameWidth - clipping.X - clipping.Z) / SceneConstants.FrameWidth,
        (SceneConstants.FrameHeight - clipping.Y - clipping.W) / SceneConstants.FrameHeight,
        1f);

    /// <summary>
    /// The matrix that maps a point of the TV frame to scene coordinates: the frame scale, then
    /// the frame origin. Column-vector convention, as SPEC-04B REQ-E3D-406 uses.
    /// </summary>
    public static Mat4 FrameMatrix(Vec4F clipping)
    {
        Vector3 scale = FrameScale(clipping);
        Vector3 origin = FrameOrigin(clipping);
        return Mat4.Translation(new Vec3(origin.X, origin.Y, origin.Z))
            * Mat4.ScaleMatrix(new Vec3(scale.X, scale.Y, scale.Z));
    }
}
