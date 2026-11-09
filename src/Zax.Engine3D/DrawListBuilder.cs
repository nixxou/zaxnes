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

    /// <summary>
    /// The per-frame background cell map of AMD-063 REQ-SCN-087 step 1: at most one instance per
    /// cell of the 35 × 33 background grid, and the tile it put there.
    /// </summary>
    private readonly FrameInstance?[] _cellInstance = new FrameInstance?[ConversionGrid.CellCount];
    private readonly FrameTile?[] _cellTile = new FrameTile?[ConversionGrid.CellCount];

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
        MapBackgroundCells(frame);
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

    /// <summary>
    /// Step 1 of AMD-063 REQ-SCN-087: walking the ordered instance list of the frame
    /// (REQ-E3D-040), each cell of the background grid keeps at most one instance — the first
    /// one that claims it, replaced only when the one held is opaque and the new one
    /// transparent.
    /// </summary>
    /// <remarks>
    /// The map is read for the <b>neighbourhood</b> of a drawn tile, so it is built on the
    /// instances of the frame and on every tile of their shapes, restored shapes included, as
    /// the requirement words it — not on the tiles the draw list ends up holding. Q-179.
    /// </remarks>
    private void MapBackgroundCells(ConversionFrame frame)
    {
        Array.Clear(_cellInstance);
        Array.Clear(_cellTile);

        foreach (FrameInstance instance in frame.Instances)
        {
            FrameShape shape = instance.Shape;
            if (!shape.Background)
            {
                continue;
            }

            bool transparent = IsTransparent(instance);
            foreach (FrameTile tile in shape.Tiles)
            {
                int index = tile.GridIndex;
                if ((uint)index >= (uint)_cellInstance.Length)
                {
                    continue;
                }

                // Nothing there yet, or what is there is opaque and this one is transparent.
                bool claim = _cellInstance[index] is not { } held || (transparent && !IsTransparent(held));
                if (claim)
                {
                    _cellInstance[index] = instance;
                    _cellTile[index] = tile;
                }
            }
        }
    }

    /// <summary>
    /// Whether an instance is <b>transparent</b> in the sense of AMD-063 REQ-SCN-087: its own
    /// opacity is strictly below 1 and its own render style is not 2. An <c>Alpha</c> above 1 is
    /// therefore opaque, which is the assumed divergence (D) of §4.
    /// </summary>
    private static bool IsTransparent(FrameInstance instance)
        => instance.Alpha < 1f && instance.Style != RenderStyle.ShadowOnly;

    /// <summary>
    /// Step 2 of AMD-063 REQ-SCN-087: the lateral sides a drawn full tile of a transparent
    /// background instance loses — those whose 4-neighbour cell holds an instance that is
    /// transparent and whose tile there is full for <b>its</b> reference 3D object. A cell
    /// outside the grid removes nothing.
    /// </summary>
    private TileSides RemovedSides(FrameTile tile, IPatternGeometryBuilder geometry)
    {
        Cell cell = tile.Position;
        TileSides sides = TileSides.None;
        sides |= Side(cell.X - 1, cell.Y, TileSides.Left, geometry);
        sides |= Side(cell.X + 1, cell.Y, TileSides.Right, geometry);
        sides |= Side(cell.X, cell.Y - 1, TileSides.Top, geometry);
        sides |= Side(cell.X, cell.Y + 1, TileSides.Bottom, geometry);
        return sides;
    }

    /// <summary><paramref name="side"/> when the neighbouring cell takes it away, none otherwise.</summary>
    private TileSides Side(int x, int y, TileSides side, IPatternGeometryBuilder geometry)
    {
        if (!ConversionGrid.Contains(new Cell(x, y)))
        {
            return TileSides.None;
        }

        int index = ConversionGrid.Index(x, y);
        return _cellInstance[index] is { } neighbour
            && IsTransparent(neighbour)
            && _cellTile[index] is { Image: not null } neighbourTile
            && geometry.IsFullTile(neighbour.Reference, neighbourTile.Image)
                ? side
                : TileSides.None;
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

        // AMD-055 REQ-SCN-082: the voxel grid of the local mesh frame is offset by D/2 on z,
        // so the renderer is handed the depth that frame was centred on. It belongs to the
        // whole 3D object, so it is read once per instance.
        int buildDepth = geometry.MeshDepthOf(instance.Reference);

        // AMD-063 REQ-SCN-087 step 2 only ever applies to a background instance the drawn
        // instance itself declares transparent; sprites are left out (§4, divergence (D)).
        bool removable = shape.Background && IsTransparent(instance);

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

            // AMD-063 REQ-SCN-087: a full tile of a transparent background instance gives up the
            // lateral faces that fall on a joint with another transparent full tile. Everything
            // else keeps the whole REQ-E3D-501 mesh, as AMD-057 left it: the sealing variants of
            // AMD-027 are gone, and the coincident inner walls of an opaque surface are kept
            // from showing by the renderer alone (SPEC-06 REQ-SCN-065).
            TileSides removed = removable && geometry.IsFullTile(instance.Reference, tile.Image)
                ? RemovedSides(tile, geometry)
                : TileSides.None;

            _list.Add(new DrawTile
            {
                Instance = instance,
                Tile = tile,
                Mesh = geometry.MeshOf(instance.Reference, tile.Image, removed),
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

                // AMD-057 amends REQ-SCN-037: the renderer's depth nudge follows the sprite
                // index of REQ-CORE-043, so the draw list hands it over (REQ-E3D-641).
                SpriteIndex = shape.Background ? -1 : tile.Ord,
                Deform = deform,
                BuildDepth = buildDepth,
            });
        }
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
