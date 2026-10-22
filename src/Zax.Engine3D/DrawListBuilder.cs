using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>
/// Turns a converted frame into the ordered list of tiles to draw (SPEC-04B REQ-E3D-641).
/// </summary>
/// <remarks>
/// <para>
/// This is the hand-over point between SPEC-04 and SPEC-04B: the pipeline supplies the
/// placement, the palette table, the material class, the shadow flags and the deformation
/// parameters; the mesh of each tile comes from the geometry builder of lot L4b and is null
/// until it is plugged in.
/// </para>
/// <para>
/// The order is the instance order of REQ-E3D-220 and, inside one instance, the tile order of
/// the shape. Instances that are hidden (REQ-E3D-191) or whose geometry is not ready
/// (REQ-E3D-361) contribute nothing; a shape restored from the pinned frame contributes only
/// the tiles that overlap one of the frame's rectangles (REQ-E3D-233).
/// </para>
/// </remarks>
public sealed class DrawListBuilder
{
    private readonly FrameDrawList _list = new();
    private readonly Dictionary<(int X, int Y), FrameTile?> _neighbours = [];

    /// <summary>Builds the draw list of <paramref name="frame"/>.</summary>
    /// <param name="frame">The frame to draw.</param>
    /// <param name="settings">The settings the pipeline consumes.</param>
    /// <param name="geometry">The geometry side, which owns the meshes.</param>
    /// <param name="formatVersion2">
    /// True when the loaded profile is in version 2 format, which moves the deformation pivot
    /// by one unit (SPEC-04B REQ-E3D-568).
    /// </param>
    public FrameDrawList Build(
        ConversionFrame frame,
        ConversionSettings settings,
        IPatternGeometryBuilder geometry,
        bool formatVersion2)
    {
        ArgumentNullException.ThrowIfNull(frame);
        ArgumentNullException.ThrowIfNull(settings);
        ArgumentNullException.ThrowIfNull(geometry);

        _list.Begin(frame);
        foreach (FrameInstance instance in frame.Instances)
        {
            if (instance.Style == RenderStyle.Hidden)
            {
                continue;
            }

            if (!geometry.IsReady(instance.Reference))
            {
                continue;
            }

            AddInstance(frame, instance, settings, geometry, formatVersion2);
        }

        return _list;
    }

    private void AddInstance(
        ConversionFrame frame,
        FrameInstance instance,
        ConversionSettings settings,
        IPatternGeometryBuilder geometry,
        bool formatVersion2)
    {
        FrameShape shape = instance.Shape;
        ZLayer layer = instance.EffectiveLayer(settings);
        LayerSetting layerSetting = settings.Layer(layer);
        bool ui = layer == ZLayer.Ui;

        MaterialClass material = MaterialOf(instance, ui);
        SceneContainer container = ui ? SceneContainer.Interface : SceneContainer.Game;

        bool castShadow = settings.Shadow
            && !ui
            && material is MaterialClass.Opaque or MaterialClass.ShadowOnly
            && instance.CastShadow
            && layerSetting.CastShadow;
        bool receiveShadow = settings.Shadow
            && !ui
            && instance.ReceiveShadow
            && layerSetting.ReceiveShadow;

        // The texture comes from the instance's own pattern instance, unless a script
        // redirected the texture slot with shape.slot = other.slot (SPEC-05 REQ-LUA-097).
        PatternInstance textureOwner = instance.TextureOwner;
        ObjectTexture? texture = geometry.TextureOf(textureOwner);
        TextureFormat format = texture?.Format
            ?? textureOwner.Texture?.Pixels?.Format
            ?? TextureFormat.Indexed;
        // REQ-E3D-560: the palette table is customPalette when it is set, and otherwise the
        // palette of **each tile of the frame** (plus 4 for a sprite shape) — not the shape's
        // own palette, which REQ-E3D-047 derives from the germ alone. A background shape that
        // spans several attribute quads therefore keeps the colours of each quad, as
        // REQ-SCN-034 restates.
        bool direct = format == TextureFormat.Direct;
        int customPalette = instance.CustomPalette;

        Vec3 anchor = instance.Pos;
        Vec3 centre = instance.Center;
        Mat4 rotation = RotationOf(instance);
        Vec3 scale = ScaleOf(instance, shape, settings);

        Vec3 size = new(
            shape.PixelEnd.X - shape.PixelStart.X + 1,
            shape.PixelEnd.Y - shape.PixelStart.Y + 1,
            instance.Depth);
        Vec3 deformPivot = DeformPivot(instance, shape, size, formatVersion2);

        // Q-067: the walls that close each tile against a neighbour of the same object show at
        // the joints of a flat surface (REQ-SCN-065). The neighbours are looked up here, once
        // per instance, so that the draw list can ask for the mesh variant without them.
        _neighbours.Clear();
        foreach (FrameTile tile in shape.Tiles)
        {
            if (!shape.RestoredFromPinnedFrame || frame.TileOverlapsPinnedRects(tile))
            {
                // A sprite shape may hold two tiles on one cell (REQ-E3D-043): which of them
                // covers the joint is then undecided, so the cell seals nothing.
                _neighbours[(tile.X, tile.Y)] = _neighbours.ContainsKey((tile.X, tile.Y)) ? null : tile;
            }
        }

        foreach (FrameTile tile in shape.Tiles)
        {
            if (shape.RestoredFromPinnedFrame && !frame.TileOverlapsPinnedRects(tile))
            {
                continue;
            }

            Vec3 tileCenter = new(
                tile.X + 4 + instance.Offset.X,
                ConversionGrid.SceneHeight - tile.Y - 4 + instance.Offset.Y,
                centre.Z);

            // P = Pos + R . (S (T - Pos + v)) : REQ-E3D-406.
            Mat4 transform = Mat4.Translation(anchor)
                * rotation
                * Mat4.ScaleMatrix(scale)
                * Mat4.Translation(tileCenter - anchor);

            int paletteTable = direct
                ? -1
                : customPalette >= 0 ? customPalette : tile.Palette + (shape.Background ? 0 : 4);

            DeformParameters? deform = null;
            if (instance.DeformEnabled)
            {
                Vec3 tileRelative = new Vec3(
                    tile.X + 4,
                    ConversionGrid.SceneHeight - tile.Y - 4,
                    0f) - deformPivot;
                deform = new DeformParameters(
                    instance.DeformAmplitude,
                    instance.DeformSpeed,
                    instance.DeformOffset,
                    size,
                    tileRelative);
            }

            _list.Add(new DrawTile
            {
                Instance = instance,
                Tile = tile,
                Mesh = geometry.MeshOf(instance.Reference, tile.Image, SealedSides(instance, tile, geometry)),
                Transform = transform,
                Anchor = anchor,
                TileCenter = tileCenter,
                Container = container,
                Material = material,
                Alpha = instance.Alpha,
                Texture = texture,
                TextureFormat = format,
                PaletteTable = paletteTable,
                TexOffset = instance.TexOffset,
                TexTilting = instance.TexTilting,
                CastShadow = castShadow,
                ReceiveShadow = receiveShadow,
                Sprite = !shape.Background,
                BehindBackground = !shape.Background && tile.Hidden,
                Deform = deform,
            });
        }
    }

    /// <summary>
    /// The lateral sides of <paramref name="tile"/> a neighbouring tile of the same object
    /// covers entirely, so that the wall between the two can be left out of the drawn mesh
    /// (Q-067, SPEC-06 REQ-SCN-065).
    /// </summary>
    /// <remarks>
    /// Both planes have to be full: the wall is then strictly inside the solid the two tiles
    /// form, and dropping it cannot open a hole. Tiles of two different objects are never
    /// sealed against each other, since nothing says their depths or their z agree.
    /// </remarks>
    private int SealedSides(FrameInstance instance, FrameTile tile, IPatternGeometryBuilder geometry)
    {
        int full = geometry.FullSidesOf(instance.Reference, tile.Image);
        if (full == 0)
        {
            return 0;
        }

        int sealedSides = 0;
        sealedSides |= Seal(instance, geometry, full, TileSides.MinusX, tile.X - 8, tile.Y);
        sealedSides |= Seal(instance, geometry, full, TileSides.PlusX, tile.X + 8, tile.Y);

        // Scene y runs the other way than the screen row (REQ-E3D-404): the neighbour below
        // the tile in the scene is the one on the row underneath on screen.
        sealedSides |= Seal(instance, geometry, full, TileSides.MinusY, tile.X, tile.Y + 8);
        sealedSides |= Seal(instance, geometry, full, TileSides.PlusY, tile.X, tile.Y - 8);
        return sealedSides;
    }

    private int Seal(
        FrameInstance instance,
        IPatternGeometryBuilder geometry,
        int full,
        int side,
        int x,
        int y)
    {
        if ((full & side) == 0
            || !_neighbours.TryGetValue((x, y), out FrameTile? neighbour)
            || neighbour is null)
        {
            return 0;
        }

        int facing = geometry.FullSidesOf(instance.Reference, neighbour.Image);
        return (facing & TileSides.Opposite(side)) != 0 ? side : 0;
    }

    /// <summary>
    /// The material class of SPEC-04B REQ-E3D-566 and REQ-E3D-642: interface in the UI layer,
    /// shadow-only for render style 2, opaque when alpha is 1, transparent otherwise.
    /// </summary>
    private static MaterialClass MaterialOf(PatternInstance instance, bool ui)
    {
        if (ui)
        {
            return MaterialClass.Interface;
        }

        if (instance.Style == RenderStyle.ShadowOnly)
        {
            return MaterialClass.ShadowOnly;
        }

        return instance.Alpha >= 1f ? MaterialClass.Opaque : MaterialClass.Transparent;
    }

    /// <summary>
    /// The rotation of SPEC-04B REQ-E3D-407: <c>Rx·Ry·Rz</c>, or <c>Ry·Rx·Rz</c> when the tags
    /// carry <c>_gaxis</c>.
    /// </summary>
    /// <remarks>
    /// <c>_gaxis</c> has to be one element of the comma-separated list, not a piece of a
    /// longer tag; one object of the built-in catalogue stores <c>mc._gaxis</c> and so keeps
    /// the default order, which is why annex A counts one more than the engine does (Q-018).
    /// </remarks>
    private static Mat4 RotationOf(PatternInstance instance)
    {
        Mat4 x = Mat4.RotationX(instance.Rotation.X);
        Mat4 y = Mat4.RotationY(instance.Rotation.Y);
        Mat4 z = Mat4.RotationZ(instance.Rotation.Z);
        return instance.HasTag("_gaxis") ? y * x * z : x * y * z;
    }

    /// <summary>
    /// The scale of an instance, with the MARCHING correction of SPEC-04B REQ-E3D-408.
    /// </summary>
    private static Vec3 ScaleOf(FrameInstance instance, FrameShape shape, ConversionSettings settings)
    {
        Vec3 scale = instance.Scale;
        if (instance.EffectiveAlgorithm(settings.RenderAlgorithm) != RenderAlgorithm.Marching)
        {
            return scale;
        }

        float sx = shape.PixelEnd.X - shape.PixelStart.X + 1;
        float sy = shape.PixelEnd.Y - shape.PixelStart.Y + 1;
        float sz = instance.Depth;
        return new Vec3(
            scale.X * (sx / (sx + 1f)),
            scale.Y * (sy / (sy + 1f)),
            sz == 0f ? 0f : scale.Z * (sz / (sz + 1f)));
    }

    /// <summary>
    /// The deformation pivot <c>Pd</c> of SPEC-04B REQ-E3D-568:
    /// <c>deltaPivot ⊙ sz + (pStart.x, Hs − pEnd.y − h₀, −D/2)</c>, with <c>h₀ = 0</c> for a
    /// version 2 profile and 1 otherwise.
    /// </summary>
    private static Vec3 DeformPivot(
        PatternInstance instance,
        FrameShape shape,
        Vec3 size,
        bool formatVersion2)
    {
        float h0 = formatVersion2 ? 0f : 1f;
        return Vec3.Scale(instance.DeformPivot, size)
            + new Vec3(
                shape.PixelStart.X,
                ConversionGrid.SceneHeight - shape.PixelEnd.Y - h0,
                -instance.Depth / 2f);
    }
}
