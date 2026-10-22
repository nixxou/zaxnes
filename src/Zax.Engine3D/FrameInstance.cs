namespace Zax.Engine3D;

/// <summary>
/// A frame instance: the copy, valid for one frame, of one instance of a shape's reference
/// pattern (REQ-E3D-190). It is the object scripts receive as <c>shape</c> (SPEC-05).
/// </summary>
/// <remarks>
/// It carries the same 3D parameters as a pattern instance, plus the quantities derived from
/// the shape it belongs to: the placement box, the anchor <see cref="Pos"/> and the values of
/// SPEC-04 §4.4.
/// </remarks>
public sealed class FrameInstance : PatternInstance
{
    internal FrameInstance(FrameShape shape)
    {
        Shape = shape;
    }

    /// <summary>The shape this instance belongs to, exposed to scripts as <c>shape2D</c>.</summary>
    public FrameShape Shape { get; }

    /// <summary>
    /// The anchor point, computed after the Lua hooks (REQ-E3D-205). Its value before the
    /// first computation of the frame is not specified (SPEC-05 REQ-LUA-083).
    /// </summary>
    public Vec3 Pos { get; set; }

    /// <summary>The same point computed with the centred pivot (REQ-E3D-205).</summary>
    public Vec3 Center { get; set; }

    /// <summary>
    /// The palette table used to colour the tiles: the custom palette when it is ≥ 0,
    /// otherwise the shape's palette (REQ-E3D-192).
    /// </summary>
    public int PaletteTable => CustomPalette >= 0 ? CustomPalette : Shape.Palette;

    /// <summary>True when the shape is a sprite shape.</summary>
    public bool Sprite => !Shape.Background;

    /// <summary>True when the shape is a background shape.</summary>
    public bool Background => Shape.Background;

    /// <summary>
    /// The effective layer (REQ-E3D-205): the instance's layer, or, when it is Unidentified,
    /// L1 for a sprite shape and the default layer of the settings for a background shape.
    /// </summary>
    public ZLayer EffectiveLayer(ConversionSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        if (Layer != ZLayer.Unidentified)
        {
            return Layer;
        }

        return Shape.Background ? settings.DefaultLayer : ZLayer.L1;
    }

    /// <summary>
    /// The depth interval <c>[z0, z1]</c> of the instance (REQ-E3D-205):
    /// <c>z0 = O + (s/2)(a − 1)</c>, <c>z1 = O + (s/2)(a + 1)</c>.
    /// </summary>
    public (float Front, float Back) DepthInterval(ConversionSettings settings)
    {
        LayerSetting layer = settings.Layer(EffectiveLayer(settings));
        float half = Depth / 2f;
        float coefficient = layer.Coefficient;
        return (layer.Offset + (half * (coefficient - 1f)), layer.Offset + (half * (coefficient + 1f)));
    }

    /// <summary>
    /// The placement box <c>Bmin</c> of REQ-E3D-205 (SPEC-04B REQ-E3D-404): the corner
    /// <c>(pStart.x, Hs − pEnd.y − 1, z0)</c>.
    /// </summary>
    public Vec3 BoxMin(ConversionSettings settings) => new(
        Shape.PixelStart.X,
        ConversionGrid.SceneHeight - Shape.PixelEnd.Y - 1,
        DepthInterval(settings).Front);

    /// <summary>
    /// The placement box <c>Bmax</c> of REQ-E3D-205: <c>(pEnd.x + 1, Hs − pStart.y, z1)</c>.
    /// </summary>
    public Vec3 BoxMax(ConversionSettings settings) => new(
        Shape.PixelEnd.X + 1,
        ConversionGrid.SceneHeight - Shape.PixelStart.Y,
        DepthInterval(settings).Back);

    /// <summary>
    /// The anchor point for an arbitrary pivot:
    /// <c>Bmin ⊙ (1 − pivot) + Bmax ⊙ pivot + offset</c> (REQ-E3D-205, SPEC-05 REQ-LUA-083).
    /// </summary>
    public Vec3 GetPos(ConversionSettings settings, Vec3 pivot)
    {
        (float front, float back) = DepthInterval(settings);
        Vec3 min = new(
            Shape.PixelStart.X,
            ConversionGrid.SceneHeight - Shape.PixelEnd.Y - 1,
            front);
        Vec3 max = new(
            Shape.PixelEnd.X + 1,
            ConversionGrid.SceneHeight - Shape.PixelStart.Y,
            back);
        return Vec3.Scale(min, Vec3.One - pivot) + Vec3.Scale(max, pivot) + Offset;
    }

    /// <summary>The anchor point for the instance's own pivot.</summary>
    public Vec3 GetPos(ConversionSettings settings) => GetPos(settings, Pivot);

    /// <summary>Recomputes <see cref="Pos"/> and <see cref="Center"/> (REQ-E3D-205).</summary>
    public void CalculatePos(ConversionSettings settings)
    {
        Pos = GetPos(settings, Pivot);
        Center = GetPos(settings, Vec3.Half);
    }

    /// <summary>
    /// <c>BottomLeft = (pStart.x, Hs − pEnd.y, −s/2)</c> (SPEC-04 §4.4). Not to be confused
    /// with <c>Bmin</c>.
    /// </summary>
    public Vec3 BottomLeft => new(
        Shape.PixelStart.X,
        ConversionGrid.SceneHeight - Shape.PixelEnd.Y,
        -Depth / 2f);

    /// <summary><c>TopRight = (pEnd.x, Hs − pStart.y, s/2)</c> (SPEC-04 §4.4).</summary>
    public Vec3 TopRight => new(
        Shape.PixelEnd.X,
        ConversionGrid.SceneHeight - Shape.PixelStart.Y,
        Depth / 2f);

    /// <summary><c>BL = (pStart.x, Hs − pEnd.y − 1, −s/2)</c> (SPEC-04 §4.4).</summary>
    public Vec3 BL => new(
        Shape.PixelStart.X,
        ConversionGrid.SceneHeight - Shape.PixelEnd.Y - 1,
        -Depth / 2f);

    /// <summary><c>TR = (pEnd.x, Hs − pStart.y − 1, s/2)</c> (SPEC-04 §4.4).</summary>
    public Vec3 TR => new(
        Shape.PixelEnd.X,
        ConversionGrid.SceneHeight - Shape.PixelStart.Y - 1,
        Depth / 2f);

    /// <summary>
    /// <c>size = (pEnd.x − pStart.x + 1, pEnd.y − pStart.y + 1, s)</c> (SPEC-04 §4.4).
    /// </summary>
    public Vec3 Size => new(
        Shape.PixelEnd.X - Shape.PixelStart.X + 1,
        Shape.PixelEnd.Y - Shape.PixelStart.Y + 1,
        Depth);

    /// <summary>
    /// <c>OriPos = BottomLeft ⊙ (1 − q) + TopRight ⊙ q</c> with <c>q = DeformPivot</c>
    /// (SPEC-05 REQ-LUA-083 point 2).
    /// </summary>
    public Vec3 OriPos => GetOriginalPos(DeformPivot);

    /// <summary><c>GetOriginalPos(q) = BottomLeft ⊙ (1 − q) + TopRight ⊙ q</c>.</summary>
    public Vec3 GetOriginalPos(Vec3 pivot)
        => Vec3.Scale(BottomLeft, Vec3.One - pivot) + Vec3.Scale(TopRight, pivot);

    /// <summary><c>GetPosWoLayer(q) = BL ⊙ (1 − q) + TR ⊙ q</c>.</summary>
    public Vec3 GetPosWithoutLayer(Vec3 pivot)
        => Vec3.Scale(BL, Vec3.One - pivot) + Vec3.Scale(TR, pivot);

    /// <summary>
    /// Strict overlap of the pixel boxes of the two shapes, with the slack
    /// <paramref name="delta"/> (SPEC-05 REQ-LUA-083 point 4).
    /// </summary>
    public bool Overlap(FrameInstance other, int delta = 0)
    {
        ArgumentNullException.ThrowIfNull(other);
        Cell aStart = Shape.PixelStart;
        Cell aEnd = Shape.PixelEnd;
        Cell bStart = other.Shape.PixelStart;
        Cell bEnd = other.Shape.PixelEnd;
        return aStart.X < bEnd.X + delta
            && aEnd.X + delta > bStart.X
            && aStart.Y < bEnd.Y + delta
            && aEnd.Y + delta > bStart.Y;
    }

    /// <summary>
    /// Strict overlap with a rectangle placed on the instance's own box, origin
    /// <c>BottomLeft</c> (SPEC-05 REQ-LUA-083 point 4).
    /// </summary>
    public bool Overlap(Rect rectangle)
    {
        Vec3 origin = BottomLeft;
        Vec3 size = Size;
        return new Rect(origin.X, origin.Y, size.X, size.Y).Overlaps(rectangle);
    }

    /// <summary>
    /// The same test with the origin <c>BL</c>, one unit lower. The one-pixel difference with
    /// <see cref="Overlap(Rect)"/> is the original's and is kept (SPEC-05 REQ-LUA-083).
    /// </summary>
    public bool OverlapRect(Rect rectangle)
    {
        Vec3 origin = BL;
        Vec3 size = Size;
        return new Rect(origin.X, origin.Y, size.X, size.Y).Overlaps(rectangle);
    }
}
