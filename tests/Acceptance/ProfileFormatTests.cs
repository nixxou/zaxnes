using Zax.Profiles;
using Zax.Profiles.Wire;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-03: wire format, shared references, tiles, patterns,
/// settings, lossless rewriting, and the global counts of §5.5.
/// </summary>
public sealed class ProfileFormatTests
{
    /// <summary>The 101 embedded profiles of <c>assets-neutral/profiles</c>.</summary>
    public static TheoryData<string> AllProfiles
    {
        get
        {
            TheoryData<string> data = [];
            foreach (string path in NeutralAssets.ProfilePaths)
            {
                data.Add(Path.GetFileName(path));
            }

            return data;
        }
    }

    private static GameProfile Load(string file)
        => ProfileFormat.Load(NeutralAssets.ReadProfile(file), sourceName: file).Profile!;

    private static IEnumerable<(string File, GameProfile Profile)> Corpus()
    {
        foreach (string path in NeutralAssets.ProfilePaths)
        {
            ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
            Assert.True(result.IsSuccess, $"{Path.GetFileName(path)}: {result.Message}");
            yield return (Path.GetFileName(path), result.Profile!);
        }
    }

    // ------------------------------------------------------------------ 3.1

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-001
    public void The_corpus_has_101_profiles()
        => Assert.Equal(101, NeutralAssets.ProfilePaths.Count);

    [Trait("Requires", "OriginalAssets")]
    [CorpusTheory] // REQ-PROF-001
    [MemberData(nameof(AllProfiles))]
    public void Every_embedded_profile_decodes_without_unknown_fields(string file)
    {
        ProfileLoadResult result = ProfileFormat.Load(NeutralAssets.ReadProfile(file), sourceName: file);
        Assert.True(result.IsSuccess, $"{file}: {result.Message}");
        Assert.Equal(0, result.Document!.UnknownFieldCount);

        // Cross-checked with the verifier's own wire scanner.
        ProfileWireScanner scanner = ProfileWireScanner.Scan(NeutralAssets.ReadProfile(file));
        Assert.Equal(0, scanner.UnknownRootFields);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-001 (an unknown field number is ignored, the model is unchanged)
    public void An_unknown_field_is_ignored()
    {
        byte[] original = NeutralAssets.ReadProfile("empty.bytes");
        GameProfile plain = ProfileFormat.Load(original).Profile!;

        // Field 99, wire type 0 (varint), value 7, appended to the root message.
        byte[] extended = [.. original, 0x98, 0x06, 0x07];
        ProfileLoadResult result = ProfileFormat.Load(extended);
        Assert.True(result.IsSuccess, result.Message);
        Assert.Equal(1, result.Document!.UnknownFieldCount);

        GameProfile extendedProfile = result.Profile!;
        Assert.Equal(plain.Scripts.Count, extendedProfile.Scripts.Count);
        Assert.Equal(plain.PermanentPatterns.Count, extendedProfile.PermanentPatterns.Count);
        Assert.Equal(plain.VersionText, extendedProfile.VersionText);
    }

    [Fact] // REQ-PROF-002
    public void Scalar_wire_encodings()
    {
        // -1 as an int32 is sign-extended to 64 bits: ten bytes, no zigzag.
        byte[] negative = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
        Assert.Equal(-1L, DecodeSignedVarint(negative));

        // 1.0f is I32, little-endian IEEE-754.
        Assert.Equal([0x00, 0x00, 0x80, 0x3F], BitConverter.GetBytes(1.0f));
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-003
    public void Absent_scalars_take_their_documented_defaults()
    {
        GameProfile profile = Load("empty.bytes");
        ProfileSettings settings = profile.Settings!;

        Assert.False(settings.EmptyTile);
        Assert.Equal(1f, settings.SceneRatio);
        Assert.Equal(70f, settings.Fov);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusTheory] // REQ-PROF-004, REQ-PROF-050, REQ-PROF-051
    [MemberData(nameof(AllProfiles))]
    public void Decode_then_re_encode_is_byte_identical(string file)
    {
        byte[] original = NeutralAssets.ReadProfile(file);
        ProfileDocument document = ProfileFormat.Load(original, sourceName: file).Document!;
        byte[] rewritten = document.Save();

        Assert.True(original.AsSpan().SequenceEqual(rewritten), $"{file}: re-encoding differs");
    }

    // ------------------------------------------------------------------ 3.2

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-010 (the reference envelope of the criterion)
    public void The_first_pattern_of_balloon_fight_is_a_new_object_of_key_one()
    {
        byte[] bytes = NeutralAssets.ReadProfile("BalloonFight.bytes");
        int at = FindRootField(bytes, fieldNumber: 4);
        Assert.True(at >= 0, "the root field 4 was not found");

        (ulong length, int after) = ReadVarint(bytes, at);
        Assert.Equal(611UL, length);
        Assert.Equal("100152DE04", Convert.ToHexString(bytes.AsSpan(after, 5).ToArray()));
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusTheory] // REQ-PROF-011 (every new key is the next one in sequence)
    [MemberData(nameof(AllProfiles))]
    public void Keys_are_strictly_sequential(string file)
    {
        ProfileWireScanner scanner = ProfileWireScanner.Scan(NeutralAssets.ReadProfile(file));
        Assert.True(scanner.KeysAreSequential, $"{file}: a key is out of sequence");
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-011 (a reference resolves to the same object, not to a copy)
    public void Shared_tiles_resolve_to_one_object()
    {
        GameProfile profile = Load("BalloonFight.bytes");
        Assert.Equal(516, profile.DistinctTileObjects.Count());
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-012, SPEC-03 §5.5
    public void Grid_cell_counts_match_the_tile_size_everywhere()
    {
        long nonEmpty = 0;
        long empty = 0;
        long gridded = 0;
        long consistent = 0;

        foreach (string path in NeutralAssets.ProfilePaths)
        {
            ProfileWireScanner scanner = ProfileWireScanner.Scan(File.ReadAllBytes(path));
            nonEmpty += scanner.NonEmptyCells;
            empty += scanner.EmptyCells;
            gridded += scanner.GriddedPatterns;
            consistent += scanner.ConsistentGrids;
        }

        Assert.Equal(452_231, nonEmpty);
        Assert.Equal(124_059, empty);
        Assert.Equal(67_832, gridded);
        Assert.Equal(gridded, consistent);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-013
    public void The_null_texture_marker_writes_field_10_first()
    {
        // 9a 01 = field 19, LEN; 08 = length 8; 52 02 10 00 = field 10 (TexPayload) with
        // format 0; then 08 00 (w = 0) and 10 00 (h = 0).
        byte[] expected = [0x9A, 0x01, 0x08, 0x52, 0x02, 0x10, 0x00, 0x08, 0x00, 0x10, 0x00];

        byte[] bytes = NeutralAssets.ReadProfile("BalloonFight.bytes");
        Assert.True(
            Contains(bytes, expected),
            "the canonical null-texture encoding was not found in BalloonFight.bytes");
    }

    // ------------------------------------------------------------------ 3.3

    [Trait("Requires", "OriginalAssets")]
    [CorpusTheory] // REQ-PROF-020
    [MemberData(nameof(AllProfiles))]
    public void Every_profile_opens_with_the_3dn_signature(string file)
    {
        byte[] bytes = NeutralAssets.ReadProfile(file);
        // 0a 03 33 64 6e = field 1 (LEN), length 3, "3dn".
        Assert.Equal("0A0333646E", Convert.ToHexString(bytes.AsSpan(0, 5).ToArray()));
        Assert.Equal("3dn", Load(file).Signature);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-020
    public void A_wrong_signature_is_refused()
    {
        byte[] wrong = NeutralAssets.ReadProfile("empty.bytes");
        wrong[2] = (byte)'3';
        wrong[3] = (byte)'d';
        wrong[4] = (byte)'x';
        Assert.False(ProfileFormat.Load(wrong).IsSuccess);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-021
    public void Version_classification()
    {
        Assert.Equal(ProfileFormatVersion.Unsupported, ProfileVersionRules.Classify("1"));
        Assert.Equal(ProfileFormatVersion.V2, ProfileVersionRules.Classify("2"));
        Assert.Equal(ProfileFormatVersion.Current, ProfileVersionRules.Classify("3"));
        Assert.Equal(ProfileFormatVersion.Current, ProfileVersionRules.Classify("2.0"));
        Assert.Equal(ProfileFormatVersion.Unsupported, ProfileVersionRules.Classify("v1"));

        int v2 = 0;
        List<string> others = [];
        foreach ((string file, GameProfile profile) in Corpus())
        {
            if (profile.IsV2Format)
            {
                v2++;
            }
            else
            {
                others.Add(file);
            }
        }

        Assert.Equal(100, v2);
        Assert.Equal(["Jackal (U).bytes"], others);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-022
    public void Metadata_counts_match_the_corpus()
    {
        int withId = 0;
        int withTitle = 0;
        int withMaker = 0;
        int withGameDev = 0;
        int withDescription = 0;


        foreach ((_, GameProfile profile) in Corpus())
        {
            withId += profile.Id is not null ? 1 : 0;
            withTitle += profile.Title is not null ? 1 : 0;
            withMaker += profile.Author is not null ? 1 : 0;
            withGameDev += profile.Publisher is not null ? 1 : 0;
            withDescription += profile.Description is not null ? 1 : 0;
        }

        Assert.Equal(56, withId);
        Assert.Equal(17, withTitle);
        Assert.Equal(17, withMaker);
        Assert.Equal(17, withGameDev);
        Assert.Equal(0, withDescription);

        Assert.Equal("Tennis", Load("Jackal (U).bytes").Title);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-023
    public void Pattern_lists_and_the_permanent_flag()
    {
        int permanentEntries = 0;
        int temporaryEntries = 0;

        foreach ((_, GameProfile profile) in Corpus())
        {
            permanentEntries += profile.PermanentPatterns.Count;
            temporaryEntries += profile.TemporaryPatterns.Count;
            Assert.All(profile.PermanentPatterns, p => Assert.True(p.Permanent));
        }

        Assert.Equal(66_609, permanentEntries);
        Assert.Equal(1_223, temporaryEntries);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-024
    public void Scripts_are_counted_and_sorted()
    {
        int profilesWithScripts = 0;
        int scripts = 0;

        foreach ((string file, GameProfile profile) in Corpus())
        {
            if (profile.Scripts.Count > 0)
            {
                profilesWithScripts++;
                scripts += profile.Scripts.Count;
            }

            for (int i = 1; i < profile.Scripts.Count; i++)
            {
                Assert.True(
                    ScriptNameComparer.Instance.Compare(profile.Scripts[i - 1].Name, profile.Scripts[i].Name) <= 0,
                    $"{file}: scripts are not sorted");
            }
        }

        Assert.Equal(89, profilesWithScripts);
        Assert.Equal(579, scripts);

        ProfileScript build = Assert.Single(Load("empty.bytes").Scripts);
        Assert.Equal("Build", build.Name);
        Assert.Equal("*", build.TagString);
        Assert.StartsWith("function BuildPattern()", build.Source, StringComparison.Ordinal);
    }

    [Fact] // REQ-PROF-025 (the five mandatory elements are present in all 101)
    public void Mandatory_settings_elements_are_present()
    {
        foreach ((string file, GameProfile profile) in Corpus())
        {
            ProfileSettings settings = profile.Settings!;
            Assert.True(settings.Light is not null, file);
            Assert.True(settings.Clipping is not null, file);
            Assert.True(settings.Layers is not null, file);
            Assert.Equal(5, settings.SavedPatterns.Count);
            Assert.Equal(5, settings.SavedAnimPatterns.Count);
        }
    }

    // ------------------------------------------------------------------ 3.4

    [Theory] // REQ-PROF-030, SPEC-03 §5.2
    [InlineData(0x0000000000000000UL, 7066876241827217075L)]
    [InlineData(0xFFFFFFFFFFFFFFFFUL, -7066876241827217076L)]
    [InlineData(0x0000000000000001UL, 7066876516705124019L)]
    [InlineData(0x8000000000000000UL, 7066876379266170547L)]
    [InlineData(0x00000000000000FFUL, 7066896857670237875L)]
    [InlineData(0x0102040810204080UL, 7638868579940777143L)]
    [InlineData(0x071F3F7F7FFFFFFFUL, -7066876509057851028L)]
    [InlineData(0xC0F0F8FCFCFEFEFEUL, 6750096703673336204L)]
    [InlineData(0xC3818100008181C3UL, 4788142726344507059L)]
    public void Plane_cipher_vectors(ulong plain, long stored)
    {
        Assert.Equal(stored, TilePlaneCipher.Encode(plain));
        Assert.Equal(plain, TilePlaneCipher.Decode(stored));
    }

    [Fact] // REQ-PROF-030 (key, rotation, bijection)
    public void The_cipher_key_rotation_and_bijection()
    {
        Assert.Equal(0x000708FACD884A5AUL, TilePlaneCipher.Key);
        Assert.Equal(26, TilePlaneCipher.Rotation);

        Random random = new(7);
        for (int i = 0; i < 1000; i++)
        {
            ulong plane = ((ulong)(uint)random.Next() << 32) | (uint)random.Next();
            Assert.Equal(plane, TilePlaneCipher.Decode(TilePlaneCipher.Encode(plane)));
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-031, SPEC-03 §5.3
    public void Balloon_fight_tile_key_twelve_draws_the_expected_picture()
    {
        GameProfile profile = Load("BalloonFight.bytes");
        ProfilePattern pattern = profile.PermanentPatterns[1];
        ProfileTile tile = pattern.Grid!.Cells[0]!;

        Assert.Equal(0x0020201800000000UL, tile.LowPlane);
        Assert.Equal(0x103A3C1800000000UL, tile.HighPlane);
        Assert.Equal(TileFlip.None, tile.Flip);

        string[] expected =
        [
            "00000000", "00000000", "00000000", "00000000",
            "00033000", "00322200", "00322020", "00020000",
        ];

        for (int y = 0; y < 8; y++)
        {
            string row = string.Concat(Enumerable.Range(0, 8).Select(x => tile.ColorAt(x, y).ToString()));
            Assert.Equal(expected[y], row);
        }
    }

    [Fact] // REQ-PROF-031 (the flip bits)
    public void Flip_bits_select_the_displayed_pixel()
    {
        Assert.Equal(1, new ProfileTile(0x0000000000000080UL, 0, TileFlip.None).ColorAt(0, 0));
        Assert.Equal(1, new ProfileTile(0x0000000000000080UL, 0, TileFlip.Horizontal).ColorAt(7, 0));
        Assert.Equal(1, new ProfileTile(0x0000000000000080UL, 0, TileFlip.Vertical).ColorAt(0, 7));
        Assert.Equal(2, new ProfileTile(0, 0x0000000000000080UL, TileFlip.None).ColorAt(0, 0));
        Assert.Equal(3, new ProfileTile(0x80UL, 0x80UL, TileFlip.None).ColorAt(0, 0));
    }

    [Fact] // REQ-PROF-032
    public void Tile_identity_is_planes_plus_flip()
    {
        ProfileTile a = new(1, 2, TileFlip.Horizontal);
        ProfileTile b = new(1, 2, TileFlip.Horizontal);
        ProfileTile c = new(1, 2, TileFlip.Vertical);

        Assert.Equal(a, b);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
        Assert.NotEqual(a, c);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-033
    public void A_truncated_profile_loads_as_an_empty_profile()
    {
        byte[] bytes = NeutralAssets.ReadProfile("Super Mario Bros.bytes");
        ProfileLoadResult result = ProfileFormat.Load(bytes.AsSpan(0, bytes.Length / 2));

        Assert.False(result.IsSuccess);
        Assert.NotNull(result.Message);

        GameProfile fallback = result.ProfileOrEmpty();
        Assert.Empty(fallback.PermanentPatterns);
        Assert.Empty(fallback.TemporaryPatterns);
        Assert.Empty(fallback.Scripts);
    }

    [Fact] // REQ-PROF-033 (random bytes do not crash either)
    public void Random_bytes_load_as_an_empty_profile()
    {
        Random random = new(99);
        for (int attempt = 0; attempt < 50; attempt++)
        {
            byte[] noise = new byte[512];
            random.NextBytes(noise);
            ProfileLoadResult result = ProfileFormat.Load(noise);
            Assert.Empty(result.ProfileOrEmpty().PermanentPatterns);
        }
    }

    // ------------------------------------------------------------------ 3.5

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-034
    public void Pattern_counts_and_variant_distribution()
    {
        int patterns = 0;
        int slotPatterns = 0;
        Dictionary<int, int> variants = [];

        foreach ((_, GameProfile profile) in Corpus())
        {
            foreach (ProfilePattern pattern in profile.PermanentPatterns.Concat(profile.TemporaryPatterns))
            {
                patterns++;
                Assert.NotEmpty(pattern.Variants);
                int key = Math.Min(pattern.Variants.Count, 4);
                variants[key] = variants.GetValueOrDefault(key) + 1;
            }

            slotPatterns += profile.Settings!.SavedPatterns.Count(s => s.Pattern is not null)
                + profile.Settings.SavedAnimPatterns.Count(s => s.Pattern is not null);
        }

        Assert.Equal(67_832, patterns);
        Assert.Equal(1_010, slotPatterns);
        Assert.Equal(68_842, patterns + slotPatterns);

        Assert.Equal(62_650, variants[1]);
        Assert.Equal(4_142, variants[2]);
        Assert.Equal(800, variants[3]);
        Assert.Equal(240, variants[4]);
    }

    [Fact] // REQ-PROF-034 (the defaults of absent fields)
    public void Pattern_defaults()
    {
        ProfilePattern pattern = new();
        Assert.Equal(new Vec2I(99, 99), pattern.ConstraintSize);
        Assert.True(pattern.SelfConnected);
        Assert.False(pattern.SkipGeometry);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-035
    public void Texture_counts_and_payload_lengths()
    {
        int total = 0;
        int nullMarkers = 0;
        int indexed = 0;
        int direct = 0;

        foreach ((string file, GameProfile profile) in Corpus())
        {
            foreach (ProfilePattern pattern in profile.PermanentPatterns.Concat(profile.TemporaryPatterns))
            {
                foreach (PatternVariant variant in pattern.Variants)
                {
                    if (variant.Texture is not { } texture)
                    {
                        continue;
                    }

                    total++;
                    if (texture.IsNullMarker)
                    {
                        nullMarkers++;
                        continue;
                    }

                    Assert.True(texture.HasConsistentLength(), $"{file}: inconsistent texture payload");
                    if (texture.Pixels!.Format == TextureFormat.Indexed)
                    {
                        indexed++;
                    }
                    else
                    {
                        direct++;
                    }
                }
            }
        }

        Assert.Equal(74_389, total);
        Assert.Equal(60_708, nullMarkers);
        Assert.Equal(13_539, indexed);
        Assert.Equal(142, direct);
    }

    [Fact] // REQ-PROF-036
    public void Tag_parsing_trims_and_drops_empties()
    {
        Assert.Equal(["char", "!cast"], ProfileTags.Parse(" char, ,!cast "));
        Assert.Empty(ProfileTags.Parse(null));
        Assert.Empty(ProfileTags.Parse("  ,  "));
        Assert.True(ProfileTags.IsUniversal("*"));
        Assert.Contains("!cast", ProfileTags.SpecialVariantTags);
        Assert.Contains("!receiv", ProfileTags.SpecialVariantTags);
        Assert.Contains("!recv", ProfileTags.SpecialVariantTags);
        Assert.Contains("!static", ProfileTags.SpecialVariantTags);
        Assert.Contains("_gaxis", ProfileTags.SpecialVariantTags);
    }

    [Fact] // REQ-PROF-037
    public void A_negative_empty_colour_is_brought_back_into_range()
    {
        PatternVariant variant = new() { StoredEmptyColor = -1 };
        Assert.Equal(4, variant.EmptyColor);

        foreach ((_, GameProfile profile) in Corpus())
        {
            foreach (ProfilePattern pattern in profile.PermanentPatterns.Concat(profile.TemporaryPatterns))
            {
                Assert.All(pattern.Variants, v => Assert.True(v.StoredEmptyColor >= 0));
            }
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-038
    public void Script_fields_match_the_corpus()
    {
        int enabled = 0;
        int disabled = 0;

        foreach ((_, GameProfile profile) in Corpus())
        {
            foreach (ProfileScript script in profile.Scripts)
            {
                Assert.False(script.StoredErrorFlag);
                if (script.Enabled)
                {
                    enabled++;
                }
                else
                {
                    disabled++;
                }
            }
        }

        Assert.Equal(556, enabled);
        Assert.Equal(23, disabled);
    }

    // ------------------------------------------------------------------ 3.7

    [Fact] // REQ-PROF-040
    public void Settings_defaults()
    {
        ProfileSettings settings = new();
        Assert.Equal(70f, settings.Fov);
        Assert.Equal(-1, settings.Skybox);
        Assert.True(settings.Mirroring);
        Assert.Equal(1f, settings.SceneRatio);
        Assert.True(settings.SkipFrame);
        Assert.True(settings.BackgroundMusic);

        LayerSettings layer = new();
        Assert.False(layer.StoredCastShadow);
        Assert.False(layer.StoredRecvShadow);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-042
    public void Layer_shadow_flags_are_rewritten_on_load()
    {
        int storedCast = 0;
        int layerCount = 0;

        foreach ((string file, GameProfile profile) in Corpus())
        {
            IList<LayerSettings> layers = profile.Settings!.Layers!.Layers;
            Assert.Equal(6, layers.Count);
            layerCount += layers.Count;

            for (int i = 0; i < 6; i++)
            {
                storedCast += layers[i].StoredCastShadow ? 1 : 0;
                Assert.Equal(i == 2, LayerSettingsSet.EffectiveCastShadow(i));
                Assert.Equal(i >= 2, LayerSettingsSet.EffectiveRecvShadow(i));
            }

            Assert.Equal(1, Enumerable.Range(0, 6).Count(LayerSettingsSet.EffectiveCastShadow));
        }

        Assert.Equal(606, layerCount);
        Assert.Equal(119, storedCast);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-044, SPEC-03 §5.4
    public void Recorded_slots_of_the_empty_profile()
    {
        GameProfile profile = Load("empty.bytes");
        Assert.Equal(
            ["Road", "BG", "Top", "Loop", "Body"],
            profile.Settings!.SavedPatterns.Select(s => s.Name ?? string.Empty).ToArray());
        Assert.Equal(
            ["Deform1", "Deform2", "Deform3", "Deform4", "Deform5"],
            profile.Settings.SavedAnimPatterns.Select(s => s.Name ?? string.Empty).ToArray());

        ProfilePattern first = profile.Settings.SavedPatterns[0].Pattern!;
        Assert.Equal(new Vec2I(99, 99), first.ConstraintSize);
        Assert.Equal(new Vec2I(0, 0), first.TileSize);
        Assert.False(first.Permanent);
        Assert.Single(first.Variants);
        Assert.True(first.SelfConnected);

        Assert.Equal(
            ["Config1", "Config2", "Config3", "Config4", "Config5"],
            RecordedPatternSlot.DefaultShapeSlots().Select(s => s.Name ?? string.Empty).ToArray());
        Assert.Equal(
            ["Deform1", "Deform2", "Deform3", "Deform4", "Deform5"],
            RecordedPatternSlot.DefaultDeformSlots().Select(s => s.Name ?? string.Empty).ToArray());
    }

    // ------------------------------------------------------------------ §5.3, §5.4, §5.5, §5.6

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // SPEC-03 §5.3 (the decoded extract of BalloonFight)
    public void Balloon_fight_extract()
    {
        GameProfile profile = Load("BalloonFight.bytes");
        Assert.Equal("3dn", profile.Signature);
        Assert.Equal("2", profile.VersionText);
        Assert.Equal("Lc89vRjlYrUL", profile.Id);
        Assert.Null(profile.Title);
        Assert.Empty(profile.Scripts);
        Assert.Equal(237, profile.PermanentPatterns.Count);
        Assert.Equal(2, profile.TemporaryPatterns.Count);
        Assert.Equal(
            296,
            profile.PermanentPatterns.Concat(profile.TemporaryPatterns).Sum(p => p.Variants.Count));

        ProfilePattern first = profile.PermanentPatterns[0];
        Assert.Equal(new Vec2I(8, 1), first.TileSize);
        Assert.Equal(8, first.Grid!.FilledCellCount);

        ProfilePattern second = profile.PermanentPatterns[1];
        Assert.Equal(new Vec2I(99, 99), second.ConstraintSize);
        Assert.Equal(new Vec2I(1, 1), second.TileSize);
        Assert.True(second.Permanent);
        Assert.Equal(0, second.PixelCount);
        Assert.Equal(12, (int)second.ColorFlags);
        Assert.Equal(5, (int)second.TileDensity);
        Assert.Equal(1, (int)second.PixelDensity);
        Assert.False(second.SelfConnected);
        Assert.False(second.SkipGeometry);

        PatternVariant variant = Assert.Single(second.Variants);
        Assert.Equal(3, variant.Depth);
        Assert.Equal(VariantGeometry.Default, variant.Geometry);
        Assert.Equal(ProfileLayer.Unidentified, variant.Layer);
        Assert.Equal(new Vec3F(1, 1, 1), variant.Scale);
        Assert.Equal(new Vec3F(0, 0, 0), variant.Rotation);
        Assert.Equal(new Vec3F(0, 0, 0), variant.Offset);
        Assert.Equal(new Vec3F(0.5f, 0.5f, 0.5f), variant.Pivot);
        Assert.Equal(1f, variant.Alpha);
        Assert.Equal(MeshAlgorithm.FromSettings, variant.MeshAlgorithm);
        Assert.Equal(new Vec3F(0, 0, 0), variant.DeformAmplitude);
        Assert.Equal(new Vec3F(0.5f, 0.5f, 0.5f), variant.DeformPivot);
        Assert.Equal(VariantRenderStyle.Normal, variant.RenderStyle);
        Assert.True(variant.Texture!.IsNullMarker);
        Assert.Equal(string.Empty, variant.TagString);
        Assert.Equal(0, variant.StoredEmptyColor);

        // deltaSpeed = sixteen zeros; deltaOffset = sixteen times pi/2.
        Mat4F speed = variant.DeformSpeed!.Value;
        Mat4F offset = variant.DeformOffset!.Value;
        for (int i = 0; i < Mat4F.CellCount; i++)
        {
            Assert.Equal(0f, speed[i]);
            Assert.Equal(BitConverter.UInt32BitsToSingle(0x3FC90FDB), offset[i]);
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // SPEC-03 §5.4 (the decoded extract of empty.bytes)
    public void Empty_profile_extract()
    {
        GameProfile profile = Load("empty.bytes");
        Assert.Equal("2", profile.VersionText);
        Assert.Null(profile.Id);
        Assert.Single(profile.Scripts);
        Assert.Empty(profile.PermanentPatterns);

        ProfileSettings settings = profile.Settings!;
        Assert.Equal(new Vec3F(1f, 0.2f, 1f), settings.Light!.Direction);
        Assert.Equal(-1, settings.Light.Color!.Value.Packed);
        Assert.Equal(0.85911f, settings.Light.Intensity, 4);
        Assert.True(settings.UseNesBackgroundColor);
        Assert.Equal(-16777216, settings.CustomBackgroundColor!.Value.Packed);
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

        float[] offsets = [0, 0, 0, 0, -4, -4];
        LayerAlignment[] alignments =
        [
            LayerAlignment.Front, LayerAlignment.Front, LayerAlignment.Back,
            LayerAlignment.Front, LayerAlignment.Front, LayerAlignment.Front,
        ];

        for (int i = 0; i < 6; i++)
        {
            Assert.Equal(offsets[i], settings.Layers.Layers[i].Offset);
            Assert.Equal(alignments[i], settings.Layers.Layers[i].Aligned);
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusTheory] // SPEC-03 §5.6
    [InlineData("empty.bytes", 3_780, 1, 0, 0, 0, 0)]
    [InlineData("Puzznic (U).bytes", 44_334, 0, 74, 0, 75, 311)]
    [InlineData("flappy_fpv.bytes", 66_849, 11, 67, 0, 77, 149)]
    [InlineData("Super Mario Bros.bytes", 179_515, 10, 257, 1, 296, 783)]
    [InlineData("Contra.bytes", 1_120_081, 3, 1_854, 65, 1_966, 5_365)]
    public void Small_profile_table(
        string file, int size, int scripts, int permanent, int temporary, int variants, int tiles)
    {
        byte[] bytes = NeutralAssets.ReadProfile(file);
        Assert.Equal(size, bytes.Length);

        GameProfile profile = ProfileFormat.Load(bytes, sourceName: file).Profile!;
        Assert.Equal(scripts, profile.Scripts.Count);
        Assert.Equal(permanent, profile.PermanentPatterns.Count);
        Assert.Equal(temporary, profile.TemporaryPatterns.Count);
        Assert.Equal(
            variants,
            profile.PermanentPatterns.Concat(profile.TemporaryPatterns).Sum(p => p.Variants.Count));
        Assert.Equal(tiles, profile.DistinctTileObjects.Count());
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // SPEC-03 §5.5 (new tiles and references to tiles already seen)
    public void Tile_sharing_totals()
    {
        long newTiles = 0;
        long references = 0;

        foreach (string path in NeutralAssets.ProfilePaths)
        {
            ProfileWireScanner scanner = ProfileWireScanner.Scan(File.ReadAllBytes(path));
            newTiles += scanner.NewObjects.GetValueOrDefault("tile");
            references += scanner.References.GetValueOrDefault("tile");
        }

        Assert.Equal(225_575, newTiles);
        Assert.Equal(226_656, references);
    }

    // ------------------------------------------------------------------ helpers

    private static long DecodeSignedVarint(byte[] bytes)
    {
        ulong value = 0;
        int shift = 0;
        foreach (byte b in bytes)
        {
            value |= (ulong)(b & 0x7F) << shift;
            shift += 7;
        }

        return (long)value;
    }

    private static bool Contains(byte[] haystack, byte[] needle)
    {
        for (int i = 0; i + needle.Length <= haystack.Length; i++)
        {
            if (haystack.AsSpan(i, needle.Length).SequenceEqual(needle))
            {
                return true;
            }
        }

        return false;
    }

    private static (ulong Value, int Next) ReadVarint(byte[] bytes, int at)
    {
        ulong value = 0;
        int shift = 0;
        while (true)
        {
            byte b = bytes[at++];
            value |= (ulong)(b & 0x7F) << shift;
            if ((b & 0x80) == 0)
            {
                return (value, at);
            }

            shift += 7;
        }
    }

    /// <summary>Index of the payload length of the first element of a root field.</summary>
    private static int FindRootField(byte[] bytes, int fieldNumber)
    {
        int at = 0;
        while (at < bytes.Length)
        {
            (ulong tag, int next) = ReadVarint(bytes, at);
            int number = (int)(tag >> 3);
            int wireType = (int)(tag & 7);
            if (number == fieldNumber && wireType == 2)
            {
                return next;
            }

            if (wireType == 2)
            {
                (ulong length, int after) = ReadVarint(bytes, next);
                at = after + (int)length;
            }
            else if (wireType == 0)
            {
                (_, at) = ReadVarint(bytes, next);
            }
            else if (wireType == 5)
            {
                at = next + 4;
            }
            else
            {
                return -1;
            }
        }

        return -1;
    }
}
