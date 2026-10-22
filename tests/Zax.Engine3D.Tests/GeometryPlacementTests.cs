using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// The test vectors of SPEC-04B §5.5 — placement, colour and deformation — and the output
/// contract of §3.8, driven end to end through the pipeline of SPEC-04.
/// </summary>
public sealed class GeometryPlacementTests
{
    [Fact]
    public void REQ_E3D_410_the_scene_frame_places_the_nes_image_in_y_from_eight_to_two_hundred_forty_eight()
    {
        // Pixel (0, 0) occupies y in [247, 248], pixel (0, 239) y in [8, 9].
        Assert.Equal(248, ConversionGrid.SceneHeight);
        Assert.Equal(247f, Bottom(0));
        Assert.Equal(248f, Top(0));
        Assert.Equal(8f, Bottom(239));
        Assert.Equal(9f, Top(239));

        // The geometric centre of the visible image is (128, 128); the reference frame is
        // centred on C = (128, 120, 0), eight units below it.
        Assert.Equal(128f, (Bottom(239) + Top(0)) / 2f);
        Assert.Equal(120f, 240f / 2f);

        // Inverse relation: a point (x, y) of the plane z = 0 shows the pixel
        // (floor x, floor (Hs − y)).
        Assert.Equal(239, (int)MathF.Floor(ConversionGrid.SceneHeight - 8.5f));
        Assert.Equal(0, (int)MathF.Floor(ConversionGrid.SceneHeight - 247.5f));
    }

    [Fact]
    public void TV_W1_a_background_tile_is_placed_in_front_of_its_layer()
    {
        // REQ-E3D-403, 404, 405, 406.
        (ConversionEngine engine, GeometryBuilder geometry) = Engine(depth: 4);
        FrameDrawList list = Draw(engine, Img.A, 16, 32);

        DrawTile tile = Assert.Single(list.Tiles);
        Assert.Equal(new Vec3(20f, 212f, 2f), tile.Anchor);
        Assert.Equal(new Vec3(20f, 212f, 2f), tile.TileCenter);
        AssertSceneBox(tile, 16f, 24f, 208f, 216f, 0f, 4f);
        Assert.NotNull(tile.Mesh);
        Assert.Equal(6, Geo.Rectangles(tile.Mesh!));

        // In L2 the whole object moves twelve units away.
        (ConversionEngine second, GeometryBuilder _) = Engine(depth: 4, layer: ProfileLayer.L2);
        AssertSceneBox(Assert.Single(Draw(second, Img.A, 16, 32).Tiles), 16f, 24f, 208f, 216f, 12f, 16f);

        // CENTER and BACK alignments, with a zero layer offset.
        engine.Settings.Layer(ZLayer.L1).Alignment = ZAlignment.Center;
        AssertSceneBox(Assert.Single(Draw(engine, Img.A, 16, 32).Tiles), 16f, 24f, 208f, 216f, -2f, 2f);

        engine.Settings.Layer(ZLayer.L1).Alignment = ZAlignment.Back;
        AssertSceneBox(Assert.Single(Draw(engine, Img.A, 16, 32).Tiles), 16f, 24f, 208f, 216f, -4f, 0f);
        Assert.Equal(1, geometry.ObjectCount);
    }

    [Fact]
    public void TV_W2_a_sprite_tile_gets_the_same_transform_and_the_nudge_flags()
    {
        // REQ-E3D-405, DEC-R2-T4: the ±0.1 nudge is the renderer's job, not ours.
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);

        Batch background = new();
        background.Bg(Img.A, 16, 32);
        DrawTile backgroundTile = Assert.Single(engine.BuildDrawList(background.Convert(engine)).Tiles);

        Batch sprite = new();
        sprite.Sprite(Img.A, 16, 32);
        DrawTile spriteTile = Assert.Single(engine.BuildDrawList(sprite.Convert(engine, 2)).Tiles);

        Assert.Equal(backgroundTile.Transform, spriteTile.Transform);
        Assert.True(spriteTile.Sprite);
        Assert.False(spriteTile.BehindBackground);
        Assert.False(backgroundTile.Sprite);
        AssertSceneBox(spriteTile, 16f, 24f, 208f, 216f, 0f, 4f);

        Batch behind = new();
        behind.Sprite(Img.A, 16, 32, hidden: true);
        DrawTile hiddenTile = Assert.Single(engine.BuildDrawList(behind.Convert(engine, 3)).Tiles);
        Assert.True(hiddenTile.Sprite);
        Assert.True(hiddenTile.BehindBackground);
    }

    [Fact]
    public void TV_W3_scale_pivot_and_rotation_act_around_the_anchor()
    {
        // REQ-E3D-406, 407.
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);
        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];

        reference.Scale = new Vec3(2f, 1f, 1f);
        AssertSceneBox(Assert.Single(Draw(engine, Img.A, 16, 32).Tiles), 12f, 28f, 208f, 216f, 0f, 4f);

        reference.Pivot = Vec3.Zero;
        DrawTile shifted = Assert.Single(Draw(engine, Img.A, 16, 32).Tiles);
        Assert.Equal(new Vec3(16f, 208f, 0f), shifted.Anchor);
        AssertSceneBox(shifted, 16f, 32f, 208f, 216f, 0f, 4f);

        // A quarter turn about z keeps the square where it is and moves its pixel (0, 0) from
        // the top left corner to the bottom left one.
        reference.Scale = Vec3.One;
        reference.Pivot = Vec3.Half;
        reference.Rotation = new Vec3(0f, 0f, 90f);
        DrawTile turned = Assert.Single(Draw(engine, Img.A, 16, 32).Tiles);
        AssertSceneBox(turned, 16f, 24f, 208f, 216f, 0f, 4f);

        // The local corner of the pixel (0, 0) is (−4, 4); a 90° turn sends it to (−4, −4).
        Vec3 corner = turned.Transform.TransformPoint(new Vec3(-4f, 4f, 0f));
        Assert.Equal(16f, corner.X, 3);
        Assert.Equal(208f, corner.Y, 3);

        // REQ-E3D-407: rot = (0, 0, 90) sends the relative vector (8, 0, 0) to (0, 8, 0).
        Vec3 turnedVector = Mat4.RotationZ(90f).TransformPoint(new Vec3(8f, 0f, 0f));
        Assert.Equal(0f, turnedVector.X, 3);
        Assert.Equal(8f, turnedVector.Y, 3);
    }

    [Fact]
    public void REQ_E3D_407_the_gaxis_tag_swaps_the_first_two_rotations()
    {
        // Without _gaxis the rotation is Rx·Ry·Rz, with it Ry·Rx·Rz: (0, 0, 1) goes to
        // (1, 0, 0) or to (0, −1, 0).
        (ConversionEngine plain, GeometryBuilder _) = Engine(depth: 4);
        plain.Patterns.Permanent[0].Instances[0].Rotation = new Vec3(90f, 90f, 0f);
        Mat4 withoutTag = Assert.Single(Draw(plain, Img.A, 16, 32).Tiles).Transform;

        (ConversionEngine tagged, GeometryBuilder _) = Engine(depth: 4, tags: "_gaxis");
        tagged.Patterns.Permanent[0].Instances[0].Rotation = new Vec3(90f, 90f, 0f);
        Mat4 withTag = Assert.Single(Draw(tagged, Img.A, 16, 32).Tiles).Transform;

        Vec3 anchor = new(20f, 212f, 2f);
        Vec3 first = withoutTag.TransformPoint(new Vec3(0f, 0f, 1f) + anchor - new Vec3(20f, 212f, 2f))
            - withoutTag.TransformPoint(Vec3.Zero);
        Vec3 second = withTag.TransformPoint(new Vec3(0f, 0f, 1f)) - withTag.TransformPoint(Vec3.Zero);

        Assert.Equal(1f, first.X, 3);
        Assert.Equal(0f, first.Y, 3);
        Assert.Equal(0f, first.Z, 3);

        Assert.Equal(0f, second.X, 3);
        Assert.Equal(-1f, second.Y, 3);
        Assert.Equal(0f, second.Z, 3);
    }

    [Fact]
    public void TV_W4_marching_shrinks_the_scale_back_to_the_pixel_box()
    {
        // REQ-E3D-408: a one-pixel shape with D = 1 shows the octahedron of radius 1 at
        // radius 0.5, centred on the anchor.
        (ConversionEngine engine, GeometryBuilder geometry) = Engine(depth: 1);
        geometry.Settings.RenderAlgorithm = RenderAlgorithm.Marching;
        engine.LoadProfile(OnePixelProfile(depth: 1));

        Batch batch = new();
        batch.Bg(Img.Pixel(0, 0), 16, 32);
        DrawTile tile = Assert.Single(engine.BuildDrawList(batch.Convert(engine)).Tiles);

        Assert.Equal(new Vec3(16.5f, 215.5f, 0.5f), tile.Anchor);
        AssertSceneBox(tile, 16f, 17f, 215f, 216f, 0f, 1f);
        Assert.NotNull(tile.Mesh);
        Assert.Equal(8, tile.Mesh!.TriangleCount);

        // The octahedron is centred on the anchor, with a radius of 0.5.
        Vec3 centre = new(16.5f, 215.5f, 0.5f);
        foreach (MeshVertex vertex in tile.Mesh.Vertices)
        {
            Vec3 scene = tile.Transform.TransformPoint(vertex.Position);
            float distance = Vec3.Distance(scene, centre);
            Assert.Equal(0.5f, distance, 3);
        }
    }

    [Fact]
    public void TV_K1_indexed_colours_come_from_the_frame_palette_table()
    {
        // REQ-E3D-560, 561: colour = NES_RGB[pal[4·P + (t mod 4)]].
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);

        Batch batch = new();
        batch.Palette(0, 0x0F);
        for (int entry = 1; entry < 32; entry++)
        {
            batch.Palette(entry, (byte)entry);
        }

        batch.Bg(Img.A, 16, 32, palette: 1).Sprite(Img.A, 64, 64, palette: 1);
        FrameDrawList list = engine.BuildDrawList(batch.Convert(engine));

        Assert.Equal(2, list.Tiles.Count);
        DrawTile background = list.Tiles[0];
        DrawTile sprite = list.Tiles[1];

        Assert.Equal(1, background.PaletteTable);
        Assert.Equal(5, sprite.PaletteTable);

        // Background tile of palette 1, texel 2 -> core[6]; the same sprite -> core[22].
        Assert.Equal(NesPalette.Color(6), list.Palette[(4 * background.PaletteTable) + 2]);
        Assert.Equal(NesPalette.Color(22), list.Palette[(4 * sprite.PaletteTable) + 2]);

        // Texel 0 is the universal background colour in every table.
        Assert.Equal(NesPalette.Color(0x0F), list.Palette[4 * background.PaletteTable]);
        Assert.Equal(NesPalette.Color(0x0F), list.Palette[4 * sprite.PaletteTable]);

        // A custom palette of 9 reaches the same colour as the table 1 (REQ-E3D-561). It is
        // set on the frame instance: a pattern instance's value is reset every frame
        // (REQ-E3D-190).
        ConversionFrame frame = batch.Convert(engine, 2);
        frame.Instances[0].CustomPalette = 9;
        FrameDrawList custom = engine.BuildDrawList(frame);
        Assert.Equal(9, custom.Tiles[0].PaletteTable);
        Assert.Equal(NesPalette.Color(6), custom.Palette[(4 * 9) + 2]);
    }

    [Fact]
    public void REQ_E3D_560_the_palette_table_is_the_one_of_each_tile_not_of_the_shape()
    {
        // REQ-E3D-560 and REQ-SCN-034: "la palette est celle de chaque tuile de la frame".
        // A background shape that spans two attribute quads therefore keeps the colours of
        // both, instead of taking the germ's palette for all of its tiles (REQ-E3D-047).
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);

        Batch batch = new();
        batch.Palette(0, 0x0F);
        for (int entry = 1; entry < 32; entry++)
        {
            batch.Palette(entry, (byte)entry);
        }

        // Four neighbouring tiles of the same image: two of attribute palette 0, two of 1.
        batch.Bg(Img.A, 16, 32, palette: 0)
            .Bg(Img.A, 24, 32, palette: 0)
            .Bg(Img.A, 32, 32, palette: 1)
            .Bg(Img.A, 40, 32, palette: 1);

        FrameDrawList list = engine.BuildDrawList(batch.Convert(engine));

        Assert.Equal(4, list.Tiles.Count);
        Assert.Equal([0, 0, 1, 1], list.Tiles.Select(tile => tile.PaletteTable).ToArray());

        // The tiles belong to one single shape, whose own palette is the germ's.
        Assert.Single(list.Tiles.Select(tile => tile.Instance.Shape).Distinct());
        Assert.Equal(0, list.Tiles[0].Instance.PaletteTable);
    }

    [Fact]
    public void REQ_E3D_560_a_custom_palette_overrides_every_tile_of_the_shape()
    {
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);

        Batch batch = new();
        batch.Palette(0, 0x0F);
        batch.Bg(Img.A, 16, 32, palette: 0).Bg(Img.A, 24, 32, palette: 1);

        ConversionFrame frame = batch.Convert(engine);
        frame.Instances[0].CustomPalette = 9;
        FrameDrawList list = engine.BuildDrawList(frame);

        Assert.Equal([9, 9], list.Tiles.Select(tile => tile.PaletteTable).ToArray());
    }

    [Fact]
    public void REQ_E3D_562_a_direct_texture_freezes_its_zero_texels_when_it_is_built()
    {
        EnginePattern pattern = Geo.OneTile(Geo.Solid(1));
        VariantTexture texture = new(
            2,
            1,
            new TexturePixels(TextureFormat.Direct, [0x00FF0000, 0]));

        ObjectTexture resolved = ObjectTexture.For(pattern, texture, new Rgb24(1, 2, 3));
        Assert.Equal(TextureFormat.Direct, resolved.Format);
        Assert.Equal(new Rgb24(0xFF, 0, 0), resolved.Color(0, 0));
        Assert.Equal(new Rgb24(1, 2, 3), resolved.Color(1, 0));

        // Changing the background colour afterwards does not move the texels already loaded.
        // The value used before the first frame is the builder's (Q-017).
        ObjectTexture later = ObjectTexture.For(pattern, texture, new Rgb24(9, 9, 9));
        Assert.Equal(new Rgb24(1, 2, 3), resolved.Color(1, 0));
        Assert.Equal(new Rgb24(9, 9, 9), later.Color(1, 0));

        // REQ-E3D-424: with a direct texture only the texel whose integer value is exactly e
        // is transparent.
        Assert.Equal(0, resolved.Texel(1, 0));
        Assert.Equal(0x00FF0000, resolved.Texel(0, 0));
    }

    [Fact]
    public void REQ_E3D_563_the_texture_repeats_outside_its_bounds()
    {
        // DEC-04B-Q3: every read is modulo the texture size.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("123....."));
        ObjectTexture texture = ObjectTexture.FromPattern(pattern);

        Assert.Equal(texture.Texel(0, 0), texture.Texel(8, 0));
        Assert.Equal(texture.Texel(1, 0), texture.Texel(-7, 0));
        Assert.Equal(texture.Texel(2, 3), texture.Texel(2, 11));
    }

    [Fact]
    public void TV_D1_the_deformation_is_a_standing_wave_across_the_shape()
    {
        // REQ-E3D-568, 641.
        GameProfile profile = new ProfileBuilder()
            .Pattern(2, 2, [Img.A, Img.B, Img.C, Img.D], depth: 4)
            .Build();
        profile.VersionText = "2";
        Assert.True(profile.IsV2Format);

        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.LoadProfile(profile);

        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];
        reference.DeformAmplitude = new Vec3(0f, 0.1f, 0f);
        reference.DeformSpeed[1, 0] = 2f * MathF.PI;

        Batch batch = new();
        batch.Bg(Img.A, 0, 0).Bg(Img.B, 8, 0).Bg(Img.C, 0, 8).Bg(Img.D, 8, 8);
        FrameDrawList list = engine.BuildDrawList(batch.Convert(engine, 0));

        Assert.Equal(4, list.Tiles.Count);
        Assert.Equal(0f, list.Time);

        DrawTile right = list.Tiles.First(t => t.Tile.X == 8 && t.Tile.Y == 0);
        DeformParameters deform = Geo.Present(right.Deform);

        // sz = (16, 16, 4) and Pd = (8, 241, 0), so the tile sits at (4, 3, 0) from the pivot.
        Assert.Equal(new Vec3(16f, 16f, 4f), deform.Size);
        Assert.Equal(new Vec3(4f, 3f, 0f), deform.TileRelativeToPivot);

        // nrm.x = 0.25 -> no displacement; 0 -> +1.6; 0.5 -> −1.6.
        Assert.Equal(0f, deform.Displacement(new Vec3(0f, 0f, 0f), 0f).Y, 3);
        Assert.Equal(1.6f, deform.Displacement(new Vec3(-4f, 0f, 0f), 0f).Y, 3);
        Assert.Equal(-1.6f, deform.Displacement(new Vec3(4f, 0f, 0f), 0f).Y, 3);
        Assert.Equal(0f, deform.Displacement(new Vec3(0f, 0f, 0f), 0f).X, 3);
        Assert.Equal(0f, deform.Displacement(new Vec3(0f, 0f, 0f), 0f).Z, 3);
    }

    [Fact]
    public void REQ_E3D_568_the_default_deformation_is_a_constant_translation()
    {
        // Zero speeds and pi/2 phases give delta = deltaAmp ⊙ sz.
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);
        engine.Patterns.Permanent[0].Instances[0].DeformAmplitude = new Vec3(0.25f, 0.5f, 1f);

        DrawTile tile = Assert.Single(Draw(engine, Img.A, 16, 32).Tiles);
        DeformParameters deform = Geo.Present(tile.Deform);
        Vec3 delta = deform.Displacement(Vec3.Zero, 1.25f);

        Assert.Equal(0.25f * 8f, delta.X, 3);
        Assert.Equal(0.5f * 8f, delta.Y, 3);
        Assert.Equal(1f * 4f, delta.Z, 3);
    }

    [Fact]
    public void REQ_E3D_569_the_free_script_row_never_moves_a_vertex()
    {
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);
        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];
        reference.DeformAmplitude = new Vec3(0f, 0.1f, 0f);

        DeformParameters before = Geo.Present(Assert.Single(Draw(engine, Img.A, 16, 32).Tiles).Deform);
        Vec3 plain = before.Displacement(new Vec3(1f, 2f, 0f), 0.5f);

        for (int index = 0; index < 8; index++)
        {
            reference.SetCustomData(index, 3.5f + index);
        }

        DeformParameters after = Geo.Present(Assert.Single(Draw(engine, Img.A, 16, 32).Tiles).Deform);
        Vec3 withData = after.Displacement(new Vec3(1f, 2f, 0f), 0.5f);

        Assert.Equal(plain.X, withData.X, 5);
        Assert.Equal(plain.Y, withData.Y, 5);
        Assert.Equal(plain.Z, withData.Z, 5);

        for (int index = 0; index < 8; index++)
        {
            Assert.Equal(3.5f + index, reference.CustomData(index));
        }
    }

    [Fact]
    public void REQ_E3D_566_the_material_class_follows_alpha_and_the_render_style()
    {
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);
        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];

        Assert.Equal(MaterialClass.Opaque, Assert.Single(Draw(engine, Img.A, 16, 32).Tiles).Material);

        reference.Alpha = 0.5f;
        DrawTile transparent = Assert.Single(Draw(engine, Img.A, 16, 32).Tiles);
        Assert.Equal(MaterialClass.Transparent, transparent.Material);
        Assert.Equal(0.5f, transparent.Alpha);
        Assert.False(transparent.CastShadow);

        reference.Style = RenderStyle.ShadowOnly;
        DrawTile shadowOnly = Assert.Single(Draw(engine, Img.A, 16, 32).Tiles);
        Assert.Equal(MaterialClass.ShadowOnly, shadowOnly.Material);
        Assert.True(shadowOnly.CastShadow);

        reference.Style = RenderStyle.Hidden;
        Assert.Empty(Draw(engine, Img.A, 16, 32).Tiles);
    }

    [Fact]
    public void REQ_E3D_640_the_output_is_platform_free_data_with_one_submesh()
    {
        (ConversionEngine engine, GeometryBuilder _) = Engine(depth: 4);
        DrawTile tile = Assert.Single(Draw(engine, Img.A, 16, 32).Tiles);
        TileMesh mesh = Geo.Present(tile.Mesh);

        Assert.Equal(1, Zax.Engine3D.TileMesh.SubmeshCount);
        Assert.Equal(0, mesh.Triangles.Length % 3);
        Assert.Equal(mesh.TriangleCount * 3, mesh.Triangles.Length);
        Assert.True(Geo.Close(new Vec3(-4f, -4f, -2f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(4f, 4f, 2f), mesh.BoundsMax));

        // The texture reference and its format reach the renderer (REQ-E3D-641).
        ObjectTexture texture = Geo.Present(tile.Texture);
        Assert.Equal(TextureFormat.Indexed, texture.Format);
        Assert.Equal(8, texture.Width);
        Assert.Equal(8, texture.Height);
        Assert.Equal(SceneContainer.Game, tile.Container);
        Assert.Equal(Vec2.Zero, tile.TexOffset);
        Assert.Equal(Vec2.One, tile.TexTilting);
    }

    private static (ConversionEngine Engine, GeometryBuilder Geometry) Engine(
        int depth = 3,
        ProfileLayer layer = ProfileLayer.L1,
        string? tags = null)
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: depth, layer: layer, tags: tags)
            .Build());
        return (engine, geometry);
    }

    private static GameProfile OnePixelProfile(int depth)
        => new ProfileBuilder()
            .Pattern(1, 1, [Img.Pixel(0, 0)], depth: depth)
            .Build();

    private static FrameDrawList Draw(ConversionEngine engine, TileImageKey image, int x, int y)
    {
        Batch batch = new();
        batch.Bg(image, x, y);
        return engine.BuildDrawList(batch.Convert(engine));
    }

    private static float Bottom(int py) => ConversionGrid.SceneHeight - py - 1;

    private static float Top(int py) => ConversionGrid.SceneHeight - py;

    private static void AssertSceneBox(
        DrawTile tile,
        float minX,
        float maxX,
        float minY,
        float maxY,
        float minZ,
        float maxZ)
    {
        TileMesh mesh = Geo.Present(tile.Mesh);
        float x0 = float.MaxValue;
        float y0 = float.MaxValue;
        float z0 = float.MaxValue;
        float x1 = float.MinValue;
        float y1 = float.MinValue;
        float z1 = float.MinValue;

        foreach (MeshVertex vertex in mesh.Vertices)
        {
            Vec3 scene = tile.Transform.TransformPoint(vertex.Position);
            x0 = MathF.Min(x0, scene.X);
            y0 = MathF.Min(y0, scene.Y);
            z0 = MathF.Min(z0, scene.Z);
            x1 = MathF.Max(x1, scene.X);
            y1 = MathF.Max(y1, scene.Y);
            z1 = MathF.Max(z1, scene.Z);
        }

        Assert.Equal(minX, x0, 3);
        Assert.Equal(maxX, x1, 3);
        Assert.Equal(minY, y0, 3);
        Assert.Equal(maxY, y1, 3);
        Assert.Equal(minZ, z0, 3);
        Assert.Equal(maxZ, z1, 3);
    }
}
