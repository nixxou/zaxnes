using System.Globalization;
using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// Multi-tile shapes, multiple 3D objects, the mesh lifetime of SPEC-04B §3.7 and the
/// determinism REQ-ARCH-012 asks of the geometry.
/// </summary>
public sealed class GeometryLifetimeTests
{
    [Fact]
    public void REQ_E3D_540_a_shape_draws_one_mesh_per_tile_and_shares_the_repeated_ones()
    {
        // Four tiles drawn, two distinct meshes: three cells carry the same image.
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder()
            .Pattern(2, 2, [Img.A, Img.A, Img.A, Img.B], depth: 2)
            .Build());

        Batch batch = new();
        batch.Bg(Img.A, 0, 0).Bg(Img.A, 8, 0).Bg(Img.A, 0, 8).Bg(Img.B, 8, 8);
        FrameDrawList list = engine.BuildDrawList(batch.Convert(engine));

        Assert.Equal(4, list.Tiles.Count);

        // REQ-E3D-540: one mesh per distinct 2D tile. Q-067 adds that what is *drawn* is the
        // variant of that mesh without the walls the neighbours cover, so the criterion is
        // read on the mesh of the image, which is the one the object owns.
        HashSet<TileMesh> meshes = [];
        foreach (DrawTile tile in list.Tiles)
        {
            meshes.Add(Geo.Present(geometry.MeshOf(tile.Instance.Reference, tile.Tile.Image)));
        }

        Assert.Equal(2, meshes.Count);
        Assert.Equal(1, geometry.ObjectCount);
    }

    [Fact]
    public void REQ_E3D_541_each_3d_object_of_a_pattern_gets_its_own_geometry()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        GameProfile profile = new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 3, instanceCount: 2)
            .Build();
        profile.PermanentPatterns[0].Variants[1].Geometry = VariantGeometry.Cube;
        profile.PermanentPatterns[0].Variants[1].Layer = ProfileLayer.L2;
        profile.PermanentPatterns[0].Variants[1].Depth = 4;
        engine.LoadProfile(profile);

        EnginePattern pattern = engine.Patterns.Permanent[0];
        Assert.Equal(2, pattern.Instances.Count);
        Assert.Equal(2, geometry.ObjectCount);

        ObjectGeometry first = Geo.Present(geometry.Of(pattern.Instances[0]));
        ObjectGeometry second = Geo.Present(geometry.Of(pattern.Instances[1]));
        Assert.NotSame(first, second);
        Assert.Equal(3, first.BuildDepth);
        Assert.Equal(4, second.BuildDepth);

        // The Default object is a prism of six rectangles, the CUBE object a box of twelve.
        Assert.Equal(6, Geo.Rectangles(Geo.Present(first.Mesh(pattern.Images[0]))));
        Assert.Equal(12, Geo.Rectangles(Geo.Present(second.Mesh(pattern.Images[0]))));

        Batch batch = new();
        batch.Bg(Img.A, 16, 32);
        FrameDrawList list = engine.BuildDrawList(batch.Convert(engine));
        Assert.Equal(2, list.Tiles.Count);
        Assert.NotEqual(list.Tiles[0].Transform, list.Tiles[1].Transform);
    }

    [Fact]
    public void REQ_E3D_542_changing_a_frame_instance_moves_it_without_remeshing_it()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 4).Build());

        Batch batch = new();
        batch.Bg(Img.A, 16, 32);
        ConversionFrame frame = batch.Convert(engine);
        int builds = geometry.BuildCount;

        DrawTile before = Assert.Single(engine.BuildDrawList(frame).Tiles);
        TileMesh mesh = Geo.Present(before.Mesh);
        float thickness = mesh.BoundsMax.Z - mesh.BoundsMin.Z;
        Assert.Equal(4f, thickness, 3);

        // A script makes the instance deeper: the placement box follows, the mesh does not.
        frame.Instances[0].Depth = 16;
        engine.CalculatePositions(frame);
        DrawTile after = Assert.Single(engine.BuildDrawList(frame).Tiles);

        Assert.Same(mesh, after.Mesh);
        Assert.Equal(4f, mesh.BoundsMax.Z - mesh.BoundsMin.Z, 3);
        Assert.NotEqual(before.Anchor.Z, after.Anchor.Z);
        Assert.Equal(builds, geometry.BuildCount);
    }

    [Fact]
    public void REQ_E3D_543_a_hidden_or_unready_object_is_not_drawn()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.DeferTransfer = true;
        engine.LoadProfile(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 2).Build());

        Batch batch = new();
        batch.Bg(Img.A, 16, 32);
        ConversionFrame frame = batch.Convert(engine);

        // REQ-E3D-601: the shape is recognised and visible to scripts, but not drawn until
        // its meshes have been handed over.
        Assert.Single(frame.Instances);
        Assert.Empty(engine.BuildDrawList(frame).Tiles);

        Assert.Equal(1, geometry.Commit());
        Assert.Single(engine.BuildDrawList(frame).Tiles);

        engine.Patterns.Permanent[0].Instances[0].Style = RenderStyle.Hidden;
        Assert.Empty(engine.BuildDrawList(batch.Convert(engine, 2)).Tiles);
    }

    [Fact]
    public void REQ_E3D_600_a_mass_build_reports_its_progress_in_two_halves()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.DeferTransfer = true;

        ProfileBuilder builder = new();
        for (int n = 0; n < 4; n++)
        {
            builder.Pattern(1, 1, [Img.Full(n)], depth: 2);
        }

        engine.LoadProfile(builder.Build());

        // Four objects built, none transferred: (4 + 0) / (2 × 4) = 0.5.
        Assert.Equal(4, geometry.BuildCount);
        Assert.Equal(0.5f, geometry.Progress.Value, 3);
        Assert.Equal(0.5f, engine.LoadProgress, 3);

        Assert.Equal(4, geometry.Commit());
        Assert.Equal(1f, geometry.Progress.Value, 3);
        Assert.Equal(1f, engine.LoadProgress, 3);

        // Changing the global algorithm rebuilds every object, with its own progress.
        engine.SetRenderAlgorithm(RenderAlgorithm.Marching);
        Assert.Equal(8, geometry.BuildCount);
        Assert.Equal(0.5f, geometry.Progress.Value, 3);
        geometry.Commit();
        Assert.Equal(1f, geometry.Progress.Value, 3);

        foreach (EnginePattern pattern in engine.Patterns.Permanent)
        {
            Assert.Equal(
                RenderAlgorithm.Marching,
                Geo.Present(geometry.Of(pattern.Instances[0])).Algorithm);
        }
    }

    [Fact]
    public void REQ_E3D_500_an_object_with_its_own_algorithm_ignores_the_setting()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        GameProfile profile = new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 2, instanceCount: 2)
            .Build();
        profile.PermanentPatterns[0].Variants[1].MeshAlgorithm = MeshAlgorithm.Marching;
        engine.LoadProfile(profile);

        EnginePattern pattern = engine.Patterns.Permanent[0];
        Assert.Equal(RenderAlgorithm.Greedy, Geo.Present(geometry.Of(pattern.Instances[0])).Algorithm);
        Assert.Equal(RenderAlgorithm.Marching, Geo.Present(geometry.Of(pattern.Instances[1])).Algorithm);

        engine.SetRenderAlgorithm(RenderAlgorithm.GreedyMulticolor);
        Assert.Equal(
            RenderAlgorithm.GreedyMulticolor,
            Geo.Present(geometry.Of(pattern.Instances[0])).Algorithm);
        Assert.Equal(RenderAlgorithm.Marching, Geo.Present(geometry.Of(pattern.Instances[1])).Algorithm);
    }

    [Fact]
    public void REQ_E3D_602_no_runtime_change_ever_rebuilds_a_mesh()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 4).Build());

        int builds = geometry.BuildCount;
        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];
        TileMesh mesh = Geo.Present(geometry.MeshOf(reference, engine.Images.ById(0)));

        for (int counter = 1; counter <= 3; counter++)
        {
            Batch batch = new();
            batch.Palette(1, (byte)(0x10 + counter)).Bg(Img.A, 16 + counter, 32);
            ConversionFrame frame = batch.Convert(engine, counter);

            FrameInstance instance = frame.Instances[0];
            instance.Offset = new Vec3(counter, 0f, 0f);
            instance.Scale = new Vec3(1f + counter, 1f, 1f);
            instance.Rotation = new Vec3(0f, 0f, 10f * counter);
            instance.Alpha = 0.5f;
            instance.Layer = ZLayer.L3;
            instance.TagString = "!cast";
            instance.DeformAmplitude = new Vec3(0f, 0.1f, 0f);
            instance.CustomPalette = counter;
            engine.CalculatePositions(frame);

            DrawTile tile = Assert.Single(engine.BuildDrawList(frame).Tiles);
            Assert.Same(mesh, tile.Mesh);
        }

        Assert.Equal(builds, geometry.BuildCount);
        Assert.Equal(1f, geometry.Progress.Value, 3);
    }

    [Fact]
    public void REQ_E3D_600_learning_a_pattern_builds_only_its_geometry()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 4).Build());

        int builds = geometry.BuildCount;

        // An unknown sprite in the middle of the screen is learned (REQ-E3D-161) with the 3D
        // defaults of REQ-E3D-453: Default, depth 3, layer L1.
        Batch batch = new();
        batch.Sprite(Img.U, 64, 64);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(builds + 1, geometry.BuildCount);
        Assert.Equal(2, engine.Patterns.Permanent.Count);

        PatternInstance learned = engine.Patterns.Permanent[1].Instances[0];
        Assert.Equal(3, learned.Depth);
        Assert.Equal(ZLayer.L1, learned.Layer);
        Assert.Equal(Geo3DType.Default, learned.Geometry);

        DrawTile tile = Assert.Single(engine.BuildDrawList(frame).Tiles);
        Assert.Equal(6, Geo.Rectangles(Geo.Present(tile.Mesh)));
    }

    [Fact]
    public void REQ_E3D_603_releasing_a_pattern_frees_its_meshes_and_nothing_caps_a_frame()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder()
            .Pattern(2, 1, [Img.A, Img.B], depth: 2)
            .Build());

        Assert.Equal(1, geometry.ObjectCount);
        engine.Unload();
        Assert.Equal(0, geometry.ObjectCount);

        // 1 500 instanced shapes over 3 000 tiles all reach the draw list (REQ-E3D-603).
        ConversionEngine crowd = new();
        crowd.UseGeometry();
        crowd.LoadProfile(new ProfileBuilder().Pattern(2, 1, [Img.A, Img.B], depth: 2).Build());

        EnginePattern pattern = crowd.Patterns.Permanent[0];
        Batch empty = new();
        empty.Convert(crowd);
        for (int n = 0; n < 1500; n++)
        {
            crowd.AddNewShape(pattern, new Vec3(n % 200, n / 200, 0f), palette: 4);
        }

        crowd.CalculatePositions(crowd.ConvertingFrame);
        FrameDrawList list = crowd.BuildDrawList(crowd.ConvertingFrame);
        Assert.Equal(3000, list.Tiles.Count);
    }

    [Fact]
    public void REQ_E3D_643_the_clip_margins_never_touch_the_meshes()
    {
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 4).Build());

        engine.Settings.Clipping = ConversionSettings.DefaultClipping;
        string wide = Describe(Draw(engine, 64, 64, counter: 1));

        engine.Settings.Clipping = new Vec4F(40f, 40f, 40f, 40f);
        string narrow = Describe(Draw(engine, 64, 64, counter: 2));

        Assert.Equal(wide, narrow);
    }

    [Fact]
    public void REQ_ARCH_012_two_identical_builds_give_identical_geometry()
    {
        string first = BuildAll();
        string second = BuildAll();
        Assert.Equal(first, second);
    }

    [Fact]
    public void Q_013_rebuilding_a_cylinder_does_not_shrink_it_again()
    {
        // REQ-E3D-452 writes D' back into sizeZ; a rebuild must still start from the depth the
        // profile asked for, or every algorithm change would thin the object down.
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        GameProfile profile = new ProfileBuilder()
            .Pattern(1, 1, [Img.Pixel(0, 0)], depth: 8)
            .Build();
        profile.PermanentPatterns[0].Variants[0].Geometry = VariantGeometry.VerticalCylinder;
        engine.LoadProfile(profile);

        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];
        ObjectGeometry built = Geo.Present(geometry.Of(reference));
        Assert.Equal(8, built.RequestedDepth);
        Assert.Equal(1, built.EffectiveDepth);
        Assert.Equal(1, reference.Depth);

        engine.SetRenderAlgorithm(RenderAlgorithm.Marching);
        ObjectGeometry rebuilt = Geo.Present(geometry.Of(reference));
        Assert.Equal(8, rebuilt.RequestedDepth);
        Assert.Equal(1, rebuilt.EffectiveDepth);

        // A BuildPattern hook that moves sizeZ still wins (REQ-E3D-455, REQ-E3D-322).
        RecordingScriptHost host = new() { OnBuildPattern = instance => instance.Depth = 4 };
        engine.ScriptHost = host;
        engine.SetRenderAlgorithm(RenderAlgorithm.Greedy);
        Assert.Equal(4, Geo.Present(geometry.Of(reference)).RequestedDepth);
    }

    private static FrameDrawList Draw(ConversionEngine engine, int x, int y, int counter)
    {
        Batch batch = new();
        batch.Bg(Img.A, x, y);
        return engine.BuildDrawList(batch.Convert(engine, counter));
    }

    private static string Describe(FrameDrawList list)
    {
        List<string> lines = [];
        foreach (DrawTile tile in list.Tiles)
        {
            float[] matrix = new float[16];
            tile.Transform.CopyTo(matrix);
            lines.Add(string.Create(
                CultureInfo.InvariantCulture,
                $"{tile.Material} {tile.Alpha} {tile.PaletteTable} {string.Join(',', matrix)}"));

            TileMesh mesh = Geo.Present(tile.Mesh);
            foreach (MeshVertex vertex in mesh.Vertices)
            {
                lines.Add(string.Create(
                    CultureInfo.InvariantCulture,
                    $"  {vertex.Position} {vertex.Normal} {vertex.Uv}"));
            }
        }

        return string.Join('\n', lines);
    }

    private static string BuildAll()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        ProfileBuilder builder = new();
        TileImageKey[] tiles =
        [
            Geo.Solid(1),
            Geo.Tile("12345678"[..8]),
            Geo.Tile("11111111", "1......1", "1......1", "11111111"),
            Geo.Tile("1......."),
        ];

        for (int n = 0; n < tiles.Length; n++)
        {
            builder.Pattern(1, 1, [tiles[n]], depth: 2 + (2 * n));
        }

        GameProfile profile = builder.Build();
        VariantGeometry[] kinds =
        [
            VariantGeometry.Default,
            VariantGeometry.VerticalCylinder,
            VariantGeometry.HorizontalCylinder,
            VariantGeometry.Cube,
        ];

        for (int n = 0; n < profile.PermanentPatterns.Count; n++)
        {
            PatternVariant variant = profile.PermanentPatterns[n].Variants[0];
            variant.Geometry = kinds[n];
            variant.Solid = n % 2 == 0;
            variant.FullWarp = n % 3 == 0;
        }

        engine.LoadProfile(profile);

        List<string> lines = [];
        foreach (EnginePattern pattern in engine.Patterns.Permanent)
        {
            foreach (PatternInstance instance in pattern.Instances)
            {
                ObjectGeometry built = Geo.Present(geometry.Of(instance));
                lines.Add(string.Create(
                    CultureInfo.InvariantCulture,
                    $"{built.Algorithm} {built.BuildDepth} {built.EffectiveDepth}"));
                foreach (TileMesh mesh in built.Meshes)
                {
                    foreach (MeshVertex vertex in mesh.Vertices)
                    {
                        lines.Add(string.Create(
                            CultureInfo.InvariantCulture,
                            $"  {vertex.Position} {vertex.Normal} {vertex.Uv}"));
                    }

                    foreach (int index in mesh.Triangles)
                    {
                        lines.Add(string.Create(CultureInfo.InvariantCulture, $"   {index}"));
                    }
                }
            }
        }

        return string.Join('\n', lines);
    }
}
