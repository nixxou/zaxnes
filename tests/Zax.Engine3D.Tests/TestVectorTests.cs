namespace Zax.Engine3D.Tests;

/// <summary>
/// The fourteen test vectors of SPEC-04 §5 (TV-E3D-01 to TV-E3D-14).
/// </summary>
public sealed class TestVectorTests
{
    [Fact]
    public void TV_E3D_01_region_of_a_periodic_pattern_REQ_E3D_090_095()
    {
        // Profile [A B] sc true. Row 5: A B A B A B at columns 10 to 15.
        ConversionEngine engine = new ProfileBuilder().Row([Img.A, Img.B]).Load();
        Batch batch = new();
        batch.BgRow(5, 10, Img.A, Img.B, Img.A, Img.B, Img.A, Img.B);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        FrameShape shape = frame.Shapes[0];
        Assert.True(shape.Background);
        Assert.Equal(6, shape.TileCount);
        Assert.Same(engine.Patterns.Permanent[0], shape.ReferencePattern);
        Assert.True(shape.Completed);
    }

    [Fact]
    public void TV_E3D_01_eight_tiles_are_kept_by_pass_1_REQ_E3D_092()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A, Img.B]).Load();
        Batch batch = new();
        batch.BgRow(5, 8, Img.A, Img.B, Img.A, Img.B, Img.A, Img.B, Img.A, Img.B);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(8, frame.Shapes[0].TileCount);

        // A row of 8 tiles is full, has 8 tiles and 2 distinct images, so pass 1 keeps it.
        Assert.True(frame.Shapes[0].IsFull());
    }

    [Fact]
    public void TV_E3D_02_self_connected_false_REQ_E3D_061()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B], selfConnected: false)
            .Load();
        Batch batch = new();
        batch.BgRow(5, 10, Img.A, Img.B, Img.A, Img.B, Img.A, Img.B);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(3, frame.Shapes.Count);
        foreach (FrameShape shape in frame.Shapes)
        {
            Assert.Equal(2, shape.TileCount);
            Assert.Equal(2, shape.Images.Count);
        }

        // Left to right.
        Assert.Equal(11, frame.Shapes[0].TileStart.X);
        Assert.Equal(13, frame.Shapes[1].TileStart.X);
        Assert.Equal(15, frame.Shapes[2].TileStart.X);
    }

    [Fact]
    public void TV_E3D_02_the_pair_B_A_does_not_exist_REQ_E3D_061()
    {
        ConversionEngine notConnected = new ProfileBuilder()
            .Row([Img.A, Img.B], selfConnected: false)
            .Load();
        Batch batch = new();
        batch.BgRow(5, 10, Img.B, Img.A);
        Assert.Equal(2, batch.Convert(notConnected).Shapes.Count);

        ConversionEngine connected = new ProfileBuilder().Row([Img.A, Img.B]).Load();
        Batch other = new();
        other.BgRow(5, 10, Img.B, Img.A);
        Assert.Single(other.Convert(connected).Shapes);
    }

    [Fact]
    public void TV_E3D_03_constraint_size_REQ_E3D_062()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A], constraintWidth: 2, constraintHeight: 99)
            .Load();
        Batch batch = new();
        batch.BgRow(5, 10, Img.A, Img.A, Img.A, Img.A, Img.A);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(3, frame.Shapes.Count);
        Assert.Equal([2, 2, 1], frame.Shapes.Select(static s => s.TileCount));
        Assert.All(frame.Shapes, s => Assert.Same(engine.Patterns.Permanent[0], s.ReferencePattern));
    }

    [Fact]
    public void TV_E3D_03_constraint_on_the_other_axis_REQ_E3D_062()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A], constraintWidth: 99, constraintHeight: 1)
            .Load();
        Batch batch = new();
        batch.BgCell(Img.A, 10, 5).BgCell(Img.A, 10, 6).BgCell(Img.A, 10, 7);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(3, frame.Shapes.Count);
        Assert.All(frame.Shapes, static s => Assert.Equal(1, s.TileCount));
    }

    [Fact]
    public void TV_E3D_04_skip_geo_REQ_E3D_062()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B], selfConnected: false, skipGeometry: true)
            .Load();
        Batch batch = new();
        batch.Bg(Img.A, 80, 40).Bg(Img.B, 80, 48);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(2, frame.Shapes[0].TileCount);
    }

    [Fact]
    public void TV_E3D_04_without_skip_geo_the_vertical_pair_is_missing_REQ_E3D_062()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B], selfConnected: false)
            .Load();
        Batch batch = new();
        batch.Bg(Img.A, 80, 40).Bg(Img.B, 80, 48);

        Assert.Equal(2, batch.Convert(engine).Shapes.Count);
    }

    [Fact]
    public void TV_E3D_05_candidate_shrinking_and_smallest_pattern_REQ_E3D_090()
    {
        // P1 = [A B], P2 = [A B C], both sc false, in this order.
        ProfileBuilder profile = new ProfileBuilder()
            .Row([Img.A, Img.B], selfConnected: false)
            .Row([Img.A, Img.B, Img.C], selfConnected: false);

        ConversionEngine engine = profile.Load();
        EnginePattern p1 = engine.Patterns.Permanent[0];
        EnginePattern p2 = engine.Patterns.Permanent[1];

        Batch ab = new();
        ab.BgRow(5, 10, Img.A, Img.B);
        ConversionFrame frameAb = ab.Convert(engine);
        Assert.Single(frameAb.Shapes);
        Assert.Same(p1, frameAb.Shapes[0].ReferencePattern);
        Assert.True(frameAb.Shapes[0].Completed);

        Batch abc = new();
        abc.BgRow(5, 10, Img.A, Img.B, Img.C);
        ConversionFrame frameAbc = abc.Convert(engine);
        Assert.Single(frameAbc.Shapes);
        Assert.Same(p2, frameAbc.Shapes[0].ReferencePattern);

        Batch cab = new();
        cab.BgRow(5, 10, Img.C, Img.A, Img.B);
        ConversionFrame frameCab = cab.Convert(engine);
        Assert.Equal(2, frameCab.Shapes.Count);
        Assert.Equal(1, frameCab.Shapes[0].TileCount);
        Assert.Same(p2, frameCab.Shapes[0].ReferencePattern);
        Assert.False(frameCab.Shapes[0].Completed);
        Assert.Equal(2, frameCab.Shapes[1].TileCount);
        Assert.Same(p1, frameCab.Shapes[1].ReferencePattern);
    }

    [Fact]
    public void TV_E3D_06_order_of_the_passes_REQ_E3D_092()
    {
        // Q = [A B; B A] sc true, and [D] sc true.
        ConversionEngine engine = new ProfileBuilder()
            .Pattern(2, 2, [Img.A, Img.B, Img.B, Img.A])
            .Row([Img.D])
            .Load();

        Batch batch = new();
        batch.BgRow(2, 4, Img.A, Img.B, Img.A, Img.B);
        batch.BgRow(3, 4, Img.B, Img.A, Img.B, Img.A);
        batch.Bg(Img.U, 160, 0);
        batch.Bg(Img.D, 160, 120);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(3, frame.Shapes.Count);

        // (1) the chequerboard, 8 tiles, pattern Q, pass 1.
        Assert.Equal(8, frame.Shapes[0].TileCount);
        Assert.Same(engine.Patterns.Permanent[0], frame.Shapes[0].ReferencePattern);

        // (2) the unknown tile U at the top border, pass 2: a temporary pattern is learned.
        Assert.Equal(1, frame.Shapes[1].TileCount);
        Assert.True(frame.Shapes[1].AtBorder);
        Assert.Single(engine.Patterns.Temporary);
        Assert.Same(engine.Patterns.Temporary[0], frame.Shapes[1].ReferencePattern);

        // (3) the D tile, pass 3.
        Assert.Equal(1, frame.Shapes[2].TileCount);
        Assert.Same(engine.Patterns.Permanent[1], frame.Shapes[2].ReferencePattern);
    }

    [Fact]
    public void TV_E3D_06_the_passes_win_over_the_germination_order_REQ_E3D_092()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Pattern(2, 2, [Img.A, Img.B, Img.B, Img.A])
            .Row([Img.D])
            .Load();
        engine.Settings.ScanDirection = 1;

        Batch batch = new();
        batch.BgRow(2, 4, Img.A, Img.B, Img.A, Img.B);
        batch.BgRow(3, 4, Img.B, Img.A, Img.B, Img.A);
        batch.Bg(Img.U, 160, 0);
        batch.Bg(Img.D, 160, 120);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(3, frame.Shapes.Count);
        Assert.Equal(8, frame.Shapes[0].TileCount);
        Assert.Same(engine.Patterns.Temporary[0], frame.Shapes[1].ReferencePattern);
        Assert.Same(engine.Patterns.Permanent[1], frame.Shapes[2].ReferencePattern);
    }

    [Fact]
    public void TV_E3D_07_learning_REQ_E3D_160_161()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();

        Batch first = new();
        first.BgRow(10, 10, Img.U, Img.U, Img.U);
        ConversionFrame frame1 = first.Convert(engine, 1);

        Assert.Equal(2, frame1.Shapes.Count);

        // Pass 2 learns a permanent 1x1 pattern from the first U.
        Assert.Equal(1, frame1.Shapes[0].TileCount);
        Assert.Single(engine.Patterns.Permanent);
        EnginePattern learned = engine.Patterns.Permanent[0];
        Assert.Same(learned, frame1.Shapes[0].ReferencePattern);
        Assert.Equal(1, learned.Width);
        Assert.Equal(1, learned.Height);
        Assert.True(learned.SelfConnected);
        Assert.False(learned.SkipGeometry);
        Assert.Equal(new Cell(99, 99), learned.ConstraintSize);
        Assert.Single(learned.Instances);
        Assert.Equal(3, learned.Instances[0].Depth);
        Assert.Equal(ZLayer.L1, learned.Instances[0].Layer);
        Assert.Equal(Geo3DType.Default, learned.Instances[0].Geometry);

        // The two remaining U are known by then, so pass 3 groups them.
        Assert.Equal(2, frame1.Shapes[1].TileCount);
        Assert.Same(learned, frame1.Shapes[1].ReferencePattern);

        Batch second = new();
        second.BgRow(10, 10, Img.U, Img.U, Img.U);
        ConversionFrame frame2 = second.Convert(engine, 2);

        Assert.Single(frame2.Shapes);
        Assert.Equal(3, frame2.Shapes[0].TileCount);
        Assert.Same(learned, frame2.Shapes[0].ReferencePattern);
    }

    [Fact]
    public void TV_E3D_07_a_border_tile_only_learns_a_temporary_pattern_REQ_E3D_160()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();

        for (int counter = 1; counter <= 3; counter++)
        {
            Batch batch = new();
            batch.Mask(8, 0).BgCell(Img.U, 0, 10);
            ConversionFrame frame = batch.Convert(engine, counter);

            Assert.Single(frame.Shapes);
            Assert.True(frame.Shapes[0].AtBorder);
            Assert.Empty(engine.Patterns.Permanent);
            Assert.Single(engine.Patterns.Temporary);
            Assert.Same(engine.Patterns.Temporary[0], frame.Shapes[0].ReferencePattern);
            Assert.False(frame.Shapes[0].Images[0].IsKnown);
        }
    }

    [Fact]
    public void TV_E3D_08_sprites_with_an_empty_profile_REQ_E3D_131()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Sprite(Img.A, 100, 50).Sprite(Img.B, 108, 50).Sprite(Img.C, 116, 52);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(2, frame.Shapes.Count);
        Assert.Equal(0, frame.BackgroundShapeCount);
        Assert.Equal(2, frame.Shapes[0].TileCount);
        Assert.Equal(1, frame.Shapes[1].TileCount);
        Assert.Equal(4, frame.Shapes[0].Palette);
        Assert.Equal(4, frame.Shapes[1].Palette);

        Assert.Equal(2, engine.Patterns.Permanent.Count);
        Assert.Equal(2, engine.Patterns.Permanent[0].Width);
        Assert.Equal(1, engine.Patterns.Permanent[0].Height);
        Assert.Equal(1, engine.Patterns.Permanent[1].Width);
    }

    [Fact]
    public void TV_E3D_08_sprite_tolerance_REQ_E3D_130_131()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B, Img.C], selfConnected: false)
            .Load();
        engine.Settings.SpriteTolerance = 2;

        Batch batch = new();
        batch.Sprite(Img.A, 100, 50).Sprite(Img.B, 108, 50).Sprite(Img.C, 116, 52);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(3, frame.Shapes[0].TileCount);

        engine.Settings.SpriteTolerance = 0;
        Batch exact = new();
        exact.Sprite(Img.A, 100, 50).Sprite(Img.B, 108, 50).Sprite(Img.C, 116, 52);
        ConversionFrame tight = exact.Convert(engine, 1);
        Assert.Equal(2, tight.Shapes.Count);
        Assert.All(tight.Shapes, s => Assert.Same(engine.Patterns.Permanent[0], s.ReferencePattern));
    }

    [Fact]
    public void TV_E3D_08_same_sprite_palette_REQ_E3D_131()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B, Img.C], selfConnected: false)
            .Load();
        engine.Settings.SpriteTolerance = 2;
        engine.Settings.SameSpritePalette = true;

        Batch batch = new();
        batch.Sprite(Img.A, 100, 50).Sprite(Img.B, 108, 50, palette: 1).Sprite(Img.C, 116, 52);
        ConversionFrame frame = batch.Convert(engine);
        Assert.Equal(3, frame.Shapes.Count);

        engine.Settings.SameSpritePalette = false;
        Batch mixed = new();
        mixed.Sprite(Img.A, 100, 50).Sprite(Img.B, 108, 50, palette: 1).Sprite(Img.C, 116, 52);
        ConversionFrame merged = mixed.Convert(engine, 1);
        Assert.Single(merged.Shapes);
        Assert.Equal(4, merged.Shapes[0].Palette);
    }

    [Fact]
    public void TV_E3D_09_frame_palette_REQ_E3D_250_251()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Palette(0, 0x0F).Palette(1, 0x30).Palette(4, 0x12)
            .Palette(5, 0x16).Palette(17, 0x27);

        ConversionFrame frame = batch.Convert(engine);
        FramePalette palette = frame.Palette;

        Assert.Equal(0x0F, palette[0]);
        Assert.Equal(0x30, palette[1]);
        Assert.Equal(0x0F, palette[4]);
        Assert.Equal(0x16, palette[5]);
        Assert.Equal(0x0F, palette[16]);
        Assert.Equal(0x27, palette[17]);
        Assert.Equal(0x16, palette[37]);
        Assert.Equal(0x27, palette[49]);

        Assert.Equal(new Rgb24(0xFC, 0x38, 0x00), palette.Color(5));
        Assert.Equal(new Rgb24(0xFC, 0xA0, 0x48), palette.Color(17));
    }

    [Fact]
    public void TV_E3D_10_image_identity_and_flips_REQ_E3D_020()
    {
        TileImageKey x = Img.Pixel(1, 2);
        ConversionEngine engine = new ProfileBuilder().Row([x]).Load();

        Batch batch = new();
        batch.Sprite(Img.Flipped(x, flipX: true), 100, 50);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        TileImage flipped = frame.Shapes[0].Images[0];
        Assert.Equal(1, flipped.PixelCount);
        Assert.Equal(new Cell(6, 2), flipped.PixelStart);
        Assert.Equal(new Cell(6, 2), flipped.PixelEnd);

        // X' is unknown: the sprite made a new pattern and is not bound to [X].
        Assert.Equal(2, engine.Patterns.Permanent.Count);
        Assert.NotSame(engine.Patterns.Permanent[0], frame.Shapes[0].ReferencePattern);
        Assert.Same(engine.Patterns.Permanent[1], frame.Shapes[0].ReferencePattern);
    }

    [Fact]
    public void TV_E3D_11_clipping_REQ_E3D_002_003_005()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Bg(Img.A, -8, 100)
            .Bg(Img.B, 248, 100)
            .Sprite(Img.C, 100, 240)
            .Bg(Img.D, -7, -7);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Empty(frame.Sprites);
        FrameTile? kept = frame.BackgroundAt(new Cell(1, 1));
        Assert.NotNull(kept);
        Assert.Equal(-7, kept.X);
        Assert.Equal(-7, kept.Y);
        Assert.Equal(1, frame.BackgroundTileCount);
    }

    [Fact]
    public void TV_E3D_12_tracker_REQ_E3D_300()
    {
        // The vector is expressed in shape centres, so the frames are built so that the sprite
        // shapes have the required ones: a one-tile shape at (x, y) has centre (x + 4, y + 4).
        ConversionEngine engine = ProfileBuilder.EmptyEngine();

        Batch start = new();
        start.Sprite(Img.A, 96, 96);
        ConversionFrame frame1 = start.Convert(engine, 1);
        FrameShape tracked = frame1.Shapes[0];
        Assert.Equal(new Vec3(100f, 100f, 0f), tracked.Center);

        ShapeTracker tracker = engine.StartTracker(tracked);
        EnginePattern patternT = tracked.ReferencePattern!;

        // Frame 2: X (another pattern) at centre (104, 100) then Y (pattern of T) at (140, 100).
        Batch second = new();
        second.Sprite(Img.B, 100, 96).Sprite(Img.A, 136, 96);
        ConversionFrame frame2 = second.Convert(engine, 2);
        Assert.Equal(2, frame2.Shapes.Count);
        EnginePattern patternX = frame2.Shapes[0].ReferencePattern!;

        FrameShape? found = tracker.Track(frame2);
        Assert.Same(frame2.Shapes[1], found);
        Assert.Same(patternT, found!.ReferencePattern);
        Assert.Contains(patternX, tracker.Excluded);
        Assert.Equal(new Vec3(140f, 100f, 0f), tracker.Position);

        // Frame 3: X alone at (141, 100): its pattern is excluded, nothing is found.
        Batch third = new();
        third.Sprite(Img.B, 137, 96);
        ConversionFrame frame3 = third.Convert(engine, 3);
        Assert.Null(tracker.Track(frame3));
        Assert.Equal(new Vec3(140f, 100f, 0f), tracker.Position);

        // Frame 4: Z at centre (145, 103), 5.83 pixels away: found by proximity.
        Batch fourth = new();
        fourth.Sprite(Img.C, 141, 99);
        ConversionFrame frame4 = fourth.Convert(engine, 4);
        FrameShape? z = tracker.Track(frame4);
        Assert.Same(frame4.Shapes[0], z);
        Assert.Contains(z!.ReferencePattern!, tracker.Included);
        Assert.Equal(new Vec3(145f, 103f, 0f), tracker.Position);
    }

    [Fact]
    public void TV_E3D_13_a_frame_instance_is_reset_every_frame_REQ_E3D_190()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A], tags: "t").Load();
        RecordingScriptHost host = new()
        {
            OnUpdateShape = static instance => instance.Offset.X += 5f,
        };
        engine.ScriptHost = host;

        List<float> positions = [];
        for (int counter = 1; counter <= 3; counter++)
        {
            Batch batch = new();
            batch.BgCell(Img.A, 10, 5);
            ConversionFrame frame = batch.Convert(engine, counter);
            positions.Add(frame.Instances[0].Pos.X);
        }

        Assert.Equal(3, positions.Count);
        Assert.Equal(positions[0], positions[1]);
        Assert.Equal(positions[1], positions[2]);

        // Without the script the anchor would be at the centre of the tile box.
        ConversionEngine plain = new ProfileBuilder().Row([Img.A], tags: "t").Load();
        Batch bare = new();
        bare.BgCell(Img.A, 10, 5);
        float expected = bare.Convert(plain).Instances[0].Pos.X;
        Assert.Equal(expected + 5f, positions[0], 3);
    }

    [Fact]
    public void TV_E3D_14_injected_shape_REQ_E3D_230()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        EnginePattern pattern = engine.Patterns.Permanent[0];

        RecordingScriptHost host = new();
        engine.ScriptHost = host;
        host.OnUpdate = () => engine.CreateNewShape(pattern, new Vec3(10.5f, 20.25f, 3f), 5);

        ConversionFrame frame = new Batch().Convert(engine, 1);

        Assert.Single(frame.Shapes);
        FrameShape shape = frame.Shapes[0];
        Assert.False(shape.Background);
        Assert.False(shape.Scriptable);
        Assert.Empty(frame.ScriptableInstances);

        FrameTile tile = shape.Tiles[0];
        Assert.Equal(10, tile.X);
        Assert.Equal(221, tile.Y);
        Assert.Equal(1, tile.Palette);
        Assert.False(tile.Hidden);

        Assert.Equal(new Cell(10, 221), shape.PixelStart);
        Assert.Equal(new Cell(17, 228), shape.PixelEnd);
        Assert.Equal(new Vec2(10f, 19f), shape.BottomLeft);

        FrameInstance instance = shape.Instances[0];
        Assert.Equal(0.5f, instance.Offset.X, 4);
        Assert.Equal(-0.25f, instance.Offset.Y, 4);
        Assert.Equal(3f, instance.Offset.Z, 4);
        Assert.Equal(14.5f, instance.Pos.X, 4);
        Assert.Equal(22.75f, instance.Pos.Y, 4);
        Assert.Equal(4.5f, instance.Pos.Z, 4);
    }
}
