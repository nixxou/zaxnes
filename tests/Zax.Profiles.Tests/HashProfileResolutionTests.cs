using Zax.Profiles.Hash;

namespace Zax.Profiles.Tests;

/// <summary>
/// Resolving the tiles of a « 3dh » profile from the ROM and rebuilding the textures of its
/// variants (AMD-062 REQ-PROF-078, REQ-PROF-079, REQ-PROF-083).
/// </summary>
public sealed class HashProfileResolutionTests
{
    /// <summary>
    /// The two profiles whose patterns were added in 3dSen 2.0 and that therefore do not pair
    /// with their 1.x counterpart (AMD-062 §1).
    /// </summary>
    private static readonly string[] NotPairable = ["DQ3_DW3.bytes", "Final_Fantasy_USA.bytes"];

    // REQ-PROF-078: for each of the 99 pairable profiles, a synthetic iNES file whose CHR zone
    // holds the raw bytes of each tile of the 3dn counterpart, at positions that are not
    // aligned on sixteen, resolves the whole manifest; the same file without one tile leaves
    // that tile alone pending; a tile placed mirrored is resolved as well (REQ-PROF-079).
    [HashAssetFact]
    public void A_synthetic_rom_resolves_the_whole_manifest_of_every_pairable_profile()
    {
        int checked_ = 0;
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            if (NotPairable.Contains(name))
            {
                continue;
            }

            List<byte[]> raw = CounterpartTiles(name);
            byte[] rom = SyntheticRom.WithTiles(raw);

            HashProfileTiles tiles = Load(file).Tiles!;
            tiles.ResolveFromRom(rom);
            Assert.Equal(0, tiles.PendingCount);
            checked_++;
        }

        Assert.Equal(99, checked_);
    }

    [HashAssetFact]
    public void A_mirrored_tile_in_the_rom_is_resolved_as_well()
    {
        int checked_ = 0;
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            if (NotPairable.Contains(name))
            {
                continue;
            }

            byte[] rom = SyntheticRom.WithTiles(CounterpartTiles(name), TileFlip.Horizontal);

            HashProfileTiles tiles = Load(file).Tiles!;
            tiles.ResolveFromRom(rom);
            Assert.Equal(0, tiles.PendingCount);
            checked_++;
        }

        Assert.Equal(99, checked_);
    }

    [HashAssetFact]
    public void A_rom_without_one_tile_leaves_that_tile_alone_pending()
    {
        int checked_ = 0;
        int skipped = 0;
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            if (NotPairable.Contains(name))
            {
                continue;
            }

            List<byte[]> raw = CounterpartTiles(name);

            // The tile taken out has to be one the file cannot give back another way: not the
            // all-zero tile, which the zeros of the file hold anyway (AMD-062 §9, divergence
            // (D)), and not a tile that is the mirror of another one, which the four mirrorings
            // of REQ-PROF-079 would resolve from its twin.
            int removed = FirstUnmirroredTile(raw);
            if (removed < 0)
            {
                // empty.bytes is the one profile with no tile at all.
                Assert.Equal(0, Load(file).Tiles!.TotalCount);
                skipped++;
                continue;
            }

            byte[] missing = raw[removed];
            List<byte[]> kept = [.. raw];
            kept.RemoveAt(removed);

            HashProfileTiles tiles = Load(file).Tiles!;
            tiles.ResolveFromRom(SyntheticRom.WithTiles(kept));

            string key = TileId.Compute(missing).ToText();
            Assert.Equal<string>([key], tiles.PendingKeys);
            checked_++;
        }

        Assert.Equal(98, checked_);
        Assert.Equal(1, skipped);
    }

    [HashAssetFact]
    public void A_rom_that_is_not_iNES_resolves_nothing()
    {
        HashProfileTiles tiles = LoadByName("BalloonFight.bytes").Tiles!;
        int before = tiles.PendingCount;

        Assert.Equal(0, tiles.ResolveFromRom(new byte[4096]));
        Assert.Equal(before, tiles.PendingCount);
        Assert.Equal(330, before);
    }

    // REQ-PROF-078, optional criterion on the user's own dumps: Super Mario Bros 481 out of
    // 481, Legend Of Zelda 1 035 out of 1 035, Mega Man 2 5 353 out of 5 497 — the other 144
    // tiles of Mega Man 2 wait for the game (REQ-PROF-080).
    [HashRomFact]
    public void The_three_dumps_resolve_the_counts_of_the_criterion()
    {
        (string Profile, string Rom, int Resolved, int Total)[] cases =
        [
            ("Super Mario Bros.bytes", "Super Mario Bros. (World).nes", 481, 481),
            ("Legend Of Zelda.bytes", "Legend of Zelda, The (USA).nes", 1_035, 1_035),
            ("Mega Man 2.bytes", "Mega Man 2 (USA).nes", 5_353, 5_497),
        ];

        foreach ((string profile, string rom, int resolved, int total) in cases)
        {
            HashProfileTiles tiles = LoadByName(profile).Tiles!;
            Assert.Equal(total, tiles.TotalCount);

            tiles.ResolveFromRom(File.ReadAllBytes(UserRoms.Path(rom)!));
            Assert.Equal(resolved, tiles.ResolvedCount);
        }
    }

    // REQ-PROF-078 step 1: the trainer is skipped and the NES 2.0 sizes, exponent-multiplier
    // notation included, place the CHR zone where it really is.
    [Fact]
    public void The_zones_follow_the_header()
    {
        byte[] plain = new byte[16 + (2 * 16 * 1024) + (3 * 8 * 1024)];
        WriteSignature(plain);
        plain[4] = 2;
        plain[5] = 3;
        RomTileZones zones = RomTileZones.Of(plain);
        Assert.Equal(new RomTileZones(16, 2 * 16 * 1024, 16 + (2 * 16 * 1024), 3 * 8 * 1024), zones);

        byte[] withTrainer = new byte[16 + 512 + (16 * 1024) + (8 * 1024)];
        WriteSignature(withTrainer);
        withTrainer[4] = 1;
        withTrainer[5] = 1;
        withTrainer[6] = 0x04;
        Assert.Equal(
            new RomTileZones(16 + 512, 16 * 1024, 16 + 512 + (16 * 1024), 8 * 1024),
            RomTileZones.Of(withTrainer));

        // NES 2.0, exponent-multiplier: byte 5 = 0b000010_01 is (2 × 1 + 1) × 2^2 = 12 bytes.
        byte[] nes20 = new byte[16 + (16 * 1024) + 12];
        WriteSignature(nes20);
        nes20[4] = 1;
        nes20[5] = 0b0000_1001;
        nes20[7] = 0x08;
        nes20[9] = 0xF0;
        RomTileZones exponent = RomTileZones.Of(nes20);
        Assert.Equal(16 * 1024, exponent.PrgLength);
        Assert.Equal(12, exponent.ChrLength);

        // A zone is truncated to the length of the file.
        byte[] truncated = new byte[16 + 100];
        WriteSignature(truncated);
        truncated[4] = 1;
        truncated[5] = 1;
        RomTileZones cut = RomTileZones.Of(truncated);
        Assert.Equal(100, cut.PrgLength);
        Assert.Equal(0, cut.ChrLength);

        Assert.True(RomTileZones.Of(new byte[32]).IsEmpty);
    }

    // REQ-PROF-083: on the 99 pairable profiles, once the tiles are resolved from the tiles of
    // the 3dn counterpart, the k-th variant has the texture of the k-th variant of the 3dn.
    [HashAssetFact]
    public void The_reconstructed_textures_match_the_3dn_counterparts()
    {
        int nullMarkers = 0;
        int format0SameSize = 0;
        int format0OtherSize = 0;
        int format1 = 0;

        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            if (NotPairable.Contains(name))
            {
                continue;
            }

            ProfileDocument document = Load(file);
            ProfileDocument counterpart = LoadCounterpart(name);

            // The tiles come straight from the counterpart's tiles, not from a ROM.
            HashProfileTiles tiles = document.Tiles!;
            foreach (byte[] raw in SyntheticRom.DistinctRawTiles(counterpart.Profile))
            {
                (ulong low, ulong high) = RawTile.ToPlanes(raw);
                tiles.ResolveFromFrameTile(low, high);
            }

            Assert.Equal(0, tiles.PendingCount);
            tiles.RebuildAllTextures();

            List<(ProfilePattern Pattern, PatternVariant Variant)> mine =
                [.. HashProfileReaderTests.AllPatternVariants(document.Profile, includeSlots: false)];
            List<PatternVariant> theirs =
                [.. HashProfileReaderTests.AllVariants(counterpart.Profile, includeSlots: false)];
            Assert.Equal(theirs.Count, mine.Count);

            for (int k = 0; k < mine.Count; k++)
            {
                (ProfilePattern pattern, PatternVariant variant) = mine[k];
                VariantTexture? expected = theirs[k].Texture;
                VariantTexture? actual = variant.Texture;
                if (variant.TextureKind == 1)
                {
                    Assert.True(
                        expected is null || expected.IsNullMarker,
                        $"{name} variant {k}: the counterpart should use the default texture.");
                    Assert.True(actual is { IsNullMarker: true });
                    nullMarkers++;
                    continue;
                }

                Assert.Equal(2, variant.TextureKind);
                Assert.NotNull(expected);
                Assert.NotNull(actual);
                AssertSameTexture(expected!, actual!, $"{name} variant {k}");

                if (expected!.Pixels?.Format == TextureFormat.Direct)
                {
                    format1++;
                }
                else if (expected.Width == PatternDefaultTexture.CellsX(pattern) * 8
                    && expected.Height == PatternDefaultTexture.CellsY(pattern) * 8)
                {
                    format0SameSize++;
                }
                else
                {
                    format0OtherSize++;
                }
            }
        }

        Assert.Equal(55_575, nullMarkers);
        Assert.Equal(12_418, format0SameSize);
        Assert.Equal(4, format0OtherSize);
        Assert.Equal(130, format1);
        Assert.Equal(68_127, nullMarkers + format0SameSize + format0OtherSize + format1);
    }

    // REQ-PROF-083: the three kinds, on a synthetic profile.
    [Fact]
    public void The_three_texture_kinds_are_reconstructed()
    {
        TileId id = TileId.Compute(RawTile.FromPlanes(0xFFUL, 0UL));

        ProfileDocument none = LoadBytes(SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], textureKind: 0));
        Assert.Null(none.Profile.PermanentPatterns[0].Variants[0].Texture);

        ProfileDocument defaulted = LoadBytes(SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], textureKind: 1));
        Assert.True(defaulted.Profile.PermanentPatterns[0].Variants[0].Texture is { IsNullMarker: true });

        // textureKind 2: the stored words are the wanted texture XOR the default one, so a
        // stored zero gives the default texture back.
        VariantTexture stored = new(8, 8, new TexturePixels(
            TextureFormat.Indexed, new int[4], new int[4]));
        ProfileDocument xor = LoadBytes(SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], textureKind: 2, xorTexture: stored));
        HashProfileTiles tiles = xor.Tiles!;
        Assert.Equal(1, tiles.PendingCount);

        tiles.ResolveFromFrameTile(0xFFUL, 0UL);
        Assert.Equal(0, tiles.PendingCount);
        tiles.RebuildAllTextures();

        // A one-cell pattern: its default texture is 8 × 8, four words of data, and the top row
        // is colour 1 because the low plane of the tile is 0xFF — 0x5555 in the first word.
        ProfilePattern pattern = xor.Profile.PermanentPatterns[0];
        VariantTexture expectedDefault = PatternDefaultTexture.Build(pattern);
        Assert.Equal(8, expectedDefault.Width);
        Assert.Equal(8, expectedDefault.Height);
        Assert.Equal<int>([0x5555, 0, 0, 0], [.. expectedDefault.Pixels!.Data]);

        AssertSameTexture(expectedDefault, pattern.Variants[0].Texture!, "synthetic xor");
    }

    // REQ-PROF-083: while a tile waits and the shape of field 24 differs from the default
    // texture, the variant uses the default texture.
    [Fact]
    public void A_pending_tile_with_a_differently_shaped_xor_texture_falls_back_on_the_default()
    {
        TileId id = TileId.Compute(RawTile.FromPlanes(0xFFUL, 0UL));
        VariantTexture stored = new(4, 4, new TexturePixels(TextureFormat.Direct, new int[16]));

        ProfileDocument document = LoadBytes(SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], textureKind: 2, xorTexture: stored));
        PatternVariant variant = document.Profile.PermanentPatterns[0].Variants[0];

        Assert.True(variant.Texture is { IsNullMarker: true });

        document.Tiles!.ResolveFromFrameTile(0xFFUL, 0UL);
        document.Tiles.RebuildAllTextures();

        Assert.False(variant.Texture!.IsNullMarker);
        Assert.Equal(4, variant.Texture.Width);
        Assert.Equal(TextureFormat.Direct, variant.Texture.Pixels!.Format);
    }

    // REQ-PROF-076: a tile carried by its legacy digest resolves like one carried by its
    // identifier, and the two are distinct while they wait (REQ-PROF-081).
    [Fact]
    public void A_tile_carried_by_its_legacy_digest_resolves_too()
    {
        byte[] raw = RawTile.FromPlanes(0x1234_5678UL, 0xABCDUL);
        TileId id = TileId.Compute(raw);
        string digest = RawTile.LegacyDigest(raw);

        ProfileDocument document = LoadBytes(SyntheticHashProfile.Build(
            manifest: [id],
            hashes: [digest],
            cells: [new SyntheticCell(id.ToText()), new SyntheticCell(digest, AsIdentifier: false)]));

        PatternGrid grid = document.Profile.PermanentPatterns[0].Grid!;
        ProfileTile byId = grid[0, 0]!;
        ProfileTile byDigest = grid[1, 0]!;

        Assert.True(byId.IsPending);
        Assert.True(byDigest.IsPending);
        Assert.NotEqual(byId, byDigest);

        Assert.Equal(2, document.Tiles!.TotalCount);
        document.Tiles.ResolveFromFrameTile(0x1234_5678UL, 0xABCDUL);

        Assert.Equal(0, document.Tiles.PendingCount);
        Assert.False(byId.IsPending);
        Assert.False(byDigest.IsPending);
        Assert.Equal(byId, byDigest);
    }

    // REQ-PROF-082: an entry whose string does not recompute from its bytes is ignored.
    [Fact]
    public void Only_bytes_that_recompute_their_string_are_applied()
    {
        byte[] raw = RawTile.FromPlanes(0x99UL, 0UL);
        TileId id = TileId.Compute(raw);
        ProfileDocument document = LoadBytes(SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()]));
        HashProfileTiles tiles = document.Tiles!;

        Assert.False(tiles.Apply(id.ToText(), RawTile.FromPlanes(0x98UL, 0UL)));
        Assert.False(tiles.Apply(new string('0', 32), raw));
        Assert.False(tiles.Apply(id.ToText(), new byte[15]));
        Assert.Equal(1, tiles.PendingCount);

        Assert.True(tiles.Apply(id.ToText(), raw));
        Assert.Equal(0, tiles.PendingCount);
        Assert.False(tiles.Apply(id.ToText(), raw));
    }

    internal static void AssertSameTexture(VariantTexture expected, VariantTexture actual, string what)
    {
        Assert.Equal(expected.Width, actual.Width);
        Assert.Equal(expected.Height, actual.Height);
        TexturePixels expectedPixels = expected.Pixels ?? new TexturePixels(TextureFormat.Indexed);
        TexturePixels actualPixels = actual.Pixels ?? new TexturePixels(TextureFormat.Indexed);
        Assert.Equal(expectedPixels.Format, actualPixels.Format);
        Assert.Equal(expectedPixels.Data, actualPixels.Data);

        // The canonical 3dn writer leaves out an all-zero extra plane, so a shorter array means
        // zeros (SPEC-03 REQ-PROF-035).
        int length = Math.Max(expectedPixels.Extra.Count, actualPixels.Extra.Count);
        for (int i = 0; i < length; i++)
        {
            int left = i < expectedPixels.Extra.Count ? expectedPixels.Extra[i] : 0;
            int right = i < actualPixels.Extra.Count ? actualPixels.Extra[i] : 0;
            Assert.True(left == right, $"{what}: extra[{i}] is {right}, expected {left}.");
        }
    }

    /// <summary>
    /// The first tile that draws something and whose four mirrorings match no other tile of the
    /// list, or -1.
    /// </summary>
    private static int FirstUnmirroredTile(List<byte[]> tiles)
    {
        HashSet<string> contents = [];
        foreach (byte[] tile in tiles)
        {
            contents.Add(Convert.ToHexStringLower(tile));
        }

        for (int index = 0; index < tiles.Count; index++)
        {
            if (RawTile.IsBlank(tiles[index]))
            {
                continue;
            }

            bool mirrored = false;
            for (int flip = 1; flip < 4 && !mirrored; flip++)
            {
                string candidate = Convert.ToHexStringLower(RawTile.Flipped(tiles[index], (TileFlip)flip));
                mirrored = !string.Equals(candidate, Convert.ToHexStringLower(tiles[index]), StringComparison.Ordinal)
                    && contents.Contains(candidate);
            }

            if (!mirrored)
            {
                return index;
            }
        }

        return -1;
    }

    private static List<byte[]> CounterpartTiles(string hashFileName)
        => SyntheticRom.DistinctRawTiles(LoadCounterpart(hashFileName).Profile);

    private static ProfileDocument LoadCounterpart(string hashFileName)
    {
        string path = Path.Combine(
            NeutralAssets.ProfilesDirectory!, NeutralAssets.CounterpartName(hashFileName));
        return LoadBytes(File.ReadAllBytes(path), Path.GetFileName(path));
    }

    private static ProfileDocument Load(string path) => LoadBytes(File.ReadAllBytes(path), path);

    private static ProfileDocument LoadByName(string fileName)
        => LoadBytes(NeutralAssets.ReadHashProfile(fileName), fileName);

    private static ProfileDocument LoadBytes(byte[] bytes, string? name = null)
    {
        ProfileLoadResult result = ProfileFormat.Load(bytes, sourceName: name);
        Assert.True(result.IsSuccess, $"{name}: {result.Message}");
        return result.Document!;
    }

    private static void WriteSignature(byte[] file)
    {
        file[0] = 0x4E;
        file[1] = 0x45;
        file[2] = 0x53;
        file[3] = 0x1A;
    }
}
