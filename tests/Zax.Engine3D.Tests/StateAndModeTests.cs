using System.Globalization;
using System.Text;
using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// Determinism (REQ-ARCH-012), the learned state a savestate carries (REQ-E3D-008,
/// DEC-R2-T5), 2D mode (SPEC-04 §3.17) and the default texture of REQ-PROF-052.
/// </summary>
public sealed class StateAndModeTests
{
    /// <summary>
    /// A textual digest of everything a frame exposes, used to compare two runs byte for byte.
    /// </summary>
    private static string Digest(ConversionEngine engine, ConversionFrame frame)
    {
        StringBuilder text = new();
        text.Append(CultureInfo.InvariantCulture, $"frame {frame.Counter} t={frame.Time:R} ");
        text.Append(CultureInfo.InvariantCulture, $"bg={frame.BackgroundTileCount} sp={frame.Sprites.Count} ");
        text.Append(CultureInfo.InvariantCulture, $"shapes={frame.Shapes.Count}/{frame.BackgroundShapeCount}\n");

        foreach (FrameShape shape in frame.Shapes)
        {
            text.Append(CultureInfo.InvariantCulture,
                $"  shape bg={shape.Background} t={shape.TileStart}-{shape.TileEnd} p={shape.PixelStart}-{shape.PixelEnd} ")
                .Append(CultureInfo.InvariantCulture,
                    $"pal={shape.Palette} hidden={shape.Hidden} tiles={shape.TileCount} ")
                .Append(CultureInfo.InvariantCulture,
                    $"pattern={shape.ReferencePattern?.RegistrationIndex} completed={shape.Completed}\n");
        }

        foreach (FrameInstance instance in frame.Instances)
        {
            text.Append(CultureInfo.InvariantCulture,
                $"  instance {instance.Index} pos={instance.Pos} centre={instance.Center} ")
                .Append(CultureInfo.InvariantCulture,
                    $"layer={instance.Layer} depth={instance.Depth} palette={instance.PaletteTable}\n");
        }

        foreach (DrawTile tile in engine.BuildDrawList(frame).Tiles)
        {
            text.Append(CultureInfo.InvariantCulture,
                $"  draw {tile.Tile.X},{tile.Tile.Y} anchor={tile.Anchor} centre={tile.TileCenter} ")
                .Append(CultureInfo.InvariantCulture,
                    $"mat={tile.Material} pal={tile.PaletteTable} cast={tile.CastShadow} sprite={tile.Sprite}\n");
        }

        for (int entry = 0; entry < FramePalette.EntryCount; entry++)
        {
            text.Append(CultureInfo.InvariantCulture, $"{frame.Palette[entry]:X2}");
        }

        return text.ToString();
    }

    private static GameProfile SampleProfile() => new ProfileBuilder()
        .Row([Img.A, Img.B])
        .Row([Img.C], selfConnected: false)
        .Pattern(2, 2, [Img.D, Img.A, Img.A, Img.D])
        .Row([Img.Full(10)], permanent: false)
        .Build();

    private static List<Batch> SampleBatches()
    {
        List<Batch> batches = [];
        for (int n = 0; n < 6; n++)
        {
            Batch batch = new();
            batch.Palette(0, (byte)(0x0F + n)).Palette(5, 0x16).Mask(8, 0);
            batch.BgRow(5 + n, 4, Img.A, Img.B, Img.A, Img.B, Img.C);
            batch.BgRow(6 + n, 4, Img.D, Img.A, Img.A, Img.D);
            batch.BgCell(Img.Full(20 + n), 20, 20);
            batch.BgCell(Img.Full(10), 0, 10);
            batch.Sprite(Img.A, 100 + n, 50).Sprite(Img.B, 108 + n, 50);
            batch.Sprite(Img.Full(30 + n), 200, 100);
            batches.Add(batch);
        }

        return batches;
    }

    [Fact]
    public void REQ_ARCH_012_two_runs_of_the_same_batches_give_the_same_frames()
    {
        string first = Run();
        string second = Run();
        Assert.Equal(first, second);

        static string Run()
        {
            ConversionEngine engine = new();
            engine.LoadProfile(SampleProfile());
            StringBuilder log = new();
            int counter = 1;
            foreach (Batch batch in SampleBatches())
            {
                ConversionFrame frame = batch.Convert(engine, counter++);
                log.Append(Digest(engine, frame));
                engine.Publish();
            }

            log.Append(Convert.ToHexString(EngineStateCodec.Write(engine)));
            return log.ToString();
        }
    }

    [Fact]
    public void REQ_ARCH_011_two_engines_in_the_same_process_are_independent()
    {
        ConversionEngine a = new();
        a.LoadProfile(SampleProfile());
        ConversionEngine b = new();
        b.LoadProfile(new ProfileBuilder().Row([Img.C]).Build());

        List<Batch> batches = SampleBatches();
        StringBuilder interleaved = new();
        int counter = 1;
        foreach (Batch batch in batches)
        {
            interleaved.Append(Digest(a, batch.Convert(a, counter)));
            Batch other = new();
            other.BgCell(Img.C, 3, 3);
            other.Convert(b, counter);
            counter++;
        }

        ConversionEngine alone = new();
        alone.LoadProfile(SampleProfile());
        StringBuilder solo = new();
        counter = 1;
        foreach (Batch batch in SampleBatches())
        {
            solo.Append(Digest(alone, batch.Convert(alone, counter++)));
        }

        Assert.Equal(solo.ToString(), interleaved.ToString());
    }

    [Fact]
    public void DEC_R2_T5_the_learned_state_round_trips()
    {
        GameProfile profile = SampleProfile();
        ConversionEngine engine = new();
        engine.LoadProfile(profile);

        int counter = 1;
        foreach (Batch batch in SampleBatches())
        {
            batch.Convert(engine, counter++);
            engine.Publish();
        }

        // Something really was learned.
        Assert.True(engine.Patterns.Permanent.Count > engine.Patterns.ProfilePatternCount - 1);
        Assert.NotEmpty(engine.Patterns.Temporary);
        Assert.True(engine.Images.Count > engine.ProfileImageCount);

        FrameShape tracked = engine.PublishedFrame!.Shapes[0];
        engine.StartTracker(tracked);
        engine.Tracker.Track(engine.PublishedFrame!);

        byte[] state = EngineStateCodec.Write(engine);
        Assert.True(EngineStateCodec.FitsReservedCapacity(state.Length));

        // The same state written twice gives the same bytes.
        Assert.Equal(state, EngineStateCodec.Write(engine));

        string before = Describe(engine);

        // A fresh engine with the same profile, then the state put back.
        ConversionEngine restored = new();
        restored.LoadProfile(SampleProfile());
        EngineStateCodec.Read(restored, state);

        Assert.Equal(before, Describe(restored));
        Assert.Equal(engine.Images.Count, restored.Images.Count);
        Assert.Equal(
            Convert.ToHexString(state),
            Convert.ToHexString(EngineStateCodec.Write(restored)));

        // And the next frame converts the same way on both.
        Batch next = new();
        next.BgRow(12, 4, Img.A, Img.B, Img.C).Sprite(Img.Full(33), 60, 60);
        Assert.Equal(
            Digest(engine, next.Convert(engine, 99)),
            Digest(restored, next.Convert(restored, 99)));

        static string Describe(ConversionEngine engine)
        {
            StringBuilder text = new();
            foreach (EnginePattern pattern in engine.Patterns.Permanent)
            {
                text.Append(CultureInfo.InvariantCulture,
                    $"perm {pattern.Width}x{pattern.Height} |Img|={pattern.Key.Count} sc={pattern.SelfConnected} ")
                    .Append(CultureInfo.InvariantCulture,
                        $"skip={pattern.SkipGeometry} c={pattern.ConstraintSize} ins={pattern.Instances.Count} ")
                    .Append(CultureInfo.InvariantCulture,
                        $"depth={pattern.Instances[0].Depth} layer={pattern.Instances[0].Layer}\n");
            }

            foreach (EnginePattern pattern in engine.Patterns.Temporary)
            {
                text.Append(CultureInfo.InvariantCulture,
                    $"temp {pattern.Width}x{pattern.Height} |Img|={pattern.Key.Count}\n");
            }

            text.Append(CultureInfo.InvariantCulture,
                $"tracker running={engine.Tracker.Running} sp={engine.Tracker.Sprites} ")
                .Append(CultureInfo.InvariantCulture,
                    $"pos={engine.Tracker.Position} in={engine.Tracker.Included.Count} ")
                .Append(CultureInfo.InvariantCulture, $"out={engine.Tracker.Excluded.Count}\n");
            return text.ToString();
        }
    }

    [Fact]
    public void DEC_R2_T5_the_restored_patterns_get_their_geometry_back()
    {
        ConversionEngine engine = new();
        engine.LoadProfile(SampleProfile());
        Batch learning = new();
        learning.BgCell(Img.U, 10, 10);
        learning.Convert(engine, 1);
        byte[] state = EngineStateCodec.Write(engine);

        ConversionEngine restored = new();
        RecordingGeometryBuilder geometry = new();
        RecordingScriptHost host = new();
        restored.Geometry = geometry;
        restored.ScriptHost = host;
        restored.LoadProfile(SampleProfile());
        int builtAtLoad = geometry.Built.Count;
        int hookedAtLoad = host.BuiltPatterns.Count;

        EngineStateCodec.Read(restored, state);

        // Two patterns come back: the learned one and the profile's temporary pattern, which
        // belongs to the learned state as well (DEC-R2-T5).
        Assert.Equal(builtAtLoad + 2, geometry.Built.Count);
        Assert.Equal(hookedAtLoad + 2, host.BuiltPatterns.Count);
    }

    [Fact]
    public void DEC_R2_T5_a_state_written_with_another_profile_is_refused()
    {
        ConversionEngine engine = new();
        engine.LoadProfile(SampleProfile());
        byte[] state = EngineStateCodec.Write(engine);

        ConversionEngine other = new();
        other.LoadProfile(new ProfileBuilder().Row([Img.C]).Build());

        Assert.Throws<InvalidDataException>(() => EngineStateCodec.Read(other, state));
        Assert.Throws<InvalidDataException>(() => EngineStateCodec.Read(engine, new byte[8]));
    }

    [Fact]
    public void REQ_E3D_380_two_dimensional_mode_starts_from_an_empty_registry()
    {
        ConversionEngine engine = new() { TwoDimensionalMode = true };
        engine.LoadProfile(GameProfile.CreateEmpty());

        Batch batch = new();
        batch.BgCell(Img.A, 10, 10);
        batch.Convert(engine, 1);
        Assert.Single(engine.Patterns.Permanent);

        engine.Unload();
        Assert.Empty(engine.Patterns.Permanent);
        Assert.Equal(0, engine.Images.Count);

        engine.LoadProfile(GameProfile.CreateEmpty());
        Assert.Empty(engine.Patterns.Permanent);
    }

    [Fact]
    public void REQ_E3D_381_a_two_image_brick_wall_gives_a_two_tile_shape_in_2D_mode()
    {
        ConversionEngine engine = new() { TwoDimensionalMode = true };
        engine.LoadProfile(GameProfile.CreateEmpty());

        Batch batch = new();
        batch.BgRow(10, 4, Img.A, Img.B, Img.A, Img.B, Img.A, Img.B);
        ConversionFrame frame = batch.Convert(engine, 1);

        // Pass 2 stops at one tile per image, instead of swallowing the whole wall.
        Assert.Equal(2, frame.Shapes[0].TileCount);
        Assert.Equal(2, frame.Shapes[0].Images.Count);

        // In 3D mode the same batch gives a single-tile shape from pass 2 instead.
        ConversionEngine threeD = new();
        threeD.LoadProfile(GameProfile.CreateEmpty());
        Batch same = new();
        same.BgRow(10, 4, Img.A, Img.B, Img.A, Img.B, Img.A, Img.B);
        Assert.Equal(1, same.Convert(threeD, 1).Shapes[0].TileCount);
    }

    [Fact]
    public void REQ_E3D_381_a_big_unfilled_shape_is_cut_down_to_its_first_full_band()
    {
        ConversionEngine engine = new() { TwoDimensionalMode = true };
        engine.LoadProfile(GameProfile.CreateEmpty());

        // Row 10 holds 28 distinct images, row 11 only 10: the shape is big and does not fill
        // its box, so the first run of entirely full rows is kept.
        Batch batch = new();
        for (int column = 0; column < 28; column++)
        {
            batch.BgCell(Img.Full(column), column, 10);
        }

        for (int column = 0; column < 10; column++)
        {
            batch.BgCell(Img.Full(28 + column), column, 11);
        }

        ConversionFrame frame = batch.Convert(engine, 1);

        FrameShape band = frame.Shapes[0];
        Assert.Equal(28, band.TileCount);
        Assert.Equal(new Cell(28, 1), band.TileSize);
        Assert.All(band.Tiles, static tile => Assert.Equal(80, tile.Y));
    }

    [Fact]
    public void REQ_PROF_052_the_default_texture_of_a_pattern()
    {
        ProfileBuilder builder = new();
        builder.Pattern(2, 1, [Img.Pixel(0, 0), null]);
        ProfilePattern pattern = builder.Build().PermanentPatterns[0];

        byte[] texels = DefaultTexture.Build(pattern);

        Assert.Equal(16 * 8, texels.Length);
        Assert.Equal(1, texels[0]);
        Assert.Equal(0, texels[1]);

        // The empty cell is all zero.
        for (int y = 0; y < 8; y++)
        {
            for (int x = 8; x < 16; x++)
            {
                Assert.Equal(0, texels[x + (y * 16)]);
            }
        }

        ConversionEngine engine = new();
        engine.LoadProfile(builder.Build());
        Assert.Equal(texels, DefaultTexture.Build(engine.Patterns.Permanent[0]));
    }

    [Fact]
    public void REQ_PROF_052_a_texture_equal_to_the_default_one_is_recognised()
    {
        ProfileBuilder builder = new();
        builder.Pattern(1, 1, [Img.Pixel(0, 0)]);
        ProfilePattern pattern = builder.Build().PermanentPatterns[0];
        PatternVariant variant = pattern.Variants[0];

        // No texture and the null marker both mean "use the default".
        Assert.True(DefaultTexture.IsDefaultTexture(null, pattern));
        Assert.True(DefaultTexture.IsDefaultTexture(VariantTexture.NullMarker(), pattern));

        // An explicit copy of the default texture is recognised as such.
        byte[] texels = DefaultTexture.Build(pattern);
        TexturePixels pixels = new(TextureFormat.Indexed);
        int words = (texels.Length + 15) / 16;
        for (int w = 0; w < words; w++)
        {
            pixels.Data.Add(0);
            pixels.Extra.Add(0);
        }

        for (int n = 0; n < texels.Length; n++)
        {
            pixels.Data[n / 16] |= (texels[n] & 3) << (2 * (n % 16));
            pixels.Extra[n / 16] |= ((texels[n] >> 2) & 1) << (n % 16);
        }

        variant.Texture = new VariantTexture(8, 8, pixels);
        Assert.True(DefaultTexture.IsDefaultTexture(variant.Texture, pattern));

        // One texel apart it is not.
        pixels.Data[0] ^= 3;
        Assert.False(DefaultTexture.IsDefaultTexture(variant.Texture, pattern));
    }
}
