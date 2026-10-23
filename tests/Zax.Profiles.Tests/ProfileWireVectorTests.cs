using Zax.Profiles.Wire;

namespace Zax.Profiles.Tests;

/// <summary>
/// The test vectors of SPEC-03 §5: the small messages of §5.1, the plane encoding of §5.2 and
/// the decoded extracts of §5.3 and §5.4.
/// </summary>
public sealed class ProfileWireVectorTests
{
    [Fact]
    public void REQ_PROF_002_scalars_use_the_stated_encoding()
    {
        // -1 as an int32 is sign-extended to 64 bits, ten bytes, no zig-zag; 1.0f is four
        // little-endian bytes.
        GameProfile profile = new()
        {
            Settings = new ProfileSettings
            {
                CustomBackgroundColor = new ArgbColor(-1),
                Volume = 1f,
            },
        };

        byte[] bytes = ProfileFormat.Save(profile);
        Assert.Contains("ff ff ff ff ff ff ff ff ff 01", Hex(bytes));
        Assert.Contains("00 00 80 3f", Hex(bytes));
    }

    [Fact]
    public void REQ_PROF_020_every_profile_starts_with_the_signature_and_the_version()
    {
        GameProfile profile = new() { Signature = "3dn", VersionText = "2" };
        byte[] bytes = ProfileFormat.Save(profile);
        Assert.Equal("0a 03 33 64 6e 12 01 32", Hex(bytes));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_020_the_built_in_profiles_all_start_with_the_signature()
    {
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            byte[] bytes = File.ReadAllBytes(path);
            Assert.Equal("0a 03 33 64 6e", Hex(bytes.AsSpan(0, 5)));
        }
    }

    [Fact]
    public void REQ_PROF_038_a_script_is_written_as_the_stated_bytes()
    {
        GameProfile profile = new();
        profile.Scripts.Add(new ProfileScript("Build", enabled: true, source: null, tagString: null));

        Assert.Contains("0a 05 42 75 69 6c 64 10 01 18 00", Hex(ProfileFormat.Save(profile)));
    }

    [Fact]
    public void REQ_PROF_002_a_vector_of_ones_in_field_8_is_written_as_the_stated_bytes()
    {
        GameProfile profile = new();
        ProfilePattern pattern = new();
        pattern.Variants.Add(new PatternVariant { Scale = new Vec3F(1f, 1f, 1f) });
        profile.PermanentPatterns.Add(pattern);

        Assert.Contains(
            "42 0f 0d 00 00 80 3f 15 00 00 80 3f 1d 00 00 80 3f",
            Hex(ProfileFormat.Save(profile)));
    }

    [Fact]
    public void REQ_PROF_012_an_empty_cell_is_an_empty_group()
    {
        GameProfile profile = new();
        ProfilePattern pattern = new() { TileSize = new Vec2I(1, 1) };
        pattern.Grid = new PatternGrid(1, 1, [null]);
        pattern.Variants.Add(new PatternVariant());
        profile.PermanentPatterns.Add(pattern);

        Assert.Contains("0b 0c", Hex(ProfileFormat.Save(profile)));
    }

    [Fact]
    public void REQ_PROF_011_the_twelfth_key_goes_to_the_twelfth_new_object()
    {
        // One pattern (key 1), then eleven new tiles (keys 2 to 12), then a cell that names the
        // tile of key 12 again. That reproduces both cell vectors of §5.1 and shows the keys
        // are handed out in prefix order over the whole file.
        ProfileTile shared = new(0x0020201800000000UL, 0x103A3C1800000000UL, TileFlip.None);
        List<ProfileTile?> cells = [];
        for (int i = 0; i < 10; i++)
        {
            cells.Add(new ProfileTile((ulong)(i + 1), 0UL, TileFlip.None));
        }

        cells.Add(shared);
        cells.Add(shared);

        GameProfile profile = new();
        ProfilePattern pattern = new() { TileSize = new Vec2I(12, 1) };
        pattern.Grid = new PatternGrid(12, 1, cells);
        pattern.Variants.Add(new PatternVariant());
        profile.PermanentPatterns.Add(pattern);

        string hex = Hex(ProfileFormat.Save(profile));

        // New tile, key 12, the planes of §5.3.
        Assert.Contains(
            "0b 0a 1c 10 0c 52 18 0a 16 08 b3 f1 a8 ce 80 d0 a5 89 62 "
            + "10 b3 f1 b4 fa c0 d0 a5 89 62 18 00 0c",
            hex);

        // The cell that names the existing key 12.
        Assert.Contains("0b 0a 02 08 0c 0c", hex);
    }

    [Fact]
    public void REQ_PROF_013_the_null_texture_marker_is_written_payload_first()
    {
        GameProfile profile = new();
        ProfilePattern pattern = new();
        pattern.Variants.Add(new PatternVariant { Texture = VariantTexture.NullMarker() });
        profile.PermanentPatterns.Add(pattern);

        Assert.Contains("9a 01 08 52 02 10 00 08 00 10 00", Hex(ProfileFormat.Save(profile)));
    }

    [Fact]
    public void REQ_PROF_002_an_opaque_black_colour_is_written_as_the_stated_varint()
    {
        GameProfile profile = new()
        {
            Settings = new ProfileSettings { CustomBackgroundColor = new ArgbColor(-16777216) },
        };

        Assert.Contains("80 80 80 f8 ff ff ff ff ff 01", Hex(ProfileFormat.Save(profile)));
    }

    [Theory]
    [InlineData(0x0000000000000000UL, 0x6212968001C23EB3UL, 7066876241827217075L)]
    [InlineData(0xFFFFFFFFFFFFFFFFUL, 0x9DED697FFE3DC14CUL, -7066876241827217076L)]
    [InlineData(0x0000000000000001UL, 0x621296C001C23EB3UL, 7066876516705124019L)]
    [InlineData(0x8000000000000000UL, 0x621296A001C23EB3UL, 7066876379266170547L)]
    [InlineData(0x00000000000000FFUL, 0x6212A94001C23EB3UL, 7066896857670237875L)]
    [InlineData(0x0102040810204080UL, 0x6A02B68041433CB7UL, 7638868579940777143L)]
    [InlineData(0x071F3F7F7FFFFFFFUL, 0x9DED6941C60DE16CUL, -7066876509057851028L)]
    [InlineData(0xC0F0F8FCFCFEFEFEUL, 0x5DAD29303DFC018CUL, 6750096703673336204L)]
    [InlineData(0xC3818100008181C3UL, 0x4272E670E1A27EB3UL, 4788142726344507059L)]
    public void REQ_PROF_030_the_plane_encoding_matches_the_test_vectors(
        ulong plane,
        ulong storedUnsigned,
        long storedSigned)
    {
        Assert.Equal(0x000708FACD884A5AUL, TilePlaneCipher.Key);
        Assert.Equal(26, TilePlaneCipher.Rotation);

        long stored = TilePlaneCipher.Encode(plane);
        Assert.Equal(storedSigned, stored);
        Assert.Equal(storedUnsigned, (ulong)stored);
        Assert.Equal(plane, TilePlaneCipher.Decode(stored));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_010_the_first_pattern_envelope_of_balloon_fight_is_the_stated_bytes()
    {
        byte[] bytes = NeutralAssets.ReadProfile("BalloonFight.bytes");

        // Root field 4, length 611, then: new object of key 1, then field 10 of length 606.
        Assert.Contains("22 e3 04 10 01 52 de 04", Hex(bytes));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_031_the_tile_of_key_12_of_balloon_fight_draws_the_expected_pattern()
    {
        GameProfile profile = NeutralAssets.LoadProfile("BalloonFight.bytes").Profile;

        Assert.Equal("3dn", profile.Signature);
        Assert.Equal("2", profile.VersionText);
        Assert.Equal("Lc89vRjlYrUL", profile.Id);
        Assert.Null(profile.Title);
        Assert.Empty(profile.Scripts);
        Assert.Equal(237, profile.PermanentPatterns.Count);
        Assert.Equal(2, profile.TemporaryPatterns.Count);

        ProfilePattern first = profile.PermanentPatterns[0];
        Assert.Equal(new Vec2I(8, 1), first.TileSize);
        Assert.Equal(8, first.Grid!.FilledCellCount);

        ProfilePattern second = profile.PermanentPatterns[1];
        Assert.Equal(Vec2I.Unconstrained, second.ConstraintSize);
        Assert.Equal(new Vec2I(1, 1), second.TileSize);
        Assert.True(second.Permanent);
        Assert.Equal(0, second.PixelCount);
        Assert.Equal(PatternColorFlags.Color2 | PatternColorFlags.Color3, second.ColorFlags);
        Assert.Equal(PatternDensity.Solid, second.TileDensity);
        Assert.Equal(PatternDensity.Low, second.PixelDensity);
        Assert.False(second.SelfConnected);
        Assert.False(second.SkipGeometry);

        ProfileTile tile = second.Grid!.Cells[0]!;
        Assert.Equal(0x0020201800000000UL, tile.LowPlane);
        Assert.Equal(0x103A3C1800000000UL, tile.HighPlane);
        Assert.Equal(TileFlip.None, tile.Flip);
        Assert.Equal(7066876241961957555L, TilePlaneCipher.Encode(tile.LowPlane));
        Assert.Equal(7066876259234298035L, TilePlaneCipher.Encode(tile.HighPlane));

        string[] expected =
        [
            "00000000",
            "00000000",
            "00000000",
            "00000000",
            "00033000",
            "00322200",
            "00322020",
            "00020000",
        ];

        for (int y = 0; y < 8; y++)
        {
            char[] row = new char[8];
            for (int x = 0; x < 8; x++)
            {
                row[x] = (char)('0' + tile.ColorAt(x, y));
            }

            Assert.Equal(expected[y], new string(row));
        }

        PatternVariant variant = Assert.Single(second.Variants);
        Assert.Equal(3, variant.Depth);
        Assert.Equal(VariantGeometry.Default, variant.Geometry);
        Assert.Equal(ProfileLayer.Unidentified, variant.Layer);
        Assert.Equal(new Vec3F(1f, 1f, 1f), variant.Scale);
        Assert.Equal(Vec3F.Zero, variant.Rotation);
        Assert.Equal(Vec3F.Zero, variant.Offset);
        Assert.Equal(new Vec3F(0.5f, 0.5f, 0.5f), variant.Pivot);
        Assert.Equal(1f, variant.Alpha);
        Assert.Equal(MeshAlgorithm.FromSettings, variant.MeshAlgorithm);
        Assert.Equal(Vec3F.Zero, variant.DeformAmplitude);
        Assert.Equal(new Vec3F(0.5f, 0.5f, 0.5f), variant.DeformPivot);
        Assert.Equal(VariantRenderStyle.Normal, variant.RenderStyle);
        Assert.True(variant.Texture!.IsNullMarker);
        Assert.Equal(string.Empty, variant.TagString);
        Assert.Equal(0, variant.StoredEmptyColor);

        float quarterTurn = BitConverter.Int32BitsToSingle(0x3FC90FDB);
        for (int cell = 0; cell < Mat4F.CellCount; cell++)
        {
            Assert.Equal(0f, variant.DeformSpeed!.Value[cell]);
            Assert.Equal(quarterTurn, variant.DeformOffset!.Value[cell]);
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_003_empty_bytes_decodes_to_the_values_of_section_5_4()
    {
        ProfileDocument document = NeutralAssets.LoadProfile("empty.bytes");
        GameProfile profile = document.Profile;

        Assert.Equal("2", profile.VersionText);
        Assert.Null(profile.Id);
        Assert.Empty(profile.PermanentPatterns);
        Assert.Empty(profile.TemporaryPatterns);

        ProfileScript script = Assert.Single(profile.Scripts);
        Assert.Equal("Build", script.Name);
        Assert.Equal("*", script.TagString);
        Assert.True(script.Enabled);
        Assert.StartsWith("function BuildPattern()", script.Source);

        ProfileSettings settings = profile.Settings!;
        Assert.Equal(new Vec3F(1f, 0.2f, 1f), settings.Light!.Direction);
        Assert.Equal(new ArgbColor(-1), settings.Light.Color);
        Assert.Equal(0.85911f, settings.Light.Intensity, 0.00001f);
        Assert.True(settings.UseNesBackgroundColor);
        Assert.Equal(new ArgbColor(-16777216), settings.CustomBackgroundColor);
        Assert.Equal(new Vec4F(4.1f, 16.1f, 4.1f, 0.1f), settings.Clipping);
        Assert.Equal(ProfileLayer.L1, settings.Layers!.DefaultLayer);
        Assert.False(settings.Layers.Auto);
        Assert.Equal(2, settings.GraphicQuality);
        Assert.Equal(120, settings.FrameRate);
        Assert.Equal(1f, settings.Volume);
        Assert.Equal(GlobalMeshAlgorithm.Greedy, settings.MeshAlgorithm);
        Assert.Equal(0, settings.Skybox);
        Assert.Equal(2, settings.Core);
        Assert.True(settings.Mirroring);

        (float Offset, LayerAlignment Aligned, bool Cast, bool Recv)[] expectedLayers =
        [
            (0f, LayerAlignment.Front, false, false),
            (0f, LayerAlignment.Front, false, false),
            (0f, LayerAlignment.Back, true, true),
            (0f, LayerAlignment.Front, false, true),
            (-4f, LayerAlignment.Front, false, true),
            (-4f, LayerAlignment.Front, false, true),
        ];

        for (int i = 0; i < expectedLayers.Length; i++)
        {
            LayerSettings layer = settings.Layers.Layers[i];
            Assert.Equal(expectedLayers[i].Offset, layer.Offset);
            Assert.Equal(expectedLayers[i].Aligned, layer.Aligned);
            Assert.Equal(expectedLayers[i].Cast, layer.StoredCastShadow);
            Assert.Equal(expectedLayers[i].Recv, layer.StoredRecvShadow);
        }

        Assert.Equal(
            new[] { "Road", "BG", "Top", "Loop", "Body" },
            settings.SavedPatterns.Select(slot => slot.Name));
        Assert.Equal(
            new[] { "Deform1", "Deform2", "Deform3", "Deform4", "Deform5" },
            settings.SavedAnimPatterns.Select(slot => slot.Name));

        ProfilePattern slotPattern = settings.SavedPatterns[0].Pattern!;
        Assert.Equal(Vec2I.Unconstrained, slotPattern.ConstraintSize);
        Assert.Equal(new Vec2I(0, 0), slotPattern.TileSize);
        Assert.False(slotPattern.Permanent);
        Assert.Null(slotPattern.Grid);
        Assert.Single(slotPattern.Variants);
        Assert.True(slotPattern.SelfConnected);
        Assert.Null(slotPattern.Variants[0].Texture);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_040_the_settings_absent_from_empty_bytes_take_their_defaults()
    {
        ProfileSettings settings = NeutralAssets.LoadProfile("empty.bytes").Profile.Settings!;

        // empty.bytes stores fields 1 to 23 only, so fields 24 to 38 take the defaults of
        // REQ-PROF-040.
        Assert.False(settings.EmptyTile);
        Assert.False(settings.LocalCamera);
        Assert.Equal(1f, settings.SceneRatio);
        Assert.False(settings.EmulatedMenuButton);
        Assert.Equal(0, settings.RenderMethod);
        Assert.Equal(0, settings.ResolutionScaleIndex);
        Assert.True(settings.SkipFrame);
        Assert.True(settings.BackgroundMusic);
        Assert.Equal(0, settings.AntiAlias);
        Assert.False(settings.AutoTvMode);
        Assert.Equal(TvRegionMode.Ntsc, settings.TvMode);
        Assert.Equal(0, settings.TvBox);
        Assert.Equal(0, settings.SortOption);
        Assert.False(settings.SameSpritePalette);
        Assert.Equal(70f, settings.Fov);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_040_the_field_of_view_is_absent_from_every_built_in_profile()
    {
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            ProfileDocument document = NeutralAssets.LoadProfile(Path.GetFileName(path));
            Assert.Equal(70f, document.Profile.Settings!.Fov);
        }
    }

    /// <summary>
    /// AMD-031 §3 — the TV box field of the embedded settings (field 35) is still decoded and
    /// written back as it was found, byte for byte, and no longer reaches any setting.
    /// </summary>
    [Theory] // AMD-031 §3, REQ-PROF-051
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(3)]
    public void AMD_031_the_tv_box_field_survives_a_round_trip_and_applies_nothing(int stored)
    {
        GameProfile written = GameProfile.CreateEmpty();
        written.Settings!.TvBox = stored;
        byte[] original = ProfileFormat.Save(written);

        // Field 35 is really on the wire: its tag, 35 << 3, is the two bytes 98 02.
        Assert.Contains($"98 02 {stored:x2}", Hex(original));

        ProfileLoadResult result = ProfileFormat.Load(original, sourceName: "tv-box");
        Assert.True(result.IsSuccess, result.Message);

        ProfileDocument document = result.Document!;
        Assert.Equal(0, document.UnknownFieldCount);
        Assert.Equal(stored, document.Profile.Settings!.TvBox);
        Assert.Equal(original, document.Save());

        // No effect: the field is in neither list of REQ-PROF-041 and reaches no setting key.
        Assert.DoesNotContain("scene.tv_box", ProfileSettingApplication.AppliedKeys);
        Assert.DoesNotContain("scene.tv_box", ProfileSettingApplication.PreservedKeys);
        Assert.DoesNotContain(
            "scene.tv_box",
            ProfileSettingApplication.ValuesToApply(document.Profile.Settings!).Keys);
    }

    private static string Hex(ReadOnlySpan<byte> bytes)
    {
        char[] text = new char[(bytes.Length * 3) - (bytes.Length > 0 ? 1 : 0)];
        const string digits = "0123456789abcdef";
        for (int i = 0; i < bytes.Length; i++)
        {
            int at = i * 3;
            text[at] = digits[bytes[i] >> 4];
            text[at + 1] = digits[bytes[i] & 0xF];
            if (at + 2 < text.Length)
            {
                text[at + 2] = ' ';
            }
        }

        return new string(text);
    }
}
