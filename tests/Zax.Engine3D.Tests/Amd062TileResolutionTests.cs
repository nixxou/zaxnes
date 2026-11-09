using Zax.Profiles;
using Zax.Profiles.Hash;
using Zax.Profiles.Tests;

namespace Zax.Engine3D.Tests;

/// <summary>
/// A pending tile in the conversion, and the resolution of a « 3dh » profile while the game runs
/// (AMD-062 REQ-PROF-080, REQ-PROF-081, REQ-PROF-082).
/// </summary>
public sealed class Amd062TileResolutionTests
{
    /// <summary>The raw bytes of the two tiles the tests use; neither is blank.</summary>
    private static byte[] RawOne => RawTile.FromPlanes(0x0F0F_0F0F_0F0F_0F0FUL, 0x00FF_00FF_00FF_00FFUL);

    private static byte[] RawTwo => RawTile.FromPlanes(0x1234_5678_9ABC_DEF0UL, 0x0FED_CBA9_8765_4321UL);

    /// <summary>
    /// A « 3dh » profile with a 2 × 1 pattern: the first cell carries <see cref="RawOne"/>, the
    /// second <see cref="RawTwo"/>, both pending until they are resolved.
    /// </summary>
    private static byte[] TwoCellProfile(
        VariantGeometry geometry = VariantGeometry.HorizontalCylinder,
        bool fullWarp = true,
        int textureKind = 1,
        VariantTexture? xorTexture = null)
    {
        TileId first = TileId.Compute(RawOne);
        TileId second = TileId.Compute(RawTwo);
        return SyntheticHashProfile.Build(
            manifest: [first, second],
            tileKeys: [first.ToText(), second.ToText()],
            textureKind: textureKind,
            xorTexture: xorTexture,
            geometry: geometry,
            fullWarp: fullWarp);
    }

    // REQ-PROF-081: a 2 × 1 pattern with one pending tile is drawn with its only resolved cell;
    // once resolved it is the same pattern as the one a 3dn carries.
    [Fact]
    public void A_pattern_with_one_pending_tile_keeps_its_resolved_cell_only()
    {
        ProfileDocument document = Load(TwoCellProfile());
        HashProfileTiles tiles = document.Tiles!;

        // Only the first tile is found, so the second cell is still waiting.
        Assert.Equal(1, tiles.ResolveFromFrameTile(
            RawTile.ToPlanes(RawOne).Low, RawTile.ToPlanes(RawOne).High));
        Assert.Equal(1, tiles.PendingCount);

        ConversionEngine engine = new();
        engine.LoadProfile(document.Profile, applySettings: false, tiles);

        EnginePattern pattern = engine.Patterns.Permanent[0];
        Assert.Equal(2, pattern.Width);
        Assert.Equal(1, pattern.Height);
        Assert.NotNull(pattern.ImageAt(0, 0));
        Assert.Null(pattern.ImageAt(1, 0));
        Assert.Single(pattern.Images);

        // REQ-PROF-081: the variant is built with the Default geometry, without fullWarp, over
        // the resolved cells only.
        Assert.Equal(Geo3DType.Default, pattern.Instances[0].Geometry);
        Assert.False(pattern.Instances[0].FullWarp);

        // The resolved cell alone recognises the pattern: one tile on screen is enough.
        ConversionFrame frame = new Batch()
            .Sprite(Key(RawOne), 80, 80)
            .Convert(engine, 1);
        Assert.Single(frame.Shapes);
        Assert.Single(frame.Shapes[0].Tiles);

        // Once the second tile is resolved, the pattern is the one the 3dn counterpart gives.
        Assert.Equal(1, tiles.ResolveFromFrameTile(
            RawTile.ToPlanes(RawTwo).Low, RawTile.ToPlanes(RawTwo).High));
        Assert.Equal(0, tiles.PendingCount);

        ConversionEngine plain = new();
        plain.LoadProfile(Counterpart(), applySettings: false);
        EnginePattern expected = plain.Patterns.Permanent[0];

        ConversionEngine resolved = new();
        resolved.LoadProfile(document.Profile, applySettings: false, tiles);
        EnginePattern actual = resolved.Patterns.Permanent[0];

        AssertSamePattern(expected, actual);
    }

    // REQ-PROF-080: a tile that appears on the frame being converted is resolved before the
    // recognition of that very frame, and what contains it is rebuilt before it is drawn.
    [Fact]
    public void A_tile_of_the_frame_is_resolved_before_the_recognition_of_that_frame()
    {
        ProfileDocument document = Load(TwoCellProfile(VariantGeometry.Cube, fullWarp: true));
        HashProfileTiles tiles = document.Tiles!;

        ConversionEngine engine = new();
        engine.LoadProfile(document.Profile, applySettings: false, tiles);

        // Both tiles are waiting, so the pattern holds no image and could recognise nothing;
        // its variant carries the Default geometry meanwhile (REQ-PROF-081).
        EnginePattern pattern = engine.Patterns.Permanent[0];
        Assert.Empty(pattern.Images);
        Assert.Equal(Geo3DType.Default, pattern.Instances[0].Geometry);

        // The two tiles arrive in the same frame: they are resolved, the pattern is rebuilt,
        // and the recognition of this frame already sees it.
        ConversionFrame frame = new Batch()
            .Sprite(Key(RawOne), 48, 56)
            .Sprite(Key(RawTwo), 56, 56)
            .Convert(engine, 2);

        Assert.Equal(0, tiles.PendingCount);
        Assert.Equal(2, engine.TilesResolvedWhilePlaying);
        Assert.Equal(2, pattern.Images.Count);
        Assert.Equal(Geo3DType.Cube, pattern.Instances[0].Geometry);
        Assert.True(pattern.Instances[0].FullWarp);
        Assert.Single(frame.Shapes);
        Assert.Equal(2, frame.Shapes[0].Tiles.Count);
    }

    // REQ-PROF-083, V2-34: the texture of a « textureKind = 2 » variant is the exclusive or of
    // what the file stores with the pattern's default texture, so a resolution rebuilds it as a
    // new object. The instance has to take that new object: a tile resolved while the game runs
    // must leave the pattern drawing exactly what an already resolved profile draws.
    [Fact]
    public void A_resolution_while_the_game_runs_gives_the_instance_its_rebuilt_texture()
    {
        VariantTexture stored = XorPayload();

        // The reference: everything resolved before the engine ever sees the profile, which is
        // the load-time path of REQ-PROF-078.
        ProfileDocument ahead = Load(TwoCellProfile(textureKind: 2, xorTexture: XorPayload()));
        ahead.Tiles!.PrepareForRom(ahead.Profile.Id, Rom([RawOne, RawTwo]), ResolvedTileCache.Disabled);
        Assert.Equal(0, ahead.Tiles.PendingCount);

        ConversionEngine reference = new();
        reference.LoadProfile(ahead.Profile, applySettings: false, ahead.Tiles);
        VariantTexture? expected = reference.Patterns.Permanent[0].Instances[0].Texture;
        Assert.NotNull(expected);

        // The same profile, loaded with nothing resolved: the second cell contributes zeros to
        // the default texture, so the texture the instance starts with is not the one above.
        ProfileDocument late = Load(TwoCellProfile(textureKind: 2, xorTexture: stored));
        ConversionEngine engine = new();
        engine.LoadProfile(late.Profile, applySettings: false, late.Tiles);

        PatternInstance instance = engine.Patterns.Permanent[0].Instances[0];
        Assert.NotNull(instance.Texture);
        Assert.NotEqual(expected!.Pixels!.Data, instance.Texture!.Pixels!.Data);

        // Both tiles turn up on the frame being converted: the variant's texture is rebuilt and
        // the instance must be holding the rebuilt one before the frame is drawn.
        new Batch()
            .Sprite(Key(RawOne), 48, 56)
            .Sprite(Key(RawTwo), 56, 56)
            .Convert(engine, 1);

        Assert.Equal(0, late.Tiles!.PendingCount);
        instance = engine.Patterns.Permanent[0].Instances[0];
        Assert.NotNull(instance.Texture);
        Assert.Equal(expected.Width, instance.Texture!.Width);
        Assert.Equal(expected.Height, instance.Texture.Height);
        Assert.Equal(expected.Pixels!.Format, instance.Texture.Pixels!.Format);
        Assert.Equal(expected.Pixels.Data, instance.Texture.Pixels.Data);
        Assert.Equal(expected.Pixels.Extra, instance.Texture.Pixels.Extra);
    }

    /// <summary>
    /// The stored field 24 of the test variant: an indexed 16 × 8 texture — the shape of the
    /// 2 × 1 pattern's default texture — whose eight words are all different from zero, so that
    /// the exclusive or of REQ-PROF-083 cannot give the same answer twice by accident.
    /// </summary>
    private static VariantTexture XorPayload()
    {
        int[] data = new int[8];
        int[] extra = new int[8];
        for (int word = 0; word < data.Length; word++)
        {
            data[word] = unchecked((int)(0x9E37_79B9U * (uint)(word + 1)));
            extra[word] = (word + 1) * 0x1111;
        }

        return new VariantTexture(16, 8, new TexturePixels(TextureFormat.Indexed, data, extra));
    }

    // REQ-PROF-080: the CHR-RAM is read in 16-byte aligned blocks, all four mirrorings tried.
    [Fact]
    public void The_chr_ram_resolves_a_tile_that_is_not_on_screen()
    {
        ProfileDocument document = Load(TwoCellProfile());
        HashProfileTiles tiles = document.Tiles!;

        ConversionEngine engine = new();
        engine.LoadProfile(document.Profile, applySettings: false, tiles);

        // The tile sits at an aligned position, mirrored vertically.
        byte[] ram = new byte[1024];
        RawTile.Flip(RawOne, TileFlip.Vertical, ram.AsSpan(0x200, 16));
        engine.ChrRamSource = new FakeChrRam(ram);

        new Batch().Sprite(new TileImageKey(0xAAUL, 0x55UL, false, false), 40, 40).Convert(engine, 1);

        Assert.Equal(1, tiles.PendingCount);
        Assert.Equal(1, engine.TilesResolvedWhilePlaying);
        Assert.Single(engine.Patterns.Permanent[0].Images);

        // The rule is stateless: a second frame over the same memory resolves nothing new and
        // puts nothing back to pending.
        new Batch().Sprite(new TileImageKey(0xAAUL, 0x55UL, false, false), 40, 40).Convert(engine, 2);
        Assert.Equal(1, tiles.PendingCount);
        Assert.Equal(1, engine.TilesResolvedWhilePlaying);
    }

    [Fact]
    public void An_unaligned_tile_in_the_chr_ram_is_not_resolved()
    {
        ProfileDocument document = Load(TwoCellProfile());
        HashProfileTiles tiles = document.Tiles!;

        ConversionEngine engine = new();
        engine.LoadProfile(document.Profile, applySettings: false, tiles);

        byte[] ram = new byte[1024];
        RawOne.CopyTo(ram, 0x201);
        engine.ChrRamSource = new FakeChrRam(ram);

        new Batch().Sprite(new TileImageKey(0xAAUL, 0x55UL, false, false), 40, 40).Convert(engine, 1);

        Assert.Equal(2, tiles.PendingCount);
    }

    // REQ-PROF-080: resolution is monotone. A tile that has been resolved never goes back to
    // pending, whatever happens to the emulation afterwards.
    [Fact]
    public void A_resolved_tile_never_goes_back_to_pending()
    {
        ProfileDocument document = Load(TwoCellProfile());
        HashProfileTiles tiles = document.Tiles!;

        ConversionEngine engine = new();
        engine.LoadProfile(document.Profile, applySettings: false, tiles);

        new Batch()
            .Sprite(Key(RawOne), 48, 56)
            .Convert(engine, 1);
        Assert.Equal(1, tiles.PendingCount);

        // What a rollback, a run-ahead, a state load and a reset do to the engine: the frames go,
        // what was learned may go, the resolved set stays.
        engine.ClearFrames();
        engine.Patterns.ClearLearned();
        engine.Restart();

        Assert.Equal(1, tiles.PendingCount);
        Assert.False(document.Profile.PermanentPatterns[0].Grid![0, 0]!.IsPending);
    }

    // REQ-PROF-082: the cache resolves everything before the first frame, an unreadable cache is
    // ignored, and the profile file is not touched.
    [Fact]
    public void The_cache_gives_back_what_a_previous_session_resolved()
    {
        string directory = Path.Combine(
            Path.GetTempPath(), "zax-tiles-" + Guid.NewGuid().ToString("N"));
        try
        {
            byte[] bytes = TwoCellProfile();
            ResolvedTileCache cache = new(directory);

            // First session: both tiles found, then kept.
            ProfileDocument first = Load(bytes);
            first.Tiles!.ResolveFromFrameTile(RawTile.ToPlanes(RawOne).Low, RawTile.ToPlanes(RawOne).High);
            first.Tiles.ResolveFromFrameTile(RawTile.ToPlanes(RawTwo).Low, RawTile.ToPlanes(RawTwo).High);
            cache.Save(first.Profile.Id, first.Tiles);

            // Second session, with a ROM that holds none of its tiles: the cache resolves
            // everything before the first frame.
            ProfileDocument second = Load(bytes);
            second.Tiles!.PrepareForRom(second.Profile.Id, Rom([]), cache);
            Assert.Equal(0, second.Tiles.PendingCount);

            // A truncated cache is ignored without an error.
            string file = cache.FileOf(second.Profile.Id!)!;
            byte[] kept = File.ReadAllBytes(file);
            File.WriteAllBytes(file, kept[..(kept.Length / 2)]);
            ProfileDocument truncated = Load(bytes);
            truncated.Tiles!.PrepareForRom(truncated.Profile.Id, Rom([]), cache);
            Assert.True(truncated.Tiles.PendingCount > 0);

            // So is a file of arbitrary content.
            File.WriteAllBytes(file, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
            ProfileDocument arbitrary = Load(bytes);
            arbitrary.Tiles!.PrepareForRom(arbitrary.Profile.Id, Rom([]), cache);
            Assert.Equal(2, arbitrary.Tiles.PendingCount);

            // And the disabled cache keeps nothing at all (determinism tests).
            Assert.Null(ResolvedTileCache.Disabled.FileOf("anything"));
            Assert.Equal(0, ResolvedTileCache.Disabled.Apply("anything", arbitrary.Tiles));

            // The bytes of the profile never change: the cache has no bearing on the format.
            Assert.Equal(bytes, TwoCellProfile());
        }
        finally
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, recursive: true);
            }
        }
    }

    [Fact]
    public void An_entry_whose_string_does_not_recompute_is_ignored()
    {
        string directory = Path.Combine(
            Path.GetTempPath(), "zax-tiles-" + Guid.NewGuid().ToString("N"));
        try
        {
            ResolvedTileCache cache = new(directory);
            ProfileDocument first = Load(TwoCellProfile());

            // A hand-made cache whose entry names one tile but carries the bytes of the other.
            first.Tiles!.ResolveFromFrameTile(RawTile.ToPlanes(RawOne).Low, RawTile.ToPlanes(RawOne).High);
            cache.Save(first.Profile.Id, first.Tiles);
            string file = cache.FileOf(first.Profile.Id!)!;
            byte[] kept = File.ReadAllBytes(file);
            RawTwo.CopyTo(kept, kept.Length - 16);
            File.WriteAllBytes(file, kept);

            ProfileDocument second = Load(TwoCellProfile());
            Assert.Equal(0, cache.Apply(second.Profile.Id, second.Tiles!));
            Assert.Equal(2, second.Tiles!.PendingCount);
        }
        finally
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, recursive: true);
            }
        }
    }

    /// <summary>The image key of a raw tile, not mirrored.</summary>
    private static TileImageKey Key(byte[] raw)
    {
        (ulong low, ulong high) = RawTile.ToPlanes(raw);
        return new TileImageKey(low, high, false, false);
    }

    /// <summary>The 3dn equivalent of <see cref="TwoCellProfile"/>, built in memory.</summary>
    private static GameProfile Counterpart()
    {
        (ulong lowOne, ulong highOne) = RawTile.ToPlanes(RawOne);
        (ulong lowTwo, ulong highTwo) = RawTile.ToPlanes(RawTwo);
        ProfilePattern pattern = new()
        {
            TileSize = new Vec2I(2, 1),
            ConstraintSize = new Vec2I(99, 99),
            Permanent = true,
            SelfConnected = true,
            Grid = new PatternGrid(2, 1,
            [
                new ProfileTile(lowOne, highOne, TileFlip.None),
                new ProfileTile(lowTwo, highTwo, TileFlip.None),
            ]),
        };
        pattern.Variants.Add(new PatternVariant
        {
            Depth = 3,
            Geometry = VariantGeometry.HorizontalCylinder,
            FullWarp = true,
            Alpha = 1f,
            Texture = VariantTexture.NullMarker(),
        });

        GameProfile profile = GameProfile.CreateEmpty();
        profile.PermanentPatterns.Add(pattern);
        return profile;
    }

    private static void AssertSamePattern(EnginePattern expected, EnginePattern actual)
    {
        Assert.Equal(expected.Width, actual.Width);
        Assert.Equal(expected.Height, actual.Height);
        Assert.Equal(expected.Images.Count, actual.Images.Count);
        for (int j = 0; j < expected.Height; j++)
        {
            for (int i = 0; i < expected.Width; i++)
            {
                TileImage? left = expected.ImageAt(i, j);
                TileImage? right = actual.ImageAt(i, j);
                Assert.Equal(left is null, right is null);
                if (left is not null && right is not null)
                {
                    Assert.Equal(left.Key, right.Key);
                }
            }
        }

        Assert.Equal(expected.PixelStart, actual.PixelStart);
        Assert.Equal(expected.PixelEnd, actual.PixelEnd);
        Assert.Equal(expected.HorizontalPairCount, actual.HorizontalPairCount);
        Assert.Equal(expected.VerticalPairCount, actual.VerticalPairCount);
        Assert.Equal(expected.Instances.Count, actual.Instances.Count);
        for (int index = 0; index < expected.Instances.Count; index++)
        {
            Assert.Equal(expected.Instances[index].Geometry, actual.Instances[index].Geometry);
            Assert.Equal(expected.Instances[index].FullWarp, actual.Instances[index].FullWarp);
            Assert.Equal(expected.Instances[index].Depth, actual.Instances[index].Depth);
        }
    }

    /// <summary>A minimal iNES file whose CHR zone holds the given raw tiles.</summary>
    private static byte[] Rom(IEnumerable<byte[]> tiles)
    {
        List<byte[]> list = [.. tiles];
        byte[] file = new byte[16 + (16 * 1024) + (8 * 1024)];
        file[0] = 0x4E;
        file[1] = 0x45;
        file[2] = 0x53;
        file[3] = 0x1A;
        file[4] = 1;
        file[5] = 1;
        for (int index = 0; index < list.Count; index++)
        {
            list[index].CopyTo(file, 16 + (16 * 1024) + 8 + (index * 32));
        }

        return file;
    }

    private static ProfileDocument Load(byte[] bytes)
    {
        ProfileLoadResult result = ProfileFormat.Load(bytes);
        Assert.True(result.IsSuccess, result.Message);
        return result.Document!;
    }

    private sealed class FakeChrRam(byte[] memory) : IChrRamSource
    {
        public int ChrRamSize => memory.Length;

        public ReadOnlySpan<byte> ChrRam => memory;
    }
}
